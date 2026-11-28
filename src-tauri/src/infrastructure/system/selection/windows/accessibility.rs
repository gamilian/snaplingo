use std::time::Duration;

use async_trait::async_trait;

use crate::application::selected_text::SelectionMethod;
use crate::domain::{
    MethodAvailability, SelectionAttempt, SelectionContext, SelectionMethodKind, SelectionSource,
};

const SELECTION_TIMEOUT: Duration = Duration::from_millis(500);

pub struct AccessibilitySelectionMethod;

#[async_trait]
impl SelectionMethod for AccessibilitySelectionMethod {
    fn kind(&self) -> SelectionMethodKind {
        SelectionMethodKind::Accessibility
    }

    fn availability(&self, _context: &SelectionContext) -> MethodAvailability {
        MethodAvailability::Available
    }

    async fn acquire(&self, context: &SelectionContext) -> SelectionAttempt {
        acquire_with_reader(context, read_selected_text).await
    }
}

async fn acquire_with_reader(
    context: &SelectionContext,
    read: impl FnOnce() -> Result<String, String> + Send + 'static,
) -> SelectionAttempt {
    // UI Automation calls into other applications. Keep COM on a worker thread
    // and allow shortcut-copy fallback even if a provider stops responding.
    let result = tokio::time::timeout(SELECTION_TIMEOUT, tokio::task::spawn_blocking(read)).await;
    let result = match result {
        Ok(Ok(result)) => result,
        Ok(Err(error)) => Err(format!("Windows selection worker failed: {error}")),
        Err(_) => Err("Windows UI Automation selection timed out".to_string()),
    };
    match result {
        Ok(text) if !text.trim().is_empty() => SelectionAttempt::success(
            SelectionMethodKind::Accessibility,
            SelectionSource::Accessibility,
            text,
            context.clone(),
        ),
        Ok(_) => SelectionAttempt::empty(SelectionMethodKind::Accessibility, context.clone()),
        Err(error) => {
            SelectionAttempt::failed(SelectionMethodKind::Accessibility, context.clone(), error)
        }
    }
}

#[cfg(target_os = "windows")]
fn read_selected_text() -> Result<String, String> {
    super::ui_automation::read_selected_text().map_err(|error| error.to_string())
}

#[cfg(not(target_os = "windows"))]
fn read_selected_text() -> Result<String, String> {
    Err("UI Automation selection requires Windows".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::{SelectedTextAcquirer, SelectionScheme};
    use crate::domain::SelectionAttemptStatus;
    use std::sync::{Arc, Mutex};

    struct ReaderMethod(Mutex<Option<Result<String, String>>>);

    #[async_trait]
    impl SelectionMethod for ReaderMethod {
        fn kind(&self) -> SelectionMethodKind {
            SelectionMethodKind::Accessibility
        }
        fn availability(&self, _: &SelectionContext) -> MethodAvailability {
            MethodAvailability::Available
        }
        async fn acquire(&self, context: &SelectionContext) -> SelectionAttempt {
            let result = self.0.lock().unwrap().take().unwrap();
            acquire_with_reader(context, move || result).await
        }
    }

    struct CopyMethod(Arc<Mutex<usize>>);

    #[async_trait]
    impl SelectionMethod for CopyMethod {
        fn kind(&self) -> SelectionMethodKind {
            SelectionMethodKind::ShortcutCopy
        }
        fn availability(&self, _: &SelectionContext) -> MethodAvailability {
            MethodAvailability::Available
        }
        async fn acquire(&self, context: &SelectionContext) -> SelectionAttempt {
            *self.0.lock().unwrap() += 1;
            SelectionAttempt::success(
                self.kind(),
                SelectionSource::ShortcutCopy,
                "copied text".into(),
                context.clone(),
            )
        }
    }

    #[tokio::test]
    async fn windows_scheme_copies_only_when_direct_selection_has_no_text() {
        use crate::application::SystemSelectionProvider;
        for (read, source, text, copies) in [
            (
                Ok("  selected 文本\n".into()),
                SelectionSource::Accessibility,
                "  selected 文本\n",
                0,
            ),
            (
                Ok(" \r\n".into()),
                SelectionSource::ShortcutCopy,
                "copied text",
                1,
            ),
            (
                Err("TextPattern unavailable".into()),
                SelectionSource::ShortcutCopy,
                "copied text",
                1,
            ),
        ] {
            let provider = super::super::PlatformSelectionProvider;
            let calls = Arc::new(Mutex::new(0));
            let acquirer = SelectedTextAcquirer::new(
                SelectionScheme::new(provider.default_scheme()),
                vec![
                    Box::new(ReaderMethod(Mutex::new(Some(read)))),
                    Box::new(CopyMethod(calls.clone())),
                ],
                Arc::new(provider),
            );
            let result = acquirer.acquire().await.unwrap();
            assert_eq!(result.source, source);
            assert_eq!(result.text, text);
            assert_eq!(*calls.lock().unwrap(), copies);
        }
    }

    #[tokio::test]
    async fn stalled_provider_releases_the_fallback_path() {
        let result = acquire_with_reader(&SelectionContext::default(), || {
            std::thread::sleep(SELECTION_TIMEOUT + Duration::from_millis(100));
            Ok("late text".into())
        })
        .await;
        assert!(
            matches!(result.status, SelectionAttemptStatus::Failed(message) if message.contains("timed out"))
        );
    }
}
