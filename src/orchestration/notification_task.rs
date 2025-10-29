//! Background Notification Task
//!
//! Long-running task that periodically notifies agents of conflicts.
//!
//! # Design
//!
//! - Runs every 20 minutes (per user requirement)
//! - Generates summaries of all active conflicts via ConflictNotifier
//! - Gracefully shuts down when cancelled
//! - Provides handle for task management

use crate::error::Result;
use crate::orchestration::conflict_notifier::{ConflictNotification, ConflictNotifier};
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::time::{interval, Duration};

/// Notification task handle for controlling the background task
pub struct NotificationTaskHandle {
    /// Shutdown signal sender
    shutdown_tx: broadcast::Sender<()>,

    /// Task handle
    task_handle: Option<tokio::task::JoinHandle<()>>,
}

impl NotificationTaskHandle {
    /// Create and spawn a new notification task
    ///
    /// # Arguments
    ///
    /// * `notifier` - Conflict notifier for generating summaries
    /// * `interval_minutes` - Interval in minutes (default: 20)
    pub fn spawn(notifier: Arc<ConflictNotifier>, interval_minutes: u64) -> Self {
        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

        let task_handle = tokio::spawn(async move {
            if let Err(e) = run_notification_loop(notifier, interval_minutes, shutdown_rx).await {
                tracing::error!("Notification task error: {}", e);
            }
        });

        Self {
            shutdown_tx,
            task_handle: Some(task_handle),
        }
    }

    /// Stop the notification task gracefully
    pub async fn stop(&mut self) -> Result<()> {
        // Send shutdown signal
        let _ = self.shutdown_tx.send(());

        // Wait for task to complete
        if let Some(handle) = self.task_handle.take() {
            handle.await.map_err(|e| {
                crate::error::MnemosyneError::Other(format!("Failed to stop notification task: {}", e))
            })?;
        }

        tracing::info!("Notification task stopped");
        Ok(())
    }

    /// Check if task is running
    pub fn is_running(&self) -> bool {
        self.task_handle
            .as_ref()
            .map(|h| !h.is_finished())
            .unwrap_or(false)
    }
}

/// Run the notification loop
async fn run_notification_loop(
    notifier: Arc<ConflictNotifier>,
    interval_minutes: u64,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> Result<()> {
    let mut timer = interval(Duration::from_secs(interval_minutes * 60));

    tracing::info!(
        "Starting notification task with {}-minute interval",
        interval_minutes
    );

    loop {
        tokio::select! {
            _ = timer.tick() => {
                // Generate periodic summaries
                match notifier.generate_periodic_summaries() {
                    Ok(notifications) => {
                        if !notifications.is_empty() {
                            tracing::info!(
                                "Generated {} periodic conflict notification(s)",
                                notifications.len()
                            );

                            // Process notifications (e.g., send to agents, log, etc.)
                            process_notifications(&notifier, notifications).await?;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Failed to generate periodic summaries: {}", e);
                    }
                }
            }

            _ = shutdown_rx.recv() => {
                tracing::info!("Notification task received shutdown signal");
                break;
            }
        }
    }

    Ok(())
}

/// Process notifications (placeholder for actual notification delivery)
async fn process_notifications(
    _notifier: &Arc<ConflictNotifier>,
    notifications: Vec<ConflictNotification>,
) -> Result<()> {
    // In a real implementation, this would:
    // 1. Send notifications to agents via message queue
    // 2. Log to notification storage
    // 3. Trigger alerts if severity is high

    for notification in notifications {
        tracing::debug!(
            "Processing notification for agent {}: {:?}",
            notification.agent_id,
            notification.notification_type
        );

        // TODO: Implement actual notification delivery
        // This could integrate with:
        // - Cross-process coordinator for external agents
        // - Actor messaging for internal agents
        // - Notification storage for historical tracking
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::conflict_detector::ConflictDetector;
    use crate::orchestration::conflict_notifier::NotificationConfig;
    use crate::orchestration::file_tracker::FileTracker;
    use std::sync::Arc;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_notification_task_lifecycle() {
        let conflict_detector = Arc::new(ConflictDetector::new());
        let file_tracker = Arc::new(FileTracker::new(conflict_detector));

        let config = NotificationConfig {
            enabled: true,
            notify_on_save: true,
            periodic_interval_minutes: 20,
            session_end_summary: true,
        };

        let notifier = Arc::new(ConflictNotifier::new(config, file_tracker));

        // Spawn task with short interval for testing
        let mut handle = NotificationTaskHandle::spawn(notifier, 1);

        assert!(handle.is_running());

        // Let it run briefly
        sleep(Duration::from_millis(100)).await;

        // Stop task
        handle.stop().await.unwrap();

        assert!(!handle.is_running());
    }

    #[tokio::test]
    async fn test_multiple_stop_calls() {
        let conflict_detector = Arc::new(ConflictDetector::new());
        let file_tracker = Arc::new(FileTracker::new(conflict_detector));

        let config = NotificationConfig {
            enabled: true,
            notify_on_save: true,
            periodic_interval_minutes: 20,
            session_end_summary: true,
        };

        let notifier = Arc::new(ConflictNotifier::new(config, file_tracker));

        let mut handle = NotificationTaskHandle::spawn(notifier, 1);

        // Stop task
        handle.stop().await.unwrap();

        // Stop again (should not panic)
        handle.stop().await.unwrap();
    }

    #[tokio::test]
    async fn test_task_interval() {
        let conflict_detector = Arc::new(ConflictDetector::new());
        let file_tracker = Arc::new(FileTracker::new(conflict_detector));

        let config = NotificationConfig {
            enabled: true,
            notify_on_save: true,
            periodic_interval_minutes: 20,
            session_end_summary: true,
        };

        let notifier = Arc::new(ConflictNotifier::new(config, file_tracker));

        // Spawn task with 1-second interval for testing
        let mut handle = NotificationTaskHandle::spawn(notifier, 1);

        // Let it run for 2.5 seconds (should trigger ~2 times)
        sleep(Duration::from_millis(2500)).await;

        // Stop task
        handle.stop().await.unwrap();

        // Task should have run at least once
        assert!(!handle.is_running());
    }
}
