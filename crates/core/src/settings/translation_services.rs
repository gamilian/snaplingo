//! Translation service settings workflows, independent of the desktop renderer.
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::providers::common::CredentialField;
use crate::providers::AddCustomTranslationProviderInput;
use crate::providers::LLMProtocol;
use crate::providers::ModelInfo;
use crate::providers::ProviderAdministration;
use crate::providers::ProviderConnectionTestInput;
use crate::providers::ProviderInfo;
use crate::providers::ProviderModelListInput;
use crate::providers::TranslationPromptStrategy;
use crate::providers::TranslationPromptStrategyConfig;
use crate::providers::UpdateCustomTranslationProviderInput;
use crate::providers::DEFAULT_PROMPT_STRATEGY_ID;
use crate::providers::SMART_PROMPT_STRATEGY_ID;

/// The settings workflow seam. Production delegates provider transactions and
/// validation to ProviderAdministration; tests replace only this outward work.
#[async_trait]
pub trait TranslationServices: Send + Sync {
    fn list(&self) -> Vec<ProviderInfo>;
    fn credential_schema(&self, id: &str) -> crate::Result<Vec<CredentialField>>;
    fn configure(&self, id: &str, credentials: HashMap<String, String>) -> crate::Result<()>;
    fn set_active(&self, id: &str, active: bool) -> crate::Result<()>;
    fn reorder(&self, ids: Vec<String>, active: bool) -> crate::Result<()>;
    fn add(&self, input: AddCustomTranslationProviderInput) -> crate::Result<()>;
    fn update(&self, id: &str, input: UpdateCustomTranslationProviderInput) -> crate::Result<()>;
    fn remove(&self, id: &str) -> crate::Result<()>;
    async fn test_saved(&self, id: &str) -> crate::Result<()>;
    async fn list_models(
        &self,
        protocol: LLMProtocol,
        input: ProviderModelListInput,
    ) -> crate::Result<Vec<ModelInfo>>;
    async fn test_connection(
        &self,
        protocol: LLMProtocol,
        input: ProviderConnectionTestInput,
    ) -> crate::Result<()>;
    fn prompt_strategies(&self) -> TranslationPromptStrategyConfig;
    fn save_prompt_strategies(
        &self,
        config: TranslationPromptStrategyConfig,
    ) -> crate::Result<TranslationPromptStrategyConfig>;
}

#[async_trait]
impl TranslationServices for ProviderAdministration {
    fn list(&self) -> Vec<ProviderInfo> {
        self.list_translation_providers()
    }
    fn credential_schema(&self, id: &str) -> crate::Result<Vec<CredentialField>> {
        self.translation_credential_schema(id.into())
    }
    fn configure(&self, id: &str, credentials: HashMap<String, String>) -> crate::Result<()> {
        self.configure_translation_provider_credentials(id.into(), credentials)
    }
    fn set_active(&self, id: &str, active: bool) -> crate::Result<()> {
        if active {
            self.activate_translation_provider(id.into())
        } else {
            self.deactivate_translation_provider(id.into())
        }
    }
    fn reorder(&self, ids: Vec<String>, active: bool) -> crate::Result<()> {
        if active {
            self.reorder_active_translation_providers(ids)
        } else {
            self.reorder_inactive_translation_providers(ids)
        }
    }
    fn add(&self, input: AddCustomTranslationProviderInput) -> crate::Result<()> {
        self.add_custom_translation_provider(input).map(|_| ())
    }
    fn update(&self, id: &str, input: UpdateCustomTranslationProviderInput) -> crate::Result<()> {
        self.update_custom_translation_provider(id.into(), input)
            .map(|_| ())
    }
    fn remove(&self, id: &str) -> crate::Result<()> {
        self.remove_custom_translation_provider(id.into())
    }
    async fn test_saved(&self, id: &str) -> crate::Result<()> {
        self.test_custom_translation_provider(id.into()).await
    }
    async fn list_models(
        &self,
        protocol: LLMProtocol,
        input: ProviderModelListInput,
    ) -> crate::Result<Vec<ModelInfo>> {
        match protocol {
            LLMProtocol::OpenAI | LLMProtocol::OpenAIResponses => {
                self.list_openai_compatible_models(input).await
            }
            LLMProtocol::Anthropic => self.list_anthropic_models(input).await,
            LLMProtocol::Gemini => self.list_gemini_models(input).await,
        }
    }
    async fn test_connection(
        &self,
        protocol: LLMProtocol,
        input: ProviderConnectionTestInput,
    ) -> crate::Result<()> {
        match protocol {
            LLMProtocol::OpenAI => self.test_openai_compatible_provider(input).await,
            LLMProtocol::OpenAIResponses => self.test_openai_responses_provider(input).await,
            LLMProtocol::Anthropic => self.test_anthropic_provider(input).await,
            LLMProtocol::Gemini => self.test_gemini_provider(input).await,
        }
    }
    fn prompt_strategies(&self) -> TranslationPromptStrategyConfig {
        self.list_translation_prompt_strategies()
    }
    fn save_prompt_strategies(
        &self,
        config: TranslationPromptStrategyConfig,
    ) -> crate::Result<TranslationPromptStrategyConfig> {
        self.save_translation_prompt_strategies(config)
    }
}

#[derive(Clone)]
pub struct TranslationCredentialForm {
    pub provider_id: String,
    pub provider_name: String,
    pub fields: Vec<CredentialField>,
}

/// API keys are draft-only and intentionally have no Debug implementation.
#[derive(Clone)]
pub struct CustomTranslationDraft {
    pub provider_id: Option<String>,
    pub name: String,
    pub protocol: LLMProtocol,
    pub endpoint: String,
    pub model: String,
    pub api_key: String,
    pub reasoning_level: String,
    pub prompt_strategy_id: String,
}
impl Default for CustomTranslationDraft {
    fn default() -> Self {
        let (endpoint, model) = protocol_defaults(LLMProtocol::OpenAI);
        Self {
            provider_id: None,
            name: String::new(),
            protocol: LLMProtocol::OpenAI,
            endpoint: endpoint.into(),
            model: model.into(),
            api_key: String::new(),
            reasoning_level: String::new(),
            prompt_strategy_id: SMART_PROMPT_STRATEGY_ID.into(),
        }
    }
}

#[derive(Clone, Default, PartialEq, Eq)]
pub struct TranslationPromptDraft {
    pub name: String,
    pub description: String,
    pub system_prompt: String,
}
#[derive(Clone)]
pub struct TranslationPromptWorkspace {
    pub strategies: TranslationPromptStrategyConfig,
    pub draft: TranslationPromptDraft,
    pub error: Option<String>,
}
#[derive(Clone)]
pub enum TranslationServiceForm {
    Credentials(TranslationCredentialForm),
    Custom {
        draft: CustomTranslationDraft,
        prompts: TranslationPromptWorkspace,
    },
}
#[derive(Clone)]
pub struct TranslationServicesProjection {
    pub providers: Vec<ProviderInfo>,
    /// Changes when a form is opened or dismissed. Pass it with form requests.
    pub form_revision: u64,
    pub form: Option<TranslationServiceForm>,
    pub busy: bool,
    pub loading_models: bool,
    pub testing_connection: bool,
    pub models: Vec<ModelInfo>,
    pub model_error: Option<String>,
    pub test_status: Option<Result<String, String>>,
    pub error: Option<String>,
}

#[derive(Clone)]
pub enum CustomTranslationEdit {
    Name(String),
    Protocol(LLMProtocol),
    Endpoint(String),
    Model(String),
    ApiKey(String),
    ReasoningLevel(String),
    PromptStrategy(String),
}
/// Remove is dispatched only after the host confirms deletion. All form
/// requests carry the revision to reject events from a dismissed/replaced form.
#[derive(Clone)]
pub enum TranslationServicesRequest {
    Configure(String),
    AddCustom,
    Cancel,
    EditCustom {
        revision: u64,
        edit: CustomTranslationEdit,
    },
    SaveCredentials {
        revision: u64,
        credentials: HashMap<String, String>,
    },
    SaveCustom {
        revision: u64,
    },
    SetActive {
        provider_id: String,
        active: bool,
    },
    Reorder {
        source_id: String,
        target_id: String,
    },
    Remove(String),
    TestSaved(String),
    ListModels {
        revision: u64,
    },
    TestConnection {
        revision: u64,
    },
    EditPrompt {
        revision: u64,
        draft: TranslationPromptDraft,
    },
    SavePrompt {
        revision: u64,
    },
    AddPrompt {
        revision: u64,
    },
    DeletePrompt {
        revision: u64,
    },
}

#[derive(Default)]
struct State {
    sequence: u64,
    form_revision: u64,
    form: Option<TranslationServiceForm>,
    schema: Option<u64>,
    mutation: Option<u64>,
    models_request: Option<u64>,
    test_request: Option<u64>,
    models: Vec<ModelInfo>,
    model_error: Option<String>,
    test_status: Option<Result<String, String>>,
    error: Option<String>,
    disposed: bool,
}
impl State {
    fn next(&mut self) -> u64 {
        self.sequence += 1;
        self.sequence
    }
    fn clear_introspection(&mut self) {
        self.models_request = None;
        self.test_request = None;
        self.models.clear();
        self.model_error = None;
        self.test_status = None;
    }
    fn close(&mut self) {
        self.form_revision = self.next();
        self.form = None;
        self.schema = None;
        self.clear_introspection();
        self.error = None;
    }
    fn form_matches(&self, revision: u64) -> bool {
        !self.disposed && self.form_revision == revision && self.form.is_some()
    }
}

pub struct TranslationServicesController {
    services: Arc<dyn TranslationServices>,
    state: Mutex<State>,
}
impl TranslationServicesController {
    /// Apply ordered UI draft changes before scheduling slow requests. Returns
    /// false for requests that must run through the asynchronous dispatcher.
    pub fn apply(&self, request: &TranslationServicesRequest) -> bool {
        match request {
            TranslationServicesRequest::EditCustom { revision, edit } => {
                self.edit_custom(*revision, edit.clone())
            }
            TranslationServicesRequest::EditPrompt { revision, draft } => {
                let mut state = self.state.lock().unwrap();
                if state.form_matches(*revision) && state.mutation.is_none() {
                    if let Some(TranslationServiceForm::Custom { prompts, .. }) =
                        state.form.as_mut()
                    {
                        prompts.draft = draft.clone();
                        prompts.error = None;
                    }
                }
            }
            TranslationServicesRequest::Cancel => {
                let mut state = self.state.lock().unwrap();
                if state.mutation.is_none() {
                    state.close();
                }
            }
            _ => return false,
        }
        true
    }
    pub fn new(services: Arc<dyn TranslationServices>) -> Self {
        Self {
            services,
            state: Mutex::new(State::default()),
        }
    }
    pub fn projection(&self) -> TranslationServicesProjection {
        let providers = self.services.list();
        let state = self.state.lock().unwrap();
        TranslationServicesProjection {
            providers,
            form_revision: state.form_revision,
            form: state.form.clone(),
            busy: state.schema.is_some() || state.mutation.is_some(),
            loading_models: state.models_request.is_some(),
            testing_connection: state.test_request.is_some(),
            models: state.models.clone(),
            model_error: state.model_error.clone(),
            test_status: state.test_status.clone(),
            error: state.error.clone(),
        }
    }
    /// The host executes this on a worker and refreshes its projection on the
    /// UI thread when `changed` runs. No OS or rendering state lives here.
    pub async fn dispatch(
        &self,
        request: TranslationServicesRequest,
        changed: impl Fn(),
    ) -> Result<(), String> {
        if self.apply(&request) {
            if !self.state.lock().unwrap().disposed {
                changed();
            }
            return Ok(());
        }
        let result = match request {
            TranslationServicesRequest::Configure(id) => self.open(Some(id), false, &changed),
            TranslationServicesRequest::AddCustom => self.open(None, true, &changed),
            TranslationServicesRequest::Cancel => {
                let mut state = self.state.lock().unwrap();
                if state.mutation.is_none() {
                    state.close();
                }
                Ok(())
            }
            TranslationServicesRequest::EditCustom { revision, edit } => {
                self.edit_custom(revision, edit);
                Ok(())
            }
            TranslationServicesRequest::SaveCredentials {
                revision,
                credentials,
            } => self.save_credentials(revision, credentials, &changed),
            TranslationServicesRequest::SaveCustom { revision } => {
                self.save_custom(revision, &changed)
            }
            TranslationServicesRequest::SetActive {
                provider_id,
                active,
            } => self.mutate(
                None,
                &changed,
                || self.services.set_active(&provider_id, active),
                false,
            ),
            TranslationServicesRequest::Reorder {
                source_id,
                target_id,
            } => self.reorder(&source_id, &target_id, &changed),
            TranslationServicesRequest::Remove(id) => {
                self.mutate(None, &changed, || self.services.remove(&id), false)
            }
            TranslationServicesRequest::TestSaved(id) => self.test_saved(id, &changed).await,
            TranslationServicesRequest::ListModels { revision } => {
                self.list_models(revision, &changed).await
            }
            TranslationServicesRequest::TestConnection { revision } => {
                self.test_connection(revision, &changed).await
            }
            TranslationServicesRequest::EditPrompt { revision, draft } => {
                let mut state = self.state.lock().unwrap();
                if state.form_matches(revision) && state.mutation.is_none() {
                    if let Some(TranslationServiceForm::Custom { prompts, .. }) =
                        state.form.as_mut()
                    {
                        prompts.draft = draft;
                        prompts.error = None;
                    }
                }
                Ok(())
            }
            TranslationServicesRequest::SavePrompt { revision } => {
                self.save_prompt(revision, PromptOperation::Save, &changed)
            }
            TranslationServicesRequest::AddPrompt { revision } => {
                self.save_prompt(revision, PromptOperation::Add, &changed)
            }
            TranslationServicesRequest::DeletePrompt { revision } => {
                self.save_prompt(revision, PromptOperation::Delete, &changed)
            }
        };
        let alive = !self.state.lock().unwrap().disposed;
        if alive {
            changed();
        }
        result
    }
    pub fn dispose(&self) {
        let mut state = self.state.lock().unwrap();
        state.close();
        state.mutation = None;
        state.disposed = true;
    }

    fn open(&self, id: Option<String>, custom: bool, changed: &impl Fn()) -> Result<(), String> {
        let token = {
            let mut state = self.state.lock().unwrap();
            if state.disposed || state.mutation.is_some() {
                return Ok(());
            }
            state.close();
            let token = state.next();
            state.schema = Some(token);
            token
        };
        changed();
        let result: Result<TranslationServiceForm, String> = (|| {
            let provider = id
                .as_ref()
                .map(|id| {
                    self.services
                        .list()
                        .into_iter()
                        .find(|provider| &provider.id == id)
                        .ok_or_else(|| "翻译服务不存在".to_string())
                })
                .transpose()?;
            if custom
                || provider
                    .as_ref()
                    .is_some_and(|provider| !provider.is_builtin)
            {
                let mut draft = CustomTranslationDraft::default();
                if let Some(provider) = provider {
                    draft = CustomTranslationDraft {
                        provider_id: Some(provider.id),
                        name: provider.name,
                        protocol: parse_protocol(provider.protocol.as_deref()),
                        endpoint: provider.endpoint.unwrap_or(draft.endpoint),
                        model: provider.model.unwrap_or(draft.model),
                        api_key: String::new(),
                        reasoning_level: provider.reasoning_level.unwrap_or_default(),
                        prompt_strategy_id: provider
                            .prompt_strategy_id
                            .unwrap_or(SMART_PROMPT_STRATEGY_ID.into()),
                    };
                }
                let mut prompts = TranslationPromptWorkspace {
                    strategies: self.services.prompt_strategies(),
                    draft: TranslationPromptDraft::default(),
                    error: None,
                };
                select_prompt(&mut prompts, &draft.prompt_strategy_id);
                Ok(TranslationServiceForm::Custom { draft, prompts })
            } else {
                let provider = provider.unwrap();
                let fields = self
                    .services
                    .credential_schema(&provider.id)
                    .map_err(|e| e.to_string())?;
                Ok(TranslationServiceForm::Credentials(
                    TranslationCredentialForm {
                        provider_id: provider.id,
                        provider_name: provider.name,
                        fields,
                    },
                ))
            }
        })();
        let mut state = self.state.lock().unwrap();
        if state.disposed || state.schema != Some(token) {
            return Ok(());
        }
        state.schema = None;
        match result {
            Ok(form) => {
                state.form = Some(form);
                Ok(())
            }
            Err(error) => {
                state.error = Some(error.clone());
                Err(error)
            }
        }
    }
    fn edit_custom(&self, revision: u64, edit: CustomTranslationEdit) {
        let mut state = self.state.lock().unwrap();
        if !state.form_matches(revision) || state.mutation.is_some() {
            return;
        }
        let mut invalidate_models = false;
        let mut invalidate_test = false;
        if let Some(TranslationServiceForm::Custom { draft, prompts }) = state.form.as_mut() {
            match edit {
                CustomTranslationEdit::Name(value) => draft.name = value,
                CustomTranslationEdit::Protocol(value) => {
                    if draft.protocol != value {
                        draft.protocol = value;
                        let (endpoint, model) = protocol_defaults(value);
                        draft.endpoint = endpoint.into();
                        draft.model = model.into();
                        invalidate_models = true;
                    }
                }
                CustomTranslationEdit::Endpoint(value) => {
                    draft.endpoint = value;
                    invalidate_models = true;
                }
                CustomTranslationEdit::ApiKey(value) => {
                    draft.api_key = value;
                    invalidate_models = true;
                }
                CustomTranslationEdit::Model(value) => {
                    draft.model = value;
                    invalidate_test = true;
                }
                CustomTranslationEdit::ReasoningLevel(value) => draft.reasoning_level = value,
                CustomTranslationEdit::PromptStrategy(value) => {
                    draft.prompt_strategy_id = value;
                    select_prompt(prompts, &draft.prompt_strategy_id);
                }
            }
        }
        state.error = None;
        if invalidate_models {
            state.clear_introspection();
        } else if invalidate_test {
            state.test_request = None;
            state.test_status = None;
        }
    }
    fn begin_mutation(&self, revision: Option<u64>) -> Option<u64> {
        let mut state = self.state.lock().unwrap();
        if state.disposed
            || state.mutation.is_some()
            || state.schema.is_some()
            || revision.is_some_and(|revision| !state.form_matches(revision))
        {
            return None;
        }
        let token = state.next();
        state.mutation = Some(token);
        state.error = None;
        Some(token)
    }
    fn finish_mutation(
        &self,
        token: u64,
        result: Result<(), String>,
        close: bool,
    ) -> Result<(), String> {
        let mut state = self.state.lock().unwrap();
        if state.disposed || state.mutation != Some(token) {
            return Ok(());
        }
        state.mutation = None;
        if result.is_ok() && close {
            state.close();
        } else {
            state.error = result.as_ref().err().cloned();
        }
        result
    }
    fn mutate(
        &self,
        revision: Option<u64>,
        changed: &impl Fn(),
        work: impl FnOnce() -> crate::Result<()>,
        close: bool,
    ) -> Result<(), String> {
        let Some(token) = self.begin_mutation(revision) else {
            return Ok(());
        };
        changed();
        self.finish_mutation(token, work().map_err(|e| e.to_string()), close)
    }
    fn save_credentials(
        &self,
        revision: u64,
        mut credentials: HashMap<String, String>,
        changed: &impl Fn(),
    ) -> Result<(), String> {
        let form = {
            let state = self.state.lock().unwrap();
            if !state.form_matches(revision) {
                return Ok(());
            }
            match state.form.as_ref() {
                Some(TranslationServiceForm::Credentials(form)) => form.clone(),
                _ => return Ok(()),
            }
        };
        // DeepLX's three schema fields describe two mutually exclusive modes.
        if form.provider_id == "deeplx" {
            let mode = credentials
                .get("mode")
                .map(String::as_str)
                .unwrap_or("deeplx");
            credentials = if mode == "deepl" {
                HashMap::from([
                    ("mode".into(), "deepl".into()),
                    (
                        "api_key".into(),
                        credentials.remove("api_key").unwrap_or_default(),
                    ),
                ])
            } else {
                HashMap::from([
                    ("mode".into(), mode.into()),
                    (
                        "endpoint".into(),
                        credentials.remove("endpoint").unwrap_or_default(),
                    ),
                ])
            };
        }
        self.mutate(
            Some(revision),
            changed,
            || {
                self.services
                    .configure(&form.provider_id, credentials)
                    .and_then(|_| self.services.set_active(&form.provider_id, true))
            },
            true,
        )
    }
    fn custom(&self, revision: u64) -> Option<CustomTranslationDraft> {
        let state = self.state.lock().unwrap();
        if !state.form_matches(revision) {
            return None;
        }
        match state.form.as_ref() {
            Some(TranslationServiceForm::Custom { draft, .. }) => Some(draft.clone()),
            _ => None,
        }
    }
    fn save_custom(&self, revision: u64, changed: &impl Fn()) -> Result<(), String> {
        let Some(token) = self.begin_mutation(Some(revision)) else {
            return Ok(());
        };
        let Some(draft) = self.custom(revision) else {
            return self.finish_mutation(token, Ok(()), false);
        };
        changed();
        let reasoning = (!draft.reasoning_level.is_empty()).then_some(draft.reasoning_level);
        let result = (|| {
            if let Some(id) = draft.provider_id {
                self.services
                    .update(
                        &id,
                        UpdateCustomTranslationProviderInput {
                            name: draft.name.trim().into(),
                            protocol: draft.protocol.as_str().into(),
                            endpoint: draft.endpoint.trim().into(),
                            model: draft.model.trim().into(),
                            api_key: (!draft.api_key.trim().is_empty())
                                .then(|| draft.api_key.trim().into()),
                            reasoning_level: reasoning,
                            prompt_strategy_id: Some(draft.prompt_strategy_id),
                            prompt_fallback_strategy_id: Some(DEFAULT_PROMPT_STRATEGY_ID.into()),
                        },
                    )
                    .and_then(|_| self.services.set_active(&id, true))
            } else {
                self.services.add(AddCustomTranslationProviderInput {
                    name: draft.name.trim().into(),
                    protocol: draft.protocol.as_str().into(),
                    endpoint: draft.endpoint.trim().into(),
                    model: draft.model.trim().into(),
                    api_key: draft.api_key.trim().into(),
                    reasoning_level: reasoning,
                    prompt_strategy_id: Some(draft.prompt_strategy_id),
                    prompt_fallback_strategy_id: Some(DEFAULT_PROMPT_STRATEGY_ID.into()),
                })
            }
        })()
        .map_err(|error: crate::AppError| error.to_string());
        self.finish_mutation(token, result, true)
    }
    fn reorder(&self, source: &str, target: &str, changed: &impl Fn()) -> Result<(), String> {
        self.mutate(
            None,
            changed,
            || {
                let providers = self.services.list();
                let Some(source_provider) = providers.iter().find(|provider| provider.id == source)
                else {
                    return Ok(());
                };
                let Some(target_provider) = providers.iter().find(|provider| provider.id == target)
                else {
                    return Ok(());
                };
                if source == target || source_provider.is_active != target_provider.is_active {
                    return Ok(());
                }
                let active = source_provider.is_active;
                let mut ids = providers
                    .iter()
                    .filter(|provider| provider.is_active == active)
                    .map(|provider| provider.id.clone())
                    .collect::<Vec<_>>();
                let from = ids.iter().position(|id| id == source).unwrap();
                let to = ids.iter().position(|id| id == target).unwrap();
                let id = ids.remove(from);
                ids.insert(to, id);
                self.services.reorder(ids, active)
            },
            false,
        )
    }
    async fn test_saved(&self, id: String, changed: &impl Fn()) -> Result<(), String> {
        let Some(token) = self.begin_mutation(None) else {
            return Ok(());
        };
        changed();
        let result = self
            .services
            .test_saved(&id)
            .await
            .map_err(|e| e.to_string());
        let mut state = self.state.lock().unwrap();
        if state.disposed || state.mutation != Some(token) {
            return Ok(());
        }
        state.mutation = None;
        state.error = result.as_ref().err().cloned();
        state.test_status = Some(result.clone().map(|_| "检测成功".into()));
        result
    }
    fn begin_introspection(
        &self,
        revision: u64,
        models: bool,
    ) -> Option<(u64, CustomTranslationDraft)> {
        let mut state = self.state.lock().unwrap();
        if !state.form_matches(revision)
            || state.mutation.is_some()
            || if models {
                state.models_request.is_some()
            } else {
                state.test_request.is_some()
            }
        {
            return None;
        }
        let Some(TranslationServiceForm::Custom { draft, .. }) = state.form.as_ref() else {
            return None;
        };
        if draft.endpoint.trim().is_empty()
            || draft.api_key.trim().is_empty()
            || (!models && draft.model.trim().is_empty())
        {
            return None;
        }
        let draft = draft.clone();
        let token = state.next();
        if models {
            state.models_request = Some(token);
            state.model_error = None;
        } else {
            state.test_request = Some(token);
        }
        state.test_status = None;
        Some((token, draft))
    }
    async fn list_models(&self, revision: u64, changed: &impl Fn()) -> Result<(), String> {
        let Some((token, draft)) = self.begin_introspection(revision, true) else {
            return Ok(());
        };
        changed();
        let result = self
            .services
            .list_models(
                draft.protocol,
                ProviderModelListInput {
                    endpoint: draft.endpoint.trim().into(),
                    api_key: draft.api_key.trim().into(),
                },
            )
            .await;
        let mut state = self.state.lock().unwrap();
        if !state.form_matches(revision) || state.models_request != Some(token) {
            return Ok(());
        }
        state.models_request = None;
        match result {
            Ok(models) => {
                state.model_error = models.is_empty().then(|| "未返回可用模型".into());
                state.models = models;
            }
            Err(error) => {
                state.models.clear();
                state.model_error = Some(format!("获取模型失败: {error}"));
            }
        }
        Ok(())
    }
    async fn test_connection(&self, revision: u64, changed: &impl Fn()) -> Result<(), String> {
        let Some((token, draft)) = self.begin_introspection(revision, false) else {
            return Ok(());
        };
        changed();
        let result = self
            .services
            .test_connection(
                draft.protocol,
                ProviderConnectionTestInput {
                    endpoint: draft.endpoint.trim().into(),
                    api_key: draft.api_key.trim().into(),
                    model: draft.model.trim().into(),
                },
            )
            .await
            .map(|_| "检测成功".into())
            .map_err(|e| format!("检测失败: {e}"));
        let mut state = self.state.lock().unwrap();
        if state.form_matches(revision) && state.test_request == Some(token) {
            state.test_request = None;
            state.test_status = Some(result);
        }
        Ok(())
    }
    fn save_prompt(
        &self,
        revision: u64,
        operation: PromptOperation,
        changed: &impl Fn(),
    ) -> Result<(), String> {
        let (token, mut config, selected, draft) = {
            let mut state = self.state.lock().unwrap();
            if !state.form_matches(revision) || state.mutation.is_some() {
                return Ok(());
            }
            let Some(TranslationServiceForm::Custom { draft, prompts }) = state.form.as_ref()
            else {
                return Ok(());
            };
            let data = (
                prompts.strategies.clone(),
                draft.prompt_strategy_id.clone(),
                prompts.draft.clone(),
            );
            let token = state.next();
            state.mutation = Some(token);
            state.error = None;
            (token, data.0, data.1, data.2)
        };
        let next_id = match operation {
            PromptOperation::Delete => {
                let Some(strategy) = config
                    .strategies
                    .iter()
                    .find(|strategy| strategy.id == selected)
                else {
                    return self.finish_mutation(token, Ok(()), false);
                };
                if !strategy.is_deletable {
                    return self.finish_mutation(token, Ok(()), false);
                }
                config.strategies.retain(|strategy| strategy.id != selected);
                DEFAULT_PROMPT_STRATEGY_ID.into()
            }
            PromptOperation::Save | PromptOperation::Add => {
                if draft.name.trim().is_empty() || draft.system_prompt.trim().is_empty() {
                    let mut state = self.state.lock().unwrap();
                    if let Some(TranslationServiceForm::Custom { prompts, .. }) =
                        state.form.as_mut()
                    {
                        prompts.error = Some("策略名称和系统提示词不能为空".into());
                    }
                    state.mutation = None;
                    return Ok(());
                }
                match operation {
                    PromptOperation::Save => {
                        let Some(strategy) = config
                            .strategies
                            .iter_mut()
                            .find(|strategy| strategy.id == selected)
                        else {
                            return self.finish_mutation(token, Ok(()), false);
                        };
                        strategy.name = draft.name.trim().into();
                        strategy.description = draft.description.trim().into();
                        strategy.system_prompt = draft.system_prompt.trim().into();
                        selected
                    }
                    _ => {
                        let id = custom_prompt_id();
                        config.strategies.push(TranslationPromptStrategy {
                            id: id.clone(),
                            name: draft.name.trim().into(),
                            description: draft.description.trim().into(),
                            system_prompt: draft.system_prompt.trim().into(),
                            is_builtin: false,
                            is_deletable: true,
                        });
                        id
                    }
                }
            }
        };
        changed();
        let result = self
            .services
            .save_prompt_strategies(config)
            .map_err(|e| format!("保存策略失败: {e}"));
        let mut state = self.state.lock().unwrap();
        if state.disposed || state.mutation != Some(token) {
            return Ok(());
        }
        state.mutation = None;
        if let Some(TranslationServiceForm::Custom { draft, prompts }) = state.form.as_mut() {
            match result {
                Ok(config) => {
                    prompts.strategies = config;
                    prompts.error = None;
                    draft.prompt_strategy_id = next_id;
                    select_prompt(prompts, &draft.prompt_strategy_id);
                }
                Err(error) => prompts.error = Some(error),
            }
        }
        Ok(())
    }
}
enum PromptOperation {
    Save,
    Add,
    Delete,
}
fn select_prompt(workspace: &mut TranslationPromptWorkspace, id: &str) {
    workspace.error = None;
    workspace.draft = workspace
        .strategies
        .strategies
        .iter()
        .find(|strategy| strategy.id == id)
        .map(|strategy| TranslationPromptDraft {
            name: strategy.name.clone(),
            description: strategy.description.clone(),
            system_prompt: strategy.system_prompt.clone(),
        })
        .unwrap_or_default();
}
fn protocol_defaults(protocol: LLMProtocol) -> (&'static str, &'static str) {
    match protocol {
        LLMProtocol::OpenAI => ("https://api.openai.com", "gpt-4o"),
        LLMProtocol::OpenAIResponses => ("https://api.openai.com", "gpt-5-mini"),
        LLMProtocol::Anthropic => ("https://api.anthropic.com", "claude-3-5-sonnet-latest"),
        LLMProtocol::Gemini => (
            "https://generativelanguage.googleapis.com",
            "gemini-1.5-flash",
        ),
    }
}
fn parse_protocol(protocol: Option<&str>) -> LLMProtocol {
    match protocol {
        Some("openai-responses") => LLMProtocol::OpenAIResponses,
        Some("anthropic") => LLMProtocol::Anthropic,
        Some("gemini") => LLMProtocol::Gemini,
        _ => LLMProtocol::OpenAI,
    }
}
fn custom_prompt_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!(
        "custom-{}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}
