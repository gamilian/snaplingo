//! Settings service actions retain the existing Rust application services.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::application::providers::{
    common::CredentialField, OcrProviderInfo, ProviderAdministration,
};
use crate::application::settings::{GeneralField, GeneralValue};
use crate::application::{SettingsApplication, SettingsConfiguration, SystemTtsVoice, TtsRuntime};
use crate::domain::GeneralSettings;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServicesKind {
    Ocr,
    Tts,
}

/// The OCR service-page contract is independent of a desktop renderer.
pub trait OcrServices: Send + Sync {
    fn list(&self) -> Vec<OcrProviderInfo>;
    fn credential_schema(&self, id: &str) -> crate::Result<Vec<CredentialField>>;
    fn configure(&self, id: &str, credentials: HashMap<String, String>) -> crate::Result<()>;
    fn activate(&self, id: &str) -> crate::Result<()>;
}

impl OcrServices for ProviderAdministration {
    fn list(&self) -> Vec<OcrProviderInfo> {
        self.list_ocr_providers()
    }
    fn credential_schema(&self, id: &str) -> crate::Result<Vec<CredentialField>> {
        self.ocr_credential_schema(id.into())
    }
    fn configure(&self, id: &str, credentials: HashMap<String, String>) -> crate::Result<()> {
        self.configure_ocr_provider_credentials(id.into(), credentials)
    }
    fn activate(&self, id: &str) -> crate::Result<()> {
        self.activate_ocr_provider(id.into())
    }
}

pub struct ServicesDependencies {
    pub configuration: Arc<SettingsConfiguration>,
    pub settings: Arc<SettingsApplication>,
    pub providers: Arc<dyn OcrServices>,
    pub tts: Arc<TtsRuntime>,
}

#[derive(Clone)]
pub struct CredentialForm {
    pub provider_id: String,
    pub provider_name: String,
    pub fields: Vec<CredentialField>,
}

#[derive(Default)]
struct State {
    form: Option<CredentialForm>,
    voices: Vec<SystemTtsVoice>,
    status: Option<String>,
    error: Option<String>,
    active: Option<(u64, Operation)>,
    revision: u64,
    voices_request: u64,
    disposed: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Operation {
    Schema,
    Activate,
    Save,
    Speak,
}

#[derive(Clone)]
pub struct ServicesProjection {
    pub general: GeneralSettings,
    pub providers: Vec<OcrProviderInfo>,
    pub form: Option<CredentialForm>,
    pub voices: Vec<SystemTtsVoice>,
    pub status: Option<String>,
    pub error: Option<String>,
    pub busy: bool,
}

pub struct ServicesController {
    kind: ServicesKind,
    services: ServicesDependencies,
    state: Mutex<State>,
}

impl ServicesController {
    pub fn new(kind: ServicesKind, services: ServicesDependencies) -> Self {
        Self {
            kind,
            services,
            state: Mutex::new(State::default()),
        }
    }

    pub fn projection(&self) -> Result<ServicesProjection, String> {
        let general = self
            .services
            .configuration
            .snapshot()
            .map_err(|error| error.to_string())?
            .general;
        let providers = if self.kind == ServicesKind::Ocr {
            self.services.providers.list()
        } else {
            Vec::new()
        };
        let state = self.state.lock().unwrap();
        Ok(ServicesProjection {
            general,
            providers,
            form: state.form.clone(),
            voices: state.voices.clone(),
            status: state.status.clone(),
            error: state.error.clone(),
            busy: state.active.is_some(),
        })
    }

    pub fn configure(&self, id: &str) -> Result<(), String> {
        let Some(token) = self.begin(Operation::Schema) else {
            return Ok(());
        };
        let result = (|| {
            let provider = self
                .services
                .providers
                .list()
                .into_iter()
                .find(|provider| provider.id == id)
                .ok_or("OCR 服务不存在")?;
            if !provider.requires_api_key {
                self.services
                    .providers
                    .activate(id)
                    .map_err(|error| error.to_string())?;
                return Ok(None);
            }
            let fields = self
                .services
                .providers
                .credential_schema(id)
                .map_err(|error| error.to_string())?;
            Ok(Some(CredentialForm {
                provider_id: id.into(),
                provider_name: provider.name,
                fields,
            }))
        })();
        self.finish(
            token,
            result.as_ref().map(|_| ()).map_err(Clone::clone),
            |state| {
                state.form = result.as_ref().unwrap().clone();
            },
        );
        result.map(|_| ())
    }

    pub fn cancel(&self) {
        let mut state = self.state.lock().unwrap();
        if state
            .active
            .is_none_or(|(_, kind)| kind == Operation::Schema)
        {
            state.active = None;
            state.form = None;
            state.error = None;
        }
    }

    pub fn activate(&self, id: &str) -> Result<(), String> {
        let Some(token) = self.begin(Operation::Activate) else {
            return Ok(());
        };
        let result = self
            .services
            .providers
            .activate(id)
            .map_err(|error| error.to_string());
        self.finish(token, result.clone(), |_| {});
        result
    }

    pub fn save(&self, credentials: HashMap<String, String>) -> Result<(), String> {
        let Some(token) = self.begin(Operation::Save) else {
            return Ok(());
        };
        let result = (|| {
            let form = self
                .state
                .lock()
                .unwrap()
                .form
                .clone()
                .ok_or("未选择 OCR 服务")?;
            for field in &form.fields {
                if credentials
                    .get(&field.name)
                    .is_none_or(|value| value.trim().is_empty())
                {
                    return Err(format!("请填写：{}", field.label));
                }
            }
            self.services
                .providers
                .configure(&form.provider_id, credentials)
                .and_then(|_| self.services.providers.activate(&form.provider_id))
                .map_err(|error| error.to_string())
        })();
        self.finish(token, result.clone(), |state| state.form = None);
        result
    }

    pub fn edit_voice(&self, id: String) -> Result<(), String> {
        self.edit(GeneralField::SystemTtsVoice, GeneralValue::Text(id))
    }
    pub fn edit_rate(&self, value: u32) -> Result<(), String> {
        self.edit(GeneralField::SystemTtsRate, GeneralValue::Number(value))
    }
    fn edit(&self, field: GeneralField, value: GeneralValue) -> Result<(), String> {
        if self.state.lock().unwrap().disposed {
            return Ok(());
        }
        let result = self
            .services
            .settings
            .edit_general(field, value)
            .map(|_| ())
            .map_err(|error| error.to_string());
        let mut state = self.state.lock().unwrap();
        if !state.disposed {
            state.error = result.as_ref().err().cloned();
        }
        result
    }

    pub async fn load_voices(&self) {
        let request = {
            let mut state = self.state.lock().unwrap();
            if state.disposed {
                return;
            }
            state.voices_request += 1;
            state.voices_request
        };
        let voices = self.services.tts.list_voices().await.unwrap_or_default();
        let mut state = self.state.lock().unwrap();
        if !state.disposed && state.voices_request == request {
            state.voices = voices;
        }
    }

    pub async fn speak(&self, changed: impl Fn()) {
        let Some(token) = self.begin(Operation::Speak) else {
            return;
        };
        changed();
        let result = self
            .services
            .tts
            .speak("SnapLingo 系统语音测试", Some("zh-CN"))
            .await
            .map_err(|error| error.to_string());
        if self.finish(token, Ok(()), |state| {
            state.status = Some(match result {
                Ok(()) => "已开始朗读".into(),
                Err(error) => error,
            });
        }) {
            changed();
        }
    }

    fn begin(&self, operation: Operation) -> Option<u64> {
        let mut state = self.state.lock().unwrap();
        if state.active.is_some() || state.disposed {
            return None;
        }
        state.revision += 1;
        let token = state.revision;
        state.active = Some((token, operation));
        state.error = None;
        if operation == Operation::Speak {
            state.status = Some("正在朗读".into());
        }
        Some(token)
    }
    fn finish(
        &self,
        token: u64,
        result: Result<(), String>,
        success: impl FnOnce(&mut State),
    ) -> bool {
        let mut state = self.state.lock().unwrap();
        if state.disposed || state.active.is_none_or(|(current, _)| current != token) {
            return false;
        }
        state.active = None;
        state.error = result.as_ref().err().cloned();
        if result.is_ok() {
            success(&mut state);
        }
        true
    }
    pub fn dispose(&self) {
        let mut state = self.state.lock().unwrap();
        state.disposed = true;
        state.active = None;
        state.form = None;
        state.voices.clear();
        state.error = None;
        state.status = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::time::Duration;

    use crate::application::{History, StartOnBoot, SystemTtsHost};
    use crate::infrastructure::storage::{
        Database, SqliteAppLogRepository, SqliteConfigStore, SqliteHistoryRepository,
    };
    use async_trait::async_trait;

    struct Gate {
        operation: &'static str,
        entered: mpsc::Sender<()>,
        release: mpsc::Receiver<()>,
    }
    #[derive(Default)]
    struct Providers {
        calls: Mutex<Vec<String>>,
        fail: Mutex<Option<&'static str>>,
        gate: Mutex<Option<Gate>>,
        credentials: Mutex<HashMap<String, String>>,
    }
    impl Providers {
        fn block(&self, operation: &'static str) -> (mpsc::Receiver<()>, mpsc::Sender<()>) {
            let (entered, receive) = mpsc::channel();
            let (release, blocked) = mpsc::channel();
            *self.gate.lock().unwrap() = Some(Gate {
                operation,
                entered,
                release: blocked,
            });
            (receive, release)
        }
        fn attempt(&self, operation: &'static str, id: &str) -> crate::Result<()> {
            self.calls.lock().unwrap().push(format!("{operation}:{id}"));
            let gate = {
                let mut gate = self.gate.lock().unwrap();
                if gate
                    .as_ref()
                    .is_some_and(|gate| gate.operation == operation)
                {
                    gate.take()
                } else {
                    None
                }
            };
            if let Some(gate) = gate {
                gate.entered.send(()).unwrap();
                gate.release.recv_timeout(Duration::from_secs(5)).unwrap();
            }
            if *self.fail.lock().unwrap() == Some(operation) {
                Err(format!("fixture {operation} failure").into())
            } else {
                Ok(())
            }
        }
        fn count(&self, operation: &str) -> usize {
            self.calls
                .lock()
                .unwrap()
                .iter()
                .filter(|call| call.starts_with(operation))
                .count()
        }
    }
    impl OcrServices for Providers {
        fn list(&self) -> Vec<OcrProviderInfo> {
            [("local", false), ("remote", true)]
                .into_iter()
                .map(|(id, requires_api_key)| OcrProviderInfo {
                    id: id.into(),
                    name: id.into(),
                    is_configured: !requires_api_key,
                    requires_api_key,
                    is_active: !requires_api_key,
                    is_local: !requires_api_key,
                    acceleration: None,
                })
                .collect()
        }
        fn credential_schema(&self, id: &str) -> crate::Result<Vec<CredentialField>> {
            self.attempt("schema", id)?;
            Ok(vec![
                CredentialField::new("api_key", "API Key", true),
                CredentialField::new("secret_key", "Secret Key", true),
            ])
        }
        fn configure(&self, id: &str, credentials: HashMap<String, String>) -> crate::Result<()> {
            self.attempt("configure", id)?;
            *self.credentials.lock().unwrap() = credentials;
            Ok(())
        }
        fn activate(&self, id: &str) -> crate::Result<()> {
            self.attempt("activate", id)
        }
    }

    #[derive(Default)]
    struct Speech {
        voices: Mutex<Vec<SystemTtsVoice>>,
        voice_gate: Mutex<VecDeque<tokio::sync::oneshot::Receiver<()>>>,
        voice_entered: tokio::sync::Notify,
        voice_calls: AtomicUsize,
        speak_gate: Mutex<Option<tokio::sync::oneshot::Receiver<()>>>,
        speak_entered: tokio::sync::Notify,
        calls: Mutex<Vec<(String, String, u16)>>,
        fail: AtomicBool,
    }
    #[async_trait]
    impl SystemTtsHost for Speech {
        async fn list_voices(&self) -> crate::Result<Vec<SystemTtsVoice>> {
            self.voice_calls.fetch_add(1, Ordering::SeqCst);
            let voices = self.voices.lock().unwrap().clone();
            let gate = self.voice_gate.lock().unwrap().pop_front();
            self.voice_entered.notify_one();
            if let Some(gate) = gate {
                let _ = gate.await;
            }
            Ok(voices)
        }
        async fn speak(&self, text: &str, voice: &str, rate: u16) -> crate::Result<()> {
            self.calls
                .lock()
                .unwrap()
                .push((text.into(), voice.into(), rate));
            let gate = self.speak_gate.lock().unwrap().take();
            self.speak_entered.notify_one();
            if let Some(gate) = gate {
                let _ = gate.await;
            }
            if self.fail.load(Ordering::SeqCst) {
                Err("fixture speech failure".into())
            } else {
                Ok(())
            }
        }
    }
    struct Boot;
    impl StartOnBoot for Boot {
        fn set_enabled(&self, _: bool) -> crate::Result<()> {
            panic!("Voice preferences must not alter startup registration")
        }
    }
    struct Fixture {
        controller: Arc<ServicesController>,
        providers: Arc<Providers>,
        speech: Arc<Speech>,
        configuration: Arc<SettingsConfiguration>,
        database: Arc<Database>,
    }
    fn fixture(kind: ServicesKind) -> Fixture {
        let database = Arc::new(Database::in_memory().unwrap());
        let configuration = Arc::new(SettingsConfiguration::new(Arc::new(
            SqliteConfigStore::new(database.clone()),
        )));
        let history = Arc::new(History::new(Arc::new(SqliteHistoryRepository::new(
            database.clone(),
        ))));
        let settings = Arc::new(SettingsApplication::new(
            configuration.clone(),
            Arc::new(Boot),
            history,
            Arc::new(SqliteAppLogRepository::new(database.clone())),
        ));
        let providers = Arc::new(Providers::default());
        let speech = Arc::new(Speech::default());
        let controller = Arc::new(ServicesController::new(
            kind,
            ServicesDependencies {
                configuration: configuration.clone(),
                settings,
                providers: providers.clone(),
                tts: Arc::new(TtsRuntime::new(speech.clone(), configuration.clone())),
            },
        ));
        Fixture {
            controller,
            providers,
            speech,
            configuration,
            database,
        }
    }
    fn credentials() -> HashMap<String, String> {
        HashMap::from([
            ("api_key".into(), "fixture-api".into()),
            ("secret_key".into(), "fixture-secret".into()),
        ])
    }
    fn voice(id: &str) -> SystemTtsVoice {
        SystemTtsVoice {
            id: id.into(),
            name: id.into(),
            locale: "zh-CN".into(),
        }
    }

    #[test]
    fn services_schema_config_and_activation_errors_leave_ocr_form_retryable() {
        let f = fixture(ServicesKind::Ocr);
        assert_eq!(
            f.controller.configure("missing").unwrap_err(),
            "OCR 服务不存在"
        );
        assert!(!f.controller.projection().unwrap().busy);
        *f.providers.fail.lock().unwrap() = Some("schema");
        assert!(f
            .controller
            .configure("remote")
            .unwrap_err()
            .contains("schema failure"));
        let projection = f.controller.projection().unwrap();
        assert!(projection.error.unwrap().contains("schema failure"));
        assert!(projection.form.is_none());
        assert!(!projection.busy);
        *f.providers.fail.lock().unwrap() = None;
        f.controller.configure("remote").unwrap();
        assert!(f
            .controller
            .projection()
            .unwrap()
            .form
            .unwrap()
            .fields
            .iter()
            .all(|field| field.secret));
        assert_eq!(
            f.controller.save(HashMap::new()).unwrap_err(),
            "请填写：API Key"
        );
        assert_eq!(f.providers.count("configure"), 0);
        assert_eq!(f.providers.count("activate"), 0);
        *f.providers.fail.lock().unwrap() = Some("configure");
        assert!(f
            .controller
            .save(credentials())
            .unwrap_err()
            .contains("configure failure"));
        assert_eq!(f.providers.count("activate"), 0);
        assert!(f.controller.projection().unwrap().form.is_some());
        *f.providers.fail.lock().unwrap() = Some("activate");
        assert!(f
            .controller
            .save(credentials())
            .unwrap_err()
            .contains("activate failure"));
        assert!(f.controller.projection().unwrap().form.is_some());
        *f.providers.fail.lock().unwrap() = None;
        f.controller.save(credentials()).unwrap();
        assert!(f.controller.projection().unwrap().form.is_none());
        assert_eq!(*f.providers.credentials.lock().unwrap(), credentials());
        f.controller.configure("local").unwrap();
        assert_eq!(
            f.providers.calls.lock().unwrap().last().unwrap(),
            "activate:local"
        );
    }

    #[test]
    fn services_busy_save_excludes_duplicate_actions_and_preserves_the_form() {
        let f = fixture(ServicesKind::Ocr);
        f.controller.configure("remote").unwrap();
        let (entered, release) = f.providers.block("configure");
        let controller = f.controller.clone();
        let pending = std::thread::spawn(move || controller.save(credentials()));
        entered.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(f.controller.projection().unwrap().busy);
        f.controller.save(credentials()).unwrap();
        f.controller.activate("local").unwrap();
        f.controller.configure("local").unwrap();
        f.controller.cancel();
        assert!(f.controller.projection().unwrap().form.is_some());
        release.send(()).unwrap();
        pending.join().unwrap().unwrap();
        assert_eq!(f.providers.count("configure"), 1);
        assert_eq!(f.providers.count("activate"), 1);
        assert!(!f.controller.projection().unwrap().busy);
        assert!(f.controller.projection().unwrap().form.is_none());
    }

    #[test]
    fn services_cancel_and_disposal_reject_late_schema_completions() {
        for dispose in [false, true] {
            let f = fixture(ServicesKind::Ocr);
            let (entered, release) = f.providers.block("schema");
            let controller = f.controller.clone();
            let pending = std::thread::spawn(move || controller.configure("remote"));
            entered.recv_timeout(Duration::from_secs(5)).unwrap();
            if dispose {
                f.controller.dispose();
            } else {
                f.controller.cancel();
                f.controller.configure("local").unwrap();
            }
            release.send(()).unwrap();
            pending.join().unwrap().unwrap();
            let projection = f.controller.projection().unwrap();
            assert!(projection.form.is_none());
            assert!(projection.error.is_none());
            assert!(!projection.busy);
        }
    }

    #[tokio::test]
    async fn services_voice_edits_persist_without_overwriting_other_general_fields_and_drive_speech(
    ) {
        let f = fixture(ServicesKind::Tts);
        let mut general = f.configuration.snapshot().unwrap().general;
        general.retry_count = 4;
        general.theme = "dark".into();
        f.configuration.update_general(general).unwrap();
        f.controller
            .edit_voice("opaque-system-voice".into())
            .unwrap();
        f.controller.edit_rate(260).unwrap();
        let persisted =
            SettingsConfiguration::new(Arc::new(SqliteConfigStore::new(f.database.clone())))
                .snapshot()
                .unwrap()
                .general;
        assert_eq!(
            (
                persisted.system_tts_voice.as_str(),
                persisted.system_tts_rate
            ),
            ("opaque-system-voice", 260)
        );
        assert_eq!(
            (persisted.retry_count, persisted.theme.as_str()),
            (4, "dark")
        );
        let changes = AtomicUsize::new(0);
        f.controller
            .speak(|| {
                changes.fetch_add(1, Ordering::SeqCst);
            })
            .await;
        assert_eq!(changes.load(Ordering::SeqCst), 2);
        assert_eq!(
            f.speech.calls.lock().unwrap().as_slice(),
            &[(
                "SnapLingo 系统语音测试".into(),
                "opaque-system-voice".into(),
                260
            )]
        );
        assert_eq!(
            f.controller.projection().unwrap().status.as_deref(),
            Some("已开始朗读")
        );
        f.speech.fail.store(true, Ordering::SeqCst);
        f.controller.speak(|| {}).await;
        assert!(f
            .controller
            .projection()
            .unwrap()
            .status
            .unwrap()
            .contains("speech failure"));
        f.controller.dispose();
        f.controller.edit_voice("disposed".into()).unwrap();
        assert_eq!(
            f.configuration.snapshot().unwrap().general.system_tts_voice,
            "opaque-system-voice"
        );
    }

    #[tokio::test]
    async fn services_latest_voice_request_wins_and_disposal_ignores_late_speech() {
        let f = fixture(ServicesKind::Tts);
        *f.speech.voices.lock().unwrap() = vec![voice("older")];
        let (release, gate) = tokio::sync::oneshot::channel();
        f.speech.voice_gate.lock().unwrap().push_back(gate);
        let controller = f.controller.clone();
        let older = tokio::spawn(async move { controller.load_voices().await });
        tokio::time::timeout(Duration::from_secs(5), f.speech.voice_entered.notified())
            .await
            .unwrap();
        *f.speech.voices.lock().unwrap() = vec![voice("newer")];
        f.controller.load_voices().await;
        release.send(()).unwrap();
        older.await.unwrap();
        assert_eq!(
            f.controller.projection().unwrap().voices,
            vec![voice("newer")]
        );

        f.controller.edit_voice("chosen".into()).unwrap();
        let (release, gate) = tokio::sync::oneshot::channel();
        *f.speech.speak_gate.lock().unwrap() = Some(gate);
        let controller = f.controller.clone();
        let changes = Arc::new(AtomicUsize::new(0));
        let delivered = changes.clone();
        let pending = tokio::spawn(async move {
            controller
                .speak(|| {
                    delivered.fetch_add(1, Ordering::SeqCst);
                })
                .await
        });
        tokio::time::timeout(Duration::from_secs(5), f.speech.speak_entered.notified())
            .await
            .unwrap();
        assert!(f.controller.projection().unwrap().busy);
        f.controller
            .speak(|| panic!("Duplicate speech must not begin"))
            .await;
        assert_eq!(f.speech.calls.lock().unwrap().len(), 1);
        f.controller.dispose();
        release.send(()).unwrap();
        pending.await.unwrap();
        let projection = f.controller.projection().unwrap();
        assert!(!projection.busy);
        assert!(projection.status.is_none());
        assert!(projection.voices.is_empty());
        assert_eq!(changes.load(Ordering::SeqCst), 1);
    }
}
