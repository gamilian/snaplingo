use async_trait::async_trait;

use crate::domain::{
    MethodAvailability, SelectionAttempt, SelectionContext, SelectionMethodKind, SelectionSource,
};
use crate::infrastructure::system::selection::SelectionMethod;

pub struct MenuCopySelectionMethod;

#[async_trait]
impl SelectionMethod for MenuCopySelectionMethod {
    fn kind(&self) -> SelectionMethodKind {
        SelectionMethodKind::MenuCopy
    }

    fn availability(&self, _context: &SelectionContext) -> MethodAvailability {
        if super::context::accessibility_permission_granted(false) {
            MethodAvailability::Available
        } else {
            MethodAvailability::Unavailable(
                super::context::selection_accessibility_permission_error(),
            )
        }
    }

    async fn acquire(&self, context: &SelectionContext) -> SelectionAttempt {
        let result =
            super::pasteboard::with_temporary_pasteboard_text(|| async { press_copy_menu_item() })
                .await;

        match result {
            Ok(text) => SelectionAttempt::success(
                self.kind(),
                SelectionSource::MenuCopy,
                text,
                context.clone(),
            ),
            Err(err) => SelectionAttempt::failed(self.kind(), context.clone(), err),
        }
    }
}

#[cfg(test)]
fn is_copy_identifier(identifier: &str) -> bool {
    identifier == "copy:"
}

#[cfg(test)]
fn is_copy_title(title: &str) -> bool {
    matches!(
        title,
        "Copy" | "复制" | "拷贝" | "拷貝" | "複製" | "コピー" | "복사"
    )
}

fn press_copy_menu_item() -> Result<(), String> {
    let output = std::process::Command::new("osascript")
        .arg("-e")
        .arg(copy_menu_script())
        .output()
        .map_err(|e| format!("Failed to run menu copy script: {e}"))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if stderr.is_empty() {
        Err("No enabled Copy menu item found".to_string())
    } else {
        Err(stderr)
    }
}

fn copy_menu_script() -> &'static str {
    r#"
tell application "System Events"
  set frontApp to first application process whose frontmost is true
  set editTitles to {"Edit", "编辑", "編輯", "編集", "편집"}
  set copyTitles to {"Copy", "复制", "拷贝", "拷貝", "複製", "コピー", "복사"}

  repeat with editTitle in editTitles
    tell menu bar 1 of frontApp
      if exists menu bar item editTitle then
        tell menu bar item editTitle
          tell menu editTitle
            repeat with copyTitle in copyTitles
              if exists menu item copyTitle then
                tell menu item copyTitle
                  if enabled then
                    click
                    return "ok"
                  end if
                end tell
              end if
            end repeat
          end tell
        end tell
      end if
    end tell
  end repeat
end tell
error "No enabled Copy menu item found"
"#
}

#[cfg(test)]
mod menu_copy_selection_tests {
    use super::*;

    #[test]
    fn recognizes_copy_titles() {
        assert!(is_copy_title("Copy"));
        assert!(is_copy_title("复制"));
        assert!(is_copy_title("拷贝"));
    }

    #[test]
    fn recognizes_copy_action_identifier() {
        assert!(is_copy_identifier("copy:"));
        assert!(!is_copy_identifier("paste:"));
    }
}
