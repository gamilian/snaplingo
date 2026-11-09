//! Projects the shared Services canvas onto the existing portable workflows.
use std::sync::{Arc, Mutex};

use snaplingo_core::application::settings::{
    self as core, CustomTranslationEdit, ServicesController, TranslationServiceForm,
    TranslationServicesController, TranslationServicesRequest,
};
use snaplingo_ui::{services as ui, Page, Snapshot};

pub trait ServicesNavigation: Send + Sync {
    fn load(&self) -> Result<Option<String>, String>;
    fn save(&self, route: &str) -> Result<(), String>;
}

pub enum ServicesHostEffect {
    Navigate(Page),
    ConfigureCapacity,
    ConfirmRemove(String),
    CopyText(String),
    ReadClipboard,
    OpenUrl(String),
}
struct State {
    route: ui::ServicesRoute,
    revision: u64,
    ocr_revision: u64,
    disposed: bool,
}
pub struct ServicesSession {
    ocr: Arc<ServicesController>,
    tts: Arc<ServicesController>,
    translation: Arc<TranslationServicesController>,
    state: Mutex<State>,
    navigation: Option<Arc<dyn ServicesNavigation>>,
}
impl ServicesSession {
    pub fn new(
        ocr: Arc<ServicesController>,
        tts: Arc<ServicesController>,
        translation: Arc<TranslationServicesController>,
        route: ui::ServicesRoute,
    ) -> Self {
        Self {
            ocr,
            tts,
            translation,
            state: Mutex::new(State {
                route,
                revision: 0,
                ocr_revision: 1,
                disposed: false,
            }),
            navigation: None,
        }
    }
    pub fn with_navigation(mut self, navigation: Arc<dyn ServicesNavigation>) -> Self {
        if let Ok(Some(saved)) = navigation.load() {
            if let Some(route) = saved_route(&saved) {
                self.state.get_mut().unwrap().route = route;
            }
        }
        self.navigation = Some(navigation);
        self
    }
    pub fn snapshot(&self, shell: Snapshot) -> Result<ui::ServicesSnapshot, String> {
        let (route, revision, ocr_revision) = {
            let mut state = self.state.lock().unwrap();
            state.revision += 1;
            (state.route, state.revision, state.ocr_revision)
        };
        let mut snapshot = ui::ServicesSnapshot {
            shell,
            route,
            revision,
            ..Default::default()
        };
        match route {
            ui::ServicesRoute::Translation => {
                let source = self.translation.projection();
                snapshot.providers = source
                    .providers
                    .into_iter()
                    .map(|provider| {
                        let name =
                            if !provider.is_builtin && provider.name.starts_with("custom-llm-") {
                                provider.model.clone().unwrap_or(provider.name)
                            } else {
                                provider.name
                            };
                        ui::ServiceProvider {
                            id: provider.id,
                            name,
                            configured: provider.is_configured,
                            active: provider.is_active,
                            builtin: provider.is_builtin,
                            local: provider.is_local,
                            requires_api_key: provider.requires_api_key,
                            description: String::new(),
                            acceleration: None,
                            protocol: provider.protocol.as_deref().map(protocol),
                            endpoint: provider.endpoint,
                            model: provider.model,
                        }
                    })
                    .collect();
                snapshot.form_revision = source.form_revision;
                snapshot.form = source.form.map(project_form);
                snapshot.busy = source.busy;
                snapshot.loading_models = source.loading_models;
                snapshot.testing_connection = source.testing_connection;
                snapshot.models = source.models.into_iter().map(|model| model.id).collect();
                snapshot.model_error = source.model_error;
                snapshot.test_status = source.test_status;
                snapshot.error = source.error;
            }
            ui::ServicesRoute::Ocr | ui::ServicesRoute::Tts => {
                let source = if route == ui::ServicesRoute::Ocr {
                    self.ocr.projection()?
                } else {
                    self.tts.projection()?
                };
                snapshot.providers = source
                    .providers
                    .into_iter()
                    .map(|provider| ui::ServiceProvider {
                        id: provider.id,
                        name: provider.name,
                        configured: provider.is_configured,
                        active: provider.is_active,
                        builtin: true,
                        local: provider.is_local,
                        requires_api_key: provider.requires_api_key,
                        description: String::new(),
                        acceleration: provider.acceleration.map(|status| {
                            status
                                .fallback_reason
                                .unwrap_or_else(|| match status.device {
                                    Some(
                                        snaplingo_core::application::providers::ocr::OcrExecutionDevice::Gpu,
                                    ) => "GPU 加速模式".into(),
                                    Some(_) => "CPU 模式".into(),
                                    None if status.gpu_enabled => "自动选择设备".into(),
                                    _ => "CPU 模式".into(),
                                })
                        }),
                        protocol: None,
                        endpoint: None,
                        model: None,
                    })
                    .collect();
                snapshot.voices = source
                    .voices
                    .into_iter()
                    .map(|voice| ui::ServiceVoice {
                        id: voice.id,
                        name: voice.name,
                        locale: voice.locale,
                    })
                    .collect();
                snapshot.voice_id = source.general.system_tts_voice;
                snapshot.rate = source.general.system_tts_rate.into();
                snapshot.busy = source.busy;
                snapshot.status = source.status;
                snapshot.error = source.error;
                snapshot.form_revision = ocr_revision;
                snapshot.form = source.form.map(|form| ui::ServiceForm::Credentials {
                    provider_id: form.provider_id,
                    name: form.provider_name,
                    fields: form.fields.into_iter().map(credential).collect(),
                });
            }
        }
        Ok(snapshot)
    }
    pub fn dispose(&self) {
        self.state.lock().unwrap().disposed = true;
        self.ocr.dispose();
        self.tts.dispose();
        self.translation.dispose();
    }
    /// Apply immediate UI events in delivery order. After Route, dispatch
    /// Refresh on a worker to load installed voices for the current route.
    pub fn edit(&self, intent: &ui::ServicesIntent) -> Result<bool, String> {
        if self.state.lock().unwrap().disposed {
            return Ok(true);
        }
        match intent {
            ui::ServicesIntent::Route(route) => {
                if let Some(navigation) = &self.navigation {
                    navigation
                        .save(match route {
                            ui::ServicesRoute::Ocr => "ocr",
                            ui::ServicesRoute::Translation => "translation",
                            ui::ServicesRoute::Tts => "tts",
                        })
                        .map_err(|error| error.to_string())?;
                }
                self.ocr.cancel();
                self.translation.apply(&TranslationServicesRequest::Cancel);
                let mut state = self.state.lock().unwrap();
                state.route = *route;
                state.ocr_revision += 1;
                Ok(true)
            }
            ui::ServicesIntent::CancelForm => {
                self.ocr.cancel();
                self.translation.apply(&TranslationServicesRequest::Cancel);
                self.state.lock().unwrap().ocr_revision += 1;
                Ok(true)
            }
            ui::ServicesIntent::Voice(id) => {
                self.tts.edit_voice(id.clone())?;
                Ok(true)
            }
            ui::ServicesIntent::Rate(rate) => {
                self.tts.edit_rate(*rate)?;
                Ok(true)
            }
            ui::ServicesIntent::EditCustom { .. }
            | ui::ServicesIntent::Protocol { .. }
            | ui::ServicesIntent::Reasoning { .. }
            | ui::ServicesIntent::PromptStrategy { .. }
            | ui::ServicesIntent::EditPrompt { .. } => {
                self.translation
                    .apply(&translation_request(intent.clone())?);
                Ok(true)
            }
            _ => Ok(false),
        }
    }
    pub async fn dispatch(
        &self,
        intent: ui::ServicesIntent,
        changed: impl Fn(),
    ) -> Result<Vec<ServicesHostEffect>, String> {
        let result = self.dispatch_inner(intent, &changed).await;
        let alive = !self.state.lock().unwrap().disposed;
        if alive {
            changed();
        }
        result
    }
    async fn dispatch_inner(
        &self,
        intent: ui::ServicesIntent,
        changed: &impl Fn(),
    ) -> Result<Vec<ServicesHostEffect>, String> {
        if self.state.lock().unwrap().disposed {
            return Ok(vec![]);
        }
        if self.edit(&intent)? {
            if matches!(intent, ui::ServicesIntent::Route(_)) {
                self.refresh().await;
            }
            return Ok(vec![]);
        }
        match intent {
            ui::ServicesIntent::Navigate(page) => {
                return Ok(vec![ServicesHostEffect::Navigate(page)])
            }
            ui::ServicesIntent::ConfigureCapacity => {
                return Ok(vec![ServicesHostEffect::ConfigureCapacity])
            }
            ui::ServicesIntent::ConfirmRemove(id) => {
                return Ok(vec![ServicesHostEffect::ConfirmRemove(id)])
            }
            ui::ServicesIntent::CopyText(text) => {
                return Ok(vec![ServicesHostEffect::CopyText(text)])
            }
            ui::ServicesIntent::ReadClipboard => {
                return Ok(vec![ServicesHostEffect::ReadClipboard])
            }
            ui::ServicesIntent::OpenUrl(url) => return Ok(vec![ServicesHostEffect::OpenUrl(url)]),
            ui::ServicesIntent::Refresh => self.refresh().await,
            ui::ServicesIntent::Configure {
                route: ui::ServicesRoute::Ocr,
                provider_id,
            } => {
                {
                    let mut state = self.state.lock().unwrap();
                    if state.route != ui::ServicesRoute::Ocr {
                        return Ok(vec![]);
                    }
                    state.ocr_revision += 1;
                }
                self.ocr.configure(&provider_id)?;
            }
            ui::ServicesIntent::SetActive {
                route: ui::ServicesRoute::Ocr,
                provider_id,
                active,
            } => {
                if active {
                    self.ocr.activate(&provider_id)?;
                }
            }
            ui::ServicesIntent::SaveCredentials {
                route: ui::ServicesRoute::Ocr,
                revision,
                provider_id,
                credentials,
            } => {
                let current = self.ocr.projection()?;
                if self.state.lock().unwrap().ocr_revision == revision
                    && current
                        .form
                        .is_some_and(|form| form.provider_id == provider_id)
                {
                    self.ocr.save(credentials.into_iter().collect())?;
                }
            }
            ui::ServicesIntent::Speak => self.tts.speak(changed).await,
            other => {
                if self.state.lock().unwrap().route != ui::ServicesRoute::Translation {
                    return Ok(vec![]);
                }
                self.translation
                    .dispatch(translation_request(other)?, changed)
                    .await?
            }
        }
        Ok(vec![])
    }
    async fn refresh(&self) {
        let route = self.state.lock().unwrap().route;
        if route == ui::ServicesRoute::Tts {
            self.tts.load_voices().await;
        }
    }
    /// The host confirms removal before invoking this method.
    pub async fn remove_confirmed(&self, id: String, changed: impl Fn()) -> Result<(), String> {
        if self.state.lock().unwrap().disposed {
            return Ok(());
        }
        self.translation
            .dispatch(TranslationServicesRequest::Remove(id), changed)
            .await
    }
}
fn translation_request(intent: ui::ServicesIntent) -> Result<TranslationServicesRequest, String> {
    Ok(match intent {
        ui::ServicesIntent::Configure { provider_id, .. } => {
            TranslationServicesRequest::Configure(provider_id)
        }
        ui::ServicesIntent::SetActive {
            provider_id,
            active,
            ..
        } => TranslationServicesRequest::SetActive {
            provider_id,
            active,
        },
        ui::ServicesIntent::AddCustom => TranslationServicesRequest::AddCustom,
        ui::ServicesIntent::CancelForm => TranslationServicesRequest::Cancel,
        ui::ServicesIntent::SaveCredentials {
            revision,
            credentials,
            ..
        } => TranslationServicesRequest::SaveCredentials {
            revision,
            credentials: credentials.into_iter().collect(),
        },
        ui::ServicesIntent::EditCustom {
            revision,
            field,
            value,
        } => TranslationServicesRequest::EditCustom {
            revision,
            edit: match field {
                ui::ServiceCustomField::Name => CustomTranslationEdit::Name(value),
                ui::ServiceCustomField::Endpoint => CustomTranslationEdit::Endpoint(value),
                ui::ServiceCustomField::ApiKey => CustomTranslationEdit::ApiKey(value),
                ui::ServiceCustomField::Model => CustomTranslationEdit::Model(value),
                _ => return Err("提示词草稿使用独立编辑操作".into()),
            },
        },
        ui::ServicesIntent::Protocol { revision, protocol } => {
            TranslationServicesRequest::EditCustom {
                revision,
                edit: CustomTranslationEdit::Protocol(core_protocol(protocol)),
            }
        }
        ui::ServicesIntent::Reasoning { revision, value } => {
            TranslationServicesRequest::EditCustom {
                revision,
                edit: CustomTranslationEdit::ReasoningLevel(value),
            }
        }
        ui::ServicesIntent::PromptStrategy {
            revision,
            provider_strategy_id,
        } => TranslationServicesRequest::EditCustom {
            revision,
            edit: CustomTranslationEdit::PromptStrategy(provider_strategy_id),
        },
        ui::ServicesIntent::SaveCustom { revision } => {
            TranslationServicesRequest::SaveCustom { revision }
        }
        ui::ServicesIntent::Reorder {
            source_id,
            target_id,
        } => TranslationServicesRequest::Reorder {
            source_id,
            target_id,
        },
        ui::ServicesIntent::TestSaved(id) => TranslationServicesRequest::TestSaved(id),
        ui::ServicesIntent::LoadModels { revision } => {
            TranslationServicesRequest::ListModels { revision }
        }
        ui::ServicesIntent::TestConnection { revision } => {
            TranslationServicesRequest::TestConnection { revision }
        }
        ui::ServicesIntent::EditPrompt { revision, draft } => {
            TranslationServicesRequest::EditPrompt {
                revision,
                draft: core::TranslationPromptDraft {
                    name: draft.name,
                    description: draft.description,
                    system_prompt: draft.system_prompt,
                },
            }
        }
        ui::ServicesIntent::SavePrompt { revision } => {
            TranslationServicesRequest::SavePrompt { revision }
        }
        ui::ServicesIntent::AddPrompt { revision } => {
            TranslationServicesRequest::AddPrompt { revision }
        }
        ui::ServicesIntent::DeletePrompt { revision } => {
            TranslationServicesRequest::DeletePrompt { revision }
        }
        _ => return Err("此服务操作由窗口宿主执行".into()),
    })
}

fn credential(
    field: snaplingo_core::application::providers::common::CredentialField,
) -> ui::ServiceCredentialField {
    ui::ServiceCredentialField {
        name: field.name,
        label: field.label,
        secret: field.secret,
    }
}
fn protocol(value: &str) -> ui::ServiceProtocol {
    match value {
        "openai-responses" => ui::ServiceProtocol::OpenAiResponses,
        "anthropic" => ui::ServiceProtocol::Anthropic,
        "gemini" => ui::ServiceProtocol::Gemini,
        _ => ui::ServiceProtocol::OpenAi,
    }
}
fn core_protocol(
    value: ui::ServiceProtocol,
) -> snaplingo_core::application::providers::LLMProtocol {
    use snaplingo_core::application::providers::LLMProtocol;
    match value {
        ui::ServiceProtocol::OpenAi => LLMProtocol::OpenAI,
        ui::ServiceProtocol::OpenAiResponses => LLMProtocol::OpenAIResponses,
        ui::ServiceProtocol::Anthropic => LLMProtocol::Anthropic,
        ui::ServiceProtocol::Gemini => LLMProtocol::Gemini,
    }
}
fn project_form(form: TranslationServiceForm) -> ui::ServiceForm {
    match form {
        TranslationServiceForm::Credentials(form) => ui::ServiceForm::Credentials {
            provider_id: form.provider_id,
            name: form.provider_name,
            fields: form.fields.into_iter().map(credential).collect(),
        },
        TranslationServiceForm::Custom { draft, prompts } => ui::ServiceForm::Custom {
            draft: ui::ServiceCustomDraft {
                provider_id: draft.provider_id,
                name: draft.name,
                protocol: protocol(draft.protocol.as_str()),
                endpoint: draft.endpoint,
                api_key: draft.api_key,
                model: draft.model,
                reasoning_level: draft.reasoning_level,
                prompt_strategy_id: draft.prompt_strategy_id,
            },
            prompts: ui::ServicePromptWorkspace {
                strategies: prompts
                    .strategies
                    .strategies
                    .into_iter()
                    .map(|strategy| ui::ServicePrompt {
                        id: strategy.id,
                        name: strategy.name,
                        description: strategy.description,
                        system_prompt: strategy.system_prompt,
                        builtin: strategy.is_builtin,
                        deletable: strategy.is_deletable,
                    })
                    .collect(),
                draft: ui::ServicePromptDraft {
                    name: prompts.draft.name,
                    description: prompts.draft.description,
                    system_prompt: prompts.draft.system_prompt,
                },
                error: prompts.error,
            },
        },
    }
}

#[cfg(test)]
pub(crate) mod tests;

fn saved_route(value: &str) -> Option<ui::ServicesRoute> {
    match value {
        "ocr" => Some(ui::ServicesRoute::Ocr),
        "translation" => Some(ui::ServicesRoute::Translation),
        "tts" => Some(ui::ServicesRoute::Tts),
        _ => None,
    }
}
