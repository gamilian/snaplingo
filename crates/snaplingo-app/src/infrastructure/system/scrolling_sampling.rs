//! Sampling restores overlays on failure and when the caller is cancelled.
use std::future::Future;
struct Restore(Option<Box<dyn FnOnce() + Send>>);
impl Drop for Restore {
    fn drop(&mut self) {
        if let Some(restore) = self.0.take() {
            restore();
        }
    }
}
pub(super) async fn sample<T>(
    hide: impl Future<Output = crate::Result<bool>>,
    capture: impl Future<Output = crate::Result<T>>,
    restore: impl FnOnce() + Send + 'static,
) -> crate::Result<T> {
    let _restore = Restore(Some(Box::new(restore)));
    if hide.await? {
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    capture.await
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    #[tokio::test]
    async fn failed_sampling_restores_the_overlay_and_keeps_the_source_error() {
        let visible = Arc::new(AtomicBool::new(true));
        let hidden = visible.clone();
        let restored = visible.clone();
        let result = sample(
            async move {
                hidden.store(false, Ordering::SeqCst);
                Ok(true)
            },
            async { Err::<(), _>("sample unavailable".into()) },
            move || restored.store(true, Ordering::SeqCst),
        )
        .await;
        assert_eq!(result.unwrap_err().to_string(), "sample unavailable");
        assert!(visible.load(Ordering::SeqCst));
    }
    #[tokio::test]
    async fn cancellation_after_hiding_also_restores_the_overlay() {
        let visible = Arc::new(AtomicBool::new(true));
        let hidden = visible.clone();
        let restored = visible.clone();
        let (sender, receiver) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(sample(
            async move {
                hidden.store(false, Ordering::SeqCst);
                sender.send(()).unwrap();
                Ok(true)
            },
            std::future::pending::<crate::Result<()>>(),
            move || restored.store(true, Ordering::SeqCst),
        ));
        receiver.await.unwrap();
        assert!(!visible.load(Ordering::SeqCst));
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(visible.load(Ordering::SeqCst));
    }
}
