//! Evolution System Integration
//!
//! Allows evolution jobs (consolidation, archival, importance recalibration)
//! to be executed through the orchestration engine as work items.

use crate::error::Result;
use crate::evolution::{EvolutionJob, JobConfig};
use crate::launcher::agents::AgentRole;
use crate::orchestration::messages::{OrchestratorMessage, WorkResult};
use crate::orchestration::state::{Phase, WorkItem, WorkItemId};
use ractor::ActorRef;
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, info};

/// Integration layer between evolution jobs and orchestration
pub struct EvolutionIntegration {
    orchestrator: ActorRef<OrchestratorMessage>,
}

impl EvolutionIntegration {
    /// Create a new evolution integration
    pub fn new(orchestrator: ActorRef<OrchestratorMessage>) -> Self {
        Self { orchestrator }
    }

    /// Submit an evolution job as an orchestration work item
    pub async fn submit_evolution_job(
        &self,
        job: Arc<dyn EvolutionJob>,
        config: JobConfig,
        priority: u8,
    ) -> Result<WorkItemId> {
        let job_name = job.name().to_string();
        info!("Submitting evolution job '{}' to orchestration", job_name);

        // Create work item for evolution job
        let work_item = WorkItem::new(
            format!("Evolution: {}", job_name),
            AgentRole::Optimizer, // Evolution jobs are handled by Optimizer
            Phase::PromptToSpec,   // Evolution jobs are simple, start at first phase
            priority,
        );

        let item_id = work_item.id.clone();

        // Submit work to orchestrator
        self.orchestrator
            .cast(OrchestratorMessage::SubmitWork(work_item))
            .map_err(|e| crate::error::MnemosyneError::ActorError(e.to_string()))?;

        // Spawn async task to execute the evolution job and report back
        let orchestrator = self.orchestrator.clone();
        let item_id_clone = item_id.clone();

        tokio::spawn(async move {
            debug!("Executing evolution job: {}", job_name);
            let start = std::time::Instant::now();

            // Run the evolution job
            let report_result = job.run(&config).await;
            let duration = start.elapsed();

            // Build work result based on job outcome
            let result = match report_result {
                Ok(report) => {
                    info!(
                        "Evolution job '{}' completed: {} memories processed, {} changes made",
                        job_name, report.memories_processed, report.changes_made
                    );

                    WorkResult::success(item_id_clone.clone(), duration)
                }
                Err(e) => {
                    info!("Evolution job '{}' failed: {}", job_name, e);
                    WorkResult::failure(item_id_clone.clone(), e.to_string(), duration)
                }
            };

            // Notify orchestrator of completion
            let _ = orchestrator.cast(OrchestratorMessage::WorkCompleted {
                item_id: item_id_clone,
                result,
            });
        });

        Ok(item_id)
    }

    /// Check if an evolution job should run (delegates to job's should_run)
    pub async fn should_run_job(job: Arc<dyn EvolutionJob>) -> Result<bool> {
        job.should_run()
            .await
            .map_err(|e| crate::error::MnemosyneError::Other(e.to_string()))
    }

    /// Submit multiple evolution jobs with dependency ordering
    pub async fn submit_evolution_batch(
        &self,
        jobs: Vec<(Arc<dyn EvolutionJob>, JobConfig, u8)>,
    ) -> Result<Vec<WorkItemId>> {
        let mut work_ids = Vec::new();

        for (job, config, priority) in jobs {
            let work_id = self.submit_evolution_job(job, config, priority).await?;
            work_ids.push(work_id);
        }

        Ok(work_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evolution::{JobError, JobReport};
    use crate::orchestration::supervision::SupervisionConfig;
    use crate::orchestration::OrchestrationEngine;
    use crate::storage::libsql::ConnectionMode;
    use crate::types::Namespace;
    use crate::LibsqlStorage;
    use async_trait::async_trait;
    use std::sync::Arc;
    use tempfile::TempDir;

    /// Mock evolution job for testing
    struct MockEvolutionJob {
        name: String,
        should_fail: bool,
    }

    #[async_trait]
    impl EvolutionJob for MockEvolutionJob {
        fn name(&self) -> &str {
            &self.name
        }

        async fn run(&self, _config: &JobConfig) -> std::result::Result<JobReport, JobError> {
            tokio::time::sleep(Duration::from_millis(50)).await;

            if self.should_fail {
                return Err(JobError::ExecutionError("Mock failure".to_string()));
            }

            Ok(JobReport {
                memories_processed: 100,
                changes_made: 10,
                duration: Duration::from_millis(50),
                errors: 0,
                error_message: None,
            })
        }

        async fn should_run(&self) -> std::result::Result<bool, JobError> {
            Ok(true)
        }
    }

    async fn create_test_storage() -> (Arc<LibsqlStorage>, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let storage = Arc::new(
            LibsqlStorage::new_with_validation(
                ConnectionMode::Local(db_path.to_str().unwrap().to_string()),
                true,
            )
            .await
            .expect("Failed to create storage"),
        );

        (storage, temp_dir)
    }

    fn create_test_namespace() -> Namespace {
        Namespace::Session {
            project: "test".to_string(),
            session_id: format!("test-{}", uuid::Uuid::new_v4()),
        }
    }

    #[tokio::test]
    async fn test_submit_evolution_job() {
        let (storage, _temp) = create_test_storage().await;
        let namespace = create_test_namespace();

        let config = SupervisionConfig::default();
        let mut engine = OrchestrationEngine::new_with_namespace(
            storage.clone(),
            config,
            namespace,
        )
        .await
        .expect("Failed to create engine");

        engine.start().await.expect("Failed to start");

        let orchestrator = engine.orchestrator().clone();
        let integration = EvolutionIntegration::new(orchestrator);

        let job = Arc::new(MockEvolutionJob {
            name: "test_job".to_string(),
            should_fail: false,
        });

        let job_config = JobConfig {
            enabled: true,
            interval: Duration::from_secs(3600),
            batch_size: 100,
            max_duration: Duration::from_secs(60),
        };

        let work_id = integration
            .submit_evolution_job(job, job_config, 5)
            .await
            .expect("Failed to submit job");

        assert!(!work_id.to_string().is_empty());

        // Give time for job to complete
        tokio::time::sleep(Duration::from_millis(200)).await;

        engine.stop().await.expect("Failed to stop");
    }

    #[tokio::test]
    async fn test_evolution_job_failure() {
        let (storage, _temp) = create_test_storage().await;
        let namespace = create_test_namespace();

        let config = SupervisionConfig::default();
        let mut engine = OrchestrationEngine::new_with_namespace(
            storage.clone(),
            config,
            namespace,
        )
        .await
        .expect("Failed to create engine");

        engine.start().await.expect("Failed to start");

        let orchestrator = engine.orchestrator().clone();
        let integration = EvolutionIntegration::new(orchestrator);

        let job = Arc::new(MockEvolutionJob {
            name: "failing_job".to_string(),
            should_fail: true,
        });

        let job_config = JobConfig {
            enabled: true,
            interval: Duration::from_secs(3600),
            batch_size: 100,
            max_duration: Duration::from_secs(60),
        };

        let work_id = integration
            .submit_evolution_job(job, job_config, 5)
            .await
            .expect("Failed to submit job");

        assert!(!work_id.to_string().is_empty());

        // Give time for job to fail
        tokio::time::sleep(Duration::from_millis(200)).await;

        engine.stop().await.expect("Failed to stop");
    }

    #[tokio::test]
    async fn test_evolution_batch_submission() {
        let (storage, _temp) = create_test_storage().await;
        let namespace = create_test_namespace();

        let config = SupervisionConfig::default();
        let mut engine = OrchestrationEngine::new_with_namespace(
            storage.clone(),
            config,
            namespace,
        )
        .await
        .expect("Failed to create engine");

        engine.start().await.expect("Failed to start");

        let orchestrator = engine.orchestrator().clone();
        let integration = EvolutionIntegration::new(orchestrator);

        let jobs = vec![
            (
                Arc::new(MockEvolutionJob {
                    name: "job1".to_string(),
                    should_fail: false,
                }) as Arc<dyn EvolutionJob>,
                JobConfig {
                    enabled: true,
                    interval: Duration::from_secs(3600),
                    batch_size: 100,
                    max_duration: Duration::from_secs(60),
                },
                5,
            ),
            (
                Arc::new(MockEvolutionJob {
                    name: "job2".to_string(),
                    should_fail: false,
                }) as Arc<dyn EvolutionJob>,
                JobConfig {
                    enabled: true,
                    interval: Duration::from_secs(3600),
                    batch_size: 100,
                    max_duration: Duration::from_secs(60),
                },
                7,
            ),
        ];

        let work_ids = integration
            .submit_evolution_batch(jobs)
            .await
            .expect("Failed to submit batch");

        assert_eq!(work_ids.len(), 2);

        // Give time for jobs to complete
        tokio::time::sleep(Duration::from_millis(300)).await;

        engine.stop().await.expect("Failed to stop");
    }
}
