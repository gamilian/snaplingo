use crate::application::result_window::ResultWindowClipboardPort;
use crate::Result;

pub(crate) struct ArboardResultWindowClipboard;

impl ArboardResultWindowClipboard {
    pub(crate) fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl ResultWindowClipboardPort for ArboardResultWindowClipboard {
    async fn read_text(&self) -> Result<String> {
        let mut clipboard =
            arboard::Clipboard::new().map_err(|error| clipboard_open_error(error))?;
        clipboard
            .get_text()
            .map_err(|error| clipboard_read_error(error).into())
    }
}

fn clipboard_open_error(error: impl std::fmt::Display) -> String {
    format!("Failed to open clipboard: {error}")
}

fn clipboard_read_error(error: impl std::fmt::Display) -> String {
    format!("Failed to read clipboard text: {error}")
}

#[cfg(test)]
mod tests {
    #[test]
    fn result_window_clipboard_errors_preserve_existing_operation_context() {
        assert_eq!(
            super::clipboard_open_error("clipboard unavailable"),
            "Failed to open clipboard: clipboard unavailable"
        );
        assert_eq!(
            super::clipboard_read_error("clipboard unavailable"),
            "Failed to read clipboard text: clipboard unavailable"
        );
    }
}
