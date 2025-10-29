// Automatic Archival Job
//
// Periodically archives unused memories based on:
// - Never accessed + >180 days old
// - Low importance (<3.0) + >90 days since access
// - Very low importance (<2.0) + >30 days since access
//
// Archival is non-destructive - memories remain searchable with flag.

use super::config::JobConfig;
use super::scheduler::{EvolutionJob, JobError, JobReport};
use crate::storage::libsql::LibsqlStorage;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use std::time::Instant;

/// Archival job
pub struct ArchivalJob {
    storage: Arc<LibsqlStorage>,
}

impl ArchivalJob {
    pub fn new(storage: Arc<LibsqlStorage>) -> Self {
        Self { storage }
    }

    /// Determine if a memory should be archived
    ///
    /// Criteria:
    /// 1. Never accessed AND >180 days old
    /// 2. Importance <3.0 AND >90 days since access
    /// 3. Importance <2.0 AND >30 days since access
    pub fn should_archive(&self, memory: &MemoryData) -> Result<bool, JobError> {
        let days_since_access = memory.days_since_last_access();
        let importance = memory.importance;
        let access_count = memory.access_count;

        // Already archived
        if memory.archived_at.is_some() {
            return Ok(false);
        }

        // Never archive high-importance memories (>= 7.0)
        if importance >= 7.0 {
            return Ok(false);
        }

        // Rule 1: Never accessed and very old
        if access_count == 0 && days_since_access > 180.0 {
            return Ok(true);
        }

        // Rule 2: Low importance and old
        if importance < 3.0 && days_since_access > 90.0 {
            return Ok(true);
        }

        // Rule 3: Very low importance and moderately old
        if importance < 2.0 && days_since_access > 30.0 {
            return Ok(true);
        }

        // Don't archive
        Ok(false)
    }

    /// Get archival reason for logging
    pub fn archival_reason(&self, memory: &MemoryData) -> String {
        let days_since_access = memory.days_since_last_access();
        let importance = memory.importance;
        let access_count = memory.access_count;

        if access_count == 0 && days_since_access > 180.0 {
            format!("Never accessed and {} days old", days_since_access.round())
        } else if importance < 3.0 && days_since_access > 90.0 {
            format!(
                "Low importance ({:.1}) and not accessed for {} days",
                importance,
                days_since_access.round()
            )
        } else if importance < 2.0 && days_since_access > 30.0 {
            format!(
                "Very low importance ({:.1}) and not accessed for {} days",
                importance,
                days_since_access.round()
            )
        } else {
            "Unknown reason".to_string()
        }
    }
}

#[async_trait]
impl EvolutionJob for ArchivalJob {
    fn name(&self) -> &str {
        "archival"
    }

    async fn run(&self, config: &JobConfig) -> Result<JobReport, JobError> {
        let start = Instant::now();
        let mut memories_processed = 0;
        let mut changes_made = 0;
        let mut errors = 0;

        tracing::info!(
            "Starting archival job (batch_size: {})",
            config.batch_size
        );

        // Find archival candidates from storage
        let candidates = self
            .storage
            .find_archival_candidates(config.batch_size)
            .await
            .map_err(|e| JobError::ExecutionError(e.to_string()))?;

        for memory in candidates {
            memories_processed += 1;

            // Get access stats for archival decision
            let (access_count, last_accessed_at) = self
                .storage
                .get_access_stats(&memory.id)
                .await
                .unwrap_or((0, None));

            // Convert MemoryNote to MemoryData for decision
            let memory_data = MemoryData {
                id: memory.id.to_string(),
                importance: memory.importance as f32,
                access_count,
                created_at: memory.created_at,
                last_accessed_at,
                archived_at: if memory.is_archived { Some(memory.updated_at) } else { None },
            };

            // Check if should archive
            if self.should_archive(&memory_data)? {
                let reason = self.archival_reason(&memory_data);
                tracing::info!("Archiving memory {}: {}", memory.id, reason);

                match self.storage.archive_memory_with_timestamp(&memory.id).await {
                    Ok(_) => {
                        changes_made += 1;
                        tracing::debug!("Successfully archived memory {}", memory.id);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to archive memory {}: {:?}", memory.id, e);
                        errors += 1;
                    }
                }
            }
        }

        tracing::info!(
            "Archival complete: {} processed, {} archived in {:?}",
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

/// Memory data needed for archival decision
pub struct MemoryData {
    pub id: String,
    pub importance: f32,
    pub access_count: u32,
    pub created_at: DateTime<Utc>,
    pub last_accessed_at: Option<DateTime<Utc>>,
    pub archived_at: Option<DateTime<Utc>>,
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

    /// Check if memory is archived
    pub fn is_archived(&self) -> bool {
        self.archived_at.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration as ChronoDuration;

    fn create_test_memory(
        importance: f32,
        access_count: u32,
        days_old: i64,
        days_since_access: i64,
        archived: bool,
    ) -> MemoryData {
        let now = Utc::now();
        MemoryData {
            id: "test-mem-1".to_string(),
            importance,
            access_count,
            created_at: now - ChronoDuration::days(days_old),
            last_accessed_at: Some(now - ChronoDuration::days(days_since_access)),
            archived_at: if archived { Some(now) } else { None },
        }
    }

    #[test]
    fn test_should_archive_never_accessed_old() {
        let job = ArchivalJob::new();

        // Never accessed, 200 days old
        let memory = create_test_memory(5.0, 0, 200, 200, false);
        assert!(job.should_archive(&memory).unwrap());

        // Never accessed, only 100 days old (don't archive)
        let memory_recent = create_test_memory(5.0, 0, 100, 100, false);
        assert!(!job.should_archive(&memory_recent).unwrap());
    }

    #[test]
    fn test_should_archive_low_importance() {
        let job = ArchivalJob::new();

        // Low importance (2.5), 100 days since access
        let memory = create_test_memory(2.5, 10, 110, 100, false);
        assert!(job.should_archive(&memory).unwrap());

        // Low importance but recently accessed (don't archive)
        let memory_recent = create_test_memory(2.5, 10, 110, 50, false);
        assert!(!job.should_archive(&memory_recent).unwrap());
    }

    #[test]
    fn test_should_archive_very_low_importance() {
        let job = ArchivalJob::new();

        // Very low importance (1.5), 40 days since access
        let memory = create_test_memory(1.5, 5, 50, 40, false);
        assert!(job.should_archive(&memory).unwrap());

        // Very low importance but recently accessed (don't archive)
        let memory_recent = create_test_memory(1.5, 5, 50, 20, false);
        assert!(!job.should_archive(&memory_recent).unwrap());
    }

    #[test]
    fn test_should_not_archive_high_importance() {
        let job = ArchivalJob::new();

        // High importance, never accessed, very old (don't archive)
        let memory = create_test_memory(9.0, 0, 300, 300, false);
        assert!(!job.should_archive(&memory).unwrap());
    }

    #[test]
    fn test_should_not_archive_already_archived() {
        let job = ArchivalJob::new();

        // Already archived, meets criteria but should not archive again
        let memory = create_test_memory(1.5, 0, 200, 200, true);
        assert!(!job.should_archive(&memory).unwrap());
    }

    #[test]
    fn test_archival_reason_never_accessed() {
        let job = ArchivalJob::new();
        let memory = create_test_memory(5.0, 0, 200, 200, false);

        let reason = job.archival_reason(&memory);
        assert!(reason.contains("Never accessed"));
        assert!(reason.contains("200 days"));
    }

    #[test]
    fn test_archival_reason_low_importance() {
        let job = ArchivalJob::new();
        let memory = create_test_memory(2.5, 10, 110, 100, false);

        let reason = job.archival_reason(&memory);
        assert!(reason.contains("Low importance"));
        assert!(reason.contains("2.5"));
    }

    #[test]
    fn test_archival_reason_very_low_importance() {
        let job = ArchivalJob::new();
        let memory = create_test_memory(1.5, 5, 50, 40, false);

        let reason = job.archival_reason(&memory);
        assert!(reason.contains("Very low importance"));
        assert!(reason.contains("1.5"));
    }

    #[test]
    fn test_memory_data_is_archived() {
        let archived = create_test_memory(5.0, 10, 100, 50, true);
        assert!(archived.is_archived());

        let active = create_test_memory(5.0, 10, 100, 50, false);
        assert!(!active.is_archived());
    }

    #[test]
    fn test_memory_data_days_since_creation() {
        let memory = create_test_memory(5.0, 10, 30, 10, false);
        let days = memory.days_since_creation();
        assert!((days - 30.0).abs() < 1.0); // Allow 1-day tolerance
    }

    #[test]
    fn test_memory_data_days_since_access() {
        let memory = create_test_memory(5.0, 10, 30, 10, false);
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
            archived_at: None,
        };

        let days = memory.days_since_last_access();
        assert!((days - 30.0).abs() < 1.0);
    }

    #[test]
    fn test_boundary_conditions() {
        let job = ArchivalJob::new();

        // Exactly at boundary (should NOT archive)
        let memory_180 = create_test_memory(5.0, 0, 180, 180, false);
        assert!(!job.should_archive(&memory_180).unwrap());

        // Just over boundary (should archive)
        let memory_181 = create_test_memory(5.0, 0, 181, 181, false);
        assert!(job.should_archive(&memory_181).unwrap());

        // Importance exactly at 3.0 (should NOT archive)
        let memory_imp3 = create_test_memory(3.0, 10, 100, 100, false);
        assert!(!job.should_archive(&memory_imp3).unwrap());

        // Importance just below 3.0 (should archive)
        let memory_imp2_9 = create_test_memory(2.9, 10, 100, 100, false);
        assert!(job.should_archive(&memory_imp2_9).unwrap());
    }

    #[test]
    fn test_multiple_criteria_met() {
        let job = ArchivalJob::new();

        // Meets multiple criteria (never accessed + low importance + old)
        let memory = create_test_memory(1.5, 0, 200, 200, false);

        // Should archive due to multiple criteria
        assert!(job.should_archive(&memory).unwrap());

        // Reason should mention never accessed (first matching rule)
        let reason = job.archival_reason(&memory);
        assert!(reason.contains("Never accessed"));
    }
}
