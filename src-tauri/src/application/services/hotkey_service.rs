use crate::error::Result;
use crate::infrastructure::system::hotkey::{HotkeyBackend, HotkeyId};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// HotkeyService coordinates global hotkey registration.
/// It wraps the platform-specific hotkey backend and manages
/// callback mappings for registered hotkeys.
pub struct HotkeyService {
    hotkey_backend: Arc<dyn HotkeyBackend>,
    callbacks: Arc<Mutex<HashMap<HotkeyId, String>>>,
}

impl HotkeyService {
    /// Create a new HotkeyService with the given hotkey backend
    pub fn new(hotkey_backend: Arc<dyn HotkeyBackend>) -> Self {
        Self {
            hotkey_backend,
            callbacks: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Register a global hotkey with a callback identifier
    /// Returns the HotkeyId that can be used to unregister
    pub async fn register_hotkey(
        &self,
        accelerator: &str,
        callback_id: String,
    ) -> Result<HotkeyId> {
        let id = self.hotkey_backend.register(accelerator).await?;
        self.callbacks.lock().await.insert(id, callback_id);
        Ok(id)
    }

    /// Unregister a previously registered hotkey
    pub async fn unregister_hotkey(&self, id: HotkeyId) -> Result<()> {
        self.hotkey_backend.unregister(id).await?;
        self.callbacks.lock().await.remove(&id);
        Ok(())
    }

    /// Get the callback identifier for a registered hotkey
    pub async fn get_callback(&self, id: HotkeyId) -> Option<String> {
        self.callbacks.lock().await.get(&id).cloned()
    }

    /// Check if a hotkey is registered
    pub async fn is_registered(&self, id: HotkeyId) -> bool {
        self.hotkey_backend.is_registered(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::HotkeyService;
    use crate::error::AppError;
    use crate::infrastructure::system::hotkey::{HotkeyBackend, HotkeyId};
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // Mock HotkeyBackend for testing
    struct MockHotkeyBackend {
        next_id: Mutex<u32>,
        registered: Mutex<HashMap<HotkeyId, String>>,
    }

    impl MockHotkeyBackend {
        fn new() -> Self {
            Self {
                next_id: Mutex::new(1),
                registered: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait::async_trait]
    impl HotkeyBackend for MockHotkeyBackend {
        async fn register(&self, accelerator: &str) -> Result<HotkeyId, AppError> {
            let mut next_id = self.next_id.lock().await;
            // Create HotkeyId by calling a real backend method that returns one
            // For testing, we'll use a workaround: store by string and use the backend's
            // register method to get a proper HotkeyId
            let id_value = *next_id;
            *next_id += 1;

            // Use unsafe to construct HotkeyId for testing purposes
            // This is acceptable in test code since we're mocking the backend
            let id: HotkeyId = unsafe { std::mem::transmute(id_value) };

            self.registered
                .lock()
                .await
                .insert(id, accelerator.to_string());
            Ok(id)
        }

        async fn unregister(&self, id: HotkeyId) -> Result<(), AppError> {
            self.registered.lock().await.remove(&id);
            Ok(())
        }

        async fn is_registered(&self, id: HotkeyId) -> bool {
            self.registered.lock().await.contains_key(&id)
        }
    }

    #[tokio::test]
    async fn test_register_hotkey() {
        // Arrange
        let mock_backend = Arc::new(MockHotkeyBackend::new());
        let service = HotkeyService::new(mock_backend.clone());

        // Act
        let result = service
            .register_hotkey("Cmd+Shift+C", "capture_callback".to_string())
            .await;

        // Assert
        assert!(result.is_ok());
        let hotkey_id = result.unwrap();

        // Verify callback was stored
        let callback = service.get_callback(hotkey_id).await;
        assert_eq!(callback, Some("capture_callback".to_string()));
    }

    #[tokio::test]
    async fn test_unregister_hotkey() {
        // Arrange
        let mock_backend = Arc::new(MockHotkeyBackend::new());
        let service = HotkeyService::new(mock_backend.clone());

        let hotkey_id = service
            .register_hotkey("Cmd+Shift+C", "capture_callback".to_string())
            .await
            .unwrap();

        // Act
        let result = service.unregister_hotkey(hotkey_id).await;

        // Assert
        assert!(result.is_ok());

        // Verify callback was removed
        let callback = service.get_callback(hotkey_id).await;
        assert_eq!(callback, None);

        // Verify backend was called
        assert!(!service.is_registered(hotkey_id).await);
    }

    #[tokio::test]
    async fn test_multiple_hotkeys() {
        // Arrange
        let mock_backend = Arc::new(MockHotkeyBackend::new());
        let service = HotkeyService::new(mock_backend.clone());

        // Act - register multiple hotkeys
        let id1 = service
            .register_hotkey("Cmd+Shift+C", "capture".to_string())
            .await
            .unwrap();
        let id2 = service
            .register_hotkey("Cmd+Shift+T", "translate".to_string())
            .await
            .unwrap();

        // Assert
        assert_ne!(id1, id2);
        assert_eq!(
            service.get_callback(id1).await,
            Some("capture".to_string())
        );
        assert_eq!(
            service.get_callback(id2).await,
            Some("translate".to_string())
        );
    }
}
