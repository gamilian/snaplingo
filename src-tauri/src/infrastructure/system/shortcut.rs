use tauri::AppHandle;

use crate::error::Result;

pub fn register_shortcut<F>(_app: &AppHandle, accelerator: &str, _handler: F) -> Result<()>
where
    F: Fn() + Send + Sync + 'static,
{
    log::warn!(
        "Shortcut '{}' requested before capture session shortcut backend is wired",
        accelerator
    );
    Ok(())
}

pub fn unregister_shortcut(_app: &AppHandle, _accelerator: &str) -> Result<()> {
    Ok(())
}

pub fn is_shortcut_registered(_app: &AppHandle, _accelerator: &str) -> Result<bool> {
    Ok(false)
}
