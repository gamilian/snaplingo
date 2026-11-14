use std::future::Future;
use std::sync::{Arc, Mutex};

use tokio::sync::{watch, Notify};

use crate::{AppError, Result};

#[derive(Default)]
struct State {
    stopping: bool,
    active: usize,
}

/// Owns the lifetime of provider requests, including cancellation during shutdown.
pub(crate) struct RequestLifecycle {
    state: Mutex<State>,
    stopping: watch::Sender<bool>,
    idle: Notify,
}

impl Default for RequestLifecycle {
    fn default() -> Self {
        Self {
            state: Mutex::new(State::default()),
            stopping: watch::channel(false).0,
            idle: Notify::new(),
        }
    }
}

impl RequestLifecycle {
    pub(crate) fn begin_shutdown(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        if state.stopping {
            return false;
        }
        state.stopping = true;
        self.stopping.send_replace(true);
        true
    }

    pub(crate) fn is_stopping(&self) -> bool {
        self.state.lock().unwrap().stopping
    }

    pub(crate) async fn wait_for_idle(&self) {
        loop {
            let idle = self.idle.notified();
            tokio::pin!(idle);
            idle.as_mut().enable();
            if self.state.lock().unwrap().active == 0 {
                return;
            }
            idle.await;
        }
    }

    pub(crate) async fn run<T>(
        self: &Arc<Self>,
        operation: impl Future<Output = Result<T>>,
    ) -> Result<T> {
        let mut stopping = self.stopping.subscribe();
        let _request = self.enter()?;
        tokio::select! {
            biased;
            _ = stopping.changed() => Err(cancelled()),
            result = operation => result,
        }
    }

    /// Let an admitted durable write finish before shutdown drains its events.
    pub(crate) async fn finish<T>(
        self: &Arc<Self>,
        operation: impl Future<Output = Result<T>>,
    ) -> Result<T> {
        let _request = self.enter()?;
        operation.await
    }

    fn enter(self: &Arc<Self>) -> Result<ActiveRequest> {
        let mut state = self.state.lock().unwrap();
        if state.stopping {
            return Err(cancelled());
        }
        state.active += 1;
        Ok(ActiveRequest(self.clone()))
    }
}

fn cancelled() -> AppError {
    AppError::System("Application is shutting down".into())
}

struct ActiveRequest(Arc<RequestLifecycle>);

impl Drop for ActiveRequest {
    fn drop(&mut self) {
        let mut state = self.0.state.lock().unwrap();
        state.active -= 1;
        if state.active == 0 {
            self.0.idle.notify_waiters();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn shutdown_cancels_in_flight_requests_waits_for_idle_and_rejects_new_work() {
        let lifecycle = Arc::new(RequestLifecycle::default());
        let started = Arc::new(Notify::new());
        let request = {
            let lifecycle = lifecycle.clone();
            let started = started.clone();
            tokio::spawn(async move {
                lifecycle
                    .run(async {
                        started.notify_one();
                        std::future::pending::<Result<()>>().await
                    })
                    .await
            })
        };
        started.notified().await;
        assert!(lifecycle.begin_shutdown());
        assert!(!lifecycle.begin_shutdown());
        lifecycle.wait_for_idle().await;
        assert!(request.await.unwrap().is_err());
        assert!(lifecycle.run(async { Ok(()) }).await.is_err());
    }

    #[tokio::test]
    async fn shutdown_waits_for_admitted_durable_writes() {
        let lifecycle = Arc::new(RequestLifecycle::default());
        let started = Arc::new(Notify::new());
        let release = Arc::new(Notify::new());
        let write = {
            let lifecycle = lifecycle.clone();
            let started = started.clone();
            let release = release.clone();
            tokio::spawn(async move {
                lifecycle
                    .finish(async {
                        started.notify_one();
                        release.notified().await;
                        Ok(())
                    })
                    .await
            })
        };
        started.notified().await;
        lifecycle.begin_shutdown();
        assert!(lifecycle.finish(async { Ok(()) }).await.is_err());
        assert!(tokio::time::timeout(
            std::time::Duration::from_millis(20),
            lifecycle.wait_for_idle()
        )
        .await
        .is_err());
        release.notify_one();
        write.await.unwrap().unwrap();
        lifecycle.wait_for_idle().await;
    }
}
