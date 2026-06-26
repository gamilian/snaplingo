use crate::domain::events::DomainEvent;
use async_trait::async_trait;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Trait for event subscribers.
///
/// Implementations receive domain events and can react accordingly.
/// Subscribers should not panic - any errors should be handled internally.
#[async_trait]
pub trait EventSubscriber: Send + Sync {
    /// Handle a domain event
    async fn handle(&self, event: &DomainEvent);

    /// Optional: subscriber name for debugging
    fn name(&self) -> &str {
        "unnamed_subscriber"
    }
}

/// Central event bus for domain events.
///
/// Provides publish-subscribe pattern with async, non-blocking delivery.
/// Events are delivered concurrently to all subscribers with timeout protection.
pub struct EventBus {
    subscribers: Arc<RwLock<Vec<Arc<dyn EventSubscriber>>>>,
    pending_events: Arc<AtomicUsize>,
}

impl EventBus {
    /// Creates a new EventBus
    pub fn new() -> Self {
        Self {
            subscribers: Arc::new(RwLock::new(Vec::new())),
            pending_events: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Subscribe to all events
    pub async fn subscribe(&self, subscriber: Arc<dyn EventSubscriber>) {
        let mut subs = self.subscribers.write().await;
        subs.push(subscriber);
    }

    /// Publish an event to all subscribers
    ///
    /// This is fire-and-forget: subscribers are notified asynchronously
    /// and errors are logged but do not propagate to the publisher.
    pub fn publish(&self, event: DomainEvent) {
        // Clone the Arc to RwLock (not the RwLock itself)
        let subscribers = Arc::clone(&self.subscribers);
        let pending = Arc::clone(&self.pending_events);

        // Increment pending counter
        pending.fetch_add(1, Ordering::SeqCst);

        tokio::spawn(async move {
            // Acquire read lock and clone the subscriber list
            let subscriber_list: Vec<Arc<dyn EventSubscriber>> = {
                let subs = subscribers.read().await;
                subs.iter().cloned().collect()
            }; // Lock is dropped here

            // Notify all subscribers concurrently
            let handles: Vec<_> = subscriber_list
                .into_iter()
                .map(|sub| {
                    let event = event.clone();

                    tokio::spawn(async move {
                        let result: Result<(), tokio::time::error::Elapsed> = tokio::time::timeout(
                            std::time::Duration::from_secs(5),
                            sub.handle(&event),
                        )
                        .await;

                        if result.is_err() {
                            eprintln!(
                                "[EventBus] Subscriber '{}' timed out handling {}",
                                sub.name(),
                                event.event_type()
                            );
                        }
                    })
                })
                .collect();

            // Wait for all handlers
            for handle in handles {
                let _ = handle.await;
            }

            // Decrement pending counter after all handlers complete
            pending.fetch_sub(1, Ordering::SeqCst);
        });
    }

    /// Wait for all pending events to complete (for graceful shutdown)
    ///
    /// Blocks for up to `timeout` duration waiting for pending events to finish.
    /// Returns true if all events completed, false if timed out.
    pub async fn drain(&self, timeout: std::time::Duration) -> bool {
        let start = std::time::Instant::now();

        while self.pending_events.load(Ordering::SeqCst) > 0 {
            if start.elapsed() >= timeout {
                log::warn!(
                    "EventBus drain timed out with {} pending events",
                    self.pending_events.load(Ordering::SeqCst)
                );
                return false;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        log::info!("EventBus drained successfully");
        true
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}
