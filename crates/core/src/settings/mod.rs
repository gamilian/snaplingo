mod configuration;
mod feature_preferences;
pub mod model;
mod presentation;
mod runtime;
mod screenshot_preferences;
mod services;
mod store;
mod translation_services;

pub use configuration::{SettingsChangeNotifier, SettingsConfiguration};
pub use feature_preferences::{FeaturePreferences, OcrField, TranslationField};
pub use presentation::{settings_ui_text, GeneralField, GeneralSection, GeneralValue};
pub use runtime::{AppLogEntry, AppLogRepository, SettingsApplication, StartOnBoot};
pub use screenshot_preferences::{ScreenshotField, ANNOTATION_SHORTCUT_LABELS};
pub use services::{
    CredentialForm, OcrServices, ServicesController, ServicesDependencies, ServicesKind,
    ServicesProjection,
};
pub use store::SettingsStore;
pub use translation_services::{
    CustomTranslationDraft, CustomTranslationEdit, TranslationCredentialForm,
    TranslationPromptDraft, TranslationPromptWorkspace, TranslationServiceForm,
    TranslationServices, TranslationServicesController, TranslationServicesProjection,
    TranslationServicesRequest,
};
