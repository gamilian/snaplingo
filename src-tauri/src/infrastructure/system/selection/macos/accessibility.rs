use async_trait::async_trait;

use crate::application::selected_text::SelectionMethod;
use crate::domain::{
    MethodAvailability, SelectionAttempt, SelectionContext, SelectionMethodKind, SelectionSource,
};

pub struct AccessibilitySelectionMethod;

#[async_trait]
impl SelectionMethod for AccessibilitySelectionMethod {
    fn kind(&self) -> SelectionMethodKind {
        SelectionMethodKind::Accessibility
    }

    fn availability(&self, _context: &SelectionContext) -> MethodAvailability {
        if super::context::accessibility_permission_granted(false)
            || super::context::request_accessibility_permission()
        {
            MethodAvailability::Available
        } else {
            MethodAvailability::Unavailable(
                super::context::selection_accessibility_permission_error(),
            )
        }
    }

    async fn acquire(&self, context: &SelectionContext) -> SelectionAttempt {
        match super::context::read_accessibility_selected_text() {
            Ok(Some(text)) if !text.trim().is_empty() => SelectionAttempt::success(
                self.kind(),
                SelectionSource::Accessibility,
                text,
                context.clone(),
            ),
            Ok(_) => SelectionAttempt::empty(self.kind(), context.clone()),
            Err(err) => SelectionAttempt::failed(self.kind(), context.clone(), err),
        }
    }
}
