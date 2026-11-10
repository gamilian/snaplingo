use async_trait::async_trait;
use tokio::sync::{oneshot, Mutex, OnceCell};

use crate::application::{SystemTtsHost, SystemTtsVoice};
use crate::{AppError, Result};

struct ActiveSpeech {
    stop: Option<oneshot::Sender<()>>,
    task: tokio::task::JoinHandle<()>,
}

impl ActiveSpeech {
    async fn stop(mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Err(error) = self.task.await {
            log::warn!("Failed to join Windows speech process: {error}");
        }
    }
}

pub(crate) struct WindowsSystemTtsHost {
    speech: Mutex<Option<ActiveSpeech>>,
    voices: OnceCell<Vec<SystemTtsVoice>>,
}

impl WindowsSystemTtsHost {
    pub(crate) fn new() -> Self {
        Self {
            speech: Mutex::new(None),
            voices: OnceCell::new(),
        }
    }

    async fn available_voices(&self) -> Result<&Vec<SystemTtsVoice>> {
        self.voices.get_or_try_init(list_system_tts_voices).await
    }
}

#[async_trait]
impl SystemTtsHost for WindowsSystemTtsHost {
    async fn list_voices(&self) -> Result<Vec<SystemTtsVoice>> {
        Ok(self.available_voices().await?.clone())
    }

    async fn speak(&self, text: &str, voice_id: &str, words_per_minute: u16) -> Result<()> {
        let mut speech = self.speech.lock().await;
        if let Some(previous) = speech.take() {
            previous.stop().await;
        }

        let child = start_speech(text, voice_id, words_per_minute)?;
        let (stop, stop_signal) = oneshot::channel();
        let task = tokio::spawn(supervise_speech(child, stop_signal));
        *speech = Some(ActiveSpeech {
            stop: Some(stop),
            task,
        });
        Ok(())
    }
}

async fn supervise_speech(mut child: tokio::process::Child, stop: oneshot::Receiver<()>) {
    tokio::select! {
        result = child.wait() => match result {
            Ok(status) if !status.success() => log::warn!("Windows speech process exited with {status}"),
            Ok(_) => {}
            Err(error) => log::warn!("Failed to wait for Windows speech process: {error}"),
        },
        _ = stop => {
            if let Err(error) = child.start_kill() {
                log::debug!("Failed to stop Windows speech process: {error}");
            }
            if let Err(error) = child.wait().await {
                log::warn!("Failed to reap stopped Windows speech process: {error}");
            }
        }
    }
}

async fn list_system_tts_voices() -> Result<Vec<SystemTtsVoice>> {
    let output = powershell_command(
        r#"Add-Type -AssemblyName System.Speech; $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; $s.GetInstalledVoices() | ForEach-Object { $v = $_.VoiceInfo; $v.Name, $v.Name, $v.Culture.Name -join "`t" }"#,
    )
    .output()
    .await
    .map_err(|error| AppError::System(format!("Failed to list Windows voices: {error}")))?;
    if !output.status.success() {
        return Err(AppError::System(format!(
            "Failed to list Windows voices: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }

    Ok(parse_windows_voice_list(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

fn start_speech(
    text: &str,
    voice_id: &str,
    words_per_minute: u16,
) -> Result<tokio::process::Child> {
    let mut command = powershell_command(
        r#"Add-Type -AssemblyName System.Speech; $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; if ($env:SNAPLINGO_TTS_VOICE) { $s.SelectVoice($env:SNAPLINGO_TTS_VOICE) }; $s.Rate = [int]$env:SNAPLINGO_TTS_RATE; $s.Speak($env:SNAPLINGO_TTS_TEXT)"#,
    );
    command
        .env("SNAPLINGO_TTS_TEXT", text)
        .env("SNAPLINGO_TTS_VOICE", voice_id)
        .env(
            "SNAPLINGO_TTS_RATE",
            windows_rate(words_per_minute).to_string(),
        )
        .spawn()
        .map_err(|error| AppError::System(format!("Failed to start Windows speech: {error}")))
}

fn powershell_command(script: &str) -> tokio::process::Command {
    let mut command = tokio::process::Command::new("powershell.exe");
    command.args([
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        script,
    ]);
    command
}

fn windows_rate(words_per_minute: u16) -> i32 {
    ((i32::from(words_per_minute) - 180) / 22).clamp(-10, 10)
}

fn parse_windows_voice_list(output: &str) -> Vec<SystemTtsVoice> {
    output
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('\t');
            let id = fields.next()?.trim();
            let name = fields.next()?.trim();
            let locale = fields.next()?.trim();
            (!id.is_empty() && !name.is_empty() && !locale.is_empty()).then(|| SystemTtsVoice {
                id: id.to_string(),
                name: name.to_string(),
                locale: locale.to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{parse_windows_voice_list, windows_rate, SystemTtsVoice};

    #[test]
    fn parses_installed_windows_voices() {
        assert_eq!(
            parse_windows_voice_list("Microsoft Zira Desktop\tMicrosoft Zira Desktop\ten-US\n"),
            vec![SystemTtsVoice {
                id: "Microsoft Zira Desktop".to_string(),
                name: "Microsoft Zira Desktop".to_string(),
                locale: "en-US".to_string(),
            }]
        );
    }

    #[test]
    fn maps_words_per_minute_to_windows_rate_range() {
        assert_eq!(windows_rate(80), -4);
        assert_eq!(windows_rate(180), 0);
        assert_eq!(windows_rate(400), 10);
    }
}
