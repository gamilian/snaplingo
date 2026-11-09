use crate::{Key, Modifiers, Page, Point, Rect, Scene, Snapshot, Target};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ServicesRoute {
    #[default]
    Ocr,
    Translation,
    Tts,
}
impl ServicesRoute {
    pub const ALL: [Self; 3] = [Self::Ocr, Self::Translation, Self::Tts];
    pub fn label(self) -> &'static str {
        match self {
            Self::Ocr => "OCR 服务",
            Self::Translation => "翻译服务",
            Self::Tts => "语音合成",
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ServiceProvider {
    pub id: String,
    pub name: String,
    pub configured: bool,
    pub active: bool,
    pub builtin: bool,
    pub local: bool,
    pub requires_api_key: bool,
    pub description: String,
    pub acceleration: Option<String>,
    pub protocol: Option<ServiceProtocol>,
    pub endpoint: Option<String>,
    pub model: Option<String>,
}
impl ServiceProvider {
    pub fn subtitle(&self) -> String {
        if let Some(endpoint) = &self.endpoint {
            return endpoint.clone();
        }
        if !self.description.is_empty() {
            return self.description.clone();
        }
        match self.id.as_str() {
            "paddle-ocr" => format!(
                "离线文字识别 · medium · {}",
                self.acceleration.as_deref().unwrap_or("自动选择设备")
            ),
            "google-translate" => "https://translate.googleapis.com".into(),
            "deeplx" => "DeepLX / 标准 DeepL".into(),
            "baidu-translate" => "百度翻译开放平台".into(),
            "baidu-ocr" => "百度智能云 OCR".into(),
            "system-ocr" => "系统内置文字识别".into(),
            "tesseract" => "本地 Tesseract 引擎".into(),
            _ => self
                .protocol
                .map(|p| format!("{} 兼容接口", p.label()))
                .unwrap_or("已接入服务".into()),
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ServiceProtocol {
    #[default]
    OpenAi,
    OpenAiResponses,
    Anthropic,
    Gemini,
}
impl ServiceProtocol {
    pub fn label(self) -> &'static str {
        match self {
            Self::OpenAi => "OpenAI",
            Self::OpenAiResponses => "OpenAI Responses",
            Self::Anthropic => "Anthropic",
            Self::Gemini => "Gemini",
        }
    }
    pub fn defaults(self) -> (&'static str, &'static str) {
        match self {
            Self::OpenAi => ("https://api.openai.com", "gpt-4o"),
            Self::OpenAiResponses => ("https://api.openai.com", "gpt-5-mini"),
            Self::Anthropic => ("https://api.anthropic.com", "claude-3-5-sonnet-latest"),
            Self::Gemini => (
                "https://generativelanguage.googleapis.com",
                "gemini-1.5-flash",
            ),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceCredentialField {
    pub name: String,
    pub label: String,
    pub secret: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceVoice {
    pub id: String,
    pub name: String,
    pub locale: String,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ServicePromptDraft {
    pub name: String,
    pub description: String,
    pub system_prompt: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServicePrompt {
    pub id: String,
    pub name: String,
    pub description: String,
    pub system_prompt: String,
    pub builtin: bool,
    pub deletable: bool,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ServicePromptWorkspace {
    pub strategies: Vec<ServicePrompt>,
    pub draft: ServicePromptDraft,
    pub error: Option<String>,
}
/// Secrets stay in drafts; these types deliberately do not implement Debug.
#[derive(Clone, PartialEq, Eq)]
pub struct ServiceCustomDraft {
    pub provider_id: Option<String>,
    pub name: String,
    pub protocol: ServiceProtocol,
    pub endpoint: String,
    pub api_key: String,
    pub model: String,
    pub reasoning_level: String,
    pub prompt_strategy_id: String,
}
impl Default for ServiceCustomDraft {
    fn default() -> Self {
        let (endpoint, model) = ServiceProtocol::OpenAi.defaults();
        Self {
            provider_id: None,
            name: String::new(),
            protocol: ServiceProtocol::OpenAi,
            endpoint: endpoint.into(),
            api_key: String::new(),
            model: model.into(),
            reasoning_level: String::new(),
            prompt_strategy_id: "smart".into(),
        }
    }
}
#[derive(Clone, PartialEq, Eq)]
pub enum ServiceForm {
    Credentials {
        provider_id: String,
        name: String,
        fields: Vec<ServiceCredentialField>,
    },
    Custom {
        draft: ServiceCustomDraft,
        prompts: ServicePromptWorkspace,
    },
}
#[derive(Clone)]
pub struct ServicesSnapshot {
    pub shell: Snapshot,
    pub route: ServicesRoute,
    pub revision: u64,
    pub form_revision: u64,
    pub providers: Vec<ServiceProvider>,
    pub voices: Vec<ServiceVoice>,
    pub voice_id: String,
    pub rate: u32,
    pub form: Option<ServiceForm>,
    pub busy: bool,
    pub loading_models: bool,
    pub testing_connection: bool,
    pub models: Vec<String>,
    pub model_error: Option<String>,
    pub test_status: Option<Result<String, String>>,
    pub status: Option<String>,
    pub error: Option<String>,
}
impl Default for ServicesSnapshot {
    fn default() -> Self {
        Self {
            shell: Snapshot::default(),
            route: ServicesRoute::Ocr,
            revision: 0,
            form_revision: 0,
            providers: vec![],
            voices: vec![],
            voice_id: String::new(),
            rate: 180,
            form: None,
            busy: false,
            loading_models: false,
            testing_connection: false,
            models: vec![],
            model_error: None,
            test_status: None,
            status: None,
            error: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServiceCustomField {
    Name,
    Endpoint,
    ApiKey,
    Model,
    PromptName,
    PromptDescription,
    PromptText,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServiceField {
    Credential { provider_id: String, name: String },
    Custom(ServiceCustomField),
}
impl ServiceField {
    pub(crate) fn multiline(&self) -> bool {
        *self == Self::Custom(ServiceCustomField::PromptText)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServicesTarget {
    pub revision: u64,
    pub control: ServiceControl,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServiceControl {
    Shell(Target),
    Route(ServicesRoute),
    Configure(String),
    SetActive { provider_id: String, active: bool },
    Drag(String),
    TestSaved(String),
    Remove(String),
    AddCustom,
    CancelForm,
    SaveForm,
    Field(ServiceField),
    DeepLMode(bool),
    Protocol(ServiceProtocol),
    ProtocolFamily(ServiceProtocol),
    Reasoning(String),
    PromptStrategy(String),
    LoadModels,
    TestConnection,
    ChooseModel(String),
    SavePrompt,
    AddPrompt,
    DeletePrompt,
    VoiceMenu,
    Voice(String),
    Rate,
    Speak,
    OpenUrl(String),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ServicesRole {
    Button,
    Switch,
    Tab,
    TextInput,
    SecretInput,
    MultilineInput,
    Slider,
    ComboBox,
    Option,
    DragHandle,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ServicesControlInfo {
    pub target: ServicesTarget,
    pub rect: Rect,
    pub hit_rect: Rect,
    pub role: ServicesRole,
    pub label: String,
    pub enabled: bool,
    pub selected: bool,
    pub value: Option<String>,
    pub range: Option<(u32, u32, u32)>,
}
#[derive(Clone, Debug)]
pub struct ServicesFrame {
    pub scene: Scene,
    pub controls: Vec<ServicesControlInfo>,
}
impl ServicesFrame {
    pub fn hit_test(&self, point: Point) -> Option<ServicesTarget> {
        self.controls
            .iter()
            .rev()
            .find(|control| control.enabled && control.hit_rect.contains(point))
            .map(|control| control.target.clone())
    }
}
#[derive(Clone)]
pub enum ServicesInput {
    Refresh,
    Snapshot(ServicesSnapshot),
    Activate(ServicesTarget),
    Focus(ServicesTarget),
    SetValue(ServicesTarget, String),
    PointerDown(Point),
    PointerMove(Point),
    PointerUp(Point),
    PointerCancel,
    Scroll {
        position: Point,
        delta_y: f32,
    },
    KeyDown {
        key: Key,
        modifiers: Modifiers,
    },
    Text(String),
    ClipboardText(String),
    Selection(std::ops::Range<usize>),
    ImePreedit {
        text: String,
        selection: Option<std::ops::Range<usize>>,
    },
    ImeCommit(String),
    ImeDisabled,
    FocusLost,
}
#[derive(Clone, PartialEq, Eq)]
pub enum ServicesIntent {
    Refresh,
    Navigate(Page),
    ConfigureCapacity,
    Route(ServicesRoute),
    Configure {
        route: ServicesRoute,
        provider_id: String,
    },
    SetActive {
        route: ServicesRoute,
        provider_id: String,
        active: bool,
    },
    AddCustom,
    CancelForm,
    SaveCredentials {
        route: ServicesRoute,
        revision: u64,
        provider_id: String,
        credentials: BTreeMap<String, String>,
    },
    EditCustom {
        revision: u64,
        field: ServiceCustomField,
        value: String,
    },
    Protocol {
        revision: u64,
        protocol: ServiceProtocol,
    },
    Reasoning {
        revision: u64,
        value: String,
    },
    PromptStrategy {
        revision: u64,
        provider_strategy_id: String,
    },
    SaveCustom {
        revision: u64,
    },
    Reorder {
        source_id: String,
        target_id: String,
    },
    TestSaved(String),
    ConfirmRemove(String),
    LoadModels {
        revision: u64,
    },
    TestConnection {
        revision: u64,
    },
    EditPrompt {
        revision: u64,
        draft: ServicePromptDraft,
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
    Voice(String),
    Rate(u32),
    Speak,
    CopyText(String),
    ReadClipboard,
    OpenUrl(String),
}
