use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::{AppError, Result};

/// Fields explicitly changed by a caller; omitted fields retain their current value.
#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct SettingsPatch(Map<String, Value>);

impl SettingsPatch {
    pub(super) fn apply<T: Serialize + DeserializeOwned>(self, current: &T) -> Result<T> {
        let Value::Object(mut fields) = serde_json::to_value(current)? else {
            return Err(AppError::Config("Settings must be an object".into()));
        };
        for (key, value) in self.0 {
            if !fields.contains_key(&key) {
                return Err(AppError::Config(format!("Unknown setting: {key}")));
            }
            fields.insert(key, value);
        }
        Ok(serde_json::from_value(Value::Object(fields))?)
    }
}

macro_rules! full_settings_patch {
    ($($settings:ty),+ $(,)?) => {
        $(impl From<$settings> for SettingsPatch {
            fn from(settings: $settings) -> Self {
                let Value::Object(fields) = serde_json::to_value(settings)
                    .expect("settings serialize as an object") else { unreachable!() };
                Self(fields)
            }
        })+
    };
}

full_settings_patch!(
    crate::domain::GeneralSettings,
    crate::domain::ScreenshotSettings,
    crate::domain::TranslationSettings,
    crate::domain::OcrSettings,
    crate::domain::HistorySettings,
);
