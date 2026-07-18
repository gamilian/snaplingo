use std::sync::Arc;

use async_trait::async_trait;
use serde::Serialize;

use crate::application::SettingsConfiguration;
use crate::Result;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemTtsVoice {
    pub name: String,
    pub locale: String,
}

#[async_trait]
pub trait SystemTtsHost: Send + Sync {
    async fn list_voices(&self) -> Result<Vec<SystemTtsVoice>>;
    async fn speak(&self, text: &str, language: Option<&str>, voice: &str, rate: u16)
        -> Result<()>;
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
        self.host
            .speak(
                normalized_text,
                language,
                &general.system_tts_voice,
                general.system_tts_rate,
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;

    use super::{SystemTtsHost, SystemTtsVoice, TtsRuntime};
    use crate::application::SettingsConfiguration;
    use crate::domain::GeneralSettings;
    use crate::infrastructure::storage::SqliteConfigStore;
    use crate::Result;

    type SpeechCall = (String, Option<String>, String, u16);

    #[derive(Default)]
    struct Host {
        calls: Mutex<Vec<SpeechCall>>,
    }

    #[async_trait]
    impl SystemTtsHost for Host {
        async fn list_voices(&self) -> Result<Vec<SystemTtsVoice>> {
            Ok(vec![SystemTtsVoice {
                name: "Tingting".to_string(),
                locale: "zh_CN".to_string(),
            }])
        }

        async fn speak(
            &self,
            text: &str,
            language: Option<&str>,
            voice: &str,
            rate: u16,
        ) -> Result<()> {
            self.calls.lock().unwrap().push((
                text.to_string(),
                language.map(str::to_string),
                voice.to_string(),
                rate,
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
            vec![(
                "你好".to_string(),
                Some("zh-CN".to_string()),
                "Tingting".to_string(),
                220,
            )]
        );
    }
}
