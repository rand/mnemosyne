// Background Job Scheduler
//
// Manages periodic execution of evolution jobs with idle detection
// and execution history tracking.

use super::config::{EvolutionConfig, JobConfig};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::time::{sleep, timeout};

#[derive(Error, Debug)]
pub enum JobError {
    #[error("Job execution timed out after {0:?}")]
    Timeout(Duration),

    #[error("Job execution failed: {0}")]
    ExecutionError(String),

    #[error("Storage error: {0}")]
    StorageError(#[from] anyhow::Error),

    #[error("Job is already running")]
    AlreadyRunning,

    #[error("Job configuration invalid: {0}")]
    ConfigError(String),
}

#[derive(Error, Debug)]
pub enum SchedulerError {
    #[error("Scheduler is already running")]
    AlreadyRunning,

    #[error("Storage error: {0}")]
    StorageError(#[from] anyhow::Error),

    #[error("Job error: {0}")]
    JobError(#[from] JobError),
}

/// Report generated after job execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobReport {
    /// Number of memories processed
    pub memories_processed: usize,

    /// Number of changes made
    pub changes_made: usize,

    /// Duration of job execution
    #[serde(with = "serde_duration_millis")]
    pub duration: Duration,

    /// Number of errors encountered
    pub errors: usize,

    /// Optional error message if job failed
    pub error_message: Option<String>,
}

// Custom serde module for Duration (serialize/deserialize as milliseconds)
mod serde_duration_millis {
    use serde::{Deserialize, Deserializer, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_u64(duration.as_millis() as u64)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

/// Trait for evolution jobs
#[async_trait]
pub trait EvolutionJob: Send + Sync {
    /// Job name (for logging and tracking)
    fn name(&self) -> &str;

    /// Run the job with given configuration
    async fn run(&self, config: &JobConfig) -> Result<JobReport, JobError>;

    /// Check if job should run now (based on last run time)
    async fn should_run(&self) -> Result<bool, JobError>;
}

/// Job execution record for tracking history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRun {
    pub id: String,
    pub job_name: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: JobStatus,
    pub report: Option<JobReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobStatus {
    Running,
    Success,
    Failed,
    Timeout,
}

/// Background job scheduler
pub struct BackgroundScheduler {
    config: EvolutionConfig,
    jobs: Vec<Arc<dyn EvolutionJob>>,
    running: Arc<AtomicBool>,
    // Note: In a full implementation, this would hold:
    // - storage: Arc<LibSqlStorage> for tracking job runs
    // - llm: Arc<LlmService> for consolidation decisions
    // For now, we'll use a simplified structure
}

impl BackgroundScheduler {
    /// Create a new scheduler with configuration and jobs
    pub fn new(config: EvolutionConfig) -> Self {
        Self {
            config,
            jobs: Vec::new(),
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Register a job with the scheduler
    pub fn register_job(&mut self, job: Arc<dyn EvolutionJob>) {
        self.jobs.push(job);
    }

    /// Start the scheduler (runs until stopped)
    pub async fn start(&self) -> Result<(), SchedulerError> {
        if self.running.swap(true, Ordering::SeqCst) {
            return Err(SchedulerError::AlreadyRunning);
        }

        tracing::info!("Starting background evolution scheduler");

        loop {
            if !self.running.load(Ordering::SeqCst) {
                tracing::info!("Stopping background evolution scheduler");
                break;
            }

            // Check if system is idle before running jobs
            if !self.is_idle().await? {
                tracing::debug!("System not idle, waiting before running jobs");
                sleep(Duration::from_secs(60)).await;
                continue;
            }

            // Run jobs that are due
            for job in &self.jobs {
                if !self.running.load(Ordering::SeqCst) {
                    break;
                }

                match job.should_run().await {
                    Ok(true) => {
                        tracing::info!("Running evolution job: {}", job.name());
                        if let Err(e) = self.run_job(job.as_ref()).await {
                            tracing::error!("Job {} failed: {}", job.name(), e);
                        }
                    }
                    Ok(false) => {
                        tracing::debug!("Job {} not due yet", job.name());
                    }
                    Err(e) => {
                        tracing::error!("Failed to check if job {} should run: {}", job.name(), e);
                    }
                }
            }

            // Wait before checking again (5 minutes)
            sleep(Duration::from_secs(300)).await;
        }

        Ok(())
    }

    /// Stop the scheduler
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Run a specific job with timeout
    async fn run_job(&self, job: &dyn EvolutionJob) -> Result<JobReport, SchedulerError> {
        let job_name = job.name();
        let job_config = self.get_job_config(job_name)?;

        let start_time = Utc::now();
        let job_id = uuid::Uuid::new_v4().to_string();

        tracing::info!("Starting job {} (id: {})", job_name, job_id);

        // Run job with timeout
        let result = timeout(job_config.max_duration, job.run(&job_config)).await;

        let (status, report) = match result {
            Ok(Ok(report)) => {
                tracing::info!(
                    "Job {} completed successfully: {} changes in {:?}",
                    job_name,
                    report.changes_made,
                    report.duration
                );
                (JobStatus::Success, Some(report))
            }
            Ok(Err(e)) => {
                tracing::error!("Job {} failed: {}", job_name, e);
                (
                    JobStatus::Failed,
                    Some(JobReport {
                        memories_processed: 0,
                        changes_made: 0,
                        duration: start_time
                            .signed_duration_since(Utc::now())
                            .to_std()
                            .unwrap_or_default(),
                        errors: 1,
                        error_message: Some(e.to_string()),
                    }),
                )
            }
            Err(_) => {
                tracing::error!(
                    "Job {} timed out after {:?}",
                    job_name,
                    job_config.max_duration
                );
                (
                    JobStatus::Timeout,
                    Some(JobReport {
                        memories_processed: 0,
                        changes_made: 0,
                        duration: job_config.max_duration,
                        errors: 1,
                        error_message: Some(format!("Timeout after {:?}", job_config.max_duration)),
                    }),
                )
            }
        };

        let job_run = JobRun {
            id: job_id.clone(),
            job_name: job_name.to_string(),
            started_at: start_time,
            completed_at: Some(Utc::now()),
            status: status.clone(),
            report: report.clone(),
        };

        // In full implementation, record job run to database
        self.record_job_run(&job_run).await?;

        report.ok_or_else(|| {
            SchedulerError::JobError(JobError::ExecutionError(
                "Job completed but no report generated".to_string(),
            ))
        })
    }

    /// Get job configuration by job name
    fn get_job_config(&self, job_name: &str) -> Result<JobConfig, SchedulerError> {
        let config = match job_name {
            "consolidation" => &self.config.consolidation,
            "importance_recalibration" => &self.config.importance,
            "link_decay" => &self.config.link_decay,
            "archival" => &self.config.archival,
            // For testing: allow test jobs with default config
            name if name.starts_with("test_") => {
                return Ok(JobConfig {
                    enabled: true,
                    interval: Duration::from_secs(300), // 5 minutes
                    batch_size: 1000,
                    max_duration: Duration::from_secs(300), // 5 minutes
                });
            }
            _ => {
                return Err(SchedulerError::JobError(JobError::ConfigError(format!(
                    "Unknown job name: {}",
                    job_name
                ))))
            }
        };

        if !config.enabled {
            return Err(SchedulerError::JobError(JobError::ConfigError(format!(
                "Job {} is disabled",
                job_name
            ))));
        }

        Ok(config.clone())
    }

    /// Check if system is idle (no active queries in last 5 minutes)
    async fn is_idle(&self) -> Result<bool, SchedulerError> {
        // In full implementation, this would query storage for last query time
        // For now, we'll assume idle if no specific indicator

        // Simplified idle detection: check if it's been 5 minutes since last operation
        // In production, this would check:
        // let last_query = self.storage.get_last_query_time().await?;
        // Ok(last_query < Utc::now() - Duration::from_secs(300))

        Ok(true)
    }

    /// Record job run to database
    async fn record_job_run(&self, _job_run: &JobRun) -> Result<(), SchedulerError> {
        // In full implementation, this would insert into evolution_job_runs table
        // For now, just log
        tracing::debug!(
            "Job run recorded: {} - {:?}",
            _job_run.job_name,
            _job_run.status
        );
        Ok(())
    }

    /// Get history of job runs
    pub async fn get_job_history(
        &self,
        job_name: Option<&str>,
        limit: usize,
    ) -> Result<Vec<JobRun>, SchedulerError> {
        // In full implementation, this would query evolution_job_runs table
        // For now, return empty vec
        tracing::debug!("Getting job history for: {:?} (limit: {})", job_name, limit);
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestJob {
        name: String,
        should_run: bool,
        will_fail: bool,
    }

    #[async_trait]
    impl EvolutionJob for TestJob {
        fn name(&self) -> &str {
            &self.name
        }

        async fn run(&self, _config: &JobConfig) -> Result<JobReport, JobError> {
            if self.will_fail {
                return Err(JobError::ExecutionError("Test failure".to_string()));
            }

            Ok(JobReport {
                memories_processed: 100,
                changes_made: 10,
                duration: Duration::from_millis(500),
                errors: 0,
                error_message: None,
            })
        }

        async fn should_run(&self) -> Result<bool, JobError> {
            Ok(self.should_run)
        }
    }

    #[test]
    fn test_job_report_serialization() {
        let report = JobReport {
            memories_processed: 100,
            changes_made: 10,
            duration: Duration::from_millis(500),
            errors: 0,
            error_message: None,
        };

        let json = serde_json::to_string(&report).unwrap();
        let deserialized: JobReport = serde_json::from_str(&json).unwrap();

        assert_eq!(report.memories_processed, deserialized.memories_processed);
        assert_eq!(report.changes_made, deserialized.changes_made);
    }

    #[tokio::test]
    async fn test_scheduler_creation() {
        let config = EvolutionConfig::default();
        let scheduler = BackgroundScheduler::new(config);
        assert_eq!(scheduler.jobs.len(), 0);
    }

    #[tokio::test]
    async fn test_register_job() {
        let config = EvolutionConfig::default();
        let mut scheduler = BackgroundScheduler::new(config);

        let job = Arc::new(TestJob {
            name: "test_job".to_string(),
            should_run: true,
            will_fail: false,
        });

        scheduler.register_job(job);
        assert_eq!(scheduler.jobs.len(), 1);
    }

    #[tokio::test]
    async fn test_run_successful_job() {
        let config = EvolutionConfig::default();
        let scheduler = BackgroundScheduler::new(config);

        let job = TestJob {
            name: "test_job".to_string(),
            should_run: true,
            will_fail: false,
        };

        let result = scheduler.run_job(&job).await;
        assert!(result.is_ok());

        let report = result.unwrap();
        assert_eq!(report.memories_processed, 100);
        assert_eq!(report.changes_made, 10);
        assert_eq!(report.errors, 0);
    }

    #[tokio::test]
    async fn test_run_failing_job() {
        let config = EvolutionConfig::default();
        let scheduler = BackgroundScheduler::new(config);

        let job = TestJob {
            name: "test_job".to_string(),
            should_run: true,
            will_fail: true,
        };

        let result = scheduler.run_job(&job).await;
        assert!(result.is_ok()); // Returns report even on failure

        let report = result.unwrap();
        assert_eq!(report.errors, 1);
        assert!(report.error_message.is_some());
    }
}
