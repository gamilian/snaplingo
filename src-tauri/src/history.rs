/// History management

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub id: u64,
    pub timestamp: u64,
    pub capture_mode: String,
    pub thumbnail: Option<Vec<u8>>,
    pub source_text: Option<String>,
    pub translations: Vec<Translation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Translation {
    pub provider_id: String,
    pub text: String,
}

pub struct HistoryManager;

impl HistoryManager {
    pub fn new() -> Self {
        Self
    }

    pub fn add_entry(&self, _entry: HistoryEntry) -> Result<(), String> {
        // TODO: Implement SQLite storage
        Ok(())
    }

    pub fn list_entries(&self) -> Result<Vec<HistoryEntry>, String> {
        // TODO: Query from SQLite
        Ok(vec![])
    }

    pub fn clear(&self) -> Result<(), String> {
        // TODO: Clear all history
        Ok(())
    }

    pub fn cleanup_old_entries(&self, _max_age_days: u32, _max_entries: u32) -> Result<(), String> {
        // TODO: Auto cleanup logic
        Ok(())
    }
}
