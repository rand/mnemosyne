//! Agent Message Protocol
//!
//! Defines message types for inter-agent communication using Ractor's
//! priority-based messaging system:
//!
//! - **Signals** (highest): System-level interrupts
//! - **Stop**: Graceful shutdown requests
//! - **Supervision**: Lifecycle management
//! - **User messages** (lowest): Agent-specific work messages

use crate::orchestration::state::{Phase, WorkItem, WorkItemId};
use crate::types::MemoryId;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Messages for the Orchestrator agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrchestratorMessage {
    /// Initialize work queue from stored state
    Initialize,

    /// Register agent references for inter-agent communication
    #[serde(skip)]
    RegisterAgents {
        optimizer: ractor::ActorRef<OptimizerMessage>,
        reviewer: ractor::ActorRef<ReviewerMessage>,
        executor: ractor::ActorRef<ExecutorMessage>,
    },

    /// Submit a new work item to the queue
    SubmitWork(WorkItem),

    /// Work item completed by an agent
    WorkCompleted {
        item_id: WorkItemId,
        result: WorkResult,
    },

    /// Work item failed
    WorkFailed { item_id: WorkItemId, error: String },

    /// Query for ready work items
    GetReadyWork,

    /// Deadlock detected in work queue
    DeadlockDetected { blocked_items: Vec<WorkItemId> },

    /// Context utilization threshold reached
    ContextThresholdReached { current_pct: f32 },

    /// Phase transition requested
    PhaseTransition { from: Phase, to: Phase },

    /// Review completed for work item
    ReviewCompleted {
        item_id: WorkItemId,
        passed: bool,
        feedback: ReviewFeedback,
    },

    /// Context consolidated for work item
    ContextConsolidated {
        item_id: WorkItemId,
        consolidated_memory_id: MemoryId,
        estimated_tokens: usize,
    },
}

/// Messages for the Optimizer agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OptimizerMessage {
    /// Initialize optimizer with current context
    Initialize,

    /// Register orchestrator reference for communication
    #[serde(skip)]
    RegisterOrchestrator(ractor::ActorRef<OrchestratorMessage>),

    /// Discover skills for a task description
    DiscoverSkills {
        task_description: String,
        max_skills: usize,
    },

    /// Load memories relevant to current work
    LoadContextMemories {
        work_item_id: WorkItemId,
        query: String,
    },

    /// Monitor context usage
    MonitorContext,

    /// Compact context (remove non-critical)
    CompactContext { target_pct: f32 },

    /// Checkpoint context at threshold
    CheckpointContext { reason: String },

    /// Consolidate work item context (review feedback + execution memories)
    ConsolidateWorkItemContext {
        item_id: WorkItemId,
        execution_memory_ids: Vec<MemoryId>,
        review_feedback: Vec<String>,
        suggested_tests: Vec<String>,
        review_attempt: u32,
    },

    /// Load optimized context for work item dispatch
    LoadWorkItemContext {
        item_id: WorkItemId,
        work_item: WorkItem,
    },
}

/// Review feedback from Reviewer to Orchestrator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReviewFeedback {
    /// All quality gates results
    pub gates_passed: bool,

    /// Specific issues found
    pub issues: Vec<String>,

    /// Tests suggested by Reviewer
    pub suggested_tests: Vec<String>,

    /// Execution context memory IDs
    pub execution_context: Vec<MemoryId>,
}

/// Messages for the Reviewer agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReviewerMessage {
    /// Initialize reviewer
    Initialize,

    /// Register orchestrator reference for communication
    #[serde(skip)]
    RegisterOrchestrator(ractor::ActorRef<OrchestratorMessage>),

    /// Review work item results (with full work item for context)
    ReviewWork {
        item_id: WorkItemId,
        result: WorkResult,
        work_item: WorkItem,
    },

    /// Validate phase transition
    ValidatePhaseTransition { from: Phase, to: Phase },

    /// Check quality gates
    CheckQualityGates { item_id: WorkItemId },
}

/// Messages for the Executor agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutorMessage {
    /// Initialize executor
    Initialize,

    /// Execute a work item
    ExecuteWork(WorkItem),

    /// Spawn a sub-agent for parallel work
    SpawnSubAgent { work_item: WorkItem },

    /// Sub-agent completed
    SubAgentCompleted {
        item_id: WorkItemId,
        result: WorkResult,
    },

    /// Register orchestrator reference (for sub-agents)
    #[serde(skip)]
    RegisterOrchestrator(ractor::ActorRef<OrchestratorMessage>),
}

/// Generic agent message envelope for unified handling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentMessage {
    Orchestrator(OrchestratorMessage),
    Optimizer(OptimizerMessage),
    Reviewer(ReviewerMessage),
    Executor(ExecutorMessage),
}

/// Result of work item execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkResult {
    /// Work item ID
    pub item_id: WorkItemId,

    /// Success status
    pub success: bool,

    /// Result data (serialized)
    pub data: Option<String>,

    /// Error message if failed
    pub error: Option<String>,

    /// Duration of execution
    pub duration: Duration,

    /// Memory IDs created during execution
    pub memory_ids: Vec<MemoryId>,
}

impl WorkResult {
    /// Create a successful result
    pub fn success(item_id: WorkItemId, duration: Duration) -> Self {
        Self {
            item_id,
            success: true,
            data: None,
            error: None,
            duration,
            memory_ids: Vec::new(),
        }
    }

    /// Create a failed result
    pub fn failure(item_id: WorkItemId, error: String, duration: Duration) -> Self {
        Self {
            item_id,
            success: false,
            data: None,
            error: Some(error),
            duration,
            memory_ids: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_work_result_success() {
        let item_id = WorkItemId::new();
        let result = WorkResult::success(item_id.clone(), Duration::from_secs(1));

        assert!(result.success);
        assert_eq!(result.item_id, item_id);
        assert!(result.error.is_none());
    }

    #[test]
    fn test_work_result_failure() {
        let item_id = WorkItemId::new();
        let result = WorkResult::failure(
            item_id.clone(),
            "Test error".to_string(),
            Duration::from_secs(1),
        );

        assert!(!result.success);
        assert_eq!(result.item_id, item_id);
        assert_eq!(result.error.as_ref().unwrap(), "Test error");
    }
}
