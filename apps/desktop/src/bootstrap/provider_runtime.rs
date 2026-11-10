use std::sync::Arc;

#[cfg(all(test, target_os = "macos"))]
use crate::integrations::events::EventBus;
use crate::integrations::llm::InfrastructureLlmRuntime;
use snaplingo_core::providers::ocr::impls::BaiduOcrProvider;
use snaplingo_core::providers::ocr::impls::PaddleOcrProvider;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use snaplingo_core::providers::ocr::impls::SystemOcrProvider;
#[cfg(any(
    target_os = "linux",
    all(target_os = "macos", feature = "tesseract-ocr")
))]
use snaplingo_core::providers::ocr::impls::TesseractProvider;
use snaplingo_core::providers::ocr::OcrCoordinator;
use snaplingo_core::providers::ocr::PaddleOcrEngine;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use snaplingo_core::providers::ocr::SystemOcrEngine;
use snaplingo_core::providers::translation::BaiduTranslateProvider;
use snaplingo_core::providers::translation::DeepLProvider;
use snaplingo_core::providers::translation::GoogleTranslateProvider as GoogleTranslateProviderV2;
use snaplingo_core::providers::translation::TranslationCoordinator;
use snaplingo_core::providers::HttpClient;
use snaplingo_core::providers::LlmIntrospection;
use snaplingo_core::providers::LlmRuntime;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use snaplingo_core::providers::Provider;
use snaplingo_core::providers::ProviderAdministration;
use snaplingo_core::providers::ProviderConfigStore;
use snaplingo_core::providers::ProviderConfiguration;
use snaplingo_core::providers::ProviderCredentialStore;
use snaplingo_core::providers::ProviderEventSink;
use snaplingo_core::providers::TranslationPromptConfiguration;
#[cfg(any(
    target_os = "linux",
    all(target_os = "macos", feature = "tesseract-ocr")
))]
use snaplingo_platform::ocr::get_tesseract_engine;
#[cfg(target_os = "macos")]
use snaplingo_platform::ocr::MacOSVisionOcrEngine;
#[cfg(target_os = "windows")]
use snaplingo_platform::ocr::WindowsOcrEngine;

pub(crate) fn build_llm_runtime(http_client: Arc<dyn HttpClient>) -> Arc<dyn LlmRuntime> {
    Arc::new(InfrastructureLlmRuntime::new(http_client))
}

pub(crate) fn build_llm_introspection(llm_runtime: Arc<dyn LlmRuntime>) -> Arc<LlmIntrospection> {
    Arc::new(LlmIntrospection::new(llm_runtime))
}

pub(crate) fn build_provider_configuration(
    config_store: Arc<dyn ProviderConfigStore>,
    credential_store: Arc<dyn ProviderCredentialStore>,
    llm_runtime: Arc<dyn LlmRuntime>,
    translation_coordinator: Arc<snaplingo_core::providers::translation::TranslationCoordinator>,
    llm_introspection: Arc<LlmIntrospection>,
    change_notifier: Arc<dyn snaplingo_core::providers::ProviderChangeNotifier>,
) -> Arc<ProviderConfiguration> {
    Arc::new(
        ProviderConfiguration::new(
            config_store,
            credential_store,
            llm_runtime,
            translation_coordinator,
            llm_introspection,
        )
        .with_change_notifier(change_notifier),
    )
}

pub(crate) fn build_provider_administration(
    translation_coordinator: Arc<TranslationCoordinator>,
    ocr_coordinator: Arc<OcrCoordinator>,
    provider_configuration: Arc<ProviderConfiguration>,
    ocr_configuration: Arc<snaplingo_core::providers::ocr::OcrProviderConfiguration>,
    llm_introspection: Arc<LlmIntrospection>,
    prompt_strategies: Arc<TranslationPromptConfiguration>,
) -> Arc<ProviderAdministration> {
    Arc::new(ProviderAdministration::new(
        translation_coordinator,
        ocr_coordinator,
        provider_configuration,
        ocr_configuration,
        llm_introspection,
        prompt_strategies,
    ))
}

pub(crate) fn build_translation_coordinator(
    config_store: Arc<dyn ProviderConfigStore>,
    http_client: Arc<dyn HttpClient>,
    event_sink: Arc<dyn ProviderEventSink>,
) -> Arc<TranslationCoordinator> {
    let translation_coordinator = TranslationCoordinator::new(config_store);

    let google_provider = GoogleTranslateProviderV2::new(http_client.clone());
    if let Err(e) = translation_coordinator.register(google_provider) {
        log::warn!("Failed to register Google Translate provider: {}", e);
    }

    let deeplx_provider = DeepLProvider::new(http_client.clone());
    if let Err(e) = translation_coordinator.register(deeplx_provider) {
        log::warn!("Failed to register DeepLX provider: {}", e);
    }

    let baidu_provider = BaiduTranslateProvider::new(http_client.clone());
    translation_coordinator
        .register(baidu_provider)
        .map_err(|e| log::warn!("Failed to register Baidu Translate provider: {}", e))
        .ok();

    if let Err(e) = translation_coordinator.restore_from_config() {
        log::warn!("Failed to restore active providers from config: {}", e);
    }

    Arc::new(translation_coordinator.with_event_sink(event_sink))
}

pub(crate) fn build_ocr_coordinator(
    config_store: Arc<dyn ProviderConfigStore>,
    http_client: Arc<dyn HttpClient>,
    event_sink: Arc<dyn ProviderEventSink>,
    change_notifier: Arc<dyn snaplingo_core::providers::ProviderChangeNotifier>,
    paddle_engine: Arc<dyn PaddleOcrEngine>,
    settings: Arc<snaplingo_core::settings::SettingsConfiguration>,
) -> Arc<OcrCoordinator> {
    let ocr_coordinator = OcrCoordinator::new(config_store);
    ocr_coordinator
        .register(PaddleOcrProvider::new(
            paddle_engine,
            settings,
            change_notifier.clone(),
        ))
        .ok();

    #[cfg(any(
        target_os = "linux",
        all(target_os = "macos", feature = "tesseract-ocr")
    ))]
    {
        let tesseract_provider = TesseractProvider::new(get_tesseract_engine());
        ocr_coordinator.register(tesseract_provider).ok();
    }

    #[cfg(target_os = "macos")]
    register_system_ocr_provider(&ocr_coordinator, Arc::new(MacOSVisionOcrEngine::new()));

    #[cfg(target_os = "windows")]
    register_system_ocr_provider(&ocr_coordinator, Arc::new(WindowsOcrEngine::new()));

    let baidu_ocr_provider = BaiduOcrProvider::new(http_client);
    ocr_coordinator.register(baidu_ocr_provider).ok();
    ocr_coordinator.restore_from_config().ok();

    Arc::new(
        ocr_coordinator
            .with_event_sink(event_sink)
            .with_change_notifier(change_notifier),
    )
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn register_system_ocr_provider(
    ocr_coordinator: &OcrCoordinator,
    engine: Arc<dyn SystemOcrEngine>,
) {
    let provider = SystemOcrProvider::new(engine);
    if !provider.is_configured() {
        log::info!("System OCR is unavailable; provider registration skipped");
        return;
    }

    if let Err(error) = ocr_coordinator.register(provider) {
        log::warn!("Failed to register System OCR provider: {error}");
    }
}

pub(crate) fn hydrate_provider_credentials(administration: Arc<ProviderAdministration>) {
    administration.hydrate_credentials();
}

#[cfg(all(test, any(target_os = "macos", target_os = "windows")))]
mod tests {
    use super::*;
    #[cfg(target_os = "macos")]
    use async_trait::async_trait;
    #[cfg(target_os = "macos")]
    use snaplingo_core::providers::HttpResponse;
    use snaplingo_storage::SqliteConfigStore;
    #[cfg(target_os = "macos")]
    use std::collections::HashMap;

    #[cfg(target_os = "macos")]
    struct StubHttpClient;

    struct StubSystemOcrEngine {
        available: bool,
    }

    impl SystemOcrEngine for StubSystemOcrEngine {
        fn is_available(&self) -> bool {
            self.available
        }

        fn recognize(
            &self,
            _request: &snaplingo_core::providers::ocr::model::OcrRequest,
        ) -> crate::Result<snaplingo_core::providers::ocr::model::OcrResult> {
            unimplemented!("provider registration should not recognize text")
        }
    }

    #[cfg(target_os = "macos")]
    #[async_trait]
    impl HttpClient for StubHttpClient {
        async fn post(
            &self,
            _url: &str,
            _headers: HashMap<String, String>,
            _body: String,
        ) -> anyhow::Result<HttpResponse> {
            unimplemented!("OCR provider registration should not make HTTP requests")
        }

        async fn get(
            &self,
            _url: &str,
            _headers: HashMap<String, String>,
        ) -> anyhow::Result<HttpResponse> {
            unimplemented!("OCR provider registration should not make HTTP requests")
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn build_ocr_coordinator_registers_system_ocr_provider_on_macos() {
        let coordinator = build_ocr_coordinator(
            Arc::new(SqliteConfigStore::new_temp()),
            Arc::new(StubHttpClient),
            Arc::new(EventBus::new()),
            Arc::new(TestProviderChangeNotifier),
            Arc::new(crate::integrations::ocr::LocalPaddleOcrEngine::new(
                None,
                std::path::PathBuf::new(),
            )),
            Arc::new(snaplingo_core::settings::SettingsConfiguration::new(
                Arc::new(SqliteConfigStore::new_temp()),
            )),
        );

        assert!(coordinator.get("system-ocr").is_some());
        assert!(coordinator.get("paddle-ocr").is_some());
    }

    #[test]
    fn system_ocr_registration_includes_available_engine() {
        let coordinator = OcrCoordinator::new(Arc::new(SqliteConfigStore::new_temp()));

        register_system_ocr_provider(
            &coordinator,
            Arc::new(StubSystemOcrEngine { available: true }),
        );

        assert!(coordinator.get("system-ocr").is_some());
    }

    #[test]
    fn system_ocr_registration_skips_unavailable_engine() {
        let coordinator = OcrCoordinator::new(Arc::new(SqliteConfigStore::new_temp()));

        register_system_ocr_provider(
            &coordinator,
            Arc::new(StubSystemOcrEngine { available: false }),
        );

        assert!(coordinator.get("system-ocr").is_none());
    }

    #[cfg(target_os = "macos")]
    struct TestProviderChangeNotifier;

    #[cfg(target_os = "macos")]
    impl snaplingo_core::providers::ProviderChangeNotifier for TestProviderChangeNotifier {
        fn providers_changed(&self) {}
    }
}
