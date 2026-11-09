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
            log::warn!("Failed to join macOS speech process: {}", error);
        }
    }
}

pub(crate) struct MacOsSystemTtsHost {
    speech: Mutex<Option<ActiveSpeech>>,
    voices: OnceCell<Vec<SystemTtsVoice>>,
}

impl MacOsSystemTtsHost {
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
impl SystemTtsHost for MacOsSystemTtsHost {
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
        result = child.wait() => {
            match result {
                Ok(status) if !status.success() => {
                    log::warn!("macOS speech process exited with {}", status);
                }
                Ok(_) => {}
                Err(error) => {
                    log::warn!("Failed to wait for macOS speech process: {}", error);
                }
            }
        }
        _ = stop => {
            if let Err(error) = child.start_kill() {
                log::debug!("Failed to stop macOS speech process: {}", error);
            }
            if let Err(error) = child.wait().await {
                log::warn!("Failed to reap stopped macOS speech process: {}", error);
            }
        }
    }
}

async fn list_system_tts_voices() -> Result<Vec<SystemTtsVoice>> {
    let output = tokio::process::Command::new("/usr/bin/say")
        .args(["-v", "?"])
        .output()
        .await
        .map_err(|error| AppError::System(format!("Failed to list macOS voices: {error}")))?;
    if !output.status.success() {
        return Err(AppError::System("Failed to list macOS voices".to_string()));
    }

    Ok(parse_macos_voice_list(&String::from_utf8_lossy(
        &output.stdout,
    )))
}

fn start_speech(
    text: &str,
    voice_id: &str,
    words_per_minute: u16,
) -> Result<tokio::process::Child> {
    let mut command = tokio::process::Command::new("/usr/bin/say");
    command.args(speech_arguments(text, voice_id, words_per_minute));
    command
        .spawn()
        .map_err(|error| AppError::System(format!("Failed to start macOS speech: {error}")))
}

fn speech_arguments(text: &str, voice_id: &str, words_per_minute: u16) -> Vec<String> {
    let mut arguments = vec!["-r".to_string(), words_per_minute.to_string()];
    if !voice_id.is_empty() {
        arguments.extend(["-v".to_string(), voice_id.to_string()]);
    }
    arguments.extend(["--".to_string(), text.to_string()]);
    arguments
}

fn parse_macos_voice_list(output: &str) -> Vec<SystemTtsVoice> {
    output.lines().filter_map(parse_macos_voice).collect()
}

fn parse_macos_voice(line: &str) -> Option<SystemTtsVoice> {
    let metadata = line.split('#').next()?.trim();
    let parts: Vec<_> = metadata.split_whitespace().collect();
    let locale_index = parts
        .iter()
        .rposition(|part| part.contains('_') || part.contains('-'))?;
    if locale_index == 0 {
        return None;
    }

    let name = parts[..locale_index].join(" ");
    Some(SystemTtsVoice {
        id: name.clone(),
        name,
        locale: parts[locale_index].to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::{parse_macos_voice_list, speech_arguments, SystemTtsVoice};

    #[test]
    fn parses_voice_ids_names_locales_and_ignores_samples() {
        assert_eq!(
            parse_macos_voice_list(
                "Alex                 en_US    # Hello.\n\
                 Flo (English (US))    en_US    # Hello.\n"
            ),
            vec![
                SystemTtsVoice {
                    id: "Alex".to_string(),
                    name: "Alex".to_string(),
                    locale: "en_US".to_string(),
                },
                SystemTtsVoice {
                    id: "Flo (English (US))".to_string(),
                    name: "Flo (English (US))".to_string(),
                    locale: "en_US".to_string(),
                },
            ]
        );
    }

    #[test]
    fn terminates_say_options_before_user_text() {
        assert_eq!(
            speech_arguments("- a bullet", "Alex", 180),
            vec!["-r", "180", "-v", "Alex", "--", "- a bullet"]
        );
    }
}
