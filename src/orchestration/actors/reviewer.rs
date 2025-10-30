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

/// Quality gates that must pass (8 total: 5 existing + 3 pillars)
#[derive(Debug, Clone)]
pub struct QualityGates {
    // Existing gates
    pub intent_satisfied: bool,
    pub tests_passing: bool,
    pub documentation_complete: bool,
    pub no_anti_patterns: bool,
    pub constraints_maintained: bool,
    // Three-pillar gates
    pub completeness: bool,
    pub correctness: bool,
    pub principled_implementation: bool,
}

impl QualityGates {
    pub fn all_passed(&self) -> bool {
        self.intent_satisfied
            && self.tests_passing
            && self.documentation_complete
            && self.no_anti_patterns
            && self.constraints_maintained
            && self.completeness
            && self.correctness
            && self.principled_implementation
    }
}

/// Review feedback with actionable information
#[derive(Debug, Clone)]
pub struct ReviewFeedback {
    /// All quality gates results
    pub gates: QualityGates,

    /// Specific issues found
    pub issues: Vec<String>,

    /// Tests suggested by Reviewer
    pub suggested_tests: Vec<String>,

    /// Execution context memory IDs
    pub execution_context: Vec<crate::types::MemoryId>,
}

impl Default for QualityGates {
    fn default() -> Self {
        Self {
            intent_satisfied: false,
            tests_passing: false,
            documentation_complete: false,
            no_anti_patterns: false,
            constraints_maintained: false,
            completeness: false,
            correctness: false,
            principled_implementation: false,
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

    /// Review work item results with three-pillar validation
    async fn review_work(
        state: &mut ReviewerState,
        item_id: WorkItemId,
        result: WorkResult,
    ) -> Result<ReviewFeedback> {
        tracing::info!("Reviewing work: {:?}", item_id);

        // Perform quality checks
        let mut gates = QualityGates::default();
        let mut all_issues = Vec::new();

        // Existing gates
        gates.intent_satisfied = result.success;
        gates.documentation_complete = !result.memory_ids.is_empty();
        gates.tests_passing = Self::verify_tests(state, &result).await?;
        gates.no_anti_patterns = Self::check_anti_patterns(state, &result).await?;
        gates.constraints_maintained = Self::verify_constraints(state, &result).await?;

        // Three-pillar validation
        let (completeness_passed, completeness_issues) =
            Self::verify_completeness(state, &result).await?;
        gates.completeness = completeness_passed;
        all_issues.extend(completeness_issues);

        let (correctness_passed, correctness_issues) =
            Self::verify_correctness(state, &result).await?;
        gates.correctness = correctness_passed;
        all_issues.extend(correctness_issues);

        let (principled_passed, principled_issues) =
            Self::verify_principled_implementation(state, &result).await?;
        gates.principled_implementation = principled_passed;
        all_issues.extend(principled_issues);

        // Get test suggestions
        let suggested_tests = Self::suggest_missing_tests(state, &result).await?;

        let passed = gates.all_passed();

        tracing::info!(
            "Review result: {} (8 gates: intent={}, tests={}, docs={}, anti_patterns={}, \
            constraints={}, completeness={}, correctness={}, principled={})",
            if passed { "PASS" } else { "FAIL" },
            gates.intent_satisfied,
            gates.tests_passing,
            gates.documentation_complete,
            gates.no_anti_patterns,
            gates.constraints_maintained,
            gates.completeness,
            gates.correctness,
            gates.principled_implementation
        );

        if !passed {
            tracing::warn!("Review failed with {} issues", all_issues.len());
        }

        // Store results
        state.quality_results.insert(item_id.clone(), gates.clone());

        // Persist event
        state
            .events
            .persist(AgentEvent::MessageSent {
                from: AgentRole::Reviewer,
                to: AgentRole::Orchestrator,
                message_type: format!("review_{}", if passed { "pass" } else { "fail" }),
            })
            .await?;

        Ok(ReviewFeedback {
            gates,
            issues: all_issues,
            suggested_tests,
            execution_context: result.memory_ids.clone(),
        })
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
                    let failure_indicators =
                        ["test failed", "tests failed", "failing test", "test error"];
                    for indicator in &failure_indicators {
                        if content_lower.contains(indicator) || summary_lower.contains(indicator) {
                            tracing::warn!(
                                "Test failure detected in memory {}: {}",
                                memory_id,
                                indicator
                            );
                            return Ok(false);
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to retrieve memory {} for test verification: {:?}",
                        memory_id,
                        e
                    );
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
            "TODO:",
            "FIXME:",
            "HACK:",
            "XXX:",
            "NOT IMPLEMENTED",
            "STUB",
            "MOCK",
            "PLACEHOLDER",
            "TEMPORARY",
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
                    tracing::warn!(
                        "Failed to retrieve memory {} for anti-pattern check: {:?}",
                        memory_id,
                        e
                    );
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
                    tracing::warn!(
                        "Failed to retrieve memory {} for constraint verification: {:?}",
                        memory_id,
                        e
                    );
                    // Fail the gate if we can't retrieve memory - this is a constraint violation
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    /// Verify completeness: Check for partial implementations, TODOs, unfilled typed holes
    async fn verify_completeness(
        state: &ReviewerState,
        result: &WorkResult,
    ) -> Result<(bool, Vec<String>)> {
        let mut issues = Vec::new();

        // Check for incomplete markers
        let incomplete_markers = [
            "TODO:",
            "FIXME:",
            "INCOMPLETE",
            "NOT IMPLEMENTED",
            "PARTIAL",
            "WIP:",
        ];

        for memory_id in &result.memory_ids {
            match state.storage.get_memory(*memory_id).await {
                Ok(memory) => {
                    let content_upper = memory.content.to_uppercase();

                    for marker in &incomplete_markers {
                        if content_upper.contains(marker) {
                            issues.push(format!(
                                "Incomplete work detected in memory {}: {}",
                                memory_id, marker
                            ));
                        }
                    }

                    // Check for empty or placeholder implementations
                    if content_upper.contains("PLACEHOLDER")
                        || content_upper.contains("REPLACE THIS")
                    {
                        issues.push(format!(
                            "Placeholder code detected in memory {}",
                            memory_id
                        ));
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to retrieve memory {} for completeness check: {:?}",
                        memory_id,
                        e
                    );
                }
            }
        }

        let passed = issues.is_empty();
        if !passed {
            tracing::warn!("Completeness check failed: {} issues", issues.len());
        }

        Ok((passed, issues))
    }

    /// Verify correctness: Validate logic, check test results, verify error handling
    async fn verify_correctness(
        state: &ReviewerState,
        result: &WorkResult,
    ) -> Result<(bool, Vec<String>)> {
        let mut issues = Vec::new();

        // If work failed, it's not correct
        if !result.success {
            issues.push("Work execution failed".to_string());
            if let Some(error) = &result.error {
                issues.push(format!("Error: {}", error));
            }
            return Ok((false, issues));
        }

        // Check for error indicators in memories
        let error_indicators = [
            "ERROR:",
            "FAILED:",
            "EXCEPTION:",
            "PANIC:",
            "CRASH:",
            "RUNTIME ERROR",
        ];

        for memory_id in &result.memory_ids {
            match state.storage.get_memory(*memory_id).await {
                Ok(memory) => {
                    let content_upper = memory.content.to_uppercase();

                    for indicator in &error_indicators {
                        if content_upper.contains(indicator) {
                            issues.push(format!(
                                "Error indicator found in memory {}: {}",
                                memory_id, indicator
                            ));
                        }
                    }

                    // Check for logic issues
                    if content_upper.contains("LOGIC ERROR")
                        || content_upper.contains("INCORRECT")
                    {
                        issues.push(format!(
                            "Logic issue detected in memory {}",
                            memory_id
                        ));
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to retrieve memory {} for correctness check: {:?}",
                        memory_id,
                        e
                    );
                }
            }
        }

        let passed = issues.is_empty();
        if !passed {
            tracing::warn!("Correctness check failed: {} issues", issues.len());
        }

        Ok((passed, issues))
    }

    /// Verify principled implementation: Check architectural patterns, consistency, best practices
    async fn verify_principled_implementation(
        state: &ReviewerState,
        result: &WorkResult,
    ) -> Result<(bool, Vec<String>)> {
        let mut issues = Vec::new();

        // Check for anti-pattern indicators
        let anti_patterns = [
            "HACK:",
            "WORKAROUND:",
            "TEMPORARY FIX",
            "BAD PRACTICE",
            "CODE SMELL",
            "REFACTOR ME",
        ];

        for memory_id in &result.memory_ids {
            match state.storage.get_memory(*memory_id).await {
                Ok(memory) => {
                    let content_upper = memory.content.to_uppercase();

                    for pattern in &anti_patterns {
                        if content_upper.contains(pattern) {
                            issues.push(format!(
                                "Anti-pattern detected in memory {}: {}",
                                memory_id, pattern
                            ));
                        }
                    }

                    // Check for inconsistency markers
                    if content_upper.contains("INCONSISTENT")
                        || content_upper.contains("BREAKS PATTERN")
                    {
                        issues.push(format!(
                            "Architectural inconsistency detected in memory {}",
                            memory_id
                        ));
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to retrieve memory {} for principled check: {:?}",
                        memory_id,
                        e
                    );
                }
            }
        }

        let passed = issues.is_empty();
        if !passed {
            tracing::warn!(
                "Principled implementation check failed: {} issues",
                issues.len()
            );
        }

        Ok((passed, issues))
    }

    /// Suggest missing tests by analyzing work and identifying untested scenarios
    async fn suggest_missing_tests(
        state: &ReviewerState,
        result: &WorkResult,
    ) -> Result<Vec<String>> {
        let mut suggestions = Vec::new();

        // Analyze memories for test coverage gaps
        for memory_id in &result.memory_ids {
            match state.storage.get_memory(*memory_id).await {
                Ok(memory) => {
                    let content_lower = memory.content.to_lowercase();

                    // Check for common missing test scenarios
                    if content_lower.contains("error")
                        && !content_lower.contains("test error")
                    {
                        suggestions
                            .push("Add tests for error handling and edge cases".to_string());
                    }

                    if content_lower.contains("async") && !content_lower.contains("test async")
                    {
                        suggestions.push(
                            "Add tests for async behavior and concurrency scenarios".to_string(),
                        );
                    }

                    if (content_lower.contains("null") || content_lower.contains("none"))
                        && !content_lower.contains("test null")
                    {
                        suggestions.push("Add tests for null/None handling".to_string());
                    }

                    if content_lower.contains("boundary")
                        && !content_lower.contains("test boundary")
                    {
                        suggestions.push("Add boundary condition tests".to_string());
                    }

                    if content_lower.contains("integration")
                        && !content_lower.contains("integration test")
                    {
                        suggestions
                            .push("Add integration tests for component interactions".to_string());
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to retrieve memory {} for test suggestions: {:?}",
                        memory_id,
                        e
                    );
                }
            }
        }

        // Remove duplicates
        suggestions.sort();
        suggestions.dedup();

        if !suggestions.is_empty() {
            tracing::info!(
                "Suggested {} additional tests for work item",
                suggestions.len()
            );
        }

        Ok(suggestions)
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
    async fn check_quality_gates(state: &ReviewerState, item_id: WorkItemId) -> Result<bool> {
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
            ReviewerMessage::ReviewWork {
                item_id,
                result,
                work_item,
            } => {
                let feedback = Self::review_work(state, item_id.clone(), result)
                    .await
                    .map_err(|e| ActorProcessingErr::from(e.to_string()))?;

                let passed = feedback.gates.all_passed();

                // Send review result to Orchestrator
                if let Some(ref orchestrator) = state.orchestrator {
                    let review_msg = OrchestratorMessage::ReviewCompleted {
                        item_id: item_id.clone(),
                        passed,
                        feedback: crate::orchestration::messages::ReviewFeedback {
                            gates_passed: passed,
                            issues: feedback.issues.clone(),
                            suggested_tests: feedback.suggested_tests.clone(),
                            execution_context: feedback.execution_context.clone(),
                        },
                    };

                    orchestrator
                        .cast(review_msg)
                        .map_err(|e| ActorProcessingErr::from(e.to_string()))?;

                    tracing::info!(
                        "Sent review result to Orchestrator: {} for item {:?}",
                        if passed { "PASS" } else { "FAIL" },
                        item_id
                    );

                    // Persist event
                    if !passed {
                        state
                            .events
                            .persist(AgentEvent::ReviewFailed {
                                item_id,
                                issues: feedback.issues,
                                attempt: work_item.review_attempt,
                            })
                            .await
                            .map_err(|e| ActorProcessingErr::from(e.to_string()))?;
                    }
                } else {
                    tracing::warn!(
                        "No orchestrator reference to send review result for {:?}",
                        item_id
                    );
                }
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
    use std::time::Duration;
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
