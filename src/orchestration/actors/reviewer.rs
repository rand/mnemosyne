//! Reviewer Actor
//!
//! Responsibilities:
//! - Quality assurance with blocking quality gates
//! - Phase transition validation
//! - Work result review
//! - Test coverage verification
//! - Documentation completeness checks

use crate::error::{MnemosyneError, Result};
use crate::launcher::agents::AgentRole;
use crate::orchestration::events::{AgentEvent, EventPersistence};
use crate::orchestration::messages::{OrchestratorMessage, ReviewerMessage, WorkResult};
use crate::orchestration::state::{Phase, WorkItemId, WorkItem};
use crate::storage::StorageBackend;
use crate::types::Namespace;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::sync::Arc;

#[cfg(feature = "python")]
use crate::python_bindings::collect_implementation_from_memories;
#[cfg(feature = "python")]
use pyo3::prelude::*;
#[cfg(feature = "python")]
use std::collections::HashMap;

/// Quality gates that must pass (8 total: 5 existing + 3 pillars)
#[derive(Debug, Clone)]
#[derive(Default)]
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

    /// LLM-generated improvement guidance for retry (if review failed)
    pub improvement_guidance: Option<String>,
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

    /// Optional Python ReviewerAgent for LLM-based semantic validation
    #[cfg(feature = "python")]
    py_reviewer: Option<std::sync::Arc<PyObject>>,

    /// Flag to enable/disable LLM validation (false = fallback to pattern matching only)
    #[cfg(feature = "python")]
    llm_validation_enabled: bool,
}

impl ReviewerState {
    pub fn new(storage: Arc<dyn StorageBackend>, namespace: Namespace) -> Self {
        Self {
            events: EventPersistence::new(storage.clone(), namespace),
            storage,
            orchestrator: None,
            quality_results: std::collections::HashMap::new(),
            #[cfg(feature = "python")]
            py_reviewer: None,
            #[cfg(feature = "python")]
            llm_validation_enabled: false,
        }
    }

    pub fn register_orchestrator(&mut self, orchestrator: ActorRef<OrchestratorMessage>) {
        self.orchestrator = Some(orchestrator);
    }

    /// Register Python ReviewerAgent for LLM-based validation
    #[cfg(feature = "python")]
    pub fn register_py_reviewer(&mut self, py_reviewer: std::sync::Arc<PyObject>) {
        self.py_reviewer = Some(py_reviewer);
        self.llm_validation_enabled = true;
        tracing::info!("Python LLM reviewer registered, semantic validation enabled");
    }

    /// Disable LLM validation (fallback to pattern matching only)
    #[cfg(feature = "python")]
    pub fn disable_llm_validation(&mut self) {
        self.llm_validation_enabled = false;
        tracing::info!("LLM validation disabled, using pattern matching only");
    }
}

/// Reviewer actor implementation
pub struct ReviewerActor {
    #[allow(dead_code)]
    storage: Arc<dyn StorageBackend>,
    #[allow(dead_code)]
    namespace: Namespace,
}

impl ReviewerActor {
    pub fn new(storage: Arc<dyn StorageBackend>, namespace: Namespace) -> Self {
        Self { storage, namespace }
    }

    /// Review work item results with three-pillar validation and LLM semantic analysis
    async fn review_work(
        state: &mut ReviewerState,
        item_id: WorkItemId,
        result: WorkResult,
        work_item: WorkItem,
    ) -> Result<ReviewFeedback> {
        tracing::info!("Reviewing work: {:?}", item_id);

        // Perform quality checks
        let mut gates = QualityGates::default();
        let mut all_issues = Vec::new();

        // Existing gates with LLM enhancement
        let (intent_passed, intent_issues) =
            Self::verify_intent_satisfaction(state, &result, &work_item).await?;
        gates.intent_satisfied = intent_passed;
        all_issues.extend(intent_issues);

        gates.documentation_complete = !result.memory_ids.is_empty();
        gates.tests_passing = Self::verify_tests(state, &result).await?;
        gates.no_anti_patterns = Self::check_anti_patterns(state, &result).await?;
        gates.constraints_maintained = Self::verify_constraints(state, &result).await?;

        // Three-pillar validation with LLM enhancement
        let (completeness_passed, completeness_issues) =
            Self::verify_completeness(state, &result, &work_item).await?;
        gates.completeness = completeness_passed;
        all_issues.extend(completeness_issues);

        let (correctness_passed, correctness_issues) =
            Self::verify_correctness(state, &result, &work_item).await?;
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

        // Generate improvement guidance if review failed
        #[cfg(feature = "python")]
        let improvement_guidance = if !passed && state.llm_validation_enabled && state.py_reviewer.is_some() {
            tracing::info!("Generating LLM improvement guidance for failed review");

            match Self::generate_improvement_guidance(state, &gates, &all_issues, &work_item, &result).await {
                Ok(guidance) => {
                    tracing::info!("Generated improvement guidance ({} chars)", guidance.len());
                    Some(guidance)
                }
                Err(e) => {
                    tracing::warn!("Failed to generate improvement guidance: {:?}", e);
                    None
                }
            }
        } else {
            None
        };

        #[cfg(not(feature = "python"))]
        let improvement_guidance: Option<String> = None;

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
            improvement_guidance,
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

    /// Verify intent satisfaction: Does implementation match original requirements?
    ///
    /// Enhanced with LLM semantic validation when available.
    /// Falls back to basic success check if LLM unavailable.
    async fn verify_intent_satisfaction(
        state: &ReviewerState,
        result: &WorkResult,
        #[allow(unused_variables)] work_item: &WorkItem,
    ) -> Result<(bool, Vec<String>)> {
        let mut issues = Vec::new();

        // Basic check: if work failed, intent not satisfied
        if !result.success {
            issues.push("Work execution failed, intent not satisfied".to_string());
            return Ok((false, issues));
        }

        // LLM semantic validation if enabled
        #[cfg(feature = "python")]
        if state.llm_validation_enabled && state.py_reviewer.is_some() {
            tracing::debug!("Using LLM for semantic intent validation");

            // Collect implementation content from execution memories
            let implementation = collect_implementation_from_memories(
                &state.storage,
                &result.memory_ids
            ).await?;

            // Convert memory IDs to strings for Python
            let memory_id_strings: Vec<String> = result.memory_ids
                .iter()
                .map(|id| id.to_string())
                .collect();

            // Call Python LLM validator
            match Python::with_gil(|py| -> PyResult<(bool, Vec<String>)> {
                let py_reviewer = state.py_reviewer.as_ref().unwrap();

                let result = py_reviewer.call_method1(
                    py,
                    "semantic_intent_check",
                    (
                        work_item.original_intent.clone(),
                        implementation.clone(),
                        memory_id_strings.clone(),
                    ),
                )?;

                result.extract(py)
            }) {
                Ok((passed, llm_issues)) => {
                    if !passed {
                        tracing::warn!(
                            "LLM semantic validation failed with {} issues",
                            llm_issues.len()
                        );
                        issues.extend(llm_issues);
                    } else {
                        tracing::info!("LLM semantic validation: intent satisfied");
                    }
                    return Ok((passed && issues.is_empty(), issues));
                }
                Err(e) => {
                    tracing::warn!(
                        "LLM semantic validation error (falling back to pattern matching): {:?}",
                        e
                    );
                    // Fall through to pattern matching
                }
            }
        }

        // Fallback: pattern matching validation
        tracing::debug!("Using pattern matching for intent validation");

        // Check for explicit "not satisfied" markers in memories
        for memory_id in &result.memory_ids {
            match state.storage.get_memory(*memory_id).await {
                Ok(memory) => {
                    let content_upper = memory.content.to_uppercase();

                    if content_upper.contains("INTENT NOT SATISFIED")
                        || content_upper.contains("REQUIREMENTS NOT MET")
                        || content_upper.contains("PARTIALLY IMPLEMENTED") {
                        issues.push(format!(
                            "Intent satisfaction issue in memory {}: partial or incomplete",
                            memory_id
                        ));
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to retrieve memory {} for intent check: {:?}",
                        memory_id,
                        e
                    );
                }
            }
        }

        Ok((issues.is_empty(), issues))
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
    ///
    /// Enhanced with LLM semantic validation when available.
    async fn verify_completeness(
        state: &ReviewerState,
        result: &WorkResult,
        #[allow(unused_variables)] work_item: &WorkItem,
    ) -> Result<(bool, Vec<String>)> {
        let mut issues = Vec::new();

        // Pattern matching check for incomplete markers
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

        // LLM semantic validation if enabled
        #[cfg(feature = "python")]
        if state.llm_validation_enabled && state.py_reviewer.is_some() {
            tracing::debug!("Using LLM for semantic completeness validation");

            // Collect implementation content
            let implementation = collect_implementation_from_memories(
                &state.storage,
                &result.memory_ids
            ).await?;

            // Use explicit requirements if available, otherwise use original intent
            let requirements = if !work_item.requirements.is_empty() {
                work_item.requirements.clone()
            } else {
                vec![work_item.original_intent.clone()]
            };

            // Convert memory IDs to strings
            let memory_id_strings: Vec<String> = result.memory_ids
                .iter()
                .map(|id| id.to_string())
                .collect();

            // Call Python LLM validator
            match Python::with_gil(|py| -> PyResult<(bool, Vec<String>)> {
                let py_reviewer = state.py_reviewer.as_ref().unwrap();

                let result = py_reviewer.call_method1(
                    py,
                    "semantic_completeness_check",
                    (
                        requirements.clone(),
                        implementation.clone(),
                        memory_id_strings.clone(),
                    ),
                )?;

                result.extract(py)
            }) {
                Ok((passed, llm_issues)) => {
                    if !passed {
                        tracing::warn!(
                            "LLM completeness validation failed with {} issues",
                            llm_issues.len()
                        );
                        issues.extend(llm_issues);
                    } else {
                        tracing::info!("LLM completeness validation passed");
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "LLM completeness validation error (continuing with pattern matching): {:?}",
                        e
                    );
                    // Continue with pattern matching results
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
    ///
    /// Enhanced with LLM semantic validation when available.
    async fn verify_correctness(
        state: &ReviewerState,
        result: &WorkResult,
        #[allow(unused_variables)] work_item: &WorkItem,
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

        // Pattern matching check for error indicators
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

        // LLM semantic validation if enabled
        #[cfg(feature = "python")]
        if state.llm_validation_enabled && state.py_reviewer.is_some() {
            tracing::debug!("Using LLM for semantic correctness validation");

            // Collect implementation content
            let implementation = collect_implementation_from_memories(
                &state.storage,
                &result.memory_ids
            ).await?;

            // Build test results JSON (empty if no test info available)
            let test_results_json = String::new(); // TODO: Extract test results from execution memories

            // Convert memory IDs to strings
            let memory_id_strings: Vec<String> = result.memory_ids
                .iter()
                .map(|id| id.to_string())
                .collect();

            // Call Python LLM validator
            match Python::with_gil(|py| -> PyResult<(bool, Vec<String>)> {
                let py_reviewer = state.py_reviewer.as_ref().unwrap();

                let result = py_reviewer.call_method1(
                    py,
                    "semantic_correctness_check",
                    (
                        implementation.clone(),
                        test_results_json.clone(),
                        memory_id_strings.clone(),
                    ),
                )?;

                result.extract(py)
            }) {
                Ok((passed, llm_issues)) => {
                    if !passed {
                        tracing::warn!(
                            "LLM correctness validation failed with {} issues",
                            llm_issues.len()
                        );
                        issues.extend(llm_issues);
                    } else {
                        tracing::info!("LLM correctness validation passed");
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "LLM correctness validation error (continuing with pattern matching): {:?}",
                        e
                    );
                    // Continue with pattern matching results
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

    /// Generate LLM-powered improvement guidance for failed reviews
    ///
    /// Uses Claude to create detailed, actionable guidance for retry.
    #[cfg(feature = "python")]
    async fn generate_improvement_guidance(
        state: &ReviewerState,
        gates: &QualityGates,
        issues: &[String],
        work_item: &WorkItem,
        result: &WorkResult,
    ) -> Result<String> {
        // Build failed gates map
        let mut failed_gates = HashMap::new();
        failed_gates.insert("intent_satisfied".to_string(), gates.intent_satisfied);
        failed_gates.insert("tests_passing".to_string(), gates.tests_passing);
        failed_gates.insert("documentation_complete".to_string(), gates.documentation_complete);
        failed_gates.insert("no_anti_patterns".to_string(), gates.no_anti_patterns);
        failed_gates.insert("constraints_maintained".to_string(), gates.constraints_maintained);
        failed_gates.insert("completeness".to_string(), gates.completeness);
        failed_gates.insert("correctness".to_string(), gates.correctness);
        failed_gates.insert("principled_implementation".to_string(), gates.principled_implementation);

        // Convert memory IDs to strings
        let memory_id_strings: Vec<String> = result.memory_ids
            .iter()
            .map(|id| id.to_string())
            .collect();

        // Call Python LLM validator
        let guidance = Python::with_gil(|py| -> PyResult<String> {
            let py_reviewer = state.py_reviewer.as_ref().unwrap();

            let result = py_reviewer.call_method1(
                py,
                "generate_improvement_guidance",
                (
                    failed_gates.clone(),
                    issues.to_vec(),
                    work_item.original_intent.clone(),
                    memory_id_strings.clone(),
                ),
            )?;

            result.extract(py)
        })
        .map_err(|e| {
            MnemosyneError::ValidationError(format!(
                "Failed to generate improvement guidance via LLM: {}",
                e
            ))
        })?;

        Ok(guidance)
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

        // NOTE: Complete work item validation requires work queue state
        // Current architecture uses fire-and-forget cast, not call/response
        // The Orchestrator should check work completion before requesting validation
        // This validation focuses on phase transition logic correctness
        let approved = from.can_transition_to(&to);

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
        tracing::debug!("Reviewer actor starting");
        let (storage, namespace) = args;
        Ok(ReviewerState::new(storage, namespace))
    }

    async fn post_start(
        &self,
        myself: ActorRef<Self::Msg>,
        _state: &mut Self::State,
    ) -> std::result::Result<(), ActorProcessingErr> {
        tracing::debug!("Reviewer actor started: {:?}", myself.get_id());
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
                tracing::debug!("Reviewer initialized");
            }
            ReviewerMessage::RegisterOrchestrator(orchestrator_ref) => {
                tracing::debug!("Registering orchestrator reference with Reviewer");
                state.orchestrator = Some(orchestrator_ref);
            }
            #[cfg(feature = "python")]
            ReviewerMessage::RegisterPythonReviewer { py_reviewer } => {
                tracing::info!("Registering Python reviewer for LLM validation");
                state.register_py_reviewer(py_reviewer);
                state.llm_validation_enabled = true;
                tracing::info!("LLM validation enabled");
            }
            ReviewerMessage::ReviewWork {
                item_id,
                result,
                work_item,
            } => {
                let feedback = Self::review_work(state, item_id.clone(), result, work_item.clone())
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
                            improvement_guidance: feedback.improvement_guidance.clone(),
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
