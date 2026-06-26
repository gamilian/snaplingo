use super::backend::KeychainBackend;
use crate::error::{AppError, Result};

const SERVICE_NAME: &str = "com.snaplingo.app";

pub struct MacOSKeychain;

impl MacOSKeychain {
    pub fn new() -> Self {
        Self
    }
}

impl KeychainBackend for MacOSKeychain {
    fn save(&self, key: &str, value: &str) -> Result<()> {
        let entry = keyring::Entry::new(SERVICE_NAME, key).map_err(|e| AppError::Keychain(e))?;

        entry
            .set_password(value)
            .map_err(|e| AppError::Keychain(e))?;

        Ok(())
    }

    fn load(&self, key: &str) -> Result<String> {
        let entry = keyring::Entry::new(SERVICE_NAME, key).map_err(|e| AppError::Keychain(e))?;

        entry.get_password().map_err(|e| AppError::Keychain(e))
    }

    fn delete(&self, key: &str) -> Result<()> {
        let entry = keyring::Entry::new(SERVICE_NAME, key).map_err(|e| AppError::Keychain(e))?;

        entry.delete_password().map_err(|e| AppError::Keychain(e))?;

        Ok(())
    }
}
