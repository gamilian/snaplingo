use async_trait::async_trait;

use crate::application::{SystemTtsHost, SystemTtsVoice};
use crate::{AppError, Result};

pub(crate) struct UnavailableSystemTtsHost {
    platform: &'static str,
}

impl UnavailableSystemTtsHost {
    pub(crate) fn new(platform: &'static str) -> Self {
        Self { platform }
    }
}

#[async_trait]
impl SystemTtsHost for UnavailableSystemTtsHost {
    async fn list_voices(&self) -> Result<Vec<SystemTtsVoice>> {
        Err(self.unavailable_error())
    }

    async fn speak(&self, _text: &str, _voice_id: &str, _words_per_minute: u16) -> Result<()> {
        Err(self.unavailable_error())
    }
}

impl UnavailableSystemTtsHost {
    fn unavailable_error(&self) -> AppError {
        AppError::System(format!(
            "System TTS is not yet available on {}",
            self.platform
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{SystemTtsHost, UnavailableSystemTtsHost};

    #[tokio::test]
    async fn reports_platform_unavailability_for_all_operations() {
        let host = UnavailableSystemTtsHost::new("test-os");

        assert_eq!(
            host.list_voices().await.unwrap_err().to_string(),
            "System error: System TTS is not yet available on test-os"
        );
        assert_eq!(
            host.speak("hello", "", 180).await.unwrap_err().to_string(),
            "System error: System TTS is not yet available on test-os"
        );
    }
}
