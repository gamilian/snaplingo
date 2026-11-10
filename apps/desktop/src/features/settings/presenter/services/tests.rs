use super::*;
use async_trait::async_trait;
use snaplingo_core::library::history::History;
use snaplingo_core::providers::common::Provider;
use snaplingo_core::providers::ocr::OcrCoordinator;
use snaplingo_core::providers::ocr::OcrProvider;
use snaplingo_core::providers::ocr::OcrProviderConfiguration;
use snaplingo_core::providers::translation::DeepLProvider;
use snaplingo_core::providers::translation::TranslationCoordinator;
use snaplingo_core::providers::HttpClient;
use snaplingo_core::providers::HttpResponse;
use snaplingo_core::providers::LLMClient;
use snaplingo_core::providers::LLMRequest;
use snaplingo_core::providers::LLMResponse;
use snaplingo_core::providers::LlmClientConfig;
use snaplingo_core::providers::LlmIntrospection;
use snaplingo_core::providers::LlmModelLister;
use snaplingo_core::providers::LlmRuntime;
use snaplingo_core::providers::ModelInfo;
use snaplingo_core::providers::ProviderAdministration;
use snaplingo_core::providers::ProviderConfiguration;
use snaplingo_core::providers::ProviderCredentialStore;
use snaplingo_core::providers::TranslationPromptConfiguration;
use snaplingo_core::settings::ServicesDependencies;
use snaplingo_core::settings::ServicesKind;
use snaplingo_core::settings::SettingsApplication;
use snaplingo_core::settings::SettingsConfiguration;
use snaplingo_core::settings::StartOnBoot;
use snaplingo_core::speech::SystemTtsHost;
use snaplingo_core::speech::SystemTtsVoice;
use snaplingo_core::speech::TtsRuntime;
use snaplingo_storage::{
    Database, SqliteAppLogRepository, SqliteConfigStore, SqliteCredentialStore,
    SqliteHistoryRepository,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

struct Boot;
impl StartOnBoot for Boot {
    fn set_enabled(&self, _: bool) -> snaplingo_core::Result<()> {
        Ok(())
    }
}
struct Http;
#[async_trait]
impl HttpClient for Http {
    async fn get(&self, _: &str, _: HashMap<String, String>) -> anyhow::Result<HttpResponse> {
        unreachable!()
    }
    async fn post(
        &self,
        _: &str,
        _: HashMap<String, String>,
        _: String,
    ) -> anyhow::Result<HttpResponse> {
        unreachable!()
    }
}
struct Llm;
impl LlmRuntime for Llm {
    fn translation_client(&self, _: LlmClientConfig) -> Arc<dyn LLMClient> {
        Arc::new(Llm)
    }
    fn model_lister(&self, _: LlmClientConfig) -> Arc<dyn LlmModelLister> {
        Arc::new(Llm)
    }
}
#[async_trait]
impl LLMClient for Llm {
    async fn generate(&self, _: &LLMRequest) -> anyhow::Result<LLMResponse> {
        Ok(LLMResponse { text: "OK".into() })
    }
}
#[async_trait]
impl LlmModelLister for Llm {
    async fn list_models(&self) -> anyhow::Result<Vec<ModelInfo>> {
        Ok(vec![ModelInfo {
            id: "fixture-model".into(),
        }])
    }
}
struct Ocr {
    configured: bool,
}
impl Provider for Ocr {
    fn id(&self) -> &str {
        "fixture-ocr"
    }
    fn name(&self) -> &str {
        "Fixture OCR"
    }
    fn is_configured(&self) -> bool {
        self.configured
    }
    fn requires_api_key(&self) -> bool {
        true
    }
    fn reconfigure_credentials(
        &mut self,
        credentials: &HashMap<String, String>,
    ) -> snaplingo_core::Result<()> {
        self.validate_credentials(credentials)?;
        self.configured = true;
        Ok(())
    }
}
#[async_trait]
impl OcrProvider for Ocr {
    async fn recognize(
        &self,
        _: &snaplingo_core::providers::ocr::model::OcrRequest,
    ) -> snaplingo_core::Result<snaplingo_core::providers::ocr::model::OcrResult> {
        unreachable!()
    }
}
#[derive(Default)]
struct Speech {
    calls: Mutex<Vec<(String, String, u16)>>,
}
#[async_trait]
impl SystemTtsHost for Speech {
    async fn list_voices(&self) -> snaplingo_core::Result<Vec<SystemTtsVoice>> {
        Ok(vec![SystemTtsVoice {
            id: "opaque-voice".into(),
            name: "Fixture".into(),
            locale: "zh-CN".into(),
        }])
    }
    async fn speak(&self, text: &str, voice: &str, rate: u16) -> snaplingo_core::Result<()> {
        self.calls
            .lock()
            .unwrap()
            .push((text.into(), voice.into(), rate));
        Ok(())
    }
}
struct Fixture {
    session: ServicesSession,
    configuration: Arc<SettingsConfiguration>,
    credentials: Arc<SqliteCredentialStore>,
    speech: Arc<Speech>,
}
fn fixture(route: ui::ServicesRoute) -> Fixture {
    let database = Arc::new(Database::in_memory().unwrap());
    let store = Arc::new(SqliteConfigStore::new(database.clone()));
    let credentials = Arc::new(SqliteCredentialStore::new(database.clone()));
    let configuration = Arc::new(SettingsConfiguration::new(store.clone()));
    let history = Arc::new(History::new(Arc::new(SqliteHistoryRepository::new(
        database.clone(),
    ))));
    let settings = Arc::new(SettingsApplication::new(
        configuration.clone(),
        Arc::new(Boot),
        history,
        Arc::new(SqliteAppLogRepository::new(database)),
    ));
    let translation = Arc::new(TranslationCoordinator::new(store.clone()));
    translation
        .register(DeepLProvider::new(Arc::new(Http)))
        .unwrap();
    let ocr = Arc::new(OcrCoordinator::new(store.clone()));
    ocr.register(Ocr { configured: false }).unwrap();
    let runtime = Arc::new(Llm);
    let introspection = Arc::new(LlmIntrospection::new(runtime.clone()));
    let admin = Arc::new(ProviderAdministration::new(
        translation.clone(),
        ocr.clone(),
        Arc::new(ProviderConfiguration::new(
            store.clone(),
            credentials.clone(),
            runtime,
            translation,
            introspection.clone(),
        )),
        Arc::new(OcrProviderConfiguration::new(ocr, credentials.clone())),
        introspection,
        Arc::new(TranslationPromptConfiguration::new(store)),
    ));
    let speech = Arc::new(Speech::default());
    let tts = Arc::new(TtsRuntime::new(speech.clone(), configuration.clone()));
    let dependencies = || ServicesDependencies {
        configuration: configuration.clone(),
        settings: settings.clone(),
        providers: admin.clone(),
        tts: tts.clone(),
    };
    Fixture {
        session: ServicesSession::new(
            Arc::new(ServicesController::new(ServicesKind::Ocr, dependencies())),
            Arc::new(ServicesController::new(ServicesKind::Tts, dependencies())),
            Arc::new(TranslationServicesController::new(admin)),
            route,
        ),
        configuration,
        credentials,
        speech,
    }
}
async fn dispatch(session: &ServicesSession, intent: ui::ServicesIntent) {
    session.dispatch(intent, || {}).await.unwrap();
}
fn snapshot(session: &ServicesSession) -> ui::ServicesSnapshot {
    session.snapshot(Snapshot::default()).unwrap()
}

#[tokio::test]
async fn ocr_credential_form_revisions_survive_refresh_but_reject_a_cancelled_save() {
    let fixture = fixture(ui::ServicesRoute::Ocr);
    let session = &fixture.session;
    dispatch(
        session,
        ui::ServicesIntent::Configure {
            route: ui::ServicesRoute::Ocr,
            provider_id: "fixture-ocr".into(),
        },
    )
    .await;
    let first = snapshot(session);
    let second = snapshot(session);
    assert_eq!(first.form_revision, second.form_revision);
    assert!(second.revision > first.revision);
    assert!(matches!(first.form,Some(ui::ServiceForm::Credentials{fields,..})if fields[0].secret));
    session.edit(&ui::ServicesIntent::CancelForm).unwrap();
    dispatch(
        session,
        ui::ServicesIntent::Configure {
            route: ui::ServicesRoute::Ocr,
            provider_id: "fixture-ocr".into(),
        },
    )
    .await;
    dispatch(
        session,
        ui::ServicesIntent::SaveCredentials {
            route: ui::ServicesRoute::Ocr,
            revision: first.form_revision,
            provider_id: "fixture-ocr".into(),
            credentials: [("api_key".into(), "stale".into())].into(),
        },
    )
    .await;
    assert!(fixture
        .credentials
        .load_provider_credential("fixture-ocr")
        .is_err());
    let revision = snapshot(session).form_revision;
    dispatch(
        session,
        ui::ServicesIntent::SaveCredentials {
            route: ui::ServicesRoute::Ocr,
            revision,
            provider_id: "fixture-ocr".into(),
            credentials: [("api_key".into(), "persisted".into())].into(),
        },
    )
    .await;
    let next = snapshot(session);
    assert!(next.form.is_none());
    assert!(next.providers[0].active && next.providers[0].configured);
    assert_eq!(
        fixture
            .credentials
            .load_provider_credential("fixture-ocr")
            .unwrap(),
        "persisted"
    );
}

#[tokio::test]
async fn synchronous_custom_edits_reach_save_before_spawned_work_and_use_the_real_admin() {
    let fixture = fixture(ui::ServicesRoute::Translation);
    let session = &fixture.session;
    dispatch(session, ui::ServicesIntent::AddCustom).await;
    let revision = snapshot(session).form_revision;
    for (field, value) in [
        (ui::ServiceCustomField::Name, "a"),
        (ui::ServiceCustomField::Name, "ab"),
        (ui::ServiceCustomField::ApiKey, "key"),
    ] {
        assert!(session
            .edit(&ui::ServicesIntent::EditCustom {
                revision,
                field,
                value: value.into()
            })
            .unwrap());
    }
    assert!(session
        .edit(&ui::ServicesIntent::Protocol {
            revision,
            protocol: ui::ServiceProtocol::OpenAiResponses
        })
        .unwrap());
    dispatch(session, ui::ServicesIntent::SaveCustom { revision }).await;
    let provider = snapshot(session)
        .providers
        .into_iter()
        .find(|provider| !provider.builtin)
        .unwrap();
    assert_eq!(provider.name, "ab");
    assert!(provider.active);
    assert_eq!(
        provider.protocol,
        Some(ui::ServiceProtocol::OpenAiResponses)
    );
    assert_eq!(
        fixture
            .credentials
            .load_provider_credential(&provider.id)
            .unwrap(),
        "key"
    );
    dispatch(
        session,
        ui::ServicesIntent::Configure {
            route: ui::ServicesRoute::Translation,
            provider_id: provider.id.clone(),
        },
    )
    .await;
    let revision = snapshot(session).form_revision;
    session
        .edit(&ui::ServicesIntent::EditCustom {
            revision,
            field: ui::ServiceCustomField::Model,
            value: "updated".into(),
        })
        .unwrap();
    dispatch(session, ui::ServicesIntent::SaveCustom { revision }).await;
    assert_eq!(
        fixture
            .credentials
            .load_provider_credential(&provider.id)
            .unwrap(),
        "key"
    );
    assert_eq!(
        snapshot(session)
            .providers
            .into_iter()
            .find(|item| item.id == provider.id)
            .unwrap()
            .model
            .as_deref(),
        Some("updated")
    );
    dispatch(
        session,
        ui::ServicesIntent::Configure {
            route: ui::ServicesRoute::Translation,
            provider_id: provider.id.clone(),
        },
    )
    .await;
    let revision = snapshot(session).form_revision;
    session
        .edit(&ui::ServicesIntent::EditCustom {
            revision,
            field: ui::ServiceCustomField::ApiKey,
            value: "key".into(),
        })
        .unwrap();
    dispatch(session, ui::ServicesIntent::LoadModels { revision }).await;
    assert_eq!(snapshot(session).models, ["fixture-model"]);
    dispatch(session, ui::ServicesIntent::TestConnection { revision }).await;
    assert_eq!(snapshot(session).test_status, Some(Ok("检测成功".into())));
}

#[tokio::test]
async fn route_edits_persist_only_valid_routes_and_explicit_navigation_wins() {
    let mut fixture = fixture(ui::ServicesRoute::Ocr);
    let store = Arc::new(SqliteConfigStore::new(Arc::new(
        Database::in_memory().unwrap(),
    )));
    fixture.session.navigation = Some(store.clone());
    fixture
        .session
        .edit(&ui::ServicesIntent::Route(ui::ServicesRoute::Translation))
        .unwrap();
    assert_eq!(
        store.load_services_route().unwrap().as_deref(),
        Some("translation")
    );
    assert_eq!(
        saved_route(&store.load_services_route().unwrap().unwrap()),
        Some(ui::ServicesRoute::Translation)
    );
    fixture
        .session
        .edit(&ui::ServicesIntent::Route(ui::ServicesRoute::Tts))
        .unwrap();
    assert_eq!(snapshot(&fixture.session).route, ui::ServicesRoute::Tts);
    assert_eq!(store.load_services_route().unwrap().as_deref(), Some("tts"));
    assert_eq!(saved_route("unexpected"), None);
}

#[tokio::test]
async fn routes_and_tts_edits_are_ordered_and_preserve_unrelated_general_preferences() {
    let fixture = fixture(ui::ServicesRoute::Ocr);
    let session = &fixture.session;
    let mut general = fixture.configuration.snapshot().unwrap().general;
    general.retry_count = 4;
    fixture.configuration.update_general(general).unwrap();
    session
        .edit(&ui::ServicesIntent::Route(ui::ServicesRoute::Tts))
        .unwrap();
    session
        .edit(&ui::ServicesIntent::Route(ui::ServicesRoute::Translation))
        .unwrap();
    dispatch(session, ui::ServicesIntent::Refresh).await;
    assert_eq!(snapshot(session).route, ui::ServicesRoute::Translation);
    session
        .edit(&ui::ServicesIntent::Route(ui::ServicesRoute::Tts))
        .unwrap();
    dispatch(session, ui::ServicesIntent::Refresh).await;
    assert_eq!(snapshot(session).voices[0].id, "opaque-voice");
    session
        .edit(&ui::ServicesIntent::Voice("opaque-voice".into()))
        .unwrap();
    session.edit(&ui::ServicesIntent::Rate(260)).unwrap();
    dispatch(session, ui::ServicesIntent::Speak).await;
    assert_eq!(
        fixture.speech.calls.lock().unwrap().as_slice(),
        &[("SnapLingo 系统语音测试".into(), "opaque-voice".into(), 260)]
    );
    assert_eq!(
        fixture
            .configuration
            .snapshot()
            .unwrap()
            .general
            .retry_count,
        4
    );
    session.dispose();
    let changed = AtomicUsize::new(0);
    session
        .dispatch(ui::ServicesIntent::Rate(400), || {
            changed.fetch_add(1, Ordering::SeqCst);
        })
        .await
        .unwrap();
    assert_eq!(changed.load(Ordering::SeqCst), 0);
    assert_eq!(
        fixture
            .configuration
            .snapshot()
            .unwrap()
            .general
            .system_tts_rate,
        260
    );
}

#[tokio::test]
async fn destructive_and_platform_actions_are_returned_to_the_host_and_failures_refresh() {
    let fixture = fixture(ui::ServicesRoute::Translation);
    let session = &fixture.session;
    dispatch(session, ui::ServicesIntent::AddCustom).await;
    let revision = snapshot(session).form_revision;
    session
        .edit(&ui::ServicesIntent::EditCustom {
            revision,
            field: ui::ServiceCustomField::Name,
            value: "example".into(),
        })
        .unwrap();
    session
        .edit(&ui::ServicesIntent::EditCustom {
            revision,
            field: ui::ServiceCustomField::ApiKey,
            value: "key".into(),
        })
        .unwrap();
    dispatch(session, ui::ServicesIntent::SaveCustom { revision }).await;
    let id = snapshot(session)
        .providers
        .into_iter()
        .find(|provider| !provider.builtin)
        .unwrap()
        .id;
    assert!(
        matches!(session.dispatch(ui::ServicesIntent::ConfirmRemove(id.clone()),||{}).await.unwrap().as_slice(),[ServicesHostEffect::ConfirmRemove(found)]if found==&id)
    );
    assert!(snapshot(session)
        .providers
        .iter()
        .any(|provider| provider.id == id));
    session.remove_confirmed(id.clone(), || {}).await.unwrap();
    assert!(!snapshot(session)
        .providers
        .iter()
        .any(|provider| provider.id == id));
    assert!(matches!(
        session
            .dispatch(
                ui::ServicesIntent::OpenUrl("https://fixture.test".into()),
                || {}
            )
            .await
            .unwrap()
            .as_slice(),
        [ServicesHostEffect::OpenUrl(_)]
    ));
    assert!(
        matches!(session.dispatch(ui::ServicesIntent::CopyText("copy".into()),||{}).await.unwrap().as_slice(),[ServicesHostEffect::CopyText(text)]if text=="copy")
    );
    let changed = AtomicUsize::new(0);
    let error = session
        .dispatch(
            ui::ServicesIntent::Configure {
                route: ui::ServicesRoute::Translation,
                provider_id: "missing".into(),
            },
            || {
                changed.fetch_add(1, Ordering::SeqCst);
            },
        )
        .await;
    assert!(error.is_err());
    assert!(changed.load(Ordering::SeqCst) > 0);
    assert!(snapshot(session).error.is_some());
}

pub(crate) fn session() -> ServicesSession {
    fixture(ui::ServicesRoute::Ocr).session
}

impl ServicesNavigation for SqliteConfigStore {
    fn load(&self) -> Result<Option<String>, String> {
        self.load_services_route().map_err(|e| e.to_string())
    }
    fn save(&self, route: &str) -> Result<(), String> {
        self.save_services_route(route).map_err(|e| e.to_string())
    }
}
