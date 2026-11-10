//! Settings service actions retain the existing Rust application services.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::providers::common::CredentialField;
use crate::providers::OcrProviderInfo;
use crate::providers::ProviderAdministration;
use crate::settings::model::GeneralSettings;
use crate::settings::GeneralField;
use crate::settings::GeneralValue;
use crate::settings::SettingsApplication;
use crate::settings::SettingsConfiguration;
use crate::speech::SystemTtsVoice;
use crate::speech::TtsRuntime;

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
