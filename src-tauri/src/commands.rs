/// Tauri commands exposed to the frontend

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CaptureMode {
    Screenshot,
    Ocr,
    OcrTranslate,
    SelectionTranslate,
    InputTranslate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageData {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

// Placeholder commands - to be implemented

#[tauri::command]
pub async fn capture_screen(_mode: CaptureMode) -> Result<ImageData, String> {
    Err("Not implemented".to_string())
}

#[tauri::command]
pub async fn get_config() -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "version": "0.1.0"
    }))
}
