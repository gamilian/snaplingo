use super::*;
use async_trait::async_trait;
use snaplingo_core::providers::common::CredentialField;
use snaplingo_core::providers::*;
use snaplingo_core::settings::*;
use snaplingo_core::settings::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
mod cases {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;
    use std::time::Duration;
    use tokio::sync::{oneshot, Notify};

    struct Gate {
        operation: &'static str,
        entered: mpsc::Sender<()>,
        release: mpsc::Receiver<()>,
    }
    struct Providers {
        providers: Mutex<Vec<ProviderInfo>>,
        calls: Mutex<Vec<String>>,
        fail: Mutex<Option<&'static str>>,
        gate: Mutex<Option<Gate>>,
        models: Mutex<VecDeque<oneshot::Receiver<()>>>,
        tests: Mutex<VecDeque<oneshot::Receiver<()>>>,
        entered: Notify,
        credentials: Mutex<HashMap<String, String>>,
        update_key: Mutex<Option<Option<String>>>,
        prompts: Mutex<TranslationPromptStrategyConfig>,
    }
    fn provider(id: &str, active: bool, builtin: bool) -> ProviderInfo {
        ProviderInfo {
            id: id.into(),
            name: id.into(),
            is_configured: true,
            requires_api_key: true,
            is_active: active,
            is_builtin: builtin,
            is_local: false,
            protocol: (!builtin).then(|| "openai".into()),
            endpoint: (!builtin).then(|| "https://fixture.test".into()),
            model: (!builtin).then(|| "model".into()),
            reasoning_level: None,
            prompt_strategy_id: Some("general".into()),
            prompt_fallback_strategy_id: Some("general".into()),
        }
    }
    impl Providers {
        fn new() -> Self {
            Self {
                providers: Mutex::new(vec![
                    provider("first", true, true),
                    provider("second", true, false),
                    provider("deeplx", false, true),
                    provider("fourth", false, false),
                ]),
                calls: Mutex::new(vec![]),
                fail: Mutex::new(None),
                gate: Mutex::new(None),
                models: Mutex::new(VecDeque::new()),
                tests: Mutex::new(VecDeque::new()),
                entered: Notify::new(),
                credentials: Mutex::new(HashMap::new()),
                update_key: Mutex::new(None),
                prompts: Mutex::new(snaplingo_core::providers::default_prompt_strategy_config()),
            }
        }
        fn block(&self, operation: &'static str) -> (mpsc::Receiver<()>, mpsc::Sender<()>) {
            let (entered, receive) = mpsc::channel();
            let (release, gate) = mpsc::channel();
            *self.gate.lock().unwrap() = Some(Gate {
                operation,
                entered,
                release: gate,
            });
            (receive, release)
        }
        fn attempt(&self, operation: &'static str, id: &str) -> crate::Result<()> {
            self.calls.lock().unwrap().push(format!("{operation}:{id}"));
            let gate = {
                let mut slot = self.gate.lock().unwrap();
                if slot
                    .as_ref()
                    .is_some_and(|gate| gate.operation == operation)
                {
                    slot.take()
                } else {
                    None
                }
            };
            if let Some(gate) = gate {
                gate.entered.send(()).unwrap();
                gate.release.recv_timeout(Duration::from_secs(5)).unwrap();
            }
            if *self.fail.lock().unwrap() == Some(operation) {
                Err(format!("{operation} failed").into())
            } else {
                Ok(())
            }
        }
    }
    #[async_trait]
    impl TranslationServices for Providers {
        fn list(&self) -> Vec<ProviderInfo> {
            self.providers.lock().unwrap().clone()
        }
        fn credential_schema(&self, id: &str) -> crate::Result<Vec<CredentialField>> {
            self.attempt("schema", id)?;
            Ok(vec![CredentialField::new("api_key", "Key", true)])
        }
        fn configure(&self, id: &str, credentials: HashMap<String, String>) -> crate::Result<()> {
            self.attempt("configure", id)?;
            *self.credentials.lock().unwrap() = credentials;
            Ok(())
        }
        fn set_active(&self, id: &str, active: bool) -> crate::Result<()> {
            self.attempt("active", id)?;
            self.providers
                .lock()
                .unwrap()
                .iter_mut()
                .find(|provider| provider.id == id)
                .unwrap()
                .is_active = active;
            Ok(())
        }
        fn reorder(&self, ids: Vec<String>, active: bool) -> crate::Result<()> {
            self.attempt("reorder", &format!("{active}:{}", ids.join(",")))
        }
        fn add(&self, input: AddCustomTranslationProviderInput) -> crate::Result<()> {
            self.attempt("add", &input.name)?;
            if input.api_key.is_empty() {
                return Err("API key cannot be empty".into());
            }
            assert_eq!(
                input.prompt_fallback_strategy_id.as_deref(),
                Some("general")
            );
            self.providers
                .lock()
                .unwrap()
                .push(provider("new", true, false));
            Ok(())
        }
        fn update(
            &self,
            id: &str,
            input: UpdateCustomTranslationProviderInput,
        ) -> crate::Result<()> {
            self.attempt("update", id)?;
            *self.update_key.lock().unwrap() = Some(input.api_key);
            assert_eq!(
                input.prompt_fallback_strategy_id.as_deref(),
                Some("general")
            );
            Ok(())
        }
        fn remove(&self, id: &str) -> crate::Result<()> {
            self.attempt("remove", id)?;
            self.providers
                .lock()
                .unwrap()
                .retain(|provider| provider.id != id);
            Ok(())
        }
        async fn test_saved(&self, id: &str) -> crate::Result<()> {
            self.attempt("saved-test", id)
        }
        async fn list_models(
            &self,
            protocol: LLMProtocol,
            input: ProviderModelListInput,
        ) -> crate::Result<Vec<ModelInfo>> {
            let gate = self.models.lock().unwrap().pop_front();
            self.entered.notify_one();
            if let Some(gate) = gate {
                gate.await.unwrap();
            }
            self.attempt("models", protocol.as_str())?;
            Ok(vec![ModelInfo { id: input.endpoint }])
        }
        async fn test_connection(
            &self,
            protocol: LLMProtocol,
            _: ProviderConnectionTestInput,
        ) -> crate::Result<()> {
            let gate = self.tests.lock().unwrap().pop_front();
            self.entered.notify_one();
            if let Some(gate) = gate {
                gate.await.unwrap();
            }
            self.attempt("connection", protocol.as_str())
        }
        fn prompt_strategies(&self) -> TranslationPromptStrategyConfig {
            self.prompts.lock().unwrap().clone()
        }
        fn save_prompt_strategies(
            &self,
            config: TranslationPromptStrategyConfig,
        ) -> crate::Result<TranslationPromptStrategyConfig> {
            self.attempt("prompt", "")?;
            snaplingo_core::providers::validate_prompt_strategy_config(&config)?;
            let config = snaplingo_core::providers::sanitize_prompt_strategy_config(config);
            *self.prompts.lock().unwrap() = config.clone();
            Ok(config)
        }
    }
    fn fixture() -> (Arc<TranslationServicesController>, Arc<Providers>) {
        let providers = Arc::new(Providers::new());
        (
            Arc::new(TranslationServicesController::new(providers.clone())),
            providers,
        )
    }
    async fn send(controller: &TranslationServicesController, request: TranslationServicesRequest) {
        controller.dispatch(request, || {}).await.unwrap();
    }
    async fn edit(controller: &TranslationServicesController, edit: CustomTranslationEdit) {
        let revision = controller.projection().form_revision;
        send(
            controller,
            TranslationServicesRequest::EditCustom { revision, edit },
        )
        .await;
    }
    fn custom(
        controller: &TranslationServicesController,
    ) -> (CustomTranslationDraft, TranslationPromptWorkspace) {
        match controller.projection().form.unwrap() {
            TranslationServiceForm::Custom { draft, prompts } => (draft, prompts),
            _ => panic!("expected custom form"),
        }
    }

    #[tokio::test]
    async fn credentials_save_then_activate_retries_failures_and_filters_deepl_mode() {
        let (controller, providers) = fixture();
        send(
            &controller,
            TranslationServicesRequest::Configure("deeplx".into()),
        )
        .await;
        let revision = controller.projection().form_revision;
        *providers.fail.lock().unwrap() = Some("configure");
        assert!(controller
            .dispatch(
                TranslationServicesRequest::SaveCredentials {
                    revision,
                    credentials: HashMap::from([
                        ("mode".into(), "deepl".into()),
                        ("api_key".into(), "key".into()),
                        ("endpoint".into(), String::new())
                    ])
                },
                || {}
            )
            .await
            .is_err());
        assert!(controller.projection().form.is_some());
        assert!(!providers
            .calls
            .lock()
            .unwrap()
            .iter()
            .any(|call| call == "active:deeplx"));
        *providers.fail.lock().unwrap() = Some("active");
        assert!(controller
            .dispatch(
                TranslationServicesRequest::SaveCredentials {
                    revision,
                    credentials: HashMap::from([
                        ("endpoint".into(), "https://deepl.test".into()),
                        ("api_key".into(), String::new())
                    ])
                },
                || {}
            )
            .await
            .is_err());
        assert!(controller.projection().form.is_some());
        *providers.fail.lock().unwrap() = None;
        send(
            &controller,
            TranslationServicesRequest::SaveCredentials {
                revision,
                credentials: HashMap::from([
                    ("mode".into(), "deepl".into()),
                    ("api_key".into(), "key".into()),
                    ("endpoint".into(), String::new()),
                ]),
            },
        )
        .await;
        assert_eq!(
            *providers.credentials.lock().unwrap(),
            HashMap::from([
                ("mode".into(), "deepl".into()),
                ("api_key".into(), "key".into())
            ])
        );
        assert!(controller.projection().form.is_none());
    }

    #[tokio::test]
    async fn custom_save_preserves_blank_edit_keys_and_waits_for_success() {
        let (controller, providers) = fixture();
        send(
            &controller,
            TranslationServicesRequest::Configure("second".into()),
        )
        .await;
        let revision = controller.projection().form_revision;
        assert!(custom(&controller).0.api_key.is_empty());
        edit(&controller, CustomTranslationEdit::Name(" renamed ".into())).await;
        *providers.fail.lock().unwrap() = Some("update");
        assert!(controller
            .dispatch(TranslationServicesRequest::SaveCustom { revision }, || {})
            .await
            .is_err());
        assert_eq!(custom(&controller).0.name, " renamed ");
        *providers.fail.lock().unwrap() = None;
        send(
            &controller,
            TranslationServicesRequest::SaveCustom { revision },
        )
        .await;
        assert_eq!(*providers.update_key.lock().unwrap(), Some(None));
        assert!(controller.projection().form.is_none());
        send(&controller, TranslationServicesRequest::AddCustom).await;
        let revision = controller.projection().form_revision;
        edit(&controller, CustomTranslationEdit::Name("new".into())).await;
        edit(
            &controller,
            CustomTranslationEdit::ApiKey(" new-key ".into()),
        )
        .await;
        send(
            &controller,
            TranslationServicesRequest::SaveCustom { revision },
        )
        .await;
        assert!(controller
            .projection()
            .providers
            .iter()
            .any(|provider| provider.id == "new" && provider.is_active));
        send(
            &controller,
            TranslationServicesRequest::TestSaved("new".into()),
        )
        .await;
        assert_eq!(
            controller.projection().test_status,
            Some(Ok("检测成功".into()))
        );
        send(
            &controller,
            TranslationServicesRequest::Remove("new".into()),
        )
        .await;
        assert!(!controller
            .projection()
            .providers
            .iter()
            .any(|provider| provider.id == "new"));
    }

    #[tokio::test]
    async fn reorder_uses_latest_complete_group_and_never_moves_across_activation_groups() {
        let (controller, providers) = fixture();
        for (source, target) in [
            ("first", "deeplx"),
            ("missing", "first"),
            ("first", "first"),
        ] {
            send(
                &controller,
                TranslationServicesRequest::Reorder {
                    source_id: source.into(),
                    target_id: target.into(),
                },
            )
            .await;
        }
        assert!(providers.calls.lock().unwrap().is_empty());
        send(
            &controller,
            TranslationServicesRequest::Reorder {
                source_id: "second".into(),
                target_id: "first".into(),
            },
        )
        .await;
        send(
            &controller,
            TranslationServicesRequest::Reorder {
                source_id: "fourth".into(),
                target_id: "deeplx".into(),
            },
        )
        .await;
        assert_eq!(
            *providers.calls.lock().unwrap(),
            ["reorder:true:second,first", "reorder:false:fourth,deeplx"]
        );
    }

    #[test]
    fn cancelled_or_disposed_schema_cannot_reopen_an_obsolete_form() {
        for dispose in [false, true] {
            let (controller, providers) = fixture();
            let (entered, release) = providers.block("schema");
            let worker = controller.clone();
            let pending = std::thread::spawn(move || {
                tokio::runtime::Runtime::new().unwrap().block_on(
                    worker.dispatch(TranslationServicesRequest::Configure("first".into()), || {}),
                )
            });
            entered.recv_timeout(Duration::from_secs(5)).unwrap();
            if dispose {
                controller.dispose();
            } else {
                let runtime = tokio::runtime::Runtime::new().unwrap();
                runtime.block_on(send(&controller, TranslationServicesRequest::Cancel));
                runtime.block_on(send(
                    &controller,
                    TranslationServicesRequest::Configure("second".into()),
                ));
            }
            release.send(()).unwrap();
            pending.join().unwrap().unwrap();
            if dispose {
                assert!(controller.projection().form.is_none());
            } else {
                assert_eq!(custom(&controller).0.provider_id.as_deref(), Some("second"));
            }
            assert!(!controller.projection().busy);
        }
    }

    #[test]
    fn pending_save_excludes_duplicate_submit_cancel_and_edits() {
        let (controller, providers) = fixture();
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(send(
            &controller,
            TranslationServicesRequest::Configure("second".into()),
        ));
        let revision = controller.projection().form_revision;
        let (entered, release) = providers.block("update");
        let worker = controller.clone();
        let pending = std::thread::spawn(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(
                worker.dispatch(TranslationServicesRequest::SaveCustom { revision }, || {}),
            )
        });
        entered.recv_timeout(Duration::from_secs(5)).unwrap();
        assert!(controller.projection().busy);
        runtime.block_on(send(&controller, TranslationServicesRequest::Cancel));
        runtime.block_on(send(
            &controller,
            TranslationServicesRequest::SaveCustom { revision },
        ));
        runtime.block_on(edit(
            &controller,
            CustomTranslationEdit::Name("ignored".into()),
        ));
        assert_eq!(custom(&controller).0.name, "second");
        release.send(()).unwrap();
        pending.join().unwrap().unwrap();
        assert_eq!(
            providers
                .calls
                .lock()
                .unwrap()
                .iter()
                .filter(|call| *call == "update:second")
                .count(),
            1
        );
        assert!(controller.projection().form.is_none());
    }

    #[tokio::test]
    async fn draft_changes_invalidate_late_model_and_connection_results() {
        let (controller, providers) = fixture();
        send(&controller, TranslationServicesRequest::AddCustom).await;
        let revision = controller.projection().form_revision;
        edit(&controller, CustomTranslationEdit::ApiKey("key".into())).await;
        let (release, gate) = oneshot::channel();
        providers.models.lock().unwrap().push_back(gate);
        let worker = controller.clone();
        let pending = tokio::spawn(async move {
            worker
                .dispatch(TranslationServicesRequest::ListModels { revision }, || {})
                .await
        });
        providers.entered.notified().await;
        assert!(controller.projection().loading_models);
        edit(
            &controller,
            CustomTranslationEdit::Endpoint("https://new.test".into()),
        )
        .await;
        send(
            &controller,
            TranslationServicesRequest::ListModels { revision },
        )
        .await;
        assert_eq!(controller.projection().models[0].id, "https://new.test");
        providers.entered.notified().await;
        release.send(()).unwrap();
        pending.await.unwrap().unwrap();
        assert_eq!(controller.projection().models[0].id, "https://new.test");
        let (release, gate) = oneshot::channel();
        providers.tests.lock().unwrap().push_back(gate);
        let worker = controller.clone();
        let pending = tokio::spawn(async move {
            worker
                .dispatch(
                    TranslationServicesRequest::TestConnection { revision },
                    || {},
                )
                .await
        });
        providers.entered.notified().await;
        edit(
            &controller,
            CustomTranslationEdit::Model("different".into()),
        )
        .await;
        release.send(()).unwrap();
        pending.await.unwrap().unwrap();
        assert!(controller.projection().test_status.is_none());
        edit(
            &controller,
            CustomTranslationEdit::Protocol(LLMProtocol::OpenAIResponses),
        )
        .await;
        let draft = custom(&controller).0;
        assert_eq!(draft.endpoint, "https://api.openai.com");
        assert_eq!(draft.model, "gpt-5-mini");
        assert!(controller.projection().models.is_empty());
    }

    #[tokio::test]
    async fn stale_form_events_and_disposed_network_completions_do_not_modify_the_next_form() {
        let (controller, providers) = fixture();
        send(&controller, TranslationServicesRequest::AddCustom).await;
        let revision = controller.projection().form_revision;
        edit(&controller, CustomTranslationEdit::ApiKey("key".into())).await;
        let (release, gate) = oneshot::channel();
        providers.models.lock().unwrap().push_back(gate);
        let worker = controller.clone();
        let changes = Arc::new(AtomicUsize::new(0));
        let sent = changes.clone();
        let pending = tokio::spawn(async move {
            worker
                .dispatch(TranslationServicesRequest::ListModels { revision }, || {
                    sent.fetch_add(1, Ordering::SeqCst);
                })
                .await
        });
        providers.entered.notified().await;
        controller.dispose();
        release.send(()).unwrap();
        pending.await.unwrap().unwrap();
        assert!(controller.projection().form.is_none());
        assert!(controller.projection().models.is_empty());
        assert_eq!(changes.load(Ordering::SeqCst), 1);
        let (controller, _) = fixture();
        send(&controller, TranslationServicesRequest::AddCustom).await;
        let obsolete = controller.projection().form_revision;
        send(&controller, TranslationServicesRequest::Cancel).await;
        send(&controller, TranslationServicesRequest::AddCustom).await;
        send(
            &controller,
            TranslationServicesRequest::EditCustom {
                revision: obsolete,
                edit: CustomTranslationEdit::Name("obsolete".into()),
            },
        )
        .await;
        send(
            &controller,
            TranslationServicesRequest::SaveCustom { revision: obsolete },
        )
        .await;
        assert_eq!(custom(&controller).0.name, "");
    }

    #[tokio::test]
    async fn prompt_workspace_preserves_multiline_text_builtin_identity_and_retryable_errors() {
        let (controller, providers) = fixture();
        send(&controller, TranslationServicesRequest::AddCustom).await;
        let revision = controller.projection().form_revision;
        assert_eq!(custom(&controller).1.draft.name, "");
        edit(
            &controller,
            CustomTranslationEdit::PromptStrategy("general".into()),
        )
        .await;
        let draft = TranslationPromptDraft {
            name: " Custom rule ".into(),
            description: " details ".into(),
            system_prompt: " First line\n第二行 {target_lang} ".into(),
        };
        send(
            &controller,
            TranslationServicesRequest::EditPrompt { revision, draft },
        )
        .await;
        *providers.fail.lock().unwrap() = Some("prompt");
        send(
            &controller,
            TranslationServicesRequest::SavePrompt { revision },
        )
        .await;
        assert!(custom(&controller)
            .1
            .error
            .unwrap()
            .contains("prompt failed"));
        assert_eq!(custom(&controller).1.draft.name, " Custom rule ");
        *providers.fail.lock().unwrap() = None;
        send(
            &controller,
            TranslationServicesRequest::SavePrompt { revision },
        )
        .await;
        let (_, workspace) = custom(&controller);
        assert_eq!(
            workspace.draft.system_prompt,
            "First line\n第二行 {target_lang}"
        );
        assert!(!workspace.strategies.strategies[0].is_deletable);
        send(
            &controller,
            TranslationServicesRequest::DeletePrompt { revision },
        )
        .await;
        assert!(custom(&controller)
            .1
            .strategies
            .strategies
            .iter()
            .any(|strategy| strategy.id == "general"));
        send(
            &controller,
            TranslationServicesRequest::AddPrompt { revision },
        )
        .await;
        let id = custom(&controller).0.prompt_strategy_id;
        assert!(id.starts_with("custom-"));
        send(
            &controller,
            TranslationServicesRequest::DeletePrompt { revision },
        )
        .await;
        assert_eq!(custom(&controller).0.prompt_strategy_id, "general");
        assert!(!custom(&controller)
            .1
            .strategies
            .strategies
            .iter()
            .any(|strategy| strategy.id == id));
        edit(
            &controller,
            CustomTranslationEdit::PromptStrategy("smart".into()),
        )
        .await;
        assert_eq!(custom(&controller).1.draft.system_prompt, "");
        send(
            &controller,
            TranslationServicesRequest::AddPrompt { revision },
        )
        .await;
        assert!(custom(&controller).1.error.unwrap().contains("不能为空"));
        assert!(!controller.projection().busy);
    }

    #[tokio::test]
    async fn production_administration_persists_custom_credentials_metadata_and_prompt_edits() {
        use snaplingo_core::providers::ocr::OcrCoordinator;
        use snaplingo_core::providers::ocr::OcrProviderConfiguration;
        use snaplingo_core::providers::translation::TranslationCoordinator;
        use snaplingo_core::providers::LLMClient;
        use snaplingo_core::providers::LLMRequest;
        use snaplingo_core::providers::LLMResponse;
        use snaplingo_core::providers::LlmClientConfig;
        use snaplingo_core::providers::LlmIntrospection;
        use snaplingo_core::providers::LlmModelLister;
        use snaplingo_core::providers::LlmRuntime;
        use snaplingo_core::providers::ProviderConfigStore;
        use snaplingo_core::providers::ProviderConfiguration;
        use snaplingo_core::providers::ProviderCredentialStore;
        use snaplingo_core::providers::TranslationPromptConfiguration;
        use snaplingo_providers::http::HttpClient;
        use snaplingo_providers::http::HttpResponse;
        use snaplingo_providers::translation::DeepLProvider;
        use snaplingo_storage::Database;
        use snaplingo_storage::SqliteConfigStore;
        use snaplingo_storage::SqliteCredentialStore;
        struct Http;
        #[async_trait]
        impl HttpClient for Http {
            async fn get(
                &self,
                _: &str,
                _: HashMap<String, String>,
            ) -> anyhow::Result<HttpResponse> {
                unreachable!()
            }
            async fn post(
                &self,
                _: &str,
                _: HashMap<String, String>,
                _: String,
            ) -> anyhow::Result<HttpResponse> {
                unreachable!()
            }
        }
        struct Llm;
        #[async_trait]
        impl LLMClient for Llm {
            async fn generate(&self, _: &LLMRequest) -> anyhow::Result<LLMResponse> {
                Ok(LLMResponse { text: "OK".into() })
            }
        }
        #[async_trait]
        impl LlmModelLister for Llm {
            async fn list_models(&self) -> anyhow::Result<Vec<ModelInfo>> {
                Ok(vec![ModelInfo {
                    id: "real-model".into(),
                }])
            }
        }
        impl LlmRuntime for Llm {
            fn translation_client(&self, _: LlmClientConfig) -> Arc<dyn LLMClient> {
                Arc::new(Llm)
            }
            fn model_lister(&self, _: LlmClientConfig) -> Arc<dyn LlmModelLister> {
                Arc::new(Llm)
            }
        }
        fn administration(
            store: Arc<SqliteConfigStore>,
            credentials: Arc<SqliteCredentialStore>,
        ) -> Arc<ProviderAdministration> {
            let translation = Arc::new(TranslationCoordinator::new(store.clone()));
            translation
                .register(DeepLProvider::new(Arc::new(Http)))
                .unwrap();
            let ocr = Arc::new(OcrCoordinator::new(store.clone()));
            let runtime = Arc::new(Llm);
            let introspection = Arc::new(LlmIntrospection::new(runtime.clone()));
            let configuration = Arc::new(ProviderConfiguration::new(
                store.clone(),
                credentials.clone(),
                runtime,
                translation.clone(),
                introspection.clone(),
            ));
            let admin = Arc::new(ProviderAdministration::new(
                translation,
                ocr.clone(),
                configuration,
                Arc::new(OcrProviderConfiguration::new(ocr, credentials)),
                introspection,
                Arc::new(TranslationPromptConfiguration::new(store)),
            ));
            admin.hydrate_credentials();
            admin
        }
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("providers.db");
        let database = Arc::new(Database::open(&path).unwrap());
        let store = Arc::new(SqliteConfigStore::new(database.clone()));
        let credentials = Arc::new(SqliteCredentialStore::new(database));
        let admin = administration(store.clone(), credentials.clone());
        let controller = TranslationServicesController::new(admin);
        send(&controller, TranslationServicesRequest::AddCustom).await;
        edit(
            &controller,
            CustomTranslationEdit::Name(" 我的服务 ".into()),
        )
        .await;
        edit(
            &controller,
            CustomTranslationEdit::Protocol(LLMProtocol::OpenAIResponses),
        )
        .await;
        edit(
            &controller,
            CustomTranslationEdit::ApiKey(" saved-key ".into()),
        )
        .await;
        let revision = controller.projection().form_revision;
        send(
            &controller,
            TranslationServicesRequest::SaveCustom { revision },
        )
        .await;
        let info = controller
            .projection()
            .providers
            .into_iter()
            .find(|provider| !provider.is_builtin)
            .unwrap();
        assert_eq!(info.name, "我的服务");
        assert_eq!(info.protocol.as_deref(), Some("openai-responses"));
        assert!(info.is_active);
        assert_eq!(
            credentials.load_provider_credential(&info.id).unwrap(),
            "saved-key"
        );
        send(
            &controller,
            TranslationServicesRequest::SetActive {
                provider_id: info.id.clone(),
                active: false,
            },
        )
        .await;
        send(
            &controller,
            TranslationServicesRequest::Configure(info.id.clone()),
        )
        .await;
        edit(
            &controller,
            CustomTranslationEdit::Model("updated-model".into()),
        )
        .await;
        let revision = controller.projection().form_revision;
        send(
            &controller,
            TranslationServicesRequest::SaveCustom { revision },
        )
        .await;
        assert_eq!(
            credentials.load_provider_credential(&info.id).unwrap(),
            "saved-key"
        );
        send(
            &controller,
            TranslationServicesRequest::Configure(info.id.clone()),
        )
        .await;
        edit(
            &controller,
            CustomTranslationEdit::PromptStrategy("general".into()),
        )
        .await;
        let revision = controller.projection().form_revision;
        send(
            &controller,
            TranslationServicesRequest::EditPrompt {
                revision,
                draft: TranslationPromptDraft {
                    name: "通用".into(),
                    description: "".into(),
                    system_prompt: "规则一\n规则二 {target_lang}".into(),
                },
            },
        )
        .await;
        send(
            &controller,
            TranslationServicesRequest::SavePrompt { revision },
        )
        .await;
        send(&controller, TranslationServicesRequest::Cancel).await;
        send(
            &controller,
            TranslationServicesRequest::Configure("deeplx".into()),
        )
        .await;
        let revision = controller.projection().form_revision;
        send(
            &controller,
            TranslationServicesRequest::SaveCredentials {
                revision,
                credentials: HashMap::from([
                    ("mode".into(), "deepl".into()),
                    ("api_key".into(), "deepl-key".into()),
                    ("endpoint".into(), String::new()),
                ]),
            },
        )
        .await;
        assert_eq!(
            store
                .load_translation_prompt_strategies()
                .unwrap()
                .strategies[0]
                .system_prompt,
            "规则一\n规则二 {target_lang}"
        );
        drop(controller);
        drop(store);
        drop(credentials);
        let database = Arc::new(Database::open(&path).unwrap());
        let store = Arc::new(SqliteConfigStore::new(database.clone()));
        let credentials = Arc::new(SqliteCredentialStore::new(database));
        let reopened = administration(store, credentials.clone());
        let providers = reopened.list_translation_providers();
        let saved = providers
            .iter()
            .find(|provider| provider.id == info.id)
            .unwrap();
        assert_eq!(saved.model.as_deref(), Some("updated-model"));
        assert!(saved.is_active);
        assert!(
            providers
                .iter()
                .find(|provider| provider.id == "deeplx")
                .unwrap()
                .is_configured
        );
        assert_eq!(
            credentials.load_provider_credential(&info.id).unwrap(),
            "saved-key"
        );
    }
}
