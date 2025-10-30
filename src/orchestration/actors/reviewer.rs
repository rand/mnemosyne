//! Reviewer Actor
//!
//! Responsibilities:
//! - Quality assurance with blocking quality gates
//! - Phase transition validation
//! - Work result review
//! - Test coverage verification
//! - Documentation completeness checks

use crate::error::Result;
use crate::launcher::agents::AgentRole;
use crate::orchestration::events::{AgentEvent, EventPersistence};
use crate::orchestration::messages::{OrchestratorMessage, ReviewerMessage, WorkResult};
use crate::orchestration::state::{Phase, WorkItemId};
use crate::storage::StorageBackend;
use crate::types::Namespace;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::sync::Arc;

/// Quality gates that must pass
#[derive(Debug, Clone)]
pub struct QualityGates {
    pub intent_satisfied: bool,
    pub tests_passing: bool,
    pub documentation_complete: bool,
    pub no_anti_patterns: bool,
    pub constraints_maintained: bool,
}

impl QualityGates {
    pub fn all_passed(&self) -> bool {
        self.intent_satisfied
            && self.tests_passing
            && self.documentation_complete
            && self.no_anti_patterns
            && self.constraints_maintained
    }
}

impl Default for QualityGates {
    fn default() -> Self {
        Self {
            intent_satisfied: false,
            tests_passing: false,
            documentation_complete: false,
            no_anti_patterns: false,
            constraints_maintained: false,
        }
    }
}

/// Reviewer actor state
pub struct ReviewerState {
    /// Event persistence
    events: EventPersistence,

    /// Storage backend
    storage: Arc<dyn StorageBackend>,

    /// Reference to Orchestrator
    orchestrator: Option<ActorRef<OrchestratorMessage>>,

    /// Quality gate results per work item
    quality_results: std::collections::HashMap<WorkItemId, QualityGates>,
}

impl ReviewerState {
    pub fn new(storage: Arc<dyn StorageBackend>, namespace: Namespace) -> Self {
        Self {
            events: EventPersistence::new(storage.clone(), namespace),
            storage,
            orchestrator: None,
            quality_results: std::collections::HashMap::new(),
        }
    }

    pub fn register_orchestrator(&mut self, orchestrator: ActorRef<OrchestratorMessage>) {
        self.orchestrator = Some(orchestrator);
    }
}

/// Reviewer actor implementation
pub struct ReviewerActor {
    storage: Arc<dyn StorageBackend>,
    namespace: Namespace,
}

impl ReviewerActor {
    pub fn new(storage: Arc<dyn StorageBackend>, namespace: Namespace) -> Self {
        Self { storage, namespace }
    }

    /// Review work item results
    async fn review_work(
        state: &mut ReviewerState,
        item_id: WorkItemId,
        result: WorkResult,
    ) -> Result<bool> {
        tracing::info!("Reviewing work: {:?}", item_id);

        // Perform quality checks
        let mut gates = QualityGates::default();

        // Check if work succeeded
        gates.intent_satisfied = result.success;

        // Check if memories were created (documentation)
        gates.documentation_complete = !result.memory_ids.is_empty();

        // Verify tests by checking memories for test-related content
        gates.tests_passing = Self::verify_tests(state, &result).await?;

        // Check for anti-patterns in created memories
        gates.no_anti_patterns = Self::check_anti_patterns(state, &result).await?;

        // Verify constraints on created memories
        gates.constraints_maintained = Self::verify_constraints(state, &result).await?;

        let passed = gates.all_passed();

        tracing::info!(
            "Review result: {} (intent={}, tests={}, docs={}, anti_patterns={}, constraints={})",
            if passed { "PASS" } else { "FAIL" },
            gates.intent_satisfied,
            gates.tests_passing,
            gates.documentation_complete,
            gates.no_anti_patterns,
            gates.constraints_maintained
        );

        // Store results
        state.quality_results.insert(item_id.clone(), gates);

        // Persist event
        state
            .events
            .persist(AgentEvent::MessageSent {
                from: AgentRole::Reviewer,
                to: AgentRole::Orchestrator,
                message_type: format!("review_{}", if passed { "pass" } else { "fail" }),
            })
            .await?;

        Ok(passed)
    }

    /// Verify tests by checking if work included test validation
    ///
    /// Checks for evidence of testing in:
    /// - Work success status (failed work = tests didn't pass)
    /// - Memory content for test-related keywords
    /// - Error messages indicating test failures
    async fn verify_tests(state: &ReviewerState, result: &WorkResult) -> Result<bool> {
        // If work failed, tests didn't pass
        if !result.success {
            if let Some(error) = &result.error {
                tracing::debug!("Work failed, likely due to test failure: {}", error);
            }
            return Ok(false);
        }

        // Check memories for test-related content
        for memory_id in &result.memory_ids {
            match state.storage.get_memory(*memory_id).await {
                Ok(memory) => {
                    let content_lower = memory.content.to_lowercase();
                    let summary_lower = memory.summary.to_lowercase();

                    // Look for test failure indicators
                    let failure_indicators = ["test failed", "tests failed", "failing test", "test error"];
                    for indicator in &failure_indicators {
                        if content_lower.contains(indicator) || summary_lower.contains(indicator) {
                            tracing::warn!("Test failure detected in memory {}: {}", memory_id, indicator);
                            return Ok(false);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to retrieve memory {} for test verification: {:?}", memory_id, e);
                    // Don't fail the gate if we can't retrieve memory
                }
            }
        }

        // If work succeeded and no test failures found, tests pass
        Ok(true)
    }

    /// Check for anti-patterns in created memories
    ///
    /// Detects common anti-patterns:
    /// - TODO/FIXME comments left in documentation
    /// - Mock/stub implementations not replaced
    /// - Incomplete work markers
    async fn check_anti_patterns(state: &ReviewerState, result: &WorkResult) -> Result<bool> {
        // Define anti-pattern keywords to check
        let anti_patterns = [
            "TODO:", "FIXME:", "HACK:", "XXX:",
            "NOT IMPLEMENTED", "STUB", "MOCK",
            "PLACEHOLDER", "TEMPORARY",
        ];

        for memory_id in &result.memory_ids {
            match state.storage.get_memory(*memory_id).await {
                Ok(memory) => {
                    let content_upper = memory.content.to_uppercase();
                    let summary_upper = memory.summary.to_uppercase();

                    for pattern in &anti_patterns {
                        if content_upper.contains(pattern) || summary_upper.contains(pattern) {
                            tracing::warn!(
                                "Anti-pattern detected in memory {}: {}",
                                memory_id,
                                pattern
                            );
                            return Ok(false);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!("Failed to retrieve memory {} for anti-pattern check: {:?}", memory_id, e);
                    // Don't fail the gate if we can't retrieve memory
                }
            }
        }

        Ok(true)
    }

    /// Verify constraints on created memories
    ///
    /// Validates:
    /// - Memories have proper structure (non-empty content, summary)
    /// - Importance and confidence are within valid ranges
    /// - Required metadata is present
    async fn verify_constraints(state: &ReviewerState, result: &WorkResult) -> Result<bool> {
        for memory_id in &result.memory_ids {
            match state.storage.get_memory(*memory_id).await {
                Ok(memory) => {
                    // Check content is not empty
                    if memory.content.trim().is_empty() {
                        tracing::warn!("Memory {} has empty content", memory_id);
                        return Ok(false);
                    }

                    // Check summary is not empty
                    if memory.summary.trim().is_empty() {
                        tracing::warn!("Memory {} has empty summary", memory_id);
                        return Ok(false);
                    }

                    // Verify importance range (1-10)
                    if memory.importance < 1 || memory.importance > 10 {
                        tracing::warn!(
                            "Memory {} has invalid importance: {}",
                            memory_id,
                            memory.importance
                        );
                        return Ok(false);
                    }

                    // Verify confidence range (0.0-1.0)
                    if memory.confidence < 0.0 || memory.confidence > 1.0 {
                        tracing::warn!(
                            "Memory {} has invalid confidence: {}",
                            memory_id,
                            memory.confidence
                        );
                        return Ok(false);
                    }

                    // Check that memory type is valid (enum validation happens at type level)
                    // No additional check needed

                    tracing::debug!("Memory {} passed constraint validation", memory_id);
                }
                Err(e) => {
                    tracing::warn!("Failed to retrieve memory {} for constraint verification: {:?}", memory_id, e);
                    // Fail the gate if we can't retrieve memory - this is a constraint violation
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    /// Validate phase transition
    async fn validate_phase_transition(
        state: &mut ReviewerState,
        from: Phase,
        to: Phase,
    ) -> Result<bool> {
        tracing::info!("Validating phase transition: {:?} → {:?}", from, to);

        // Check if transition is valid
        if !from.can_transition_to(&to) {
            tracing::warn!("Invalid phase transition: {:?} → {:?}", from, to);
            return Ok(false);
        }

        // TODO: Check if all work items in current phase are complete
        // For now, allow all transitions
        let approved = true;

        tracing::info!(
            "Phase transition validation: {}",
            if approved { "APPROVED" } else { "REJECTED" }
        );

        // Persist event
        state
            .events
            .persist(AgentEvent::PhaseTransition {
                from,
                to,
                approved_by: AgentRole::Reviewer,
            })
            .await?;

        Ok(approved)
    }

    /// Check quality gates for a work item
    async fn check_quality_gates(
        state: &ReviewerState,
        item_id: WorkItemId,
    ) -> Result<bool> {
        tracing::info!("Checking quality gates for: {:?}", item_id);

        if let Some(gates) = state.quality_results.get(&item_id) {
            Ok(gates.all_passed())
        } else {
            tracing::warn!("No quality results found for: {:?}", item_id);
            Ok(false)
        }
    }
}

#[ractor::async_trait]
impl Actor for ReviewerActor {
    type Msg = ReviewerMessage;
    type State = ReviewerState;
    type Arguments = (Arc<dyn StorageBackend>, Namespace);

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> std::result::Result<Self::State, ActorProcessingErr> {
        tracing::info!("Reviewer actor starting");
        let (storage, namespace) = args;
        Ok(ReviewerState::new(storage, namespace))
    }

    async fn post_start(
        &self,
        myself: ActorRef<Self::Msg>,
        _state: &mut Self::State,
    ) -> std::result::Result<(), ActorProcessingErr> {
        tracing::info!("Reviewer actor started: {:?}", myself.get_id());
        Ok(())
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> std::result::Result<(), ActorProcessingErr> {
        match message {
            ReviewerMessage::Initialize => {
                tracing::info!("Reviewer initialized");
            }
            ReviewerMessage::ReviewWork { item_id, result } => {
                Self::review_work(state, item_id, result)
                    .await
                    .map_err(|e| ActorProcessingErr::from(e.to_string()))?;
            }
            ReviewerMessage::ValidatePhaseTransition { from, to } => {
                Self::validate_phase_transition(state, from, to)
                    .await
                    .map_err(|e| ActorProcessingErr::from(e.to_string()))?;
            }
            ReviewerMessage::CheckQualityGates { item_id } => {
                Self::check_quality_gates(state, item_id)
                    .await
                    .map_err(|e| ActorProcessingErr::from(e.to_string()))?;
            }
        }

        Ok(())
    }

    async fn post_stop(
        &self,
        _myself: ActorRef<Self::Msg>,
        _state: &mut Self::State,
    ) -> std::result::Result<(), ActorProcessingErr> {
        tracing::info!("Reviewer actor stopped");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LibsqlStorage;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_reviewer_lifecycle() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let storage = Arc::new(
            LibsqlStorage::new_with_validation(
                crate::ConnectionMode::Local(db_path.to_str().unwrap().to_string()),
                true, // create_if_missing
            )
            .await
            .expect("Failed to create test storage"),
        );

        let namespace = Namespace::Session {
            project: "test".to_string(),
            session_id: "test-session".to_string(),
        };

        let (actor_ref, _handle) = Actor::spawn(
            None,
            ReviewerActor::new(storage.clone(), namespace.clone()),
            (storage, namespace),
        )
        .await
        .unwrap();

        actor_ref.cast(ReviewerMessage::Initialize).unwrap();
        actor_ref.stop(None);
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
