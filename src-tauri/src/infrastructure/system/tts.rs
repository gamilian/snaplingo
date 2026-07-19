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

pub struct SystemTtsHostAdapter {
    speech: Mutex<Option<ActiveSpeech>>,
    voices: OnceCell<Vec<SystemTtsVoice>>,
}

impl Default for SystemTtsHostAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemTtsHostAdapter {
    pub fn new() -> Self {
        Self {
            speech: Mutex::new(None),
            voices: OnceCell::new(),
        }
    }
}

#[async_trait]
impl SystemTtsHost for SystemTtsHostAdapter {
    async fn list_voices(&self) -> Result<Vec<SystemTtsVoice>> {
        Ok(self.available_voices().await?.clone())
    }

    async fn speak(
        &self,
        text: &str,
        language: Option<&str>,
        voice: &str,
        rate: u16,
    ) -> Result<()> {
        let selected_voice = if !voice.trim().is_empty() {
            voice.trim().to_string()
        } else if let Some(language) = language {
            select_voice_for_language(self.available_voices().await?, language).unwrap_or_default()
        } else {
            String::new()
        };

        let mut speech = self.speech.lock().await;
        if let Some(previous) = speech.take() {
            previous.stop().await;
        }
        let child = start_speech(text, &selected_voice, rate)?;
        let (stop, stop_signal) = oneshot::channel();
        let task = tokio::spawn(supervise_speech(child, stop_signal));
        *speech = Some(ActiveSpeech {
            stop: Some(stop),
            task,
        });
        Ok(())
    }
}

impl SystemTtsHostAdapter {
    async fn available_voices(&self) -> Result<&Vec<SystemTtsVoice>> {
        self.voices
            .get_or_try_init(|| async { list_system_tts_voices().await })
            .await
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

#[cfg(target_os = "macos")]
async fn list_system_tts_voices() -> Result<Vec<SystemTtsVoice>> {
    let output = tokio::process::Command::new("say")
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

#[cfg(not(target_os = "macos"))]
async fn list_system_tts_voices() -> Result<Vec<SystemTtsVoice>> {
    Ok(Vec::new())
}

#[cfg(target_os = "macos")]
fn start_speech(text: &str, voice: &str, rate: u16) -> Result<tokio::process::Child> {
    let mut command = tokio::process::Command::new("/usr/bin/say");
    command.args(speech_arguments(text, voice, rate));
    command
        .spawn()
        .map_err(|error| AppError::System(format!("Failed to start macOS speech: {error}")))
}

#[cfg(not(target_os = "macos"))]
fn start_speech(_text: &str, _voice: &str, _rate: u16) -> Result<tokio::process::Child> {
    Err(AppError::System(
        "System TTS is currently available on macOS only".to_string(),
    ))
}

fn speech_arguments(text: &str, voice: &str, rate: u16) -> Vec<String> {
    let mut arguments = vec!["-r".to_string(), rate.to_string()];
    if !voice.is_empty() {
        arguments.extend(["-v".to_string(), voice.to_string()]);
    }
    arguments.extend(["--".to_string(), text.to_string()]);
    arguments
}

fn select_voice_for_language(voices: &[SystemTtsVoice], language: &str) -> Option<String> {
    let requested = normalize_locale(language);
    let requested_base = requested.split('-').next()?;

    voices
        .iter()
        .find(|voice| normalize_locale(&voice.locale) == requested)
        .or_else(|| {
            voices.iter().find(|voice| {
                normalize_locale(&voice.locale)
                    .split('-')
                    .next()
                    .is_some_and(|base| base == requested_base)
            })
        })
        .map(|voice| voice.name.clone())
}

fn normalize_locale(locale: &str) -> String {
    locale.trim().replace('_', "-").to_ascii_lowercase()
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

    Some(SystemTtsVoice {
        name: parts[..locale_index].join(" "),
        locale: parts[locale_index].to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        parse_macos_voice_list, select_voice_for_language, speech_arguments, SystemTtsVoice,
    };

    #[test]
    fn parses_voice_names_locales_and_ignores_samples() {
        assert_eq!(
            parse_macos_voice_list(
                "Alex                 en_US    # Hello.\n\
                 Flo (English (US))    en_US    # Hello.\n"
            ),
            vec![
                SystemTtsVoice {
                    name: "Alex".to_string(),
                    locale: "en_US".to_string(),
                },
                SystemTtsVoice {
                    name: "Flo (English (US))".to_string(),
                    locale: "en_US".to_string(),
                },
            ]
        );
    }

    #[test]
    fn selects_an_exact_voice_locale_before_a_language_fallback() {
        let voices = vec![
            SystemTtsVoice {
                name: "Chinese fallback".to_string(),
                locale: "zh_TW".to_string(),
            },
            SystemTtsVoice {
                name: "Chinese mainland".to_string(),
                locale: "zh_CN".to_string(),
            },
        ];

        assert_eq!(
            select_voice_for_language(&voices, "zh-CN"),
            Some("Chinese mainland".to_string())
        );
        assert_eq!(
            select_voice_for_language(&voices, "zh"),
            Some("Chinese fallback".to_string())
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
