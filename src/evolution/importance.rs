// Importance Recalibration Job
//
// Periodically recalculates memory importance scores based on:
// - Base importance (30%)
// - Access patterns (40%)
// - Recency (20%)
// - Link connectivity (10%)
//
// Uses exponential decay with 30-day half-life for recency.

use super::config::JobConfig;
use super::scheduler::{EvolutionJob, JobError, JobReport};
use crate::storage::libsql::LibsqlStorage;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use std::time::Instant;

/// Importance recalibration job
pub struct ImportanceRecalibrator {
    storage: Arc<LibsqlStorage>,
}

impl ImportanceRecalibrator {
    pub fn new(storage: Arc<LibsqlStorage>) -> Self {
        Self { storage }
    }

    /// Calculate new importance score for a memory
    ///
    /// Formula: base (30%) + access (40%) + recency (20%) + links (10%)
    ///
    /// Returns score clamped to [1.0, 10.0]
    pub fn calculate_importance(&self, memory: &MemoryData) -> Result<f32, JobError> {
        let base = memory.importance / 10.0; // Normalize to 0-1
        let access = self.access_factor(memory);
        let recency = self.recency_factor(memory);
        let links = self.link_factor(memory);

        // Weighted combination
        let score = (base * 0.3) + (access * 0.4) + (recency * 0.2) + (links * 0.1);

        // Denormalize back to 1-10 range
        let final_score = (score * 9.0) + 1.0;

        Ok(final_score.clamp(1.0, 10.0))
    }

    /// Calculate access factor (0-1 scale)
    ///
    /// Based on accesses per day since creation.
    /// - 10+ accesses/day → 1.0
    /// - 1 access/day → 0.5
    /// - <0.3 accesses/day → 0.3 (floor)
    ///
    /// Uses logarithmic scaling: 0.5 + log10(accesses_per_day) * 0.5
    fn access_factor(&self, memory: &MemoryData) -> f32 {
        if memory.access_count == 0 {
            return 0.3; // Minimum score for never-accessed
        }

        let days_since_creation = memory.days_since_creation().max(1.0);
        let accesses_per_day = memory.access_count as f32 / days_since_creation;

        // Logarithmic scale: log10(1) = 0 → 0.5, log10(10) = 1 → 1.0
        let log_scale = 0.5 + (accesses_per_day.max(0.1).log10() * 0.5);
        log_scale.clamp(0.3, 1.0)
    }

    /// Calculate recency factor (0-1 scale)
    ///
    /// Exponential decay with 30-day half-life:
    /// - Just accessed → 1.0
    /// - 30 days → 0.5
    /// - 60 days → 0.25
    /// - 180 days → ~0.016
    fn recency_factor(&self, memory: &MemoryData) -> f32 {
        let days_since_access = memory.days_since_last_access();

        // Exponential decay: score = 0.5^(days / 30)
        let decay = 0.5_f32.powf(days_since_access / 30.0);

        decay.clamp(0.0, 1.0)
    }

    /// Calculate link connectivity factor (0-1 scale)
    ///
    /// Based on number of inbound and outbound links.
    /// Inbound links weighted 2x outbound (being referenced is more important).
    fn link_factor(&self, memory: &MemoryData) -> f32 {
        let inbound = memory.incoming_links_count as f32;
        let outbound = memory.outgoing_links_count as f32;

        // Weight inbound links more heavily
        let weighted_links = (inbound * 2.0 + outbound) / 3.0;

        // Scale: 10+ weighted links → 1.0
        (weighted_links / 10.0).min(1.0)
    }

    /// Check if importance change is significant enough to update
    ///
    /// Avoid thrashing by only updating if change is >= 1.0
    fn is_significant_change(&self, old_importance: f32, new_importance: f32) -> bool {
        (new_importance - old_importance).abs() >= 1.0
    }
}

#[async_trait]
impl EvolutionJob for ImportanceRecalibrator {
    fn name(&self) -> &str {
        "importance_recalibration"
    }

    async fn run(&self, config: &JobConfig) -> Result<JobReport, JobError> {
        let start = Instant::now();
        let mut memories_processed = 0;
        let mut changes_made = 0;
        let mut errors = 0;

        tracing::info!(
            "Starting importance recalibration (batch_size: {})",
            config.batch_size
        );

        // Get active memories from storage
        let memories = self
            .storage
            .list_all_active(Some(config.batch_size))
            .await
            .map_err(|e| JobError::ExecutionError(e.to_string()))?;

        for memory in memories {
            memories_processed += 1;

            // Get access stats for this memory
            let (access_count, last_accessed_at) = self
                .storage
                .get_access_stats(&memory.id)
                .await
                .unwrap_or((0, None));

            // Count incoming links from database
            let incoming_links_count = self
                .storage
                .count_incoming_links(&memory.id)
                .await
                .unwrap_or(0);

            // Convert MemoryNote to MemoryData for calculation
            let memory_data = MemoryData {
                id: memory.id.to_string(),
                importance: memory.importance as f32,
                access_count,
                created_at: memory.created_at,
                last_accessed_at,
                incoming_links_count,
                outgoing_links_count: memory.links.len(),
            };

            // Calculate new importance
            let new_importance = match self.calculate_importance(&memory_data) {
                Ok(imp) => imp,
                Err(e) => {
                    tracing::warn!("Failed to calculate importance for {}: {:?}", memory.id, e);
                    errors += 1;
                    continue;
                }
            };

            // Update if change is significant
            if self.is_significant_change(memory.importance as f32, new_importance) {
                match self
                    .storage
                    .update_importance(&memory.id, new_importance)
                    .await
                {
                    Ok(_) => {
                        changes_made += 1;
                        tracing::debug!(
                            "Updated importance for {}: {} -> {}",
                            memory.id,
                            memory.importance,
                            new_importance
                        );
                    }
                    Err(e) => {
                        tracing::warn!("Failed to update importance for {}: {:?}", memory.id, e);
                        errors += 1;
                    }
                }
            }
        }

        tracing::info!(
            "Importance recalibration complete: {} processed, {} updated in {:?}",
            memories_processed,
            changes_made,
            start.elapsed()
        );

        Ok(JobReport {
            memories_processed,
            changes_made,
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

/// Memory data needed for importance calculation
pub struct MemoryData {
    pub id: String,
    pub importance: f32,
    pub access_count: u32,
    pub created_at: DateTime<Utc>,
    pub last_accessed_at: Option<DateTime<Utc>>,
    pub incoming_links_count: usize,
    pub outgoing_links_count: usize,
}

impl MemoryData {
    /// Calculate days since memory was created
    pub fn days_since_creation(&self) -> f32 {
        let now = Utc::now();
        let duration = now.signed_duration_since(self.created_at);
        duration.num_seconds() as f32 / 86400.0
    }

    /// Calculate days since memory was last accessed
    pub fn days_since_last_access(&self) -> f32 {
        let now = Utc::now();
        let last_access = self.last_accessed_at.unwrap_or(self.created_at);
        let duration = now.signed_duration_since(last_access);
        duration.num_seconds() as f32 / 86400.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConnectionMode;
    use chrono::Duration as ChronoDuration;

    fn create_test_memory(
        importance: f32,
        access_count: u32,
        days_old: i64,
        days_since_access: i64,
        incoming_links: usize,
        outgoing_links: usize,
    ) -> MemoryData {
        let now = Utc::now();
        MemoryData {
            id: "test-mem-1".to_string(),
            importance,
            access_count,
            created_at: now - ChronoDuration::days(days_old),
            last_accessed_at: Some(now - ChronoDuration::days(days_since_access)),
            incoming_links_count: incoming_links,
            outgoing_links_count: outgoing_links,
        }
    }

    #[tokio::test]
    async fn test_calculate_importance_high_access() {
        let storage = Arc::new(LibsqlStorage::new(ConnectionMode::InMemory).await.unwrap());
        let recalibrator = ImportanceRecalibrator::new(storage);
        let memory = create_test_memory(5.0, 100, 10, 0, 5, 3);

        let new_importance = recalibrator.calculate_importance(&memory).unwrap();

        // High access + recent → should increase importance
        assert!(new_importance > memory.importance);
        assert!(new_importance <= 10.0);
    }

    #[tokio::test]
    async fn test_calculate_importance_decay() {
        let storage = Arc::new(LibsqlStorage::new(ConnectionMode::InMemory).await.unwrap());
        let recalibrator = ImportanceRecalibrator::new(storage);
        let memory = create_test_memory(8.0, 0, 180, 180, 0, 0);

        let new_importance = recalibrator.calculate_importance(&memory).unwrap();

        // No access + old → should decrease importance
        assert!(new_importance < memory.importance);
        assert!(new_importance >= 1.0);
    }

    #[tokio::test]
    async fn test_access_factor() {
        let storage = Arc::new(LibsqlStorage::new(ConnectionMode::InMemory).await.unwrap());
        let recalibrator = ImportanceRecalibrator::new(storage);

        // High access rate (10 accesses/day)
        let memory_high = create_test_memory(5.0, 100, 10, 0, 0, 0);
        let factor_high = recalibrator.access_factor(&memory_high);
        assert_eq!(factor_high, 1.0);

        // Medium access rate (1 access/day)
        let memory_med = create_test_memory(5.0, 10, 10, 0, 0, 0);
        let factor_med = recalibrator.access_factor(&memory_med);
        assert!((factor_med - 0.5).abs() < 0.1);

        // Low access rate (0.1 accesses/day)
        let memory_low = create_test_memory(5.0, 1, 100, 0, 0, 0);
        let factor_low = recalibrator.access_factor(&memory_low);
        assert_eq!(factor_low, 0.3); // Floor

        // Never accessed
        let memory_none = create_test_memory(5.0, 0, 100, 100, 0, 0);
        let factor_none = recalibrator.access_factor(&memory_none);
        assert_eq!(factor_none, 0.3); // Floor
    }

    #[tokio::test]
    async fn test_recency_factor_exponential_decay() {
        let storage = Arc::new(LibsqlStorage::new(ConnectionMode::InMemory).await.unwrap());
        let recalibrator = ImportanceRecalibrator::new(storage);

        // Just accessed
        let memory_recent = create_test_memory(5.0, 10, 10, 0, 0, 0);
        let factor_recent = recalibrator.recency_factor(&memory_recent);
        assert_eq!(factor_recent, 1.0);

        // 30 days ago (half-life)
        let memory_30d = create_test_memory(5.0, 10, 40, 30, 0, 0);
        let factor_30d = recalibrator.recency_factor(&memory_30d);
        assert!((factor_30d - 0.5).abs() < 0.01);

        // 60 days ago (two half-lives)
        let memory_60d = create_test_memory(5.0, 10, 70, 60, 0, 0);
        let factor_60d = recalibrator.recency_factor(&memory_60d);
        assert!((factor_60d - 0.25).abs() < 0.01);

        // 180 days ago
        let memory_180d = create_test_memory(5.0, 10, 190, 180, 0, 0);
        let factor_180d = recalibrator.recency_factor(&memory_180d);
        assert!(factor_180d < 0.02);
    }

    #[tokio::test]
    async fn test_link_factor() {
        let storage = Arc::new(LibsqlStorage::new(ConnectionMode::InMemory).await.unwrap());
        let recalibrator = ImportanceRecalibrator::new(storage);

        // No links
        let memory_none = create_test_memory(5.0, 10, 10, 0, 0, 0);
        let factor_none = recalibrator.link_factor(&memory_none);
        assert_eq!(factor_none, 0.0);

        // High inbound links (weighted 2x)
        let memory_high_in = create_test_memory(5.0, 10, 10, 0, 10, 0);
        let factor_high_in = recalibrator.link_factor(&memory_high_in);
        assert!((factor_high_in - 0.666).abs() < 0.01);

        // High outbound links
        let memory_high_out = create_test_memory(5.0, 10, 10, 0, 0, 10);
        let factor_high_out = recalibrator.link_factor(&memory_high_out);
        assert!((factor_high_out - 0.333).abs() < 0.01);

        // Balanced links
        let memory_balanced = create_test_memory(5.0, 10, 10, 0, 5, 5);
        let factor_balanced = recalibrator.link_factor(&memory_balanced);
        assert!((factor_balanced - 0.5).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_is_significant_change() {
        let storage = Arc::new(LibsqlStorage::new(ConnectionMode::InMemory).await.unwrap());
        let recalibrator = ImportanceRecalibrator::new(storage);

        assert!(recalibrator.is_significant_change(5.0, 6.5)); // Change of 1.5
        assert!(recalibrator.is_significant_change(7.0, 5.5)); // Change of -1.5
        assert!(!recalibrator.is_significant_change(5.0, 5.8)); // Change of 0.8
        assert!(!recalibrator.is_significant_change(5.0, 5.0)); // No change
    }

    #[test]
    fn test_memory_data_days_since_creation() {
        let memory = create_test_memory(5.0, 10, 30, 10, 0, 0);
        let days = memory.days_since_creation();
        assert!((days - 30.0).abs() < 1.0); // Allow 1-day tolerance
    }

    #[test]
    fn test_memory_data_days_since_access() {
        let memory = create_test_memory(5.0, 10, 30, 10, 0, 0);
        let days = memory.days_since_last_access();
        assert!((days - 10.0).abs() < 1.0); // Allow 1-day tolerance
    }

    #[test]
    fn test_never_accessed_uses_created_at() {
        let now = Utc::now();
        let memory = MemoryData {
            id: "test".to_string(),
            importance: 5.0,
            access_count: 0,
            created_at: now - ChronoDuration::days(30),
            last_accessed_at: None,
            incoming_links_count: 0,
            outgoing_links_count: 0,
        };

        let days = memory.days_since_last_access();
        assert!((days - 30.0).abs() < 1.0);
    }
}
