// Link Strength Decay Job
//
// Periodically weakens untraversed links and removes very weak links.
// Helps keep the memory graph focused on frequently used connections.

use super::config::JobConfig;
use super::scheduler::{EvolutionJob, JobError, JobReport};
use crate::storage::libsql::LibsqlStorage;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use std::time::Instant;

/// Link decay job
pub struct LinkDecayJob {
    storage: Arc<LibsqlStorage>,
}

impl LinkDecayJob {
    pub fn new(storage: Arc<LibsqlStorage>) -> Self {
        Self { storage }
    }

    /// Calculate decay factor for a link based on traversal history
    ///
    /// Returns a multiplier (0-1) to apply to current strength:
    /// - Never traversed + >180 days old → 0.25 (quarter strength)
    /// - Never traversed + >90 days old → 0.5 (half strength)
    /// - Not traversed in 180 days → 0.25
    /// - Not traversed in 90 days → 0.5
    /// - Old link (>365 days) not traversed in 30 days → 0.8
    /// - Otherwise → 1.0 (no decay)
    pub fn calculate_decay(&self, link: &LinkData) -> Result<f32, JobError> {
        let days_since_traversal = link.days_since_last_traversal();
        let days_since_creation = link.days_since_creation();

        // User-created links don't decay
        if link.user_created {
            return Ok(1.0);
        }

        // Strong decay for very old untraversed links
        if days_since_traversal >= 180.0 {
            return Ok(0.25); // Quarter strength after 6 months
        }

        // Medium decay for old untraversed links
        if days_since_traversal >= 90.0 {
            return Ok(0.5); // Half strength after 3 months
        }

        // Slight decay for old links that haven't been used recently
        if days_since_creation >= 365.0 && days_since_traversal >= 30.0 {
            return Ok(0.8); // 20% decay for old, recently unused links
        }

        // No decay
        Ok(1.0)
    }

    /// Check if link should be removed (strength below threshold)
    pub fn should_remove(&self, strength: f32) -> bool {
        strength < 0.1
    }
}

#[async_trait]
impl EvolutionJob for LinkDecayJob {
    fn name(&self) -> &str {
        "link_decay"
    }

    async fn run(&self, config: &JobConfig) -> Result<JobReport, JobError> {
        let start = Instant::now();
        let mut memories_processed = 0;
        let mut changes_made = 0;
        let mut removed = 0;
        let mut errors = 0;

        tracing::info!(
            "Starting link decay job (batch_size: {})",
            config.batch_size
        );

        // Get link decay candidates from storage (untraversed links)
        // Using 90 days threshold as a balance between aggressive (180d) and conservative (30d)
        let links = self
            .storage
            .find_link_decay_candidates(90, config.batch_size)
            .await
            .map_err(|e| JobError::ExecutionError(e.to_string()))?;

        for (source_id, link) in links {
            memories_processed += 1;

            // Convert to LinkData for calculation
            let link_data = LinkData {
                id: format!("{}_{}", source_id, link.target_id),
                source_id: source_id.to_string(),
                target_id: link.target_id.to_string(),
                strength: link.strength,
                created_at: link.created_at,
                last_traversed_at: link.last_traversed_at,
                user_created: link.user_created,
            };

            // Calculate decay factor
            let decay_factor = match self.calculate_decay(&link_data) {
                Ok(factor) => factor,
                Err(e) => {
                    tracing::warn!("Failed to calculate decay for link: {:?}", e);
                    errors += 1;
                    continue;
                }
            };

            let new_strength = link.strength * decay_factor;

            // Remove if below threshold
            if self.should_remove(new_strength) {
                match self.storage.remove_link(&source_id, &link.target_id).await {
                    Ok(_) => {
                        removed += 1;
                        tracing::debug!(
                            "Removed weak link: {} -> {} (strength: {})",
                            source_id,
                            link.target_id,
                            new_strength
                        );
                    }
                    Err(e) => {
                        tracing::warn!("Failed to remove link: {:?}", e);
                        errors += 1;
                    }
                }
            } else if (new_strength - link.strength).abs() > 0.01 {
                // Update strength if changed significantly
                match self
                    .storage
                    .update_link_strength(&source_id, &link.target_id, new_strength)
                    .await
                {
                    Ok(_) => {
                        changes_made += 1;
                        tracing::debug!(
                            "Updated link strength: {} -> {} ({} -> {})",
                            source_id,
                            link.target_id,
                            link.strength,
                            new_strength
                        );
                    }
                    Err(e) => {
                        tracing::warn!("Failed to update link strength: {:?}", e);
                        errors += 1;
                    }
                }
            }
        }

        tracing::info!(
            "Link decay complete: {} processed, {} updated, {} removed in {:?}",
            memories_processed,
            changes_made,
            removed,
            start.elapsed()
        );

        Ok(JobReport {
            memories_processed,
            changes_made: changes_made + removed,
            duration: start.elapsed(),
            errors,
            error_message: None,
        })
    }

    async fn should_run(&self) -> Result<bool, JobError> {
        // In full implementation, would check last run time from database
        // For now, always return true (scheduler will control frequency)
        Ok(true)
    }
}

/// Link data needed for decay calculation
pub struct LinkData {
    pub id: String,
    pub source_id: String,
    pub target_id: String,
    pub strength: f32,
    pub created_at: DateTime<Utc>,
    pub last_traversed_at: Option<DateTime<Utc>>,
    pub user_created: bool,
}

impl LinkData {
    /// Calculate days since link was created
    pub fn days_since_creation(&self) -> f32 {
        let now = Utc::now();
        let duration = now.signed_duration_since(self.created_at);
        duration.num_seconds() as f32 / 86400.0
    }

    /// Calculate days since link was last traversed
    pub fn days_since_last_traversal(&self) -> f32 {
        let now = Utc::now();
        let last_traversal = self.last_traversed_at.unwrap_or(self.created_at);
        let duration = now.signed_duration_since(last_traversal);
        duration.num_seconds() as f32 / 86400.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConnectionMode;
    use chrono::Duration as ChronoDuration;

    fn create_test_link(
        strength: f32,
        days_old: i64,
        days_since_traversal: i64,
        user_created: bool,
    ) -> LinkData {
        let now = Utc::now();
        LinkData {
            id: "test-link-1".to_string(),
            source_id: "mem-1".to_string(),
            target_id: "mem-2".to_string(),
            strength,
            created_at: now - ChronoDuration::days(days_old),
            last_traversed_at: Some(now - ChronoDuration::days(days_since_traversal)),
            user_created,
        }
    }

    #[tokio::test]
    async fn test_calculate_decay_no_decay() {
        let storage = Arc::new(LibsqlStorage::new(ConnectionMode::InMemory).await.unwrap());
        let job = LinkDecayJob::new(storage);

        // Recent link
        let link_recent = create_test_link(0.8, 10, 5, false);
        let decay = job.calculate_decay(&link_recent).unwrap();
        assert_eq!(decay, 1.0); // No decay

        // Old link, recently traversed
        let link_old_recent = create_test_link(0.8, 400, 10, false);
        let decay_old = job.calculate_decay(&link_old_recent).unwrap();
        assert_eq!(decay_old, 1.0); // No decay
    }

    #[tokio::test]
    async fn test_calculate_decay_strong_decay() {
        let storage = Arc::new(LibsqlStorage::new(ConnectionMode::InMemory).await.unwrap());
        let job = LinkDecayJob::new(storage);

        // Not traversed in 6 months
        let link = create_test_link(0.8, 200, 180, false);
        let decay = job.calculate_decay(&link).unwrap();
        assert_eq!(decay, 0.25); // Quarter strength
    }

    #[tokio::test]
    async fn test_calculate_decay_medium_decay() {
        let storage = Arc::new(LibsqlStorage::new(ConnectionMode::InMemory).await.unwrap());
        let job = LinkDecayJob::new(storage);

        // Not traversed in 3 months
        let link = create_test_link(0.8, 100, 90, false);
        let decay = job.calculate_decay(&link).unwrap();
        assert_eq!(decay, 0.5); // Half strength
    }

    #[tokio::test]
    async fn test_calculate_decay_slight_decay() {
        let storage = Arc::new(LibsqlStorage::new(ConnectionMode::InMemory).await.unwrap());
        let job = LinkDecayJob::new(storage);

        // Old link (>365 days), not traversed in 30 days
        let link = create_test_link(0.8, 400, 35, false);
        let decay = job.calculate_decay(&link).unwrap();
        assert_eq!(decay, 0.8); // 20% decay
    }

    #[tokio::test]
    async fn test_calculate_decay_user_created_no_decay() {
        let storage = Arc::new(LibsqlStorage::new(ConnectionMode::InMemory).await.unwrap());
        let job = LinkDecayJob::new(storage);

        // User-created link should never decay
        let link = create_test_link(0.8, 400, 200, true);
        let decay = job.calculate_decay(&link).unwrap();
        assert_eq!(decay, 1.0); // No decay for user-created
    }

    #[tokio::test]
    async fn test_should_remove() {
        let storage = Arc::new(LibsqlStorage::new(ConnectionMode::InMemory).await.unwrap());
        let job = LinkDecayJob::new(storage);

        assert!(job.should_remove(0.05)); // Below threshold
        assert!(job.should_remove(0.09)); // Below threshold
        assert!(!job.should_remove(0.1)); // At threshold (keep)
        assert!(!job.should_remove(0.5)); // Above threshold
        assert!(!job.should_remove(1.0)); // Max strength
    }

    #[tokio::test]
    async fn test_decay_application() {
        let storage = Arc::new(LibsqlStorage::new(ConnectionMode::InMemory).await.unwrap());
        let job = LinkDecayJob::new(storage);

        // Link with 0.8 strength, 90-day decay (0.5x)
        let link = create_test_link(0.8, 100, 90, false);
        let decay = job.calculate_decay(&link).unwrap();
        let new_strength = link.strength * decay;

        assert_eq!(new_strength, 0.4); // 0.8 * 0.5
        assert!(!job.should_remove(new_strength)); // Still above threshold
    }

    #[tokio::test]
    async fn test_decay_to_removal() {
        let storage = Arc::new(LibsqlStorage::new(ConnectionMode::InMemory).await.unwrap());
        let job = LinkDecayJob::new(storage);

        // Weak link with 180-day decay (0.25x)
        let link = create_test_link(0.3, 200, 180, false);
        let decay = job.calculate_decay(&link).unwrap();
        let new_strength = link.strength * decay;

        assert_eq!(new_strength, 0.075); // 0.3 * 0.25
        assert!(job.should_remove(new_strength)); // Below threshold, should remove
    }

    #[test]
    fn test_link_data_days_since_creation() {
        let link = create_test_link(0.8, 30, 10, false);
        let days = link.days_since_creation();
        assert!((days - 30.0).abs() < 1.0); // Allow 1-day tolerance
    }

    #[test]
    fn test_link_data_days_since_traversal() {
        let link = create_test_link(0.8, 30, 10, false);
        let days = link.days_since_last_traversal();
        assert!((days - 10.0).abs() < 1.0); // Allow 1-day tolerance
    }

    #[test]
    fn test_never_traversed_uses_created_at() {
        let now = Utc::now();
        let link = LinkData {
            id: "test".to_string(),
            source_id: "mem-1".to_string(),
            target_id: "mem-2".to_string(),
            strength: 0.8,
            created_at: now - ChronoDuration::days(30),
            last_traversed_at: None,
            user_created: false,
        };

        let days = link.days_since_last_traversal();
        assert!((days - 30.0).abs() < 1.0);
    }

    #[tokio::test]
    async fn test_multiple_decay_applications() {
        let storage = Arc::new(LibsqlStorage::new(ConnectionMode::InMemory).await.unwrap());
        let job = LinkDecayJob::new(storage);

        // Simulate multiple runs with same link
        let mut current_strength = 1.0;
        let link = create_test_link(current_strength, 400, 35, false); // Old, not recently used

        // First decay
        let decay1 = job.calculate_decay(&link).unwrap();
        current_strength *= decay1;
        assert_eq!(current_strength, 0.8); // First decay to 0.8

        // Simulate more time passing and another decay
        let link2 = create_test_link(current_strength, 450, 90, false);
        let decay2 = job.calculate_decay(&link2).unwrap();
        current_strength *= decay2;
        assert_eq!(current_strength, 0.4); // Second decay to 0.4

        assert!(!job.should_remove(current_strength)); // Still above threshold
    }
}
