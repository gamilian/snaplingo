use std::time::Duration;

use async_trait::async_trait;
use objc2::{msg_send, rc::Retained, sel, MainThreadMarker};
use objc2_app_kit::{NSApplication, NSTextView, NSWindow};
use objc2_foundation::{NSAttributedString, NSNotFound, NSObjectProtocol, NSRange};

use crate::application::selected_text::SelectionMethod;
use crate::domain::{
    MethodAvailability, SelectionAttempt, SelectionContext, SelectionMethodKind, SelectionSource,
};
const READ_TIMEOUT: Duration = Duration::from_millis(300);

// Keep the persisted method/source name while reading native text controls and
// shared Rust canvases directly through AppKit text input services.
pub struct SelfWebviewSelectionMethod;

#[async_trait]
impl SelectionMethod for SelfWebviewSelectionMethod {
    fn kind(&self) -> SelectionMethodKind {
        SelectionMethodKind::SelfWebview
    }

    fn availability(&self, context: &SelectionContext) -> MethodAvailability {
        if !context.is_frontmost_self() {
            return MethodAvailability::Unavailable("frontmost app is not SnapLingo".to_string());
        }

        MethodAvailability::Available
    }

    async fn acquire(&self, context: &SelectionContext) -> SelectionAttempt {
        match read_focused_window_selection().await {
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

pub(crate) async fn read_focused_window_selection() -> Result<String, String> {
    let (sender, receiver) = tokio::sync::oneshot::channel();
    crate::infrastructure::ui_thread::dispatch(move || {
        if sender.is_closed() {
            return;
        }
        let app = NSApplication::sharedApplication(MainThreadMarker::new().unwrap());
        let Some(window) = app.keyWindow().filter(|_| app.isActive()) else {
            let _ = sender.send(Err("SnapLingo has no focused window".into()));
            return;
        };
        read_window_selection(&window, sender);
    });
    tokio::time::timeout(READ_TIMEOUT, receiver)
        .await
        .map_err(|_| "Timed out reading selected text from SnapLingo window".to_string())?
        .map_err(|e| format!("Failed to receive selected text from SnapLingo window: {e}"))?
}

fn read_window_selection(
    window: &NSWindow,
    sender: tokio::sync::oneshot::Sender<Result<String, String>>,
) {
    if let Some(text) = window
        .firstResponder()
        .and_then(|responder| responder.downcast::<NSTextView>().ok())
    {
        let selection = text
            .string()
            .substringWithRange(text.selectedRange())
            .to_string();
        let _ = sender.send(Ok(selection));
        return;
    }
    let result = window
        .firstResponder()
        .filter(|responder| {
            responder.respondsToSelector(sel!(selectedRange))
                && responder
                    .respondsToSelector(sel!(attributedSubstringForProposedRange:actualRange:))
        })
        .ok_or_else(|| "Focused SnapLingo window has no text selection reader".to_string())
        .and_then(|responder| {
            // Rust canvases implement NSTextInputClient. AppKit expresses both
            // the selected range and substring in UTF-16, including IME text.
            let range: NSRange = unsafe { msg_send![&*responder, selectedRange] };
            if range.location == NSNotFound as usize || range.length == 0 {
                return Ok(String::new());
            }
            let text: Option<Retained<NSAttributedString>> = unsafe {
                msg_send![&*responder, attributedSubstringForProposedRange: range,
                    actualRange: std::ptr::null_mut::<NSRange>()]
            };
            text.map(|text| text.string().to_string())
                .ok_or_else(|| "Focused SnapLingo text selection is unavailable".to_string())
        });
    let _ = sender.send(result);
}
