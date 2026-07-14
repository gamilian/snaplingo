use std::path::Path;

use crate::application::history::{TranslationFavoritesWriter, TranslationHistoryEntry};
use crate::Result;

pub struct JsonTranslationFavoritesWriter;

impl TranslationFavoritesWriter for JsonTranslationFavoritesWriter {
    fn write(&self, path: &str, entries: &[TranslationHistoryEntry]) -> Result<()> {
        let path = Path::new(path);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_vec_pretty(entries)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn writes_portable_json_export() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("favorites.json");
        JsonTranslationFavoritesWriter
            .write(
                path.to_str().unwrap(),
                &[TranslationHistoryEntry {
                    id: 1,
                    timestamp: Utc::now(),
                    favorite: true,
                    note: Some("keep".into()),
                    tags: vec!["work".into()],
                    source_text: "hello".into(),
                    source_lang: "en".into(),
                    target_lang: "zh-CN".into(),
                    providers_used: vec![],
                    results: vec![],
                    duration_ms: 1,
                }],
            )
            .unwrap();

        let json: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(json[0]["source_text"], "hello");
        assert_eq!(json[0]["tags"][0], "work");
    }
}
