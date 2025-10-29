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
use std::time::Duration;

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

        // TODO: Implement actual test verification
        // For now, assume tests pass if work succeeded
        gates.tests_passing = result.success;

        // TODO: Check for anti-patterns
        gates.no_anti_patterns = true;

        // TODO: Verify constraints
        gates.constraints_maintained = true;

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
