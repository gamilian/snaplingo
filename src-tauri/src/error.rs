use std::fmt;

#[derive(Debug)]
pub enum AppError {
    Io(std::io::Error),
    Json(serde_json::Error),
    Http(reqwest::Error),
    Keychain(keyring::Error),
    Config(String),
    ProviderNotFound(String),
    ProviderNotConfigured(String),
    NoActiveProvider,
    Other(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Io(e) => write!(f, "IO error: {}", e),
            AppError::Json(e) => write!(f, "JSON error: {}", e),
            AppError::Http(e) => write!(f, "HTTP error: {}", e),
            AppError::Keychain(e) => write!(f, "Keychain error: {}", e),
            AppError::Config(msg) => write!(f, "Configuration error: {}", msg),
            AppError::ProviderNotFound(id) => write!(f, "Provider not found: {}", id),
            AppError::ProviderNotConfigured(id) => write!(f, "Provider not configured: {}", id),
            AppError::NoActiveProvider => write!(f, "No active provider configured"),
            AppError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AppError::Io(e) => Some(e),
            AppError::Json(e) => Some(e),
            AppError::Http(e) => Some(e),
            AppError::Keychain(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError::Io(err)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        AppError::Json(err)
    }
}

impl From<reqwest::Error> for AppError {
    fn from(err: reqwest::Error) -> Self {
        AppError::Http(err)
    }
}

impl From<keyring::Error> for AppError {
    fn from(err: keyring::Error) -> Self {
        AppError::Keychain(err)
    }
}

impl From<String> for AppError {
    fn from(msg: String) -> Self {
        AppError::Other(msg)
    }
}

impl From<&str> for AppError {
    fn from(msg: &str) -> Self {
        AppError::Other(msg.to_string())
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
