use std::sync::Arc;

use async_trait::async_trait;
use serde::Serialize;

use crate::application::SettingsConfiguration;
use crate::Result;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemTtsVoice {
    pub id: String,
    pub name: String,
    pub locale: String,
}

#[async_trait]
pub trait SystemTtsHost: Send + Sync {
    async fn list_voices(&self) -> Result<Vec<SystemTtsVoice>>;
    async fn speak(&self, text: &str, voice_id: &str, words_per_minute: u16) -> Result<()>;
}

pub struct TtsRuntime {
    host: Arc<dyn SystemTtsHost>,
    settings: Arc<SettingsConfiguration>,
}

impl TtsRuntime {
    pub fn new(host: Arc<dyn SystemTtsHost>, settings: Arc<SettingsConfiguration>) -> Self {
        Self { host, settings }
    }

    pub async fn list_voices(&self) -> Result<Vec<SystemTtsVoice>> {
        self.host.list_voices().await
    }

    pub async fn speak(&self, text: &str, language: Option<&str>) -> Result<()> {
        let normalized_text = text.trim();
        if normalized_text.is_empty() {
            return Ok(());
        }

        let general = self.settings.snapshot()?.general;
        let voice_id = if general.system_tts_voice.is_empty() {
            match language {
                Some(language) => {
                    select_voice_for_language(&self.host.list_voices().await?, language)
                        .unwrap_or_default()
                }
                None => String::new(),
            }
        } else {
            general.system_tts_voice
        };
        self.host
            .speak(normalized_text, &voice_id, general.system_tts_rate)
            .await
    }
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
        .map(|voice| voice.id.clone())
}

fn normalize_locale(locale: &str) -> String {
    locale.trim().replace('_', "-").to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;

    use super::{select_voice_for_language, SystemTtsHost, SystemTtsVoice, TtsRuntime};
    use crate::application::SettingsConfiguration;
    use crate::domain::GeneralSettings;
    use crate::infrastructure::storage::SqliteConfigStore;
    use crate::Result;

    type SpeechCall = (String, String, u16);

    #[derive(Default)]
    struct Host {
        calls: Mutex<Vec<SpeechCall>>,
    }

    #[async_trait]
    impl SystemTtsHost for Host {
        async fn list_voices(&self) -> Result<Vec<SystemTtsVoice>> {
            Ok(vec![SystemTtsVoice {
                id: "voice-zh-cn".to_string(),
                name: "Tingting".to_string(),
                locale: "zh_CN".to_string(),
            }])
        }

        async fn speak(&self, text: &str, voice_id: &str, words_per_minute: u16) -> Result<()> {
            self.calls.lock().unwrap().push((
                text.to_string(),
                voice_id.to_string(),
                words_per_minute,
            ));
            Ok(())
        }
    }

    #[tokio::test]
    async fn uses_persisted_voice_and_rate_for_speech() {
        let settings = Arc::new(SettingsConfiguration::new(Arc::new(
            SqliteConfigStore::new_in_memory(),
        )));
        settings
            .update_general(GeneralSettings {
                system_tts_voice: "Tingting".to_string(),
                system_tts_rate: 220,
                ..GeneralSettings::default()
            })
            .unwrap();
        let host = Arc::new(Host::default());
        let runtime = TtsRuntime::new(host.clone(), settings);

        runtime.speak("  你好  ", Some("zh-CN")).await.unwrap();

        assert_eq!(
            *host.calls.lock().unwrap(),
            vec![("你好".to_string(), "Tingting".to_string(), 220)]
        );
    }

    #[tokio::test]
    async fn selects_voice_id_for_the_requested_language() {
        let settings = Arc::new(SettingsConfiguration::new(Arc::new(
            SqliteConfigStore::new_in_memory(),
        )));
        let host = Arc::new(Host::default());
        let runtime = TtsRuntime::new(host.clone(), settings);

        runtime.speak("hello", Some("zh-CN")).await.unwrap();

        assert_eq!(
            *host.calls.lock().unwrap(),
            vec![("hello".to_string(), "voice-zh-cn".to_string(), 180,)]
        );
    }

    #[test]
    fn prefers_exact_locale_before_base_language_fallback() {
        let voices = vec![
            SystemTtsVoice {
                id: "zh-tw-id".to_string(),
                name: "Chinese fallback".to_string(),
                locale: "zh_TW".to_string(),
            },
            SystemTtsVoice {
                id: "zh-cn-id".to_string(),
                name: "Chinese mainland".to_string(),
                locale: "zh_CN".to_string(),
            },
        ];

        assert_eq!(
            select_voice_for_language(&voices, "zh-CN"),
            Some("zh-cn-id".to_string())
        );
        assert_eq!(
            select_voice_for_language(&voices, "zh"),
            Some("zh-tw-id".to_string())
        );
    }
}
