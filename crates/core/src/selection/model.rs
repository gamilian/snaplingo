use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SelectionMethodKind {
    SelfWebview,
    Accessibility,
    BrowserScript,
    MenuCopy,
    ShortcutCopy,
    PrimarySelection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionSource {
    SelfWebview,
    Accessibility,
    BrowserScript,
    MenuCopy,
    ShortcutCopy,
    PrimarySelection,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct FrontmostApp {
    pub bundle_id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SelectionContext {
    pub frontmost_app: Option<FrontmostApp>,
    pub self_bundle_id: Option<String>,
}

impl SelectionContext {
    pub fn is_frontmost_self(&self) -> bool {
        let Some(self_bundle_id) = self.self_bundle_id.as_deref() else {
            return false;
        };

        self.frontmost_app
            .as_ref()
            .and_then(|app| app.bundle_id.as_deref())
            .is_some_and(|bundle_id| bundle_id == self_bundle_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SelectedTextSnapshot {
    pub text: String,
    pub source: SelectionSource,
    pub frontmost_app: Option<FrontmostApp>,
    pub is_editable: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MethodAvailability {
    Available,
    Unsupported(String),
    Unavailable(String),
}

impl MethodAvailability {
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionAttemptStatus {
    Success {
        text: String,
        source: SelectionSource,
    },
    Empty,
    Unavailable(String),
    Failed(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectionAttempt {
    pub method: SelectionMethodKind,
    pub status: SelectionAttemptStatus,
    pub context: SelectionContext,
    pub is_editable: Option<bool>,
}

impl SelectionAttempt {
    pub fn success(
        method: SelectionMethodKind,
        source: SelectionSource,
        text: String,
        context: SelectionContext,
    ) -> Self {
        Self {
            method,
            status: SelectionAttemptStatus::Success { text, source },
            context,
            is_editable: None,
        }
    }

    pub fn empty(method: SelectionMethodKind, context: SelectionContext) -> Self {
        Self {
            method,
            status: SelectionAttemptStatus::Empty,
            context,
            is_editable: None,
        }
    }

    pub fn failed(method: SelectionMethodKind, context: SelectionContext, message: String) -> Self {
        Self {
            method,
            status: SelectionAttemptStatus::Failed(message),
            context,
            is_editable: None,
        }
    }

    pub fn unavailable(
        method: SelectionMethodKind,
        context: SelectionContext,
        message: String,
    ) -> Self {
        Self {
            method,
            status: SelectionAttemptStatus::Unavailable(message),
            context,
            is_editable: None,
        }
    }

    pub fn into_valid_snapshot(self) -> Option<SelectedTextSnapshot> {
        let SelectionAttemptStatus::Success { text, source } = self.status else {
            return None;
        };

        if text.trim().is_empty() {
            return None;
        }

        Some(SelectedTextSnapshot {
            text,
            source,
            frontmost_app: self.context.frontmost_app,
            is_editable: self.is_editable,
        })
    }
}

#[cfg(test)]
mod selection_domain_tests {
    use super::*;

    #[test]
    fn attempt_converts_non_blank_text_to_snapshot() {
        let attempt = SelectionAttempt::success(
            SelectionMethodKind::Accessibility,
            SelectionSource::Accessibility,
            " selected text ".to_string(),
            SelectionContext::default(),
        );

        let snapshot = attempt.into_valid_snapshot().unwrap();

        assert_eq!(snapshot.text, " selected text ");
        assert_eq!(snapshot.source, SelectionSource::Accessibility);
    }

    #[test]
    fn attempt_rejects_blank_text() {
        let attempt = SelectionAttempt::success(
            SelectionMethodKind::ShortcutCopy,
            SelectionSource::ShortcutCopy,
            "   ".to_string(),
            SelectionContext::default(),
        );

        assert!(attempt.into_valid_snapshot().is_none());
    }
}
