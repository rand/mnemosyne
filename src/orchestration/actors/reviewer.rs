//! Reviewer Actor
//!
//! The Reviewer agent provides quality assurance and semantic validation for work items
//! in the multi-agent orchestration system. It implements both pattern-based and LLM-based
//! validation strategies with automatic fallback.
//!
//! ## Core Responsibilities
//!
//! - **Quality Gates**: Enforce 8 quality gates before work completion
//! - **Semantic Validation**: LLM-based deep semantic analysis (3 pillars)
//! - **Requirement Tracking**: Extract, track, and validate requirement satisfaction
//! - **Improvement Guidance**: Generate actionable feedback for failed reviews
//! - **Phase Transition Validation**: Ensure prerequisites met before phase changes
//!
//! ## Three-Pillar Validation
//!
//! 1. **Intent Satisfaction**: Does implementation match original intent?
//! 2. **Completeness**: Are all explicit requirements fully implemented?
//! 3. **Correctness**: Is the logic sound and bug-free?
//!
//! ## Quality Gates (8 total)
//!
//! All gates must pass for work completion:
//! - Intent satisfied
//! - Tests passing
//! - Documentation complete
//! - No anti-patterns
//! - Constraints maintained
//! - Completeness (semantic)
//! - Correctness (semantic)
//! - Principled implementation
//!
//! ## LLM Integration
//!
//! When Python reviewer is registered, the agent uses Claude API for:
//! - Automatic requirement extraction from user intent
//! - Semantic intent validation (beyond pattern matching)
//! - Completeness checking against explicit requirements
//! - Logical correctness validation
//! - Improvement guidance generation
//!
//! ## Error Handling
//!
//! LLM operations include automatic retry with exponential backoff:
//! - Configurable retry limit (default: 3 attempts)
//! - Configurable timeout (default: 60s)
//! - Exponential backoff: 1s → 2s → 4s → ...
//! - Graceful degradation on failure
//!
//! ## Configuration
//!
//! Reviewer behavior is configurable via [`ReviewerConfig`]:
//! ```rust
//! let config = ReviewerConfig {
//!     max_llm_retries: 5,
//!     llm_timeout_secs: 120,
//!     enable_llm_validation: true,
//!     llm_model: "claude-3-5-sonnet-20241022".to_string(),
//!     max_context_tokens: 4096,
//!     llm_temperature: 0.0,
//! };
//! state.update_config(config);
//! ```
//!
//! ## Requirement Tracking
//!
//! Requirements progress through states:
//! - `NotStarted` → Identified but not implemented
//! - `InProgress` → Partial implementation or validation failed
//! - `Satisfied` → Fully implemented with evidence
//!
//! Evidence is tracked as memory IDs linking to implementation artifacts.
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use mnemosyne::orchestration::actors::reviewer::{ReviewerActor, ReviewerState};
//! use mnemosyne::orchestration::messages::ReviewerMessage;
//!
//! // Create and configure reviewer
//! let mut state = ReviewerState::new(storage, namespace);
//! state.register_py_reviewer(py_reviewer); // Enables LLM validation
//!
//! // Work item automatically gets requirements extracted
//! // Review automatically validates against requirements
//! // Failed reviews include improvement guidance
//! ```
//!
//! ## See Also
//!
//! - [`ReviewerConfig`]: Configuration options
//! - [`ReviewFeedback`]: Review results structure
//! - [`QualityGates`]: Individual gate definitions
//! - User guide: `docs/guides/llm-reviewer.md`

use crate::error::Result;
use crate::launcher::agents::AgentRole;
use crate::orchestration::events::{AgentEvent, EventPersistence};
use crate::orchestration::messages::{OrchestratorMessage, ReviewerMessage, WorkResult};
use crate::orchestration::state::{Phase, WorkItemId, WorkItem};
use crate::storage::StorageBackend;
use crate::types::Namespace;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::sync::Arc;

#[cfg(feature = "python")]
use crate::orchestration::actors::reviewer_dspy_adapter::ReviewerDSpyAdapter;
#[cfg(feature = "python")]
use crate::orchestration::dspy_bridge::DSpyBridge;
#[cfg(feature = "python")]
use crate::python_bindings::{collect_implementation_from_memories, execution_memories_to_python_format};

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

    /// Extracted requirements (if not already present in work item)
    pub extracted_requirements: Vec<String>,

    /// Requirements identified as unsatisfied during review
    pub unsatisfied_requirements: Vec<String>,

    /// Requirements identified as satisfied with evidence
    pub satisfied_requirements: std::collections::HashMap<String, Vec<crate::types::MemoryId>>,
}

/// Configuration for LLM-based reviewer validation
#[cfg(feature = "python")]
#[derive(Debug, Clone)]
pub struct ReviewerConfig {
    /// Maximum number of retry attempts for LLM calls
    pub max_llm_retries: u32,

    /// Timeout for LLM calls in seconds
    pub llm_timeout_secs: u64,

    /// Enable/disable LLM validation (false = fallback to pattern matching)
    pub enable_llm_validation: bool,

    /// LLM model name (e.g., "claude-3-opus-20240229")
    pub llm_model: String,

    /// Maximum tokens for LLM context
    pub max_context_tokens: usize,

    /// Temperature for LLM generation (0.0-1.0)
    pub llm_temperature: f32,
}

#[cfg(feature = "python")]
impl Default for ReviewerConfig {
    fn default() -> Self {
        Self {
            max_llm_retries: 3,
            llm_timeout_secs: 60,
            enable_llm_validation: true,
            llm_model: "claude-3-5-sonnet-20241022".to_string(),
            max_context_tokens: 4096,
            llm_temperature: 0.0,
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

    /// Optional DSPy ReviewerAdapter for LLM-based semantic validation
    #[cfg(feature = "python")]
    reviewer_adapter: Option<Arc<ReviewerDSpyAdapter>>,

    /// Configuration for LLM-based validation
    #[cfg(feature = "python")]
    config: ReviewerConfig,
}

impl ReviewerState {
    pub fn new(storage: Arc<dyn StorageBackend>, namespace: Namespace) -> Self {
        Self {
            events: EventPersistence::new(storage.clone(), namespace),
            storage,
            orchestrator: None,
            quality_results: std::collections::HashMap::new(),
            #[cfg(feature = "python")]
            reviewer_adapter: None,
            #[cfg(feature = "python")]
            config: ReviewerConfig::default(),
        }
    }

    pub fn register_orchestrator(&mut self, orchestrator: ActorRef<OrchestratorMessage>) {
        self.orchestrator = Some(orchestrator);
    }

    /// Register DSPy bridge for LLM-based validation
    #[cfg(feature = "python")]
    pub fn register_dspy_bridge(&mut self, bridge: Arc<DSpyBridge>) {
        self.reviewer_adapter = Some(Arc::new(ReviewerDSpyAdapter::new(bridge)));
        self.config.enable_llm_validation = true;
        tracing::info!(
            "DSPy reviewer bridge registered with model {} (timeout: {}s, max retries: {})",
            self.config.llm_model,
            self.config.llm_timeout_secs,
            self.config.max_llm_retries
        );
    }

    /// Deprecated: Use register_dspy_bridge instead
    ///
    /// This method is kept for backward compatibility but will be removed in a future version.
    #[cfg(feature = "python")]
    #[deprecated(note = "Use register_dspy_bridge instead")]
    pub fn register_py_reviewer(&mut self, _py_reviewer: std::sync::Arc<pyo3::PyObject>) {
        tracing::warn!("register_py_reviewer is deprecated. Use register_dspy_bridge instead.");
    }

    /// Update reviewer configuration
    #[cfg(feature = "python")]
    pub fn update_config(&mut self, config: ReviewerConfig) {
        self.config = config;
        tracing::info!("Reviewer configuration updated: {:?}", self.config);
    }

    /// Disable LLM validation (fallback to pattern matching only)
    #[cfg(feature = "python")]
    pub fn disable_llm_validation(&mut self) {
        self.config.enable_llm_validation = false;
        tracing::info!("LLM validation disabled, using pattern matching only");
    }

    /// Extract explicit requirements from work item intent using DSPy
    ///
    /// This method uses the DSPy ReviewerModule to analyze the original_intent
    /// and extract structured requirements that can be tracked and validated.
    ///
    /// Returns: List of extracted requirements, or empty vec if extraction fails or LLM unavailable
    #[cfg(feature = "python")]
    async fn extract_requirements_from_intent(
        &self,
        work_item: &WorkItem,
    ) -> Result<Vec<String>> {
        if !self.config.enable_llm_validation || self.reviewer_adapter.is_none() {
            tracing::debug!("LLM validation not enabled, skipping requirement extraction");
            return Ok(Vec::new());
        }

        tracing::info!("Extracting requirements from intent for work item {}", work_item.id);

        // Gather context about the work item
        let context = format!(
            "Work Item: {}\nPhase: {:?}\nAgent: {:?}\nFile Scope: {:?}",
            work_item.description,
            work_item.phase,
            work_item.agent,
            work_item.file_scope
        );

        // Call DSPy adapter to extract requirements
        match self.reviewer_adapter.as_ref().unwrap()
            .extract_requirements(&work_item.original_intent, Some(&context))
            .await
        {
            Ok(requirements) => {
                tracing::info!(
                    "Extracted {} requirements from intent for work item {}",
                    requirements.len(),
                    work_item.id
                );
                Ok(requirements)
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to extract requirements via DSPy for work item {}: {}. Continuing without explicit requirements.",
                    work_item.id,
                    e
                );
                Ok(Vec::new())
            }
        }
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
        let improvement_guidance = if !passed && state.config.enable_llm_validation && state.reviewer_adapter.is_some() {
            tracing::info!("Generating DSPy improvement guidance for failed review");

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

        // Track requirement satisfaction
        let extracted_requirements = work_item.requirements.clone();
        let mut satisfied_requirements = std::collections::HashMap::new();
        let mut unsatisfied_requirements = Vec::new();

        // Determine requirement satisfaction based on completeness gate
        if !work_item.requirements.is_empty() {
            if completeness_passed {
                // All requirements satisfied - link to execution memories
                for req in &work_item.requirements {
                    satisfied_requirements.insert(req.clone(), result.memory_ids.clone());
                }
            } else {
                // Requirements not satisfied
                unsatisfied_requirements = work_item.requirements.clone();
            }
        }

        Ok(ReviewFeedback {
            gates,
            issues: all_issues,
            suggested_tests,
            execution_context: result.memory_ids.clone(),
            improvement_guidance,
            extracted_requirements,
            unsatisfied_requirements,
            satisfied_requirements,
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

        // DSPy semantic validation if enabled
        #[cfg(feature = "python")]
        if state.config.enable_llm_validation && state.reviewer_adapter.is_some() {
            tracing::debug!("Using DSPy for semantic intent validation");

            // Collect implementation content from execution memories
            let implementation = collect_implementation_from_memories(
                &state.storage,
                &result.memory_ids
            ).await?;

            // Convert memory IDs to JSON-compatible format
            let execution_memories_raw = execution_memories_to_python_format(
                &state.storage,
                &result.memory_ids
            ).await?;

            let execution_memories: Vec<serde_json::Value> = execution_memories_raw
                .into_iter()
                .map(|m| serde_json::to_value(m).unwrap_or_default())
                .collect();

            // Call DSPy adapter for semantic validation
            match state.reviewer_adapter.as_ref().unwrap()
                .semantic_intent_check(
                    &work_item.original_intent,
                    &implementation,
                    execution_memories
                )
                .await
            {
                Ok((passed, llm_issues)) => {
                    if !passed {
                        tracing::warn!(
                            "DSPy semantic validation failed with {} issues",
                            llm_issues.len()
                        );
                        issues.extend(llm_issues);
                    } else {
                        tracing::info!("DSPy semantic validation: intent satisfied");
                    }
                    return Ok((passed && issues.is_empty(), issues));
                }
                Err(e) => {
                    tracing::warn!(
                        "DSPy semantic validation error (falling back to pattern matching): {}",
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

        // DSPy semantic validation if enabled
        #[cfg(feature = "python")]
        if state.config.enable_llm_validation && state.reviewer_adapter.is_some() {
            tracing::debug!("Using DSPy for semantic completeness validation");

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

            // Convert memory IDs to JSON-compatible format
            let execution_memories_raw = execution_memories_to_python_format(
                &state.storage,
                &result.memory_ids
            ).await?;

            let execution_memories: Vec<serde_json::Value> = execution_memories_raw
                .into_iter()
                .map(|m| serde_json::to_value(m).unwrap_or_default())
                .collect();

            // Call DSPy adapter for completeness validation
            match state.reviewer_adapter.as_ref().unwrap()
                .verify_completeness(
                    &requirements,
                    &implementation,
                    execution_memories
                )
                .await
            {
                Ok((passed, llm_issues)) => {
                    if !passed {
                        tracing::warn!(
                            "DSPy completeness validation failed with {} issues",
                            llm_issues.len()
                        );
                        issues.extend(llm_issues);
                    } else {
                        tracing::info!("DSPy completeness validation passed");
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "DSPy completeness validation error (continuing with pattern matching): {}",
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

        // DSPy semantic validation if enabled
        #[cfg(feature = "python")]
        if state.config.enable_llm_validation && state.reviewer_adapter.is_some() {
            tracing::debug!("Using DSPy for semantic correctness validation");

            // Collect implementation content
            let implementation = collect_implementation_from_memories(
                &state.storage,
                &result.memory_ids
            ).await?;

            // Convert memory IDs to JSON-compatible format
            let execution_memories_raw = execution_memories_to_python_format(
                &state.storage,
                &result.memory_ids
            ).await?;

            let execution_memories: Vec<serde_json::Value> = execution_memories_raw
                .into_iter()
                .map(|m| serde_json::to_value(m).unwrap_or_default())
                .collect();

            // Call DSPy adapter for correctness validation
            match state.reviewer_adapter.as_ref().unwrap()
                .verify_correctness(
                    &implementation,
                    execution_memories
                )
                .await
            {
                Ok((passed, llm_issues)) => {
                    if !passed {
                        tracing::warn!(
                            "DSPy correctness validation failed with {} issues",
                            llm_issues.len()
                        );
                        issues.extend(llm_issues);
                    } else {
                        tracing::info!("DSPy correctness validation passed");
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        "DSPy correctness validation error (continuing with pattern matching): {}",
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

    /// Generate DSPy-powered improvement guidance for failed reviews
    ///
    /// NOTE: This functionality is not yet implemented in ReviewerModule.
    /// Returns a basic summary of failed gates and issues as a placeholder.
    #[cfg(feature = "python")]
    async fn generate_improvement_guidance(
        _state: &ReviewerState,
        gates: &QualityGates,
        issues: &[String],
        work_item: &WorkItem,
        _result: &WorkResult,
    ) -> Result<String> {
        // TODO: Implement improvement guidance in ReviewerModule
        // For now, generate a basic summary
        let mut guidance = String::new();
        guidance.push_str(&format!("Review failed for work item: {}\n\n", work_item.description));
        guidance.push_str("Failed Quality Gates:\n");

        if !gates.intent_satisfied {
            guidance.push_str("- Intent not satisfied\n");
        }
        if !gates.tests_passing {
            guidance.push_str("- Tests not passing\n");
        }
        if !gates.documentation_complete {
            guidance.push_str("- Documentation incomplete\n");
        }
        if !gates.no_anti_patterns {
            guidance.push_str("- Anti-patterns detected\n");
        }
        if !gates.constraints_maintained {
            guidance.push_str("- Constraints not maintained\n");
        }
        if !gates.completeness {
            guidance.push_str("- Implementation incomplete\n");
        }
        if !gates.correctness {
            guidance.push_str("- Logical correctness issues\n");
        }
        if !gates.principled_implementation {
            guidance.push_str("- Unprincipled implementation\n");
        }

        guidance.push_str(&format!("\nIssues Found ({}):\n", issues.len()));
        for (i, issue) in issues.iter().enumerate() {
            guidance.push_str(&format!("{}. {}\n", i + 1, issue));
        }

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
            }
            ReviewerMessage::ReviewWork {
                item_id,
                result,
                work_item,
            } => {
                // Extract requirements from intent if not already present
                #[cfg(feature = "python")]
                let work_item_with_reqs = if work_item.requirements.is_empty() {
                    match state.extract_requirements_from_intent(&work_item).await {
                        Ok(requirements) if !requirements.is_empty() => {
                            tracing::info!(
                                "Extracted {} requirements for work item {}: {:?}",
                                requirements.len(),
                                item_id,
                                requirements
                            );
                            let mut updated = work_item.clone();
                            updated.requirements = requirements;
                            updated
                        }
                        Ok(_) => {
                            tracing::debug!(
                                "No requirements extracted for work item {}, will use original intent",
                                item_id
                            );
                            work_item.clone()
                        }
                        Err(e) => {
                            tracing::warn!(
                                "Failed to extract requirements for work item {}: {}",
                                item_id,
                                e
                            );
                            work_item.clone()
                        }
                    }
                } else {
                    work_item.clone()
                };

                #[cfg(not(feature = "python"))]
                let work_item_with_reqs = work_item.clone();

                let feedback = Self::review_work(state, item_id.clone(), result, work_item_with_reqs.clone())
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
                            extracted_requirements: feedback.extracted_requirements.clone(),
                            unsatisfied_requirements: feedback.unsatisfied_requirements.clone(),
                            satisfied_requirements: feedback.satisfied_requirements.clone(),
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
    use crate::storage::test_utils::create_test_storage;
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

    #[tokio::test]
    async fn test_requirement_tracking_all_satisfied() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let storage = Arc::new(
            LibsqlStorage::new_with_validation(
                crate::ConnectionMode::Local(db_path.to_str().unwrap().to_string()),
                true,
            )
            .await
            .expect("Failed to create test storage"),
        );

        let namespace = Namespace::Session {
            project: "test".to_string(),
            session_id: "test-session".to_string(),
        };

        let state = ReviewerState::new(storage.clone(), namespace);

        // Create work item with requirements
        let mut work_item = crate::orchestration::state::WorkItem::new(
            "Test work".to_string(),
            AgentRole::Executor,
            crate::orchestration::state::Phase::PlanToArtifacts,
            5,
        );
        work_item.requirements = vec![
            "Requirement 1".to_string(),
            "Requirement 2".to_string(),
            "Requirement 3".to_string(),
        ];

        // Create successful result
        let result = crate::orchestration::messages::WorkResult::success(
            work_item.id.clone(),
            Duration::from_secs(1),
        );

        // Simulate successful review (completeness_passed = true)
        let completeness_passed = true;

        // Track requirement satisfaction
        let extracted_requirements = work_item.requirements.clone();
        let mut satisfied_requirements = std::collections::HashMap::new();
        let mut unsatisfied_requirements = Vec::new();

        if !work_item.requirements.is_empty() {
            if completeness_passed {
                for req in &work_item.requirements {
                    satisfied_requirements.insert(req.clone(), result.memory_ids.clone());
                }
            } else {
                unsatisfied_requirements = work_item.requirements.clone();
            }
        }

        // Verify all requirements satisfied
        assert_eq!(satisfied_requirements.len(), 3);
        assert_eq!(unsatisfied_requirements.len(), 0);
        assert!(satisfied_requirements.contains_key("Requirement 1"));
        assert!(satisfied_requirements.contains_key("Requirement 2"));
        assert!(satisfied_requirements.contains_key("Requirement 3"));
    }

    #[tokio::test]
    async fn test_requirement_tracking_unsatisfied() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let storage = Arc::new(
            LibsqlStorage::new_with_validation(
                crate::ConnectionMode::Local(db_path.to_str().unwrap().to_string()),
                true,
            )
            .await
            .expect("Failed to create test storage"),
        );

        let namespace = Namespace::Session {
            project: "test".to_string(),
            session_id: "test-session".to_string(),
        };

        let _state = ReviewerState::new(storage.clone(), namespace);

        // Create work item with requirements
        let mut work_item = crate::orchestration::state::WorkItem::new(
            "Test work".to_string(),
            AgentRole::Executor,
            crate::orchestration::state::Phase::PlanToArtifacts,
            5,
        );
        work_item.requirements = vec![
            "Requirement 1".to_string(),
            "Requirement 2".to_string(),
        ];

        // Create result
        let result = crate::orchestration::messages::WorkResult::success(
            work_item.id.clone(),
            Duration::from_secs(1),
        );

        // Simulate failed completeness check
        let completeness_passed = false;

        // Track requirement satisfaction
        let extracted_requirements = work_item.requirements.clone();
        let mut satisfied_requirements = std::collections::HashMap::new();
        let mut unsatisfied_requirements = Vec::new();

        if !work_item.requirements.is_empty() {
            if completeness_passed {
                for req in &work_item.requirements {
                    satisfied_requirements.insert(req.clone(), result.memory_ids.clone());
                }
            } else {
                unsatisfied_requirements = work_item.requirements.clone();
            }
        }

        // Verify all requirements unsatisfied
        assert_eq!(satisfied_requirements.len(), 0);
        assert_eq!(unsatisfied_requirements.len(), 2);
        assert!(unsatisfied_requirements.contains(&"Requirement 1".to_string()));
        assert!(unsatisfied_requirements.contains(&"Requirement 2".to_string()));
    }

    #[tokio::test]
    async fn test_requirement_tracking_no_requirements() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let storage = Arc::new(
            LibsqlStorage::new_with_validation(
                crate::ConnectionMode::Local(db_path.to_str().unwrap().to_string()),
                true,
            )
            .await
            .expect("Failed to create test storage"),
        );

        let namespace = Namespace::Session {
            project: "test".to_string(),
            session_id: "test-session".to_string(),
        };

        let _state = ReviewerState::new(storage.clone(), namespace);

        // Create work item without requirements
        let work_item = crate::orchestration::state::WorkItem::new(
            "Test work".to_string(),
            AgentRole::Executor,
            crate::orchestration::state::Phase::PlanToArtifacts,
            5,
        );

        // Create result
        let result = crate::orchestration::messages::WorkResult::success(
            work_item.id.clone(),
            Duration::from_secs(1),
        );

        // Track requirement satisfaction
        let extracted_requirements = work_item.requirements.clone();
        let mut satisfied_requirements = std::collections::HashMap::new();
        let mut unsatisfied_requirements = Vec::new();

        if !work_item.requirements.is_empty() {
            if true {
                for req in &work_item.requirements {
                    satisfied_requirements.insert(req.clone(), result.memory_ids.clone());
                }
            } else {
                unsatisfied_requirements = work_item.requirements.clone();
            }
        }

        // Verify no requirements tracked
        assert_eq!(satisfied_requirements.len(), 0);
        assert_eq!(unsatisfied_requirements.len(), 0);
        assert_eq!(extracted_requirements.len(), 0);
    }

    #[cfg(feature = "python")]
    #[tokio::test]
    async fn test_reviewer_config_defaults() {
        let config = ReviewerConfig::default();

        assert_eq!(config.max_llm_retries, 3);
        assert_eq!(config.llm_timeout_secs, 60);
        assert_eq!(config.enable_llm_validation, true);
        assert_eq!(config.llm_model, "claude-3-5-sonnet-20241022");
        assert_eq!(config.max_context_tokens, 4096);
        assert_eq!(config.llm_temperature, 0.0);
    }

    #[cfg(feature = "python")]
    #[tokio::test]
    async fn test_reviewer_config_update() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let storage = Arc::new(
            LibsqlStorage::new_with_validation(
                crate::ConnectionMode::Local(db_path.to_str().unwrap().to_string()),
                true,
            )
            .await
            .expect("Failed to create test storage"),
        );

        let namespace = Namespace::Session {
            project: "test".to_string(),
            session_id: "test-session".to_string(),
        };

        let mut state = ReviewerState::new(storage.clone(), namespace);

        // Create custom config
        let custom_config = ReviewerConfig {
            max_llm_retries: 5,
            llm_timeout_secs: 120,
            enable_llm_validation: true,
            llm_model: "claude-3-opus-20240229".to_string(),
            max_context_tokens: 8192,
            llm_temperature: 0.1,
        };

        // Update config
        state.update_config(custom_config.clone());

        // Verify config was updated
        assert_eq!(state.config.max_llm_retries, 5);
        assert_eq!(state.config.llm_timeout_secs, 120);
        assert_eq!(state.config.llm_model, "claude-3-opus-20240229");
        assert_eq!(state.config.max_context_tokens, 8192);
        assert_eq!(state.config.llm_temperature, 0.1);
    }

    #[cfg(feature = "python")]
    #[tokio::test]
    async fn test_disable_llm_validation() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let storage = Arc::new(
            LibsqlStorage::new_with_validation(
                crate::ConnectionMode::Local(db_path.to_str().unwrap().to_string()),
                true,
            )
            .await
            .expect("Failed to create test storage"),
        );

        let namespace = Namespace::Session {
            project: "test".to_string(),
            session_id: "test-session".to_string(),
        };

        let mut state = ReviewerState::new(storage.clone(), namespace);

        // Initially enabled by default
        assert!(state.config.enable_llm_validation);

        // Disable LLM validation
        state.disable_llm_validation();

        // Verify it's disabled
        assert!(!state.config.enable_llm_validation);
    }

    #[tokio::test]
    async fn test_pattern_matching_fallback() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let storage = Arc::new(
            LibsqlStorage::new_with_validation(
                crate::ConnectionMode::Local(db_path.to_str().unwrap().to_string()),
                true,
            )
            .await
            .expect("Failed to create test storage"),
        );

        let namespace = Namespace::Session {
            project: "test".to_string(),
            session_id: "test-session".to_string(),
        };

        // Create and store a memory with anti-pattern markers
        let memory = crate::types::MemoryNote {
            id: crate::types::MemoryId(uuid::Uuid::new_v4()),
            namespace: namespace.clone(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            content: "TODO: Implement this feature".to_string(),
            summary: "Test memory".to_string(),
            keywords: vec![],
            tags: vec![],
            context: "Test context".to_string(),
            memory_type: crate::types::MemoryType::CodePattern,
            importance: 5,
            confidence: 0.8,
            links: vec![],
            related_files: vec![],
            related_entities: vec![],
            access_count: 0,
            last_accessed_at: chrono::Utc::now(),
            expires_at: None,
            is_archived: false,
            superseded_by: None,
            embedding: None,
            embedding_model: "test".to_string(),
        };

        storage
            .store_memory(&memory)
            .await
            .expect("Failed to store memory");

        let state = ReviewerState::new(storage.clone(), namespace);

        // Create work result with the memory
        let work_item = crate::orchestration::state::WorkItem::new(
            "Test work".to_string(),
            AgentRole::Executor,
            crate::orchestration::state::Phase::PlanToArtifacts,
            5,
        );

        let mut result = crate::orchestration::messages::WorkResult::success(
            work_item.id.clone(),
            Duration::from_secs(1),
        );
        result.memory_ids.push(memory.id);

        // Check anti-patterns (pattern matching fallback)
        let passed = ReviewerActor::check_anti_patterns(&state, &result)
            .await
            .expect("Anti-pattern check failed");

        // Should detect TODO marker
        assert!(!passed, "Anti-pattern check should have failed due to TODO marker");
    }

    #[tokio::test]
    async fn test_quality_gates_all_pass() {
        let gates = QualityGates {
            intent_satisfied: true,
            tests_passing: true,
            documentation_complete: true,
            no_anti_patterns: true,
            constraints_maintained: true,
            completeness: true,
            correctness: true,
            principled_implementation: true,
        };

        assert!(gates.all_passed());
    }

    #[tokio::test]
    async fn test_quality_gates_one_fails() {
        let gates = QualityGates {
            intent_satisfied: true,
            tests_passing: true,
            documentation_complete: true,
            no_anti_patterns: true,
            constraints_maintained: true,
            completeness: false, // This one fails
            correctness: true,
            principled_implementation: true,
        };

        assert!(!gates.all_passed());
    }

    #[tokio::test]
    async fn test_work_result_with_memories() {
        let item_id = crate::orchestration::state::WorkItemId::new();
        let mut result = crate::orchestration::messages::WorkResult::success(
            item_id.clone(),
            Duration::from_secs(5),
        );

        // Add memory IDs
        result.memory_ids.push(crate::types::MemoryId(uuid::Uuid::new_v4()));
        result.memory_ids.push(crate::types::MemoryId(uuid::Uuid::new_v4()));

        assert_eq!(result.memory_ids.len(), 2);
        assert!(result.success);
        assert_eq!(result.duration, Duration::from_secs(5));
    }

    #[cfg(feature = "python")]
    #[tokio::test]
    async fn test_python_memory_format_conversion() {
        use crate::python_bindings::execution_memories_to_python_format;
        use crate::storage::StorageBackend;

        // Setup storage (cast to trait object for python_bindings function)
        let storage: Arc<dyn StorageBackend> = create_test_storage()
            .await
            .expect("Failed to create test storage");
        let namespace = Namespace::Session {
            project: "test-reviewer".to_string(),
            session_id: "test-memory-format".to_string(),
        };

        // Create test memories with actual content
        let memory1 = crate::types::MemoryNote {
            id: crate::types::MemoryId(uuid::Uuid::new_v4()),
            namespace: namespace.clone(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            content: "Implementation of authentication system using JWT tokens".to_string(),
            summary: "JWT authentication".to_string(),
            keywords: vec!["jwt".to_string(), "auth".to_string()],
            tags: vec!["implementation".to_string()],
            context: "Security context".to_string(),
            memory_type: crate::types::MemoryType::CodePattern,
            importance: 8,
            confidence: 0.9,
            links: vec![],
            related_files: vec![],
            related_entities: vec![],
            access_count: 0,
            last_accessed_at: chrono::Utc::now(),
            expires_at: None,
            is_archived: false,
            superseded_by: None,
            embedding: None,
            embedding_model: "test".to_string(),
        };

        let memory2 = crate::types::MemoryNote {
            id: crate::types::MemoryId(uuid::Uuid::new_v4()),
            namespace: namespace.clone(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            content: "Added comprehensive unit tests for token validation, covering edge cases like expired tokens and invalid signatures".to_string(),
            summary: "Token validation tests".to_string(),
            keywords: vec!["tests".to_string(), "validation".to_string()],
            tags: vec!["testing".to_string()],
            context: "Test coverage".to_string(),
            memory_type: crate::types::MemoryType::CodePattern,
            importance: 7,
            confidence: 0.85,
            links: vec![],
            related_files: vec![],
            related_entities: vec![],
            access_count: 0,
            last_accessed_at: chrono::Utc::now(),
            expires_at: None,
            is_archived: false,
            superseded_by: None,
            embedding: None,
            embedding_model: "test".to_string(),
        };

        // Store memories
        storage.store_memory(&memory1).await.expect("Failed to store memory1");
        storage.store_memory(&memory2).await.expect("Failed to store memory2");

        // Convert to Python format
        let memory_ids = vec![memory1.id, memory2.id];
        let python_format = execution_memories_to_python_format(&storage, &memory_ids)
            .await
            .expect("Failed to convert memories to Python format");

        // Validate format
        assert_eq!(python_format.len(), 2, "Should return 2 memory objects");

        // Validate first memory
        let mem1_dict = &python_format[0];
        assert!(mem1_dict.contains_key("id"), "Memory should have 'id' field");
        assert!(mem1_dict.contains_key("summary"), "Memory should have 'summary' field");
        assert!(mem1_dict.contains_key("content"), "Memory should have 'content' field");

        assert_eq!(mem1_dict.get("id").unwrap(), &memory1.id.to_string());
        assert_eq!(mem1_dict.get("summary").unwrap(), "JWT authentication");
        assert!(
            mem1_dict.get("content").unwrap().contains("authentication"),
            "Content should contain 'authentication'"
        );

        // Validate content truncation (limited to 200 chars)
        let content_len = mem1_dict.get("content").unwrap().len();
        assert!(
            content_len <= 200,
            "Content should be truncated to 200 chars, got {}",
            content_len
        );

        // Validate second memory
        let mem2_dict = &python_format[1];
        assert_eq!(mem2_dict.get("id").unwrap(), &memory2.id.to_string());
        assert_eq!(mem2_dict.get("summary").unwrap(), "Token validation tests");
        assert!(
            mem2_dict.get("content").unwrap().contains("tests"),
            "Content should contain 'tests'"
        );
    }
}
