//! SQLite repositories and filesystem assets shared by native desktop hosts.
//! Persistence implements portable application ports without a UI framework.
pub use snaplingo_core::{application, domain, error, AppError, Result};

mod database;
mod screenshot_favorite_assets;

pub use database::{
    Database, SqliteAppLogRepository, SqliteConfigStore, SqliteCredentialStore,
    SqliteFavoriteCapacityRepository, SqliteFavoriteRepository, SqliteHistoryRepository,
    SqliteLibraryIndexRepository, SqliteScreenshotFavoriteRepository,
};
pub use screenshot_favorite_assets::{
    FilesystemOcrHistoryAssets, FilesystemScreenshotFavoriteAssets,
};
