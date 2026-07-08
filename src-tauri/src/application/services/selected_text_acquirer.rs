use std::sync::Arc;

use crate::domain::{
    MethodAvailability, SelectedTextSnapshot, SelectionAttemptStatus, SelectionContext,
    SelectionMethodKind,
};
use crate::infrastructure::system::selection::{SelectionContextProvider, SelectionMethodRegistry};
use crate::{AppError, Result};

pub struct SelectionScheme {
    ordered_methods: Vec<SelectionMethodKind>,
}

impl SelectionScheme {
    pub fn new(ordered_methods: Vec<SelectionMethodKind>) -> Self {
        Self { ordered_methods }
    }
}

pub struct SelectedTextAcquirer {
    scheme: SelectionScheme,
    registry: SelectionMethodRegistry,
    context_provider: Arc<dyn SelectionContextProvider>,
}

impl SelectedTextAcquirer {
    pub fn new(
        scheme: SelectionScheme,
        registry: SelectionMethodRegistry,
        context_provider: Arc<dyn SelectionContextProvider>,
    ) -> Self {
        Self {
            scheme,
            registry,
            context_provider,
        }
    }

    pub async fn acquire(&self) -> Result<SelectedTextSnapshot> {
        self.acquire_with_context(self.context_provider.context())
            .await
    }

    pub async fn acquire_with_context(
        &self,
        context: SelectionContext,
    ) -> Result<SelectedTextSnapshot> {
        let mut diagnostics = Vec::new();

        for kind in &self.scheme.ordered_methods {
            let Some(method) = self.registry.get(*kind) else {
                diagnostics.push(format!("{kind:?}: not registered"));
                continue;
            };

            match method.availability(&context) {
                MethodAvailability::Available => {}
                MethodAvailability::Unsupported(reason) => {
                    diagnostics.push(format!("{kind:?}: unsupported: {reason}"));
                    continue;
                }
                MethodAvailability::Unavailable(reason) => {
                    diagnostics.push(format!("{kind:?}: unavailable: {reason}"));
                    continue;
                }
            }

            let attempt = method.acquire(&context).await;
            let method_name = format!("{:?}", attempt.method);
            let diagnostic = match &attempt.status {
                SelectionAttemptStatus::Success { .. } | SelectionAttemptStatus::Empty => {
                    format!("{method_name}: no valid text")
                }
                SelectionAttemptStatus::Unavailable(reason) => {
                    format!("{method_name}: unavailable: {reason}")
                }
                SelectionAttemptStatus::Failed(reason) => {
                    format!("{method_name}: failed: {reason}")
                }
            };

            if let Some(snapshot) = attempt.into_valid_snapshot() {
                log::info!("Selected text acquired through {method_name}");
                return Ok(snapshot);
            }
            diagnostics.push(diagnostic);
        }

        Err(AppError::System(format!(
            "划词翻译没有获取到文本。尝试过的取词方式：{}",
            diagnostics.join("; ")
        )))
    }
}

#[cfg(test)]
mod selected_text_acquirer_tests {
    use super::*;
    use crate::domain::{
        MethodAvailability, SelectionAttempt, SelectionContext, SelectionMethodKind,
        SelectionSource,
    };
    use crate::infrastructure::system::selection::{
        SelectionContextProvider, SelectionMethod, SelectionMethodRegistry,
    };
    use async_trait::async_trait;
    use std::sync::{Arc, Mutex};

    struct FakeMethod {
        kind: SelectionMethodKind,
        availability: MethodAvailability,
        result: SelectionAttemptStatusForTest,
        calls: Arc<Mutex<Vec<SelectionMethodKind>>>,
    }

    enum SelectionAttemptStatusForTest {
        Text(&'static str, SelectionSource),
        Empty,
        Unavailable(&'static str),
        Failed(&'static str),
    }

    struct FakeContextProvider;

    impl SelectionContextProvider for FakeContextProvider {
        fn context(&self) -> SelectionContext {
            SelectionContext::default()
        }
    }

    #[async_trait]
    impl SelectionMethod for FakeMethod {
        fn kind(&self) -> SelectionMethodKind {
            self.kind
        }

        fn availability(&self, _context: &SelectionContext) -> MethodAvailability {
            self.availability.clone()
        }

        async fn acquire(&self, context: &SelectionContext) -> SelectionAttempt {
            self.calls.lock().unwrap().push(self.kind);
            match self.result {
                SelectionAttemptStatusForTest::Text(text, source) => {
                    SelectionAttempt::success(self.kind, source, text.to_string(), context.clone())
                }
                SelectionAttemptStatusForTest::Empty => {
                    SelectionAttempt::empty(self.kind, context.clone())
                }
                SelectionAttemptStatusForTest::Unavailable(message) => {
                    SelectionAttempt::unavailable(self.kind, context.clone(), message.to_string())
                }
                SelectionAttemptStatusForTest::Failed(message) => {
                    SelectionAttempt::failed(self.kind, context.clone(), message.to_string())
                }
            }
        }
    }

    #[tokio::test]
    async fn acquirer_uses_scheme_order_and_stops_on_first_text() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let acquirer = SelectedTextAcquirer::new(
            SelectionScheme::new(vec![
                SelectionMethodKind::Accessibility,
                SelectionMethodKind::MenuCopy,
                SelectionMethodKind::ShortcutCopy,
            ]),
            SelectionMethodRegistry::new(vec![
                Box::new(FakeMethod {
                    kind: SelectionMethodKind::Accessibility,
                    availability: MethodAvailability::Available,
                    result: SelectionAttemptStatusForTest::Empty,
                    calls: calls.clone(),
                }),
                Box::new(FakeMethod {
                    kind: SelectionMethodKind::MenuCopy,
                    availability: MethodAvailability::Available,
                    result: SelectionAttemptStatusForTest::Text(
                        "menu text",
                        SelectionSource::MenuCopy,
                    ),
                    calls: calls.clone(),
                }),
                Box::new(FakeMethod {
                    kind: SelectionMethodKind::ShortcutCopy,
                    availability: MethodAvailability::Available,
                    result: SelectionAttemptStatusForTest::Text(
                        "shortcut text",
                        SelectionSource::ShortcutCopy,
                    ),
                    calls: calls.clone(),
                }),
            ]),
            Arc::new(FakeContextProvider),
        );

        let snapshot = acquirer
            .acquire_with_context(SelectionContext::default())
            .await
            .unwrap();

        assert_eq!(snapshot.text, "menu text");
        assert_eq!(
            *calls.lock().unwrap(),
            vec![
                SelectionMethodKind::Accessibility,
                SelectionMethodKind::MenuCopy,
            ]
        );
    }

    #[tokio::test]
    async fn acquirer_skips_unavailable_methods() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let acquirer = SelectedTextAcquirer::new(
            SelectionScheme::new(vec![
                SelectionMethodKind::BrowserScript,
                SelectionMethodKind::MenuCopy,
            ]),
            SelectionMethodRegistry::new(vec![
                Box::new(FakeMethod {
                    kind: SelectionMethodKind::BrowserScript,
                    availability: MethodAvailability::Unavailable("not browser".to_string()),
                    result: SelectionAttemptStatusForTest::Text(
                        "browser text",
                        SelectionSource::BrowserScript,
                    ),
                    calls: calls.clone(),
                }),
                Box::new(FakeMethod {
                    kind: SelectionMethodKind::MenuCopy,
                    availability: MethodAvailability::Available,
                    result: SelectionAttemptStatusForTest::Text(
                        "menu text",
                        SelectionSource::MenuCopy,
                    ),
                    calls: calls.clone(),
                }),
            ]),
            Arc::new(FakeContextProvider),
        );

        let snapshot = acquirer
            .acquire_with_context(SelectionContext::default())
            .await
            .unwrap();

        assert_eq!(snapshot.text, "menu text");
        assert_eq!(*calls.lock().unwrap(), vec![SelectionMethodKind::MenuCopy]);
    }

    #[tokio::test]
    async fn acquirer_continues_after_failed_method() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let acquirer = SelectedTextAcquirer::new(
            SelectionScheme::new(vec![
                SelectionMethodKind::Accessibility,
                SelectionMethodKind::ShortcutCopy,
            ]),
            SelectionMethodRegistry::new(vec![
                Box::new(FakeMethod {
                    kind: SelectionMethodKind::Accessibility,
                    availability: MethodAvailability::Available,
                    result: SelectionAttemptStatusForTest::Failed("ax failed"),
                    calls: calls.clone(),
                }),
                Box::new(FakeMethod {
                    kind: SelectionMethodKind::ShortcutCopy,
                    availability: MethodAvailability::Available,
                    result: SelectionAttemptStatusForTest::Text(
                        "shortcut text",
                        SelectionSource::ShortcutCopy,
                    ),
                    calls: calls.clone(),
                }),
            ]),
            Arc::new(FakeContextProvider),
        );

        let snapshot = acquirer
            .acquire_with_context(SelectionContext::default())
            .await
            .unwrap();

        assert_eq!(snapshot.text, "shortcut text");
        assert_eq!(
            *calls.lock().unwrap(),
            vec![
                SelectionMethodKind::Accessibility,
                SelectionMethodKind::ShortcutCopy,
            ]
        );
    }

    #[tokio::test]
    async fn acquirer_reports_attempted_methods_when_no_text_is_acquired() {
        let acquirer = SelectedTextAcquirer::new(
            SelectionScheme::new(vec![
                SelectionMethodKind::Accessibility,
                SelectionMethodKind::ShortcutCopy,
            ]),
            SelectionMethodRegistry::new(vec![
                Box::new(FakeMethod {
                    kind: SelectionMethodKind::Accessibility,
                    availability: MethodAvailability::Available,
                    result: SelectionAttemptStatusForTest::Failed("ax failed"),
                    calls: Arc::new(Mutex::new(Vec::new())),
                }),
                Box::new(FakeMethod {
                    kind: SelectionMethodKind::ShortcutCopy,
                    availability: MethodAvailability::Available,
                    result: SelectionAttemptStatusForTest::Empty,
                    calls: Arc::new(Mutex::new(Vec::new())),
                }),
            ]),
            Arc::new(FakeContextProvider),
        );

        let err = acquirer
            .acquire_with_context(SelectionContext::default())
            .await
            .unwrap_err();

        let AppError::System(message) = err else {
            panic!("expected system error");
        };

        assert!(message.contains("Accessibility: failed: ax failed"));
        assert!(message.contains("ShortcutCopy: no valid text"));
    }

    #[tokio::test]
    async fn acquirer_preserves_unsupported_and_unavailable_reasons() {
        let acquirer = SelectedTextAcquirer::new(
            SelectionScheme::new(vec![
                SelectionMethodKind::SelfWebview,
                SelectionMethodKind::BrowserScript,
                SelectionMethodKind::ShortcutCopy,
            ]),
            SelectionMethodRegistry::new(vec![
                Box::new(FakeMethod {
                    kind: SelectionMethodKind::SelfWebview,
                    availability: MethodAvailability::Unsupported("requires macOS".to_string()),
                    result: SelectionAttemptStatusForTest::Text(
                        "self text",
                        SelectionSource::SelfWebview,
                    ),
                    calls: Arc::new(Mutex::new(Vec::new())),
                }),
                Box::new(FakeMethod {
                    kind: SelectionMethodKind::BrowserScript,
                    availability: MethodAvailability::Available,
                    result: SelectionAttemptStatusForTest::Unavailable(
                        "browser scripting disabled",
                    ),
                    calls: Arc::new(Mutex::new(Vec::new())),
                }),
                Box::new(FakeMethod {
                    kind: SelectionMethodKind::ShortcutCopy,
                    availability: MethodAvailability::Available,
                    result: SelectionAttemptStatusForTest::Failed("shortcut failed"),
                    calls: Arc::new(Mutex::new(Vec::new())),
                }),
            ]),
            Arc::new(FakeContextProvider),
        );

        let err = acquirer
            .acquire_with_context(SelectionContext::default())
            .await
            .unwrap_err();

        let AppError::System(message) = err else {
            panic!("expected system error");
        };

        assert!(message.contains("SelfWebview: unsupported: requires macOS"));
        assert!(message.contains("BrowserScript: unavailable: browser scripting disabled"));
        assert!(message.contains("ShortcutCopy: failed: shortcut failed"));
    }

    #[tokio::test]
    async fn acquirer_preserves_existing_macos_method_ordering_assumptions() {
        let calls = Arc::new(Mutex::new(Vec::new()));
        let acquirer = SelectedTextAcquirer::new(
            SelectionScheme::new(vec![
                SelectionMethodKind::SelfWebview,
                SelectionMethodKind::Accessibility,
                SelectionMethodKind::BrowserScript,
                SelectionMethodKind::MenuCopy,
                SelectionMethodKind::ShortcutCopy,
            ]),
            SelectionMethodRegistry::new(vec![
                Box::new(FakeMethod {
                    kind: SelectionMethodKind::SelfWebview,
                    availability: MethodAvailability::Available,
                    result: SelectionAttemptStatusForTest::Empty,
                    calls: calls.clone(),
                }),
                Box::new(FakeMethod {
                    kind: SelectionMethodKind::Accessibility,
                    availability: MethodAvailability::Available,
                    result: SelectionAttemptStatusForTest::Empty,
                    calls: calls.clone(),
                }),
                Box::new(FakeMethod {
                    kind: SelectionMethodKind::BrowserScript,
                    availability: MethodAvailability::Available,
                    result: SelectionAttemptStatusForTest::Empty,
                    calls: calls.clone(),
                }),
                Box::new(FakeMethod {
                    kind: SelectionMethodKind::MenuCopy,
                    availability: MethodAvailability::Available,
                    result: SelectionAttemptStatusForTest::Empty,
                    calls: calls.clone(),
                }),
                Box::new(FakeMethod {
                    kind: SelectionMethodKind::ShortcutCopy,
                    availability: MethodAvailability::Available,
                    result: SelectionAttemptStatusForTest::Text(
                        "shortcut text",
                        SelectionSource::ShortcutCopy,
                    ),
                    calls: calls.clone(),
                }),
            ]),
            Arc::new(FakeContextProvider),
        );

        let snapshot = acquirer
            .acquire_with_context(SelectionContext::default())
            .await
            .unwrap();

        assert_eq!(snapshot.text, "shortcut text");
        assert_eq!(
            *calls.lock().unwrap(),
            vec![
                SelectionMethodKind::SelfWebview,
                SelectionMethodKind::Accessibility,
                SelectionMethodKind::BrowserScript,
                SelectionMethodKind::MenuCopy,
                SelectionMethodKind::ShortcutCopy,
            ]
        );
    }
}
