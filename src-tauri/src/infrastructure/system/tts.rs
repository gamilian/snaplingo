use async_trait::async_trait;

use crate::application::{SystemTtsHost, SystemTtsVoice};
use crate::{AppError, Result};

pub struct SystemTtsHostAdapter {
    speech: tokio::sync::Mutex<Option<tokio::process::Child>>,
}

impl Default for SystemTtsHostAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl SystemTtsHostAdapter {
    pub fn new() -> Self {
        Self {
            speech: tokio::sync::Mutex::new(None),
        }
    }
}

#[async_trait]
impl SystemTtsHost for SystemTtsHostAdapter {
    async fn list_voices(&self) -> Result<Vec<SystemTtsVoice>> {
        list_system_tts_voices().await
    }

    async fn speak(
        &self,
        text: &str,
        language: Option<&str>,
        voice: &str,
        rate: u16,
    ) -> Result<()> {
        let mut speech = self.speech.lock().await;
        if let Some(mut previous) = speech.take() {
            let _ = previous.start_kill();
        }
        *speech = Some(start_speech(text, language, voice, rate)?);
        Ok(())
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
fn start_speech(
    text: &str,
    _language: Option<&str>,
    voice: &str,
    rate: u16,
) -> Result<tokio::process::Child> {
    let mut command = tokio::process::Command::new("say");
    command.args(["-r", &rate.to_string()]);
    if !voice.is_empty() {
        command.args(["-v", voice]);
    }
    command
        .arg(text)
        .spawn()
        .map_err(|error| AppError::System(format!("Failed to start macOS speech: {error}")))
}

#[cfg(not(target_os = "macos"))]
fn start_speech(
    _text: &str,
    _language: Option<&str>,
    _voice: &str,
    _rate: u16,
) -> Result<tokio::process::Child> {
    Err(AppError::System(
        "System TTS is currently available on macOS only".to_string(),
    ))
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
    use super::{parse_macos_voice_list, SystemTtsVoice};

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
}
