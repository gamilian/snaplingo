use std::time::Duration;

use async_trait::async_trait;

use crate::domain::{
    MethodAvailability, SelectionAttempt, SelectionContext, SelectionMethodKind, SelectionSource,
};
use crate::infrastructure::system::selection::SelectionMethod;

const APPLESCRIPT_TIMEOUT: Duration = Duration::from_millis(300);
const SAFARI: &str = "com.apple.Safari";
const CHROME: &str = "com.google.Chrome";
const EDGE: &str = "com.microsoft.edgemac";

pub struct BrowserAppleScriptSelectionMethod;

#[async_trait]
impl SelectionMethod for BrowserAppleScriptSelectionMethod {
    fn kind(&self) -> SelectionMethodKind {
        SelectionMethodKind::BrowserScript
    }

    fn availability(&self, context: &SelectionContext) -> MethodAvailability {
        let Some(bundle_id) = context
            .frontmost_app
            .as_ref()
            .and_then(|app| app.bundle_id.as_deref())
        else {
            return MethodAvailability::Unavailable("frontmost app is unknown".to_string());
        };

        if is_supported_browser(bundle_id) {
            MethodAvailability::Available
        } else {
            MethodAvailability::Unavailable(format!(
                "frontmost app is not a supported browser: {bundle_id}"
            ))
        }
    }

    async fn acquire(&self, context: &SelectionContext) -> SelectionAttempt {
        let Some(bundle_id) = context
            .frontmost_app
            .as_ref()
            .and_then(|app| app.bundle_id.as_deref())
        else {
            return SelectionAttempt::failed(
                self.kind(),
                context.clone(),
                "frontmost app is unknown".to_string(),
            );
        };

        let result = async {
            let script = browser_selection_script(bundle_id)?;
            run_osascript(&script).await
        }
        .await;

        match result {
            Ok(text) => SelectionAttempt::success(
                self.kind(),
                SelectionSource::BrowserScript,
                text,
                context.clone(),
            ),
            Err(err) => SelectionAttempt::failed(self.kind(), context.clone(), err),
        }
    }
}

fn is_supported_browser(bundle_id: &str) -> bool {
    matches!(bundle_id, SAFARI | CHROME | EDGE)
}

fn browser_selection_script(bundle_id: &str) -> Result<String, String> {
    match bundle_id {
        SAFARI => Ok(format!(
            r#"tell application id "{SAFARI}"
  tell front window
    set selection_text to do JavaScript "window.getSelection().toString();" in current tab
  end tell
end tell"#
        )),
        CHROME | EDGE => Ok(format!(
            r#"tell application id "{bundle_id}"
  tell active tab of front window
    set selection_text to execute javascript "window.getSelection().toString();"
  end tell
end tell"#
        )),
        _ => Err(format!("unsupported browser bundle id: {bundle_id}")),
    }
}

async fn run_osascript(script: &str) -> Result<String, String> {
    let output = tokio::time::timeout(
        APPLESCRIPT_TIMEOUT,
        tokio::process::Command::new("osascript")
            .arg("-e")
            .arg(script)
            .output(),
    )
    .await
    .map_err(|_| "Timed out waiting for browser selection script".to_string())?
    .map_err(|e| format!("Failed to run browser selection script: {e}"))?;

    if output.status.success() {
        return Ok(String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .to_string());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        Err("Browser selection script failed".to_string())
    } else {
        Err(stderr)
    }
}

#[cfg(test)]
mod browser_applescript_selection_tests {
    use super::*;

    #[test]
    fn recognizes_supported_browser_bundle_ids() {
        assert!(is_supported_browser("com.apple.Safari"));
        assert!(is_supported_browser("com.google.Chrome"));
        assert!(is_supported_browser("com.microsoft.edgemac"));
        assert!(!is_supported_browser("com.apple.TextEdit"));
    }

    #[test]
    fn chrome_script_reads_window_selection() {
        let script = browser_selection_script("com.google.Chrome").unwrap();

        assert!(script.contains("execute javascript"));
        assert!(script.contains("window.getSelection().toString()"));
    }
}
