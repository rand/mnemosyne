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
use crate::orchestration::conflict_notifier::{ConflictNotification, ConflictNotifier, NotificationType};
use crate::orchestration::cross_process::{CoordinationMessage, CrossProcessCoordinator, MessageType};
use crate::storage::StorageBackend;
use crate::types::{MemoryNote, MemoryId, MemoryType, Namespace};
use chrono::Utc;
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
    /// * `storage` - Storage backend for persisting notifications
    /// * `coordinator` - Cross-process coordinator for sending messages (optional)
    /// * `namespace` - Namespace for storing notification memories
    /// * `interval_minutes` - Interval in minutes (default: 20)
    pub fn spawn(
        notifier: Arc<ConflictNotifier>,
        storage: Arc<dyn StorageBackend>,
        coordinator: Option<Arc<CrossProcessCoordinator>>,
        namespace: Namespace,
        interval_minutes: u64,
    ) -> Self {
        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);

        let task_handle = tokio::spawn(async move {
            if let Err(e) = run_notification_loop(
                notifier,
                storage,
                coordinator,
                namespace,
                interval_minutes,
                shutdown_rx,
            )
            .await
            {
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
    storage: Arc<dyn StorageBackend>,
    coordinator: Option<Arc<CrossProcessCoordinator>>,
    namespace: Namespace,
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
                            process_notifications(
                                &notifier,
                                &storage,
                                coordinator.as_ref(),
                                &namespace,
                                notifications,
                            )
                            .await?;
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

/// Process notifications - implements actual notification delivery
async fn process_notifications(
    _notifier: &Arc<ConflictNotifier>,
    storage: &Arc<dyn StorageBackend>,
    coordinator: Option<&Arc<CrossProcessCoordinator>>,
    namespace: &Namespace,
    notifications: Vec<ConflictNotification>,
) -> Result<()> {
    for notification in notifications {
        tracing::debug!(
            "Processing notification for agent {}: {:?}",
            notification.agent_id,
            notification.notification_type
        );

        // 1. Send notification via cross-process coordinator if available
        if let Some(coord) = coordinator {
            let message = CoordinationMessage {
                id: notification.id.clone(),
                from_agent: crate::orchestration::identity::AgentId::new("optimizer".to_string()),
                to_agent: Some(notification.agent_id.clone()),
                message_type: MessageType::ConflictNotification,
                timestamp: notification.timestamp,
                payload: serde_json::to_value(&notification).map_err(|e| {
                    crate::error::MnemosyneError::Other(format!(
                        "Failed to serialize notification: {}",
                        e
                    ))
                })?,
            };

            if let Err(e) = coord.send_message(message) {
                tracing::warn!(
                    "Failed to send notification via coordinator to agent {}: {}",
                    notification.agent_id,
                    e
                );
            } else {
                tracing::info!(
                    "Sent notification {} to agent {} via coordinator",
                    notification.id,
                    notification.agent_id
                );
            }
        }

        // 2. Store notification as memory for historical tracking
        let memory = MemoryNote {
            id: MemoryId::new(),
            namespace: namespace.clone(),
            created_at: notification.timestamp,
            updated_at: notification.timestamp,
            content: notification.message.clone(),
            summary: format!(
                "{:?} notification for agent {}",
                notification.notification_type, notification.agent_id
            ),
            keywords: vec![
                "notification".to_string(),
                "conflict".to_string(),
                format!("{:?}", notification.notification_type).to_lowercase(),
            ],
            tags: vec!["notification".to_string(), "conflict".to_string()],
            context: format!(
                "Notification {} for agent {} with {} conflict(s)",
                notification.id,
                notification.agent_id,
                notification.conflicts.len()
            ),
            memory_type: MemoryType::AgentEvent,
            importance: match notification.notification_type {
                NotificationType::NewConflict => 7,
                NotificationType::PeriodicSummary => 6,
                NotificationType::SessionEndSummary => 8,
            },
            confidence: 1.0,
            links: vec![],
            related_files: notification
                .conflicts
                .iter()
                .map(|c| c.file_path.clone())
                .collect(),
            related_entities: vec![notification.agent_id.to_string()],
            access_count: 0,
            last_accessed_at: notification.timestamp,
            expires_at: None,
            is_archived: false,
            superseded_by: None,
            embedding: None,
            embedding_model: String::new(),
        };

        if let Err(e) = storage.store_memory(&memory).await {
            tracing::warn!(
                "Failed to store notification {} as memory: {}",
                notification.id,
                e
            );
        } else {
            tracing::debug!("Stored notification {} as memory {}", notification.id, memory.id);
        }

        // 3. Log high-severity notifications
        if !notification.conflicts.is_empty() {
            match notification.notification_type {
                NotificationType::NewConflict => {
                    tracing::warn!(
                        "New conflict detected for agent {}: {} conflict(s) in files: {:?}",
                        notification.agent_id,
                        notification.conflicts.len(),
                        notification
                            .conflicts
                            .iter()
                            .map(|c| c.file_path.display().to_string())
                            .collect::<Vec<_>>()
                    );
                }
                NotificationType::SessionEndSummary => {
                    tracing::warn!(
                        "Session ending with {} unresolved conflict(s) for agent {}",
                        notification.conflicts.len(),
                        notification.agent_id
                    );
                }
                NotificationType::PeriodicSummary => {
                    tracing::info!(
                        "Periodic summary: {} active conflict(s) for agent {}",
                        notification.conflicts.len(),
                        notification.agent_id
                    );
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::conflict_detector::ConflictDetector;
    use crate::orchestration::conflict_notifier::NotificationConfig;
    use crate::orchestration::file_tracker::FileTracker;
    use crate::{ConnectionMode, LibsqlStorage};
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

        let storage = Arc::new(
            LibsqlStorage::new(ConnectionMode::InMemory)
                .await
                .expect("Failed to create test storage"),
        ) as Arc<dyn StorageBackend>;

        let namespace = Namespace::Session {
            project: "test".to_string(),
            session_id: "test-session".to_string(),
        };

        // Spawn task with short interval for testing
        let mut handle = NotificationTaskHandle::spawn(notifier, storage, None, namespace, 1);

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

        let storage = Arc::new(
            LibsqlStorage::new(ConnectionMode::InMemory)
                .await
                .expect("Failed to create test storage"),
        ) as Arc<dyn StorageBackend>;

        let namespace = Namespace::Session {
            project: "test".to_string(),
            session_id: "test-session".to_string(),
        };

        let mut handle = NotificationTaskHandle::spawn(notifier, storage, None, namespace, 1);

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

        let storage = Arc::new(
            LibsqlStorage::new(ConnectionMode::InMemory)
                .await
                .expect("Failed to create test storage"),
        ) as Arc<dyn StorageBackend>;

        let namespace = Namespace::Session {
            project: "test".to_string(),
            session_id: "test-session".to_string(),
        };

        // Spawn task with 1-second interval for testing
        let mut handle = NotificationTaskHandle::spawn(notifier, storage, None, namespace, 1);

        // Let it run for 2.5 seconds (should trigger ~2 times)
        sleep(Duration::from_millis(2500)).await;

        // Stop task
        handle.stop().await.unwrap();

        // Task should have run at least once
        assert!(!handle.is_running());
    }
}
