use crate::error::{AppError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

static TEMP_PATH_COUNTER: AtomicU64 = AtomicU64::new(0);

/// ConfigFile provides a simple JSON-based key-value store for application configuration.
/// It uses a Mutex for thread-safe access.
pub struct ConfigFile {
    path: PathBuf,
    store: Mutex<HashMap<String, serde_json::Value>>,
}

impl ConfigFile {
    /// Creates a new ConfigFile at the specified path.
    /// If the file exists, it will be loaded; otherwise, an empty store is created.
    pub fn new(path: PathBuf) -> Self {
        let store = if path.exists() {
            match fs::read_to_string(&path) {
                Ok(content) => serde_json::from_str(&content).unwrap_or_default(),
                Err(_) => HashMap::new(),
            }
        } else {
            HashMap::new()
        };

        Self {
            path,
            store: Mutex::new(store),
        }
    }

    /// Saves a value under the specified key.
    /// The value must implement Serialize.
    pub fn save<T: Serialize>(&self, key: &str, value: &T) -> Result<()> {
        let json_value = serde_json::to_value(value)?;

        let mut store = self.store.lock().unwrap();
        store.insert(key.to_string(), json_value);

        let content = serde_json::to_string_pretty(&*store)?;

        // Atomic write: write to temp file in same directory, then rename
        let parent = self
            .path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));

        // Ensure parent directory exists
        if !parent.exists() {
            fs::create_dir_all(parent)?;
        }

        let temp_path = unique_temp_write_path(parent);

        // Write to temp file
        fs::write(&temp_path, &content)?;

        // Atomic rename (on Windows, need to remove target first)
        #[cfg(windows)]
        {
            let _ = fs::remove_file(&self.path); // Ignore if doesn't exist
        }

        fs::rename(&temp_path, &self.path)?;

        Ok(())
    }

    /// Loads a value for the specified key.
    /// The value must implement Deserialize.
    /// Returns an error if the key doesn't exist or deserialization fails.
    pub fn load<T: for<'de> Deserialize<'de>>(&self, key: &str) -> Result<T> {
        let store = self.store.lock().unwrap();

        let json_value = store
            .get(key)
            .ok_or_else(|| AppError::Config(format!("Key '{}' not found", key)))?;

        let value = serde_json::from_value(json_value.clone())?;
        Ok(value)
    }

    /// Creates a temporary ConfigFile for testing purposes.
    /// The file is created in the system's temp directory and will be cleaned up automatically.
    #[cfg(test)]
    pub fn new_temp() -> Self {
        let temp_path = unique_temp_write_path(&std::env::temp_dir()).with_extension("json");
        Self::new(temp_path)
    }
}

pub(super) fn unique_temp_write_path(parent: &Path) -> PathBuf {
    use std::time::{SystemTime, UNIX_EPOCH};

    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let counter = TEMP_PATH_COUNTER.fetch_add(1, Ordering::Relaxed);
    let thread_id = format!("{:?}", std::thread::current().id())
        .replace("ThreadId(", "")
        .replace(')', "");

    parent.join(format!(
        ".config.tmp.{}.{}.{}.{}",
        std::process::id(),
        thread_id,
        nanos,
        counter
    ))
}
