use std::sync::Arc;

use super::{SelectedTextAcquirer, SelectionTextMode};
use crate::result::ResultWindowOpenRequest;
use crate::result::ResultWindowRuntime;
use crate::settings::SettingsConfiguration;

pub struct SelectionTranslation {
    settings: Arc<SettingsConfiguration>,
    acquirer: Arc<SelectedTextAcquirer>,
    result_window: Arc<ResultWindowRuntime>,
}

impl SelectionTranslation {
    pub fn new(
        settings: Arc<SettingsConfiguration>,
        acquirer: Arc<SelectedTextAcquirer>,
        result_window: Arc<ResultWindowRuntime>,
    ) -> Self {
        Self {
            settings,
            acquirer,
            result_window,
        }
    }

    pub async fn open(&self) -> crate::Result<()> {
        let settings = self.settings.snapshot()?;
        let text = self
            .acquirer
            .acquire_with_mode(SelectionTextMode::from_setting(
                &settings.translation.selection_text_mode,
            ))
            .await?;
        self.result_window
            .open(ResultWindowOpenRequest::selection_translation(text.text))
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::result::ResultWindowNotifierPort;
    use crate::result::ResultWindowOrigin;
    use crate::result::ResultWindowRequestId;
    use crate::result::ResultWindowWindowPort;
    use crate::selection::model::MethodAvailability;
    use crate::selection::model::SelectionAttempt;
    use crate::selection::model::SelectionContext;
    use crate::selection::model::SelectionMethodKind;
    use crate::selection::model::SelectionSource;
    use crate::selection::SelectionContextProvider;
    use crate::selection::SelectionMethod;
    use crate::selection::SelectionScheme;
    use crate::test_support_config::MemoryConfigStore;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Context;
    impl SelectionContextProvider for Context {
        fn context(&self) -> SelectionContext {
            SelectionContext::default()
        }
    }
    struct Method {
        kind: SelectionMethodKind,
        empty: bool,
    }
    #[async_trait]
    impl SelectionMethod for Method {
        fn kind(&self) -> SelectionMethodKind {
            self.kind
        }
        fn availability(&self, _: &SelectionContext) -> MethodAvailability {
            MethodAvailability::Available
        }
        async fn acquire(&self, context: &SelectionContext) -> SelectionAttempt {
            if self.empty {
                return SelectionAttempt::empty(self.kind, context.clone());
            }
            SelectionAttempt::success(
                self.kind,
                SelectionSource::Accessibility,
                format!("{:?}", self.kind),
                context.clone(),
            )
        }
    }
    #[derive(Default)]
    struct Window(AtomicUsize);
    #[async_trait]
    impl ResultWindowWindowPort for Window {
        async fn show_or_create(&self) -> crate::Result<()> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }
    #[async_trait]
    impl ResultWindowNotifierPort for Window {
        async fn notify_payload_ready(&self, _: ResultWindowRequestId) -> crate::Result<()> {
            Ok(())
        }
    }
    fn workflow(
        empty: bool,
    ) -> (
        SelectionTranslation,
        Arc<SettingsConfiguration>,
        Arc<Window>,
        Arc<ResultWindowRuntime>,
    ) {
        let settings = Arc::new(SettingsConfiguration::new(Arc::new(
            MemoryConfigStore::new_in_memory(),
        )));
        let window = Arc::new(Window::default());
        let result = Arc::new(ResultWindowRuntime::new(window.clone(), window.clone()));
        let acquirer = Arc::new(SelectedTextAcquirer::new(
            SelectionScheme::new(vec![
                SelectionMethodKind::MenuCopy,
                SelectionMethodKind::Accessibility,
            ]),
            [
                SelectionMethodKind::Accessibility,
                SelectionMethodKind::BrowserScript,
                SelectionMethodKind::MenuCopy,
            ]
            .into_iter()
            .map(|kind| Box::new(Method { kind, empty }) as Box<dyn SelectionMethod>)
            .collect(),
            Arc::new(Context),
        ));
        (
            SelectionTranslation::new(settings.clone(), acquirer, result.clone()),
            settings,
            window,
            result,
        )
    }

    #[tokio::test]
    async fn reads_current_mode_and_preserves_selection_origin() {
        let (workflow, settings, window, result) = workflow(false);
        for (mode, text) in [
            ("smart", "MenuCopy"),
            ("quality", "BrowserScript"),
            ("speed", "Accessibility"),
        ] {
            let mut translation = settings.snapshot().unwrap().translation;
            translation.selection_text_mode = mode.into();
            settings.update_translation(translation).unwrap();
            workflow.open().await.unwrap();
            let id = result.current_request_id().unwrap().unwrap();
            let payload = result.take_if_current(id).unwrap().unwrap();
            assert_eq!(payload.text, text);
            assert_eq!(payload.origin, ResultWindowOrigin::Selection);
            assert!(payload.auto_translate);
        }
        assert_eq!(window.0.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn failed_acquisition_does_not_open_or_replace_a_result() {
        let (workflow, _, window, result) = workflow(true);
        result
            .open(ResultWindowOpenRequest::manual_translation(
                "existing".into(),
            ))
            .await
            .unwrap();
        let id = result.current_request_id().unwrap().unwrap();
        assert!(workflow.open().await.is_err());
        assert_eq!(window.0.load(Ordering::SeqCst), 1);
        assert_eq!(
            result.take_if_current(id).unwrap().unwrap().text,
            "existing"
        );
    }
}
