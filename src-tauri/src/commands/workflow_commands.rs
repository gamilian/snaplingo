use crate::application::WorkflowOutcome;
use crate::domain::HotkeyAction;
use tauri::State;

/// Trigger a workflow manually (for testing or frontend invocation)
#[tauri::command]
pub async fn trigger_workflow(
    action: String,
    state: State<'_, crate::AppState>,
) -> Result<String, String> {
    // Parse action string to HotkeyAction
    let action = match action.as_str() {
        "screenshot" => HotkeyAction::Screenshot,
        "screenshot-ocr" => HotkeyAction::ScreenshotOcr,
        "screenshot-translate" => HotkeyAction::ScreenshotTranslate,
        "selection-translate" => HotkeyAction::SelectionTranslate,
        "input-translate" => HotkeyAction::InputTranslate,
        _ => return Err(format!("Unknown action: {}", action)),
    };

    // Execute workflow
    match state.workflow_service.execute(action).await {
        Ok(outcome) => {
            let description = match outcome {
                WorkflowOutcome::ShowScreenshot { .. } => "Screenshot captured",
                WorkflowOutcome::ShowOcrResult { .. } => "OCR completed",
                WorkflowOutcome::ShowTranslationResults { .. } => "Translation completed",
                WorkflowOutcome::ShowInputWindow => "Input window opened",
            };
            Ok(description.to_string())
        }
        Err(e) => Err(e.to_string()),
    }
}
