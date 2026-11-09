use std::future::Future;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::Mutex;

use crate::Result;

#[async_trait]
pub trait FavoriteCapacityRepository: Send + Sync {
    async fn current_count(&self) -> Result<usize>;
}

pub trait FavoriteCapacityPolicyProvider: Send + Sync {
    fn maximum_favorites(&self) -> Result<u32>;
}

pub struct FavoriteCapacity {
    repository: Option<Arc<dyn FavoriteCapacityRepository>>,
    policy_provider: Option<Arc<dyn FavoriteCapacityPolicyProvider>>,
    gate: Mutex<()>,
}

impl FavoriteCapacity {
    pub fn new(
        repository: Arc<dyn FavoriteCapacityRepository>,
        policy_provider: Arc<dyn FavoriteCapacityPolicyProvider>,
    ) -> Self {
        Self {
            repository: Some(repository),
            policy_provider: Some(policy_provider),
            gate: Mutex::new(()),
        }
    }

    #[cfg(test)]
    pub fn unlimited() -> Self {
        Self {
            repository: None,
            policy_provider: None,
            gate: Mutex::new(()),
        }
    }

    pub async fn add<T, Add, AddFuture>(&self, insert: Add) -> Result<T>
    where
        T: Send,
        Add: FnOnce() -> AddFuture + Send,
        AddFuture: Future<Output = Result<T>> + Send,
    {
        let _guard = self.gate.lock().await;
        self.ensure_available().await?;
        insert().await
    }

    pub async fn add_idempotent<T, Find, FindFuture, Add, AddFuture>(
        &self,
        find_existing: Find,
        insert: Add,
    ) -> Result<T>
    where
        T: Send,
        Find: FnOnce() -> FindFuture + Send,
        FindFuture: Future<Output = Result<Option<T>>> + Send,
        Add: FnOnce() -> AddFuture + Send,
        AddFuture: Future<Output = Result<T>> + Send,
    {
        let _guard = self.gate.lock().await;
        if let Some(existing) = find_existing().await? {
            return Ok(existing);
        }
        self.ensure_available().await?;
        insert().await
    }

    async fn ensure_available(&self) -> Result<()> {
        let (Some(repository), Some(provider)) = (&self.repository, &self.policy_provider) else {
            return Ok(());
        };
        let maximum = provider.maximum_favorites()? as usize;
        if maximum > 0 && repository.current_count().await? >= maximum {
            return Err(
                format!("收藏夹容量已满（最多 {maximum} 项），请先删除不需要的收藏").into(),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    use async_trait::async_trait;

    use super::{FavoriteCapacity, FavoriteCapacityPolicyProvider, FavoriteCapacityRepository};
    use crate::Result;

    #[derive(Default)]
    struct FakeRepository(AtomicUsize);

    #[async_trait]
    impl FavoriteCapacityRepository for FakeRepository {
        async fn current_count(&self) -> Result<usize> {
            Ok(self.0.load(Ordering::SeqCst))
        }
    }

    struct FakePolicy(u32);

    impl FavoriteCapacityPolicyProvider for FakePolicy {
        fn maximum_favorites(&self) -> Result<u32> {
            Ok(self.0)
        }
    }

    #[tokio::test]
    async fn concurrent_adds_share_one_atomic_capacity_gate() {
        let repository = Arc::new(FakeRepository::default());
        let capacity = Arc::new(FavoriteCapacity::new(
            repository.clone(),
            Arc::new(FakePolicy(1)),
        ));

        let tasks = (0..2).map(|_| {
            let capacity = capacity.clone();
            let repository = repository.clone();
            tokio::spawn(async move {
                capacity
                    .add(|| async move {
                        tokio::task::yield_now().await;
                        repository.0.fetch_add(1, Ordering::SeqCst);
                        Ok(())
                    })
                    .await
            })
        });
        let results = futures_in_order(tasks).await;

        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(repository.0.load(Ordering::SeqCst), 1);
        assert!(results
            .iter()
            .filter_map(|result| result.as_ref().err())
            .any(|error| error.to_string().contains("收藏夹容量已满")));
    }

    #[tokio::test]
    async fn an_existing_favorite_remains_idempotent_at_capacity() {
        let repository = Arc::new(FakeRepository(AtomicUsize::new(1)));
        let capacity = FavoriteCapacity::new(repository, Arc::new(FakePolicy(1)));

        let value = capacity
            .add_idempotent(|| async { Ok(Some(42)) }, || async { Ok(99) })
            .await
            .unwrap();

        assert_eq!(value, 42);
    }

    async fn futures_in_order<I>(tasks: I) -> Vec<Result<()>>
    where
        I: IntoIterator<Item = tokio::task::JoinHandle<Result<()>>>,
    {
        let mut results = Vec::new();
        for task in tasks {
            results.push(task.await.unwrap());
        }
        results
    }
}
