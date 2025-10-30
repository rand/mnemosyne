//! Conflict Notification System
//!
//! Manages notifications about file conflicts to agents according to user requirements:
//! - **On every save**: Notify about NEW conflicts immediately
//! - **Every 20 minutes**: Summary of ALL active conflicts
//! - **Before session end**: Final summary of unresolved conflicts
//!
//! # Design
//!
//! - Integrates with FileTracker for conflict detection
//! - Maintains notification history to avoid spam
//! - Background task for periodic notifications
//! - Session lifecycle hooks for final summary

use crate::orchestration::file_tracker::{ActiveConflict, FileTracker};
use crate::orchestration::identity::AgentId;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

/// Notification configuration
#[derive(Debug, Clone)]
pub struct NotificationConfig {
    /// Enable notifications (default: true)
    pub enabled: bool,

    /// Notify on every save for new conflicts (default: true)
    pub notify_on_save: bool,

    /// Periodic notification interval in minutes (default: 20)
    pub periodic_interval_minutes: i64,

    /// Send final summary before session end (default: true)
    pub session_end_summary: bool,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            notify_on_save: true,
            periodic_interval_minutes: 20,
            session_end_summary: true,
        }
    }
}

/// Notification message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictNotification {
    /// Notification ID
    pub id: String,

    /// Target agent
    pub agent_id: AgentId,

    /// Notification type
    pub notification_type: NotificationType,

    /// Conflicts included
    pub conflicts: Vec<ActiveConflict>,

    /// When notification was created
    pub timestamp: DateTime<Utc>,

    /// Message text
    pub message: String,
}

/// Type of notification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NotificationType {
    /// New conflict detected on save
    NewConflict,

    /// Periodic summary (every 20 minutes)
    PeriodicSummary,

    /// Final summary before session end
    SessionEndSummary,
}

/// Notification history entry
#[derive(Debug, Clone)]
struct NotificationRecord {
    notification_id: String,
    agent_id: AgentId,
    conflict_ids: Vec<String>,
    timestamp: DateTime<Utc>,
}

/// Conflict notifier
pub struct ConflictNotifier {
    /// Configuration
    config: NotificationConfig,

    /// File tracker for conflict detection
    file_tracker: Arc<FileTracker>,

    /// Last periodic notification time per agent
    last_periodic_notification: Arc<RwLock<HashMap<AgentId, DateTime<Utc>>>>,

    /// Notification history
    notification_history: Arc<RwLock<Vec<NotificationRecord>>>,

    /// Conflicts already notified to agents (avoid duplicate new-conflict alerts)
    notified_conflicts: Arc<RwLock<HashMap<AgentId, HashSet<String>>>>,
}

impl ConflictNotifier {
    /// Create a new conflict notifier
    pub fn new(config: NotificationConfig, file_tracker: Arc<FileTracker>) -> Self {
        Self {
            config,
            file_tracker,
            last_periodic_notification: Arc::new(RwLock::new(HashMap::new())),
            notification_history: Arc::new(RwLock::new(Vec::new())),
            notified_conflicts: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create with default configuration
    pub fn with_default(file_tracker: Arc<FileTracker>) -> Self {
        Self::new(NotificationConfig::default(), file_tracker)
    }

    /// Notify agent of new conflicts (called on every save)
    ///
    /// Only notifies about conflicts that haven't been notified before.
    /// Returns notifications that should be sent.
    pub fn notify_on_save(&self, agent_id: &AgentId) -> crate::error::Result<Vec<ConflictNotification>> {
        if !self.config.enabled || !self.config.notify_on_save {
            return Ok(vec![]);
        }

        let agent_conflicts = self.file_tracker.get_agent_conflicts(agent_id)?;

        // Filter to only NEW conflicts (not previously notified)
        let mut notified = self.notified_conflicts.write().map_err(|e| {
            crate::error::MnemosyneError::Other(format!("Failed to lock notified_conflicts: {}", e))
        })?;

        let agent_notified = notified.entry(agent_id.clone()).or_insert_with(HashSet::new);

        let new_conflicts: Vec<ActiveConflict> = agent_conflicts
            .into_iter()
            .filter(|c| !agent_notified.contains(&c.id))
            .collect();

        if new_conflicts.is_empty() {
            return Ok(vec![]);
        }

        // Mark as notified
        for conflict in &new_conflicts {
            agent_notified.insert(conflict.id.clone());
            self.file_tracker.mark_conflict_notified(&conflict.id)?;
        }

        // Generate notifications
        let notifications: Vec<ConflictNotification> = new_conflicts
            .iter()
            .map(|conflict| {
                let notification = ConflictNotification {
                    id: format!("new-{}-{}", agent_id, conflict.id),
                    agent_id: agent_id.clone(),
                    notification_type: NotificationType::NewConflict,
                    conflicts: vec![conflict.clone()],
                    timestamp: Utc::now(),
                    message: self.format_new_conflict_message(agent_id, conflict),
                };

                // Record in history
                self.record_notification(&notification);

                notification
            })
            .collect();

        Ok(notifications)
    }

    /// Generate periodic summary (called every 20 minutes)
    ///
    /// Returns notifications for agents who haven't received a summary recently.
    pub fn generate_periodic_summaries(&self) -> crate::error::Result<Vec<ConflictNotification>> {
        if !self.config.enabled {
            return Ok(vec![]);
        }

        let mut notifications = Vec::new();
        let now = Utc::now();
        let interval = Duration::minutes(self.config.periodic_interval_minutes);

        let mut last_notif = self.last_periodic_notification.write().map_err(|e| {
            crate::error::MnemosyneError::Other(format!("Failed to lock last_periodic_notification: {}", e))
        })?;

        // Get all active conflicts
        let all_conflicts = self.file_tracker.get_active_conflicts()?;

        // Group by agent
        let mut conflicts_by_agent: HashMap<AgentId, Vec<ActiveConflict>> = HashMap::new();
        for conflict in all_conflicts {
            for agent_id in &conflict.agents {
                conflicts_by_agent
                    .entry(agent_id.clone())
                    .or_insert_with(Vec::new)
                    .push(conflict.clone());
            }
        }

        // Generate summaries for agents due for notification
        for (agent_id, conflicts) in conflicts_by_agent {
            if conflicts.is_empty() {
                continue;
            }

            let last_time = last_notif.get(&agent_id);
            let should_notify = match last_time {
                Some(last) => now.signed_duration_since(*last) >= interval,
                None => true, // First notification
            };

            if should_notify {
                let notification = ConflictNotification {
                    id: format!("periodic-{}-{}", agent_id, now.timestamp()),
                    agent_id: agent_id.clone(),
                    notification_type: NotificationType::PeriodicSummary,
                    conflicts: conflicts.clone(),
                    timestamp: now,
                    message: self.format_periodic_summary(&agent_id, &conflicts),
                };

                self.record_notification(&notification);
                notifications.push(notification);

                last_notif.insert(agent_id, now);
            }
        }

        Ok(notifications)
    }

    /// Generate final session summary (called before session end)
    ///
    /// Returns notifications for all agents with unresolved conflicts.
    pub fn generate_session_end_summaries(&self) -> crate::error::Result<Vec<ConflictNotification>> {
        if !self.config.enabled || !self.config.session_end_summary {
            return Ok(vec![]);
        }

        let now = Utc::now();
        let all_conflicts = self.file_tracker.get_active_conflicts()?;

        // Group by agent
        let mut conflicts_by_agent: HashMap<AgentId, Vec<ActiveConflict>> = HashMap::new();
        for conflict in all_conflicts {
            for agent_id in &conflict.agents {
                conflicts_by_agent
                    .entry(agent_id.clone())
                    .or_insert_with(Vec::new)
                    .push(conflict.clone());
            }
        }

        let notifications: Vec<ConflictNotification> = conflicts_by_agent
            .into_iter()
            .map(|(agent_id, conflicts)| {
                let notification = ConflictNotification {
                    id: format!("session-end-{}-{}", agent_id, now.timestamp()),
                    agent_id: agent_id.clone(),
                    notification_type: NotificationType::SessionEndSummary,
                    conflicts: conflicts.clone(),
                    timestamp: now,
                    message: self.format_session_end_summary(&agent_id, &conflicts),
                };

                self.record_notification(&notification);
                notification
            })
            .collect();

        Ok(notifications)
    }

    /// Format new conflict message
    fn format_new_conflict_message(&self, _agent_id: &AgentId, conflict: &ActiveConflict) -> String {
        let other_agents: Vec<String> = conflict
            .agents
            .iter()
            .map(|id| id.to_string())
            .collect();

        format!(
            "⚠️  NEW CONFLICT DETECTED\n\
             File: {}\n\
             Other agent(s): {}\n\
             Severity: {:?}\n\
             \n\
             Detected {} ago.\n\
             \n\
             Suggestions:\n\
             - Coordinate via chat: /agent-message {}\n\
             - Work sequentially: Wait for their commit\n\
             - Split work: Different functions/areas",
            conflict.path.display(),
            other_agents.join(", "),
            conflict.severity,
            format_duration_ago(conflict.detected_at),
            other_agents.first().unwrap_or(&"".to_string())
        )
    }

    /// Format periodic summary message
    fn format_periodic_summary(&self, _agent_id: &AgentId, conflicts: &[ActiveConflict]) -> String {
        let mut message = format!(
            "📊 CONFLICT SUMMARY (20-minute update)\n\
             Active conflicts: {}\n\n",
            conflicts.len()
        );

        for (i, conflict) in conflicts.iter().take(5).enumerate() {
            let other_agents: Vec<String> = conflict
                .agents
                .iter()
                .map(|id| id.to_string())
                .collect();

            message.push_str(&format!(
                "{}. [{:?}] {} (with: {})\n",
                i + 1,
                conflict.severity,
                conflict.path.display(),
                other_agents.join(", ")
            ));
        }

        if conflicts.len() > 5 {
            message.push_str(&format!("\n... and {} more\n", conflicts.len() - 5));
        }

        message.push_str(
            "\nActions:\n\
             - Review conflicts: /branch-conflicts\n\
             - Coordinate with agents: /agent-message <agent-id>",
        );

        message
    }

    /// Format session end summary message
    fn format_session_end_summary(&self, _agent_id: &AgentId, conflicts: &[ActiveConflict]) -> String {
        let mut message = format!(
            "🔚 SESSION ENDING - Final Conflict Summary\n\
             Unresolved conflicts: {}\n\n",
            conflicts.len()
        );

        for (i, conflict) in conflicts.iter().enumerate() {
            let other_agents: Vec<String> = conflict
                .agents
                .iter()
                .map(|id| id.to_string())
                .collect();

            message.push_str(&format!(
                "{}. [{:?}] {}\n   With: {}\n   Age: {}\n\n",
                i + 1,
                conflict.severity,
                conflict.path.display(),
                other_agents.join(", "),
                format_duration_ago(conflict.detected_at)
            ));
        }

        message.push_str(
            "Actions before exit:\n\
             - Review all conflicts: /branch-conflicts\n\
             - Coordinate with other agents\n\
             - Consider committing separately to avoid merge conflicts",
        );

        message
    }

    /// Record notification in history
    fn record_notification(&self, notification: &ConflictNotification) {
        if let Ok(mut history) = self.notification_history.write() {
            history.push(NotificationRecord {
                notification_id: notification.id.clone(),
                agent_id: notification.agent_id.clone(),
                conflict_ids: notification.conflicts.iter().map(|c| c.id.clone()).collect(),
                timestamp: notification.timestamp,
            });
        }
    }

    /// Get notification history for agent
    pub fn get_agent_history(&self, agent_id: &AgentId) -> Vec<NotificationRecord> {
        if let Ok(history) = self.notification_history.read() {
            history
                .iter()
                .filter(|r| &r.agent_id == agent_id)
                .cloned()
                .collect()
        } else {
            vec![]
        }
    }

    /// Clear agent from notified conflicts (e.g., after resolving)
    pub fn clear_agent_notifications(&self, agent_id: &AgentId) -> crate::error::Result<()> {
        let mut notified = self.notified_conflicts.write().map_err(|e| {
            crate::error::MnemosyneError::Other(format!("Failed to lock notified_conflicts: {}", e))
        })?;

        notified.remove(agent_id);
        Ok(())
    }

    /// Get the current count of active conflicts
    ///
    /// Returns the total number of conflicts across all agents.
    pub fn get_conflict_count(&self) -> crate::error::Result<usize> {
        let conflicts = self.file_tracker.get_active_conflicts()?;
        Ok(conflicts.len())
    }

    /// Get the count of conflicts for a specific agent
    ///
    /// Returns the number of conflicts involving the specified agent.
    pub fn get_agent_conflict_count(&self, agent_id: &AgentId) -> crate::error::Result<usize> {
        let conflicts = self.file_tracker.get_agent_conflicts(agent_id)?;
        Ok(conflicts.len())
    }
}

/// Format duration as human-readable "ago" string
fn format_duration_ago(timestamp: DateTime<Utc>) -> String {
    let duration = Utc::now().signed_duration_since(timestamp);

    if duration.num_seconds() < 60 {
        format!("{}s", duration.num_seconds())
    } else if duration.num_minutes() < 60 {
        format!("{}m", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{}h", duration.num_hours())
    } else {
        format!("{}d", duration.num_days())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::conflict_detector::ConflictDetector;
    use std::path::PathBuf;
    use std::thread;
    use std::time::Duration as StdDuration;

    #[test]
    fn test_notify_on_save_new_conflict() {
        let detector = Arc::new(ConflictDetector::new());
        let tracker = Arc::new(FileTracker::new(detector));
        let notifier = ConflictNotifier::with_default(tracker.clone());

        let agent1 = AgentId::new();
        let agent2 = AgentId::new();
        let path = PathBuf::from("src/main.rs");

        // Create conflict
        tracker
            .record_modification(&agent1, &path, crate::orchestration::file_tracker::ModificationType::Modified)
            .unwrap();
        tracker
            .record_modification(&agent2, &path, crate::orchestration::file_tracker::ModificationType::Modified)
            .unwrap();

        // Should notify agent1 of new conflict
        let notifications = notifier.notify_on_save(&agent1).unwrap();
        assert_eq!(notifications.len(), 1);
        assert_eq!(notifications[0].notification_type, NotificationType::NewConflict);

        // Second call should not notify again (already notified)
        let notifications2 = notifier.notify_on_save(&agent1).unwrap();
        assert_eq!(notifications2.len(), 0);
    }

    #[test]
    fn test_periodic_summary() {
        let detector = Arc::new(ConflictDetector::new());
        let tracker = Arc::new(FileTracker::new(detector));
        let config = NotificationConfig {
            periodic_interval_minutes: 0, // Immediate for testing
            ..Default::default()
        };
        let notifier = ConflictNotifier::new(config, tracker.clone());

        let agent1 = AgentId::new();
        let agent2 = AgentId::new();
        let path = PathBuf::from("src/main.rs");

        // Create conflict
        tracker
            .record_modification(&agent1, &path, crate::orchestration::file_tracker::ModificationType::Modified)
            .unwrap();
        tracker
            .record_modification(&agent2, &path, crate::orchestration::file_tracker::ModificationType::Modified)
            .unwrap();

        // Generate periodic summaries
        let summaries = notifier.generate_periodic_summaries().unwrap();

        // Should have summaries for both agents
        assert_eq!(summaries.len(), 2);
        assert!(summaries
            .iter()
            .all(|n| n.notification_type == NotificationType::PeriodicSummary));
    }

    #[test]
    fn test_session_end_summary() {
        let detector = Arc::new(ConflictDetector::new());
        let tracker = Arc::new(FileTracker::new(detector));
        let notifier = ConflictNotifier::with_default(tracker.clone());

        let agent1 = AgentId::new();
        let agent2 = AgentId::new();
        let path = PathBuf::from("src/main.rs");

        // Create conflict
        tracker
            .record_modification(&agent1, &path, crate::orchestration::file_tracker::ModificationType::Modified)
            .unwrap();
        tracker
            .record_modification(&agent2, &path, crate::orchestration::file_tracker::ModificationType::Modified)
            .unwrap();

        // Generate session end summaries
        let summaries = notifier.generate_session_end_summaries().unwrap();

        assert_eq!(summaries.len(), 2);
        assert!(summaries
            .iter()
            .all(|n| n.notification_type == NotificationType::SessionEndSummary));
    }

    #[test]
    fn test_notification_history() {
        let detector = Arc::new(ConflictDetector::new());
        let tracker = Arc::new(FileTracker::new(detector));
        let notifier = ConflictNotifier::with_default(tracker.clone());

        let agent = AgentId::new();
        let agent2 = AgentId::new();
        let path = PathBuf::from("src/main.rs");

        tracker
            .record_modification(&agent, &path, crate::orchestration::file_tracker::ModificationType::Modified)
            .unwrap();
        tracker
            .record_modification(&agent2, &path, crate::orchestration::file_tracker::ModificationType::Modified)
            .unwrap();

        // Generate notifications
        notifier.notify_on_save(&agent).unwrap();

        // Check history
        let history = notifier.get_agent_history(&agent);
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn test_format_duration_ago() {
        let now = Utc::now();
        let past = now - Duration::seconds(30);
        assert_eq!(format_duration_ago(past), "30s");

        let past = now - Duration::minutes(5);
        assert_eq!(format_duration_ago(past), "5m");

        let past = now - Duration::hours(2);
        assert_eq!(format_duration_ago(past), "2h");
    }
}
