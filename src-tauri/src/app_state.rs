use std::sync::Arc;

use crate::application::providers::ocr::{OcrCoordinator, OcrProviderConfiguration};
use crate::application::providers::translation::TranslationCoordinator;
use crate::application::providers::{
    LlmIntrospection, ProviderConfiguration, TranslationPromptConfiguration,
};
use crate::application::result_window::ResultWindowRuntime;
use crate::application::{
    CaptureOutput, CaptureSessionRuntime, CaptureSessions, History, HotkeyRuntime,
    PinnedImageRuntime, SelectedTextAcquirer, SettingsConfiguration,
};
use crate::infrastructure::events::EventBus;
use crate::Result;

pub struct SettingsRuntime {
    pub configuration: Arc<SettingsConfiguration>,
    pub hotkeys: Arc<HotkeyRuntime>,
}

pub struct ProviderRuntime {
    pub translation: Arc<TranslationCoordinator>,
    pub ocr: Arc<OcrCoordinator>,
    pub ocr_configuration: Arc<OcrProviderConfiguration>,
    pub llm_introspection: Arc<LlmIntrospection>,
    pub configuration: Arc<ProviderConfiguration>,
    pub prompt_strategies: Arc<TranslationPromptConfiguration>,
}

pub struct CaptureRuntimeState {
    pub sessions: Arc<CaptureSessions>,
    pub output: Arc<CaptureOutput>,
    pub runtime: Arc<CaptureSessionRuntime>,
    pub pinned_images: Arc<PinnedImageRuntime>,
}

pub struct HistoryRuntime {
    pub history: Arc<History>,
    pub events: Arc<EventBus>,
}

pub struct SelectionRuntime {
    pub acquirer: Arc<SelectedTextAcquirer>,
}

pub struct AppState {
    pub settings: Arc<SettingsRuntime>,
    pub providers: Arc<ProviderRuntime>,
    pub capture: Arc<CaptureRuntimeState>,
    pub history: Arc<HistoryRuntime>,
    pub selection: Arc<SelectionRuntime>,
    #[allow(dead_code)] // Wired in Task 3; commands consume it in the following Task 4.
    pub(crate) result_window: Arc<ResultWindowRuntime>,
}

impl AppState {
    /// Gracefully shutdown the application, waiting for pending events to complete
    pub async fn shutdown(&self) -> Result<()> {
        log::info!("Starting graceful shutdown...");

        // Wait for all pending events to complete (max 5 seconds)
        let drained = self
            .history
            .events
            .drain(std::time::Duration::from_secs(5))
            .await;

        if !drained {
            log::warn!("Shutdown: Some events did not complete in time");
        }

        log::info!("Graceful shutdown complete");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::application::result_window::ResultWindowRuntime;

    use super::AppState;

    #[test]
    fn app_state_exposes_shared_result_window_runtime() {
        fn runtime(state: &AppState) -> &Arc<ResultWindowRuntime> {
            &state.result_window
        }

        let _ = runtime;
    }
}
