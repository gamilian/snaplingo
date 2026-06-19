use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
use crate::error::{AppError, Result};
use std::str::FromStr;

/// Register a global shortcut
pub fn register_shortcut<F>(app: &AppHandle, accelerator: &str, handler: F) -> Result<()>
where
    F: Fn() + Send + Sync + 'static,
{
    let shortcut = Shortcut::from_str(accelerator)
        .map_err(|e| AppError::Other(format!("Invalid shortcut: {}", e)))?;
    let normalized_accelerator = shortcut.to_string();
    let shortcut_label = normalized_accelerator.clone();

    app.global_shortcut().on_shortcut(shortcut, move |_app, _shortcut, event| {
        log::info!("Global shortcut event: {} ({:?})", shortcut_label, event.state);
        if event.state == ShortcutState::Pressed {
            handler();
        }
    }).map_err(|e| AppError::Other(format!("Failed to register shortcut: {}", e)))?;

    log::info!("Registered global shortcut: {} ({})", accelerator, normalized_accelerator);
    Ok(())
}

/// Unregister a global shortcut
pub fn unregister_shortcut(app: &AppHandle, accelerator: &str) -> Result<()> {
    let shortcut = Shortcut::from_str(accelerator)
        .map_err(|e| AppError::Other(format!("Invalid shortcut: {}", e)))?;

    app.global_shortcut().unregister(shortcut)
        .map_err(|e| AppError::Other(format!("Failed to unregister shortcut: {}", e)))?;

    log::info!("Unregistered global shortcut: {}", accelerator);
    Ok(())
}

/// Check if a shortcut is registered
pub fn is_shortcut_registered(app: &AppHandle, accelerator: &str) -> Result<bool> {
    let shortcut = Shortcut::from_str(accelerator)
        .map_err(|e| AppError::Other(format!("Invalid shortcut: {}", e)))?;

    Ok(app.global_shortcut().is_registered(shortcut))
}
