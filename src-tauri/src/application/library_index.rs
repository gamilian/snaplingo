use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::Result;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LibraryIndexKind {
    Translation,
    Ocr,
    Screenshot,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryIndexItem {
    pub id: i64,
    pub kind: LibraryIndexKind,
    pub source_offset: usize,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LibraryIndexQuery {
    pub search: String,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LibraryIndexPage {
    pub items: Vec<LibraryIndexItem>,
    pub total: usize,
}

#[async_trait]
pub trait LibraryIndexRepository: Send + Sync {
    async fn query_history(&self, query: &LibraryIndexQuery) -> Result<LibraryIndexPage>;
    async fn query_favorites(&self, query: &LibraryIndexQuery) -> Result<LibraryIndexPage>;
}

pub struct LibraryIndex {
    repository: Arc<dyn LibraryIndexRepository>,
}

impl LibraryIndex {
    pub fn new(repository: Arc<dyn LibraryIndexRepository>) -> Self {
        Self { repository }
    }

    pub async fn query_history(&self, mut query: LibraryIndexQuery) -> Result<LibraryIndexPage> {
        query.limit = query.limit.clamp(1, 100);
        self.repository.query_history(&query).await
    }

    pub async fn query_favorites(&self, mut query: LibraryIndexQuery) -> Result<LibraryIndexPage> {
        query.limit = query.limit.clamp(1, 100);
        self.repository.query_favorites(&query).await
    }
}
