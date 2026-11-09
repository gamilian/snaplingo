use super::*;
use crate::AppError;
use serde::{Deserialize, Serialize};
use snaplingo_core::application::providers::common::CredentialField;
use snaplingo_core::application::providers::configuration::*;
use snaplingo_core::application::providers::translation::{
    LLMTranslationProvider, TranslationCoordinator,
};
use std::collections::HashMap;
use std::sync::Arc;
mod cases {
    use snaplingo_core::application::providers::{
        LLMProtocol, LlmIntrospection, ProviderConfigStore,
    };

    use super::*;
    use crate::infrastructure::storage::{Database, SqliteConfigStore, SqliteCredentialStore};
    use anyhow::Result;
    use async_trait::async_trait;
    use snaplingo_core::application::providers::translation::TranslationCoordinator;
    use snaplingo_core::application::providers::{
        CredentialSnapshot, HttpClient, HttpResponse, LlmRuntime, ProviderCredentialStore,
    };
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    // The integration fixture retains the dependencies it injects; assertions
    // read them through their own interfaces rather than Core private fields.
    struct ProviderConfiguration {
        inner: snaplingo_core::application::providers::ProviderConfiguration,
        config_store: Arc<dyn ProviderConfigStore>,
        credential_store: Arc<dyn ProviderCredentialStore>,
        translation_coordinator: Arc<TranslationCoordinator>,
    }
    impl std::ops::Deref for ProviderConfiguration {
        type Target = snaplingo_core::application::providers::ProviderConfiguration;
        fn deref(&self) -> &Self::Target {
            &self.inner
        }
    }
    impl ProviderConfiguration {
        fn new(
            config_store: Arc<dyn ProviderConfigStore>,
            credential_store: Arc<dyn ProviderCredentialStore>,
            runtime: Arc<dyn LlmRuntime>,
            translation_coordinator: Arc<TranslationCoordinator>,
            introspection: Arc<LlmIntrospection>,
        ) -> Self {
            Self {
                inner: snaplingo_core::application::providers::ProviderConfiguration::new(
                    config_store.clone(),
                    credential_store.clone(),
                    runtime,
                    translation_coordinator.clone(),
                    introspection,
                ),
                config_store,
                credential_store,
                translation_coordinator,
            }
        }
    }
    struct MockHttpClient;

    #[async_trait]
    impl HttpClient for MockHttpClient {
        async fn get(&self, _url: &str, _headers: HashMap<String, String>) -> Result<HttpResponse> {
            Ok(HttpResponse {
                status: 200,
                body: r#"{"data":[]}"#.to_string(),
                headers: HashMap::new(),
            })
        }

        async fn post(
            &self,
            _url: &str,
            _headers: HashMap<String, String>,
            _body: String,
        ) -> Result<HttpResponse> {
            Ok(HttpResponse {
                status: 200,
                body: r#"{"choices":[{"message":{"content":"OK"}}]}"#.to_string(),
                headers: HashMap::new(),
            })
        }
    }

    fn llm_runtime_from_http(http_client: Arc<dyn HttpClient>) -> Arc<dyn LlmRuntime> {
        Arc::new(snaplingo_lib::llm::InfrastructureLlmRuntime::new(
            http_client,
        ))
    }

    fn test_provider_configuration() -> ProviderConfiguration {
        test_provider_configuration_with_database().0
    }

    fn test_provider_configuration_with_database() -> (ProviderConfiguration, Arc<Database>) {
        let database = Arc::new(Database::in_memory().unwrap());
        let credential_store = Arc::new(SqliteCredentialStore::new(database.clone()));
        let config_store = Arc::new(SqliteConfigStore::new(database.clone()));
        let http_client: Arc<dyn HttpClient> = Arc::new(MockHttpClient);
        let llm_runtime = llm_runtime_from_http(http_client.clone());
        let coordinator = Arc::new(TranslationCoordinator::new(config_store.clone()));
        let llm_introspection = Arc::new(
            snaplingo_core::application::providers::LlmIntrospection::new(llm_runtime.clone()),
        );

        // Register builtin DeepL provider for testing
        use snaplingo_core::application::providers::translation::DeepLProvider;
        let deeplx_provider = DeepLProvider::new(http_client.clone());
        let _ = coordinator.register(deeplx_provider);

        (
            ProviderConfiguration::new(
                config_store,
                credential_store,
                llm_runtime,
                coordinator,
                llm_introspection,
            ),
            database,
        )
    }

    #[test]
    fn custom_provider_credentials_are_not_written_to_settings_payload() {
        let (config, database) = test_provider_configuration_with_database();
        let secret = "sqlite-secret-sentinel";

        let view = config
            .add(AddCustomTranslationProviderInput {
                name: "Credential Boundary Test".to_string(),
                protocol: "openai".to_string(),
                endpoint: "https://api.example.com/v1".to_string(),
                model: "test-model".to_string(),
                api_key: secret.to_string(),
                reasoning_level: None,
                prompt_strategy_id: None,
                prompt_fallback_strategy_id: None,
            })
            .unwrap();

        let sqlite_payloads = database
            .with_connection(|connection| {
                Ok(connection
                    .query_row(
                        "SELECT COALESCE(group_concat(payload_json, '\n'), '') FROM settings",
                        [],
                        |row| row.get::<_, String>(0),
                    )
                    .map_err(snaplingo_core::AppError::database)?)
            })
            .unwrap();

        assert!(sqlite_payloads.contains("Credential Boundary Test"));
        assert!(!sqlite_payloads.contains(secret));
        assert_eq!(
            config
                .credential_store
                .load_provider_credential(&view.id)
                .unwrap(),
            secret
        );
    }

    #[test]
    fn remove_rejects_builtin_providers() {
        let config = test_provider_configuration();

        let result = config.remove("google-translate".to_string());

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Cannot remove builtin provider"
        );
    }

    #[test]
    fn remove_returns_error_for_nonexistent_provider() {
        let config = test_provider_configuration();

        let result = config.remove("nonexistent-id".to_string());

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Provider not found"));
    }

    #[tokio::test]
    async fn test_custom_provider_returns_error_for_nonexistent() {
        let config = test_provider_configuration();

        let result = config
            .test_custom_provider("nonexistent-id".to_string())
            .await;

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Provider not found"));
    }

    #[test]
    fn save_credentials_validates_deeplx_mode_deepl_requires_api_key() {
        let config = test_provider_configuration();

        let credentials = vec![
            CredentialValue {
                key: "mode".to_string(),
                value: "deepl".to_string(),
            },
            CredentialValue {
                key: "endpoint".to_string(),
                value: "http://example.com".to_string(),
            },
        ];

        let result = config.save_credentials("deeplx".to_string(), credentials);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("DeepL mode requires api_key"));
    }

    #[test]
    fn save_credentials_validates_deeplx_mode_deeplx_requires_endpoint() {
        let config = test_provider_configuration();

        let credentials = vec![
            CredentialValue {
                key: "mode".to_string(),
                value: "deeplx".to_string(),
            },
            CredentialValue {
                key: "api_key".to_string(),
                value: "some-key".to_string(),
            },
        ];

        let result = config.save_credentials("deeplx".to_string(), credentials);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("DeepLX mode requires endpoint"));
    }

    #[test]
    fn save_credentials_rejects_blank_values() {
        let config = test_provider_configuration();

        let credentials = vec![
            CredentialValue {
                key: "mode".to_string(),
                value: "deeplx".to_string(),
            },
            CredentialValue {
                key: "endpoint".to_string(),
                value: "   ".to_string(), // blank
            },
        ];

        let result = config.save_credentials("deeplx".to_string(), credentials);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cannot be blank"));
    }

    #[test]
    fn save_credentials_rejects_nonexistent_provider() {
        let config = test_provider_configuration();

        let credentials = vec![CredentialValue {
            key: "api_key".to_string(),
            value: "test-key".to_string(),
        }];

        let result = config.save_credentials("nonexistent-id".to_string(), credentials);

        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Provider not found"));
    }

    #[test]
    fn remove_succeeds_for_unregistered_provider() {
        let config = test_provider_configuration();

        // Add a custom provider to config but don't register it
        let def = CustomTranslationProviderDef {
            id: "test-custom-1".to_string(),
            name: "Test Custom".to_string(),
            protocol: LLMProtocol::OpenAI,
            endpoint: "https://api.example.com/v1".to_string(),
            model: "test-model".to_string(),
            reasoning_level: None,
            prompt_strategy_id: "default".to_string(),
            prompt_fallback_strategy_id: "default".to_string(),
        };

        let _ = config
            .config_store
            .save_custom_translation_providers(&vec![def]);

        // Save a credential
        let _ = config
            .credential_store
            .save_provider_credential("test-custom-1", "test-key");

        // Remove should succeed even though provider is not registered
        let result = config.remove("test-custom-1".to_string());

        assert!(result.is_ok());

        // Verify config and credentials are cleaned up.
        let remaining_defs = config
            .config_store
            .load_custom_translation_providers()
            .unwrap_or_default();
        assert!(remaining_defs.is_empty());

        let key_result = config
            .credential_store
            .load_provider_credential("test-custom-1");
        assert!(key_result.is_err());
    }

    #[test]
    fn remove_succeeds_when_credentials_are_missing() {
        let config = test_provider_configuration();

        // Add a custom provider to config but don't save any credentials
        let def = CustomTranslationProviderDef {
            id: "test-custom-2".to_string(),
            name: "Test Custom 2".to_string(),
            protocol: LLMProtocol::OpenAI,
            endpoint: "https://api.example.com/v1".to_string(),
            model: "test-model".to_string(),
            reasoning_level: None,
            prompt_strategy_id: "default".to_string(),
            prompt_fallback_strategy_id: "default".to_string(),
        };

        let _ = config
            .config_store
            .save_custom_translation_providers(&vec![def]);

        // Removal is idempotent when the credential entry does not exist.
        let result = config.remove("test-custom-2".to_string());

        assert!(result.is_ok());

        // Verify config is cleaned up
        let remaining_defs = config
            .config_store
            .load_custom_translation_providers()
            .unwrap_or_default();
        assert!(remaining_defs.is_empty());
    }

    #[derive(Clone)]
    struct FailingCredentialStore {
        inner: Arc<SqliteCredentialStore>,
        fail_delete: Arc<Mutex<bool>>,
    }

    impl FailingCredentialStore {
        fn new(database: Arc<Database>) -> Self {
            Self {
                inner: Arc::new(SqliteCredentialStore::new(database)),
                fail_delete: Arc::new(Mutex::new(false)),
            }
        }

        fn fail_on_delete(&self) {
            *self.fail_delete.lock().unwrap() = true;
        }
    }

    impl ProviderCredentialStore for FailingCredentialStore {
        fn save_provider_credential(&self, provider_id: &str, api_key: &str) -> crate::Result<()> {
            self.inner.save_provider_credential(provider_id, api_key)
        }

        fn load_provider_credential(&self, provider_id: &str) -> crate::Result<String> {
            self.inner.load_provider_credential(provider_id)
        }

        fn delete_provider_credential(&self, provider_id: &str) -> crate::Result<()> {
            self.inner.delete_provider_credential(provider_id)
        }

        fn save_provider_credentials(
            &self,
            provider_id: &str,
            credentials: &HashMap<String, String>,
        ) -> crate::Result<()> {
            self.inner
                .save_provider_credentials(provider_id, credentials)
        }

        fn snapshot_provider_credentials(
            &self,
            provider_id: &str,
            field_names: &[String],
        ) -> crate::Result<CredentialSnapshot> {
            self.inner
                .snapshot_provider_credentials(provider_id, field_names)
        }

        fn restore_provider_credentials(
            &self,
            provider_id: &str,
            snapshot: &CredentialSnapshot,
        ) -> crate::Result<()> {
            self.inner
                .restore_provider_credentials(provider_id, snapshot)
        }

        fn load_provider_credentials(
            &self,
            provider_id: &str,
            field_names: &[String],
        ) -> crate::Result<HashMap<String, String>> {
            self.inner
                .load_provider_credentials(provider_id, field_names)
        }

        fn delete_provider_credentials(
            &self,
            provider_id: &str,
            field_names: &[String],
        ) -> crate::Result<()> {
            if *self.fail_delete.lock().unwrap() {
                return Err(crate::AppError::Other(
                    "Simulated credential delete failure".to_string(),
                ));
            }
            self.inner
                .delete_provider_credentials(provider_id, field_names)
        }
    }

    #[test]
    fn remove_rolls_back_all_state_on_structured_credential_delete_failure() {
        let database = Arc::new(Database::in_memory().unwrap());
        let credential_store = Arc::new(FailingCredentialStore::new(database.clone()));
        let config_file = Arc::new(SqliteConfigStore::new(database));
        let http_client: Arc<dyn HttpClient> = Arc::new(MockHttpClient);
        let llm_runtime = llm_runtime_from_http(http_client.clone());
        let coordinator = Arc::new(TranslationCoordinator::new(config_file.clone()));
        let llm_introspection = Arc::new(
            snaplingo_core::application::providers::LlmIntrospection::new(llm_runtime.clone()),
        );

        let config = ProviderConfiguration::new(
            config_file.clone(),
            credential_store.clone(),
            llm_runtime.clone(),
            coordinator.clone(),
            llm_introspection,
        );

        // Add a custom provider with structured credentials
        let def = CustomTranslationProviderDef {
            id: "test-custom-3".to_string(),
            name: "Test Custom 3".to_string(),
            protocol: LLMProtocol::OpenAI,
            endpoint: "https://api.example.com/v1".to_string(),
            model: "test-model".to_string(),
            reasoning_level: None,
            prompt_strategy_id: "default".to_string(),
            prompt_fallback_strategy_id: "default".to_string(),
        };

        config_file
            .save("custom_translation_providers", &vec![def.clone()])
            .unwrap();

        // Register the provider
        let provider = create_llm_translation_provider(
            &def,
            llm_runtime,
            "test-key".to_string(),
            config_file.clone(),
        );
        coordinator.register(provider).unwrap();
        coordinator.activate("test-custom-3").unwrap();

        credential_store
            .save_provider_credentials(
                "test-custom-3",
                &HashMap::from([("api_key".to_string(), "test-key".to_string())]),
            )
            .unwrap();

        credential_store.fail_on_delete();

        // Attempt to remove - should fail on structured credential deletion
        let result = config.remove("test-custom-3".to_string());

        // Should fail
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Failed to delete structured credentials"));

        // Verify complete rollback:
        // 1. Config should still have the provider
        let remaining_defs = config_file
            .load::<Vec<CustomTranslationProviderDef>>("custom_translation_providers")
            .unwrap();
        assert_eq!(remaining_defs.len(), 1);
        assert_eq!(remaining_defs[0].id, "test-custom-3");

        // 2. Simple credential should still exist
        let simple_key = credential_store.load_provider_credential("test-custom-3");
        assert!(simple_key.is_ok());
        assert_eq!(simple_key.unwrap(), "test-key");

        // 3. Structured credential should still exist
        let struct_creds = credential_store
            .load_provider_credentials("test-custom-3", &vec!["api_key".to_string()])
            .unwrap();
        assert_eq!(struct_creds.get("api_key").unwrap(), "test-key");

        // 4. Provider should still be registered
        assert!(coordinator.get("test-custom-3").is_some());

        // 5. Provider should still be active
        let active = coordinator.get_active();
        let active_ids: Vec<String> = active.iter().map(|p| p.read().id().to_string()).collect();
        assert!(active_ids.contains(&"test-custom-3".to_string()));
    }

    #[test]
    fn concurrent_custom_adds_are_serialized() {
        use std::sync::{Arc, Barrier};
        use std::thread;

        let config = Arc::new(test_provider_configuration());

        // Barrier ensures all threads start simultaneously
        let barrier = Arc::new(Barrier::new(3));
        let mut handles = vec![];

        for i in 0..3 {
            let config_clone = config.clone();
            let barrier_clone = barrier.clone();

            handles.push(thread::spawn(move || {
                barrier_clone.wait(); // Sync start
                let input = AddCustomTranslationProviderInput {
                    name: format!("Concurrent Test {}", i),
                    protocol: "openai".to_string(),
                    endpoint: format!("https://api{}.example.com/v1", i),
                    model: "test-model".to_string(),
                    api_key: format!("key-{}", i),
                    reasoning_level: None,
                    prompt_strategy_id: None,
                    prompt_fallback_strategy_id: None,
                };
                config_clone.add(input)
            }));
        }

        // Collect results
        let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

        // All adds should succeed (serialized by lock)
        assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 3);

        // Verify the config, credential, and coordinator invariant.
        let custom_defs = config
            .config_store
            .load_custom_translation_providers()
            .unwrap();
        let config_ids: std::collections::HashSet<_> =
            custom_defs.iter().map(|d| d.id.clone()).collect();

        // Check all added ids are in config
        for result in &results {
            if let Ok(view) = result {
                assert!(config_ids.contains(&view.id));
                // The credential store should have the credential.
                assert!(config
                    .credential_store
                    .load_provider_credential(&view.id)
                    .is_ok());
                // Coordinator should have the provider
                assert!(config.translation_coordinator.get(&view.id).is_some());
            }
        }

        // No divergence: all 3 custom providers present everywhere
        assert_eq!(config_ids.len(), 3);
    }

    #[test]
    fn concurrent_remove_and_save_credentials_no_orphan() {
        use std::sync::{Arc, Barrier};
        use std::thread;

        let config = Arc::new(test_provider_configuration());

        // Pre-create a provider
        let add_input = AddCustomTranslationProviderInput {
            name: "Test Remove".to_string(),
            protocol: "openai".to_string(),
            endpoint: "https://api.example.com/v1".to_string(),
            model: "test-model".to_string(),
            api_key: "old-key".to_string(),
            reasoning_level: None,
            prompt_strategy_id: None,
            prompt_fallback_strategy_id: None,
        };
        let view = config.add(add_input).unwrap();
        let provider_id = view.id.clone();

        let barrier = Arc::new(Barrier::new(2));
        let config_clone1 = config.clone();
        let config_clone2 = config.clone();
        let barrier_clone1 = barrier.clone();
        let barrier_clone2 = barrier.clone();
        let provider_id_for_remove = provider_id.clone();
        let provider_id_for_save = provider_id.clone();

        // Thread 1: remove
        let h1 = thread::spawn(move || {
            barrier_clone1.wait();
            config_clone1.remove(provider_id_for_remove)
        });

        // Thread 2: save_credentials
        let h2 = thread::spawn(move || {
            barrier_clone2.wait();
            config_clone2.save_credentials(
                provider_id_for_save,
                vec![CredentialValue {
                    key: "api_key".to_string(),
                    value: "new-key".to_string(),
                }],
            )
        });

        let result1 = h1.join().unwrap();
        let result2 = h2.join().unwrap();

        // Exactly one succeeds (serialized by provider_state_lock).
        // The lock ensures mutual exclusion: remove and save_credentials cannot
        // interleave, so the loser sees "provider not found" or similar.
        assert!(result1.is_ok() ^ result2.is_ok());

        // Invariant: if remove succeeded, provider is gone from all stores
        if result1.is_ok() {
            let custom_defs = config
                .config_store
                .load_custom_translation_providers()
                .unwrap_or_default();
            assert!(!custom_defs.iter().any(|d| d.id == provider_id));
            assert!(config
                .credential_store
                .load_provider_credential(&provider_id)
                .is_err());
            assert!(config.translation_coordinator.get(&provider_id).is_none());
        }

        // If save_credentials succeeded, provider is still present with new credential
        if result2.is_ok() {
            let custom_defs = config
                .config_store
                .load_custom_translation_providers()
                .unwrap();
            assert!(custom_defs.iter().any(|d| d.id == provider_id));
            assert_eq!(
                config
                    .credential_store
                    .load_provider_credential(&provider_id)
                    .unwrap(),
                "new-key"
            );
            assert!(config.translation_coordinator.get(&provider_id).is_some());
        }
    }
}
