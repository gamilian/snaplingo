use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use tauri::Manager;

use crate::domain::{
    MethodAvailability, SelectionAttempt, SelectionContext, SelectionMethodKind, SelectionSource,
};
use crate::infrastructure::system::selection::SelectionMethod;
use crate::settings_window::SETTINGS_WINDOW_LABEL;

const EVAL_TIMEOUT: Duration = Duration::from_millis(300);

pub struct SelfWebviewSelectionMethod {
    app: tauri::AppHandle,
}

impl SelfWebviewSelectionMethod {
    pub fn new(app: tauri::AppHandle) -> Self {
        Self { app }
    }
}

#[async_trait]
impl SelectionMethod for SelfWebviewSelectionMethod {
    fn kind(&self) -> SelectionMethodKind {
        SelectionMethodKind::SelfWebview
    }

    fn availability(&self, context: &SelectionContext) -> MethodAvailability {
        if !context.is_frontmost_self() {
            return MethodAvailability::Unavailable("frontmost app is not SnapLingo".to_string());
        }

        if self.app.get_webview_window(SETTINGS_WINDOW_LABEL).is_none() {
            return MethodAvailability::Unavailable("settings window not found".to_string());
        }

        MethodAvailability::Available
    }

    async fn acquire(&self, context: &SelectionContext) -> SelectionAttempt {
        match read_settings_window_selection(&self.app).await {
            Ok(text) if !text.trim().is_empty() => SelectionAttempt::success(
                self.kind(),
                SelectionSource::SelfWebview,
                text,
                context.clone(),
            ),
            Ok(_) => SelectionAttempt::empty(self.kind(), context.clone()),
            Err(err) => SelectionAttempt::failed(self.kind(), context.clone(), err),
        }
    }
}

async fn read_settings_window_selection(app: &tauri::AppHandle) -> Result<String, String> {
    let Some(window) = app.get_webview_window(SETTINGS_WINDOW_LABEL) else {
        return Err("settings window not found".to_string());
    };

    let (sender, receiver) = tokio::sync::oneshot::channel();
    let sender = Arc::new(Mutex::new(Some(sender)));
    let callback_sender = sender.clone();

    window
        .eval_with_callback(selection_script(), move |value| {
            if let Some(sender) = callback_sender
                .lock()
                .ok()
                .and_then(|mut guard| guard.take())
            {
                let _ = sender.send(value);
            }
        })
        .map_err(|e| format!("Failed to evaluate selected text script: {e}"))?;

    let value = tokio::time::timeout(EVAL_TIMEOUT, receiver)
        .await
        .map_err(|_| "Timed out reading selected text from SnapLingo window".to_string())?
        .map_err(|e| format!("Failed to receive selected text from SnapLingo window: {e}"))?;

    Ok(parse_eval_string(value))
}

fn parse_eval_string(value: String) -> String {
    serde_json::from_str::<String>(&value).unwrap_or(value)
}

fn selection_script() -> &'static str {
    r#"
    (() => {
      const selection = window.getSelection && window.getSelection();
      return selection ? selection.toString() : "";
    })()
    "#
}

#[cfg(test)]
mod self_webview_selection_tests {
    use super::*;

    #[test]
    fn selection_script_reads_window_selection() {
        let script = selection_script();

        assert!(script.contains("window.getSelection"));
        assert!(script.contains("toString"));
    }
}
