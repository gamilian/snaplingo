//! Startup work and graceful shutdown shared by desktop shell implementations.
#[cfg(all(target_os = "macos", feature = "native-app"))]
pub(crate) mod macos;
pub(crate) mod shell;
#[cfg(all(target_os = "windows", feature = "native-app"))]
pub(crate) mod windows;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use chrono::{Days, Local, NaiveDateTime, TimeZone};
use tokio::task::JoinHandle;

use crate::infrastructure::{runtime, storage::SqliteAppLogRepository};
use crate::{AppState, Result};
use snaplingo_core::application::SettingsConfiguration;

pub struct RuntimeLifecycle {
    tasks: Mutex<Vec<JoinHandle<()>>>,
    shutdown: tokio::sync::Mutex<()>,
    shutdown_requested: AtomicBool,
    shutdown_complete: AtomicBool,
}

impl RuntimeLifecycle {
    /// Start once, after the platform's global-shortcut owner is initialized.
    pub fn start(state: &AppState) -> Self {
        if let Err(error) = state.settings.application.synchronize_start_on_boot() {
            log::warn!("Failed to synchronize start on boot: {}", error);
        }
        let hotkeys = state.settings.hotkeys.clone();
        Self::with_tasks(vec![
            crate::composition::subscribe_history(state),
            runtime::spawn(run_daily_log_cleanup(
                state.settings.configuration.clone(),
                state.logs.repository.clone(),
            )),
            runtime::spawn(async move {
                if let Err(error) = hotkeys.register_startup_hotkeys() {
                    log::error!("Failed to register startup hotkeys: {}", error);
                }
            }),
        ])
    }

    fn with_tasks(tasks: Vec<JoinHandle<()>>) -> Self {
        Self {
            tasks: Mutex::new(tasks),
            shutdown: tokio::sync::Mutex::new(()),
            shutdown_requested: AtomicBool::new(false),
            shutdown_complete: AtomicBool::new(false),
        }
    }

    #[cfg(not(target_os = "macos"))]
    pub(crate) fn track(&self, task: JoinHandle<()>) {
        self.tasks.lock().unwrap().push(task);
    }

    /// Claim an explicit exit request without starting duplicate shutdown work.
    pub(crate) fn request_shutdown(&self) -> bool {
        !self.shutdown_requested.swap(true, Ordering::AcqRel)
    }

    pub fn is_shutdown_complete(&self) -> bool {
        self.shutdown_complete.load(Ordering::Acquire)
    }

    pub async fn shutdown(&self, state: &AppState) -> Result<()> {
        let _shutdown = self.shutdown.lock().await;
        if self.is_shutdown_complete() {
            return Ok(());
        }
        self.shutdown_requested.store(true, Ordering::Release);
        let tasks = std::mem::take(&mut *self.tasks.lock().unwrap());
        for task in &tasks {
            task.abort();
        }
        for task in tasks {
            let _ = task.await;
        }
        let result = state.shutdown().await;
        self.shutdown_complete.store(true, Ordering::Release);
        result
    }
}

impl Drop for RuntimeLifecycle {
    fn drop(&mut self) {
        for task in self.tasks.get_mut().unwrap().drain(..) {
            task.abort();
        }
    }
}

pub fn should_show_settings_on_startup(is_first_launch: bool, permissions_granted: bool) -> bool {
    is_first_launch || !permissions_granted
}

fn next_daily_log_cleanup_at(now: NaiveDateTime) -> NaiveDateTime {
    let today_at_eight = now.date().and_hms_opt(8, 0, 0).expect("08:00 is valid");
    if now < today_at_eight {
        today_at_eight
    } else {
        today_at_eight
            .checked_add_days(Days::new(1))
            .expect("next cleanup date is valid")
    }
}

async fn run_daily_log_cleanup(
    settings: Arc<SettingsConfiguration>,
    repository: Arc<SqliteAppLogRepository>,
) {
    loop {
        let now = Local::now();
        let next = Local
            .from_local_datetime(&next_daily_log_cleanup_at(now.naive_local()))
            .earliest()
            .expect("local 08:00 is valid");
        tokio::time::sleep((next - now).to_std().unwrap_or_default()).await;
        let retention_days = settings
            .snapshot()
            .map(|snapshot| snapshot.general.log_retention_days)
            .unwrap_or(7);
        if let Err(error) = repository.delete_expired(retention_days) {
            log::warn!("Failed to delete expired application logs: {}", error);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;
    use std::time::Duration;

    #[test]
    fn schedules_the_same_day_before_eight_and_the_next_day_at_or_after_eight() {
        let date = NaiveDate::from_ymd_opt(2026, 7, 16).unwrap();
        assert_eq!(
            next_daily_log_cleanup_at(date.and_hms_opt(7, 59, 59).unwrap()),
            date.and_hms_opt(8, 0, 0).unwrap()
        );
        assert_eq!(
            next_daily_log_cleanup_at(date.and_hms_opt(8, 0, 0).unwrap()),
            NaiveDate::from_ymd_opt(2026, 7, 17)
                .unwrap()
                .and_hms_opt(8, 0, 0)
                .unwrap()
        );
    }

    #[test]
    fn shows_settings_on_first_launch_or_when_permissions_are_missing() {
        assert!(should_show_settings_on_startup(true, true));
        assert!(should_show_settings_on_startup(false, false));
        assert!(!should_show_settings_on_startup(false, true));
    }

    #[tokio::test]
    async fn dropping_the_owner_cancels_background_work_without_waiting() {
        let owner = Arc::new(());
        let lifetime = Arc::downgrade(&owner);
        let (started, ready) = tokio::sync::oneshot::channel();
        let task = runtime::spawn(async move {
            let _owner = owner;
            let _ = started.send(());
            std::future::pending::<()>().await;
        });
        ready.await.unwrap();
        drop(RuntimeLifecycle::with_tasks(vec![task]));
        tokio::time::timeout(Duration::from_secs(1), async {
            while lifetime.upgrade().is_some() {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("Dropping the lifecycle kept its background task alive");
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn shutdown_cancels_background_work_then_waits_for_pending_event_writes() {
        use crate::infrastructure::storage::Database;
        use snaplingo_core::application::history::EventSubscriber;
        use snaplingo_core::application::{
            SelectedTextAcquirer, SelectionScheme, SystemSelectionProvider,
        };
        use snaplingo_core::domain::events::DomainEvent;
        use tokio::sync::Notify;

        struct Cancelled(Arc<Notify>);
        impl Drop for Cancelled {
            fn drop(&mut self) {
                self.0.notify_one();
            }
        }
        struct PendingWrite {
            cancelled: Arc<Notify>,
            entered: Notify,
            logs: Arc<SqliteAppLogRepository>,
        }
        #[async_trait::async_trait]
        impl EventSubscriber for PendingWrite {
            async fn handle(&self, _: &DomainEvent) {
                self.entered.notify_one();
                self.cancelled.notified().await;
                self.logs
                    .record("INFO", "lifecycle-test", "event persisted")
                    .unwrap();
            }
        }

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("lifecycle.db");
        let provider = Arc::new(
            crate::infrastructure::system::selection::macos::MacSelectionProvider::new(Some(
                "com.snaplingo.lifecycle-test".into(),
            )),
        );
        let state = crate::composition::build_app_state(
            Arc::new(Database::open(&path).unwrap()),
            path,
            crate::composition::CompositionOptions {
                identifier: "com.snaplingo.lifecycle-test".into(),
                version: "0.1.23".into(),
                ocr_resource_root: None,
                cache_dir: Ok(directory.path().join("cache")),
                selected_text: Arc::new(SelectedTextAcquirer::new(
                    SelectionScheme::new(provider.default_scheme()),
                    provider.methods(),
                    provider,
                )),
            },
        );
        let cancelled = Arc::new(Notify::new());
        let guard = Cancelled(cancelled.clone());
        let (started, ready) = tokio::sync::oneshot::channel();
        let lifecycle = RuntimeLifecycle::with_tasks(vec![runtime::spawn(async move {
            let _guard = guard;
            let _ = started.send(());
            std::future::pending::<()>().await;
        })]);
        ready.await.unwrap();
        assert!(lifecycle.request_shutdown());
        assert!(!lifecycle.request_shutdown());
        assert!(!lifecycle.is_shutdown_complete());
        let subscriber = Arc::new(PendingWrite {
            cancelled,
            entered: Notify::new(),
            logs: state.logs.repository.clone(),
        });
        state.history.events.subscribe(subscriber.clone()).await;
        state
            .history
            .events
            .publish(DomainEvent::ProviderConfigurationFailed {
                provider_id: "fixture".into(),
                error_message: "fixture".into(),
                timestamp: chrono::Utc::now(),
            });
        subscriber.entered.notified().await;
        tokio::time::timeout(Duration::from_secs(2), lifecycle.shutdown(&state))
            .await
            .expect("Shutdown did not cancel background work before draining events")
            .unwrap();
        assert!(lifecycle.is_shutdown_complete());
        assert_eq!(
            state.logs.repository.list(20).unwrap()[0].message,
            "event persisted"
        );
        lifecycle.shutdown(&state).await.unwrap();
    }
}
