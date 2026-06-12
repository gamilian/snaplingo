use crate::config::Config;
use crate::AppState;
use anyhow::Result;

#[tauri::command]
pub fn get_config(state: tauri::State<'_, AppState>) -> Result<Config, String> {
    Ok(state.config.lock().unwrap().clone())
}

#[tauri::command]
pub fn update_config(
    updates: serde_json::Value,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    let mut config = state.config.lock().unwrap();

    // Merge updates into config
    let mut config_value = serde_json::to_value(&*config).unwrap();
    json_patch::merge(&mut config_value, &updates);
    *config = serde_json::from_value(config_value).map_err(|e| e.to_string())?;

    // Save to disk
    let config_path = state.config_path.clone();
    config.save(&config_path).map_err(|e| e.to_string())?;

    Ok(())
}
