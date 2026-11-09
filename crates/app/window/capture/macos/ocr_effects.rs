use crate::application::capture::CaptureOcrEffects;
use crate::Result;

pub(crate) struct NativeCaptureOcrEffects;

impl CaptureOcrEffects for NativeCaptureOcrEffects {
    fn copy_text(&self, text: String) -> Result<()> {
        let mut clipboard = arboard::Clipboard::new()
            .map_err(|error| format!("Failed to open clipboard: {error}"))?;
        clipboard
            .set_text(text)
            .map_err(|error| format!("Failed to copy OCR text: {error}").into())
    }

    fn show_error(&self, error: &str) {
        crate::infrastructure::system::dialogs::show_native_error("SnapLingo — OCR", error);
    }
}
