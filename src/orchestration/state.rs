//! Agent State Machine and Work Queue
//!
//! Defines the state machine for work items and agent coordination:
//! - **AgentState**: Individual agent lifecycle states
//! - **WorkItem**: Task with dependencies and phase assignment
//! - **Phase**: Work Plan Protocol phases (1→2→3→4)
//! - **WorkQueue**: Dependency-aware task scheduling

use crate::launcher::agents::AgentRole;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Agent lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentState {
    /// Agent is idle, waiting for work
    Idle,

    /// Agent has work assigned and is ready to start
    Ready,

    /// Agent is actively executing work
    Active,

    /// Agent is waiting for dependencies
    Waiting,

    /// Agent is blocked (deadlock detected)
    Blocked,

    /// Agent is waiting for review approval
    PendingReview,

    /// Agent completed work successfully
    Complete,

    /// Agent encountered an error
    Error,
}

/// Work Plan Protocol phases
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Phase {
    /// Phase 1: Prompt → Spec (clarify requirements)
    PromptToSpec,

    /// Phase 2: Spec → Full Spec (decompose, dependencies)
    SpecToFullSpec,

    /// Phase 3: Full Spec → Plan (execution strategy)
    FullSpecToPlan,

    /// Phase 4: Plan → Artifacts (implementation)
    PlanToArtifacts,

    /// Completed all phases
    Complete,
}

impl Phase {
    /// Get the next phase in the sequence
    pub fn next(&self) -> Option<Phase> {
        match self {
            Phase::PromptToSpec => Some(Phase::SpecToFullSpec),
            Phase::SpecToFullSpec => Some(Phase::FullSpecToPlan),
            Phase::FullSpecToPlan => Some(Phase::PlanToArtifacts),
            Phase::PlanToArtifacts => Some(Phase::Complete),
            Phase::Complete => None,
        }
    }

    /// Check if phase transition is valid
    pub fn can_transition_to(&self, next: &Phase) -> bool {
        self.next().as_ref() == Some(next)
    }
}

/// Unique work item identifier
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkItemId(Uuid);

impl WorkItemId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for WorkItemId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for WorkItemId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for WorkItemId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

/// Work item with dependencies and state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItem {
    /// Unique identifier
    pub id: WorkItemId,

    /// Human-readable description
    pub description: String,

    /// Assigned agent
    pub agent: AgentRole,

    /// Current state
    pub state: AgentState,

    /// Work Plan Protocol phase
    pub phase: Phase,

    /// Priority (0-10, higher = more urgent)
    pub priority: u8,

    /// Dependencies (must complete before this item)
    pub dependencies: Vec<WorkItemId>,

    /// Created timestamp
    pub created_at: DateTime<Utc>,

    /// Started timestamp
    pub started_at: Option<DateTime<Utc>>,

    /// Completed timestamp
    pub completed_at: Option<DateTime<Utc>>,

    /// Error message if failed
    pub error: Option<String>,

    /// Timeout duration (None = no timeout)
    pub timeout: Option<std::time::Duration>,

    /// Assigned git branch for this work item
    pub assigned_branch: Option<String>,

    /// Estimated duration for timeout calculation
    pub estimated_duration: Option<std::time::Duration>,

    /// File scope for work intent
    pub file_scope: Option<Vec<std::path::PathBuf>>,

    /// Review history (persisted, consolidated by Optimizer)
    pub review_feedback: Option<Vec<String>>,

    /// Tests suggested by Reviewer (persisted)
    pub suggested_tests: Option<Vec<String>>,

    /// Number of review attempts (persisted)
    pub review_attempt: u32,

    /// Memory IDs from last execution (for context retrieval)
    pub execution_memory_ids: Vec<crate::types::MemoryId>,

    /// Consolidated context memory ID (created by Optimizer)
    pub consolidated_context_id: Option<crate::types::MemoryId>,

    /// Original work intent (preserved across retries)
    pub original_intent: String,

    /// Estimated context tokens (tracked by Optimizer)
    pub estimated_context_tokens: usize,
}

impl WorkItem {
    /// Create a new work item
    pub fn new(description: String, agent: AgentRole, phase: Phase, priority: u8) -> Self {
        let original_intent = description.clone();
        Self {
            id: WorkItemId::new(),
            description,
            agent,
            state: AgentState::Ready,
            phase,
            priority: priority.min(10),
            dependencies: Vec::new(),
            created_at: Utc::now(),
            started_at: None,
            completed_at: None,
            error: None,
            timeout: Some(std::time::Duration::from_secs(60)), // Default 60s timeout
            assigned_branch: None,
            estimated_duration: None,
            file_scope: None,
            review_feedback: None,
            suggested_tests: None,
            review_attempt: 0,
            execution_memory_ids: Vec::new(),
            consolidated_context_id: None,
            original_intent,
            estimated_context_tokens: 0,
        }
    }

    /// Add a dependency
    pub fn add_dependency(&mut self, dep: WorkItemId) {
        if !self.dependencies.contains(&dep) {
            self.dependencies.push(dep);
        }
    }

    /// Check if all dependencies are satisfied
    pub fn dependencies_satisfied(&self, completed: &HashSet<WorkItemId>) -> bool {
        self.dependencies.iter().all(|dep| completed.contains(dep))
    }

    /// Transition to a new state
    pub fn transition(&mut self, new_state: AgentState) {
        self.state = new_state;

        match new_state {
            AgentState::Active => {
                if self.started_at.is_none() {
                    self.started_at = Some(Utc::now());
                }
            }
            AgentState::Complete | AgentState::Error => {
                if self.completed_at.is_none() {
                    self.completed_at = Some(Utc::now());
                }
            }
            _ => {}
        }
    }

    /// Check if work item has timed out
    pub fn is_timed_out(&self) -> bool {
        if let (Some(started), Some(timeout)) = (self.started_at, self.timeout) {
            let elapsed = Utc::now().signed_duration_since(started);
            elapsed.num_seconds() as u64 > timeout.as_secs()
        } else {
            false
        }
    }
}

/// Work queue with dependency-aware scheduling
pub struct WorkQueue {
    /// All work items
    items: HashMap<WorkItemId, WorkItem>,

    /// Completed work item IDs
    completed: HashSet<WorkItemId>,

    /// Current phase
    current_phase: Phase,
}

impl WorkQueue {
    /// Create a new empty work queue
    pub fn new() -> Self {
        Self {
            items: HashMap::new(),
            completed: HashSet::new(),
            current_phase: Phase::PromptToSpec,
        }
    }

    /// Add a work item
    pub fn add(&mut self, item: WorkItem) {
        self.items.insert(item.id.clone(), item);
    }

    /// Get work item by ID
    pub fn get(&self, id: &WorkItemId) -> Option<&WorkItem> {
        self.items.get(id)
    }

    /// Get mutable work item by ID
    pub fn get_mut(&mut self, id: &WorkItemId) -> Option<&mut WorkItem> {
        self.items.get_mut(id)
    }

    /// Mark work item as completed
    pub fn mark_completed(&mut self, id: &WorkItemId) {
        if let Some(item) = self.items.get_mut(id) {
            item.transition(AgentState::Complete);
        }
        self.completed.insert(id.clone());
    }

    /// Re-enqueue a work item (for review failures)
    ///
    /// This is used when a work item fails review and needs to be retried.
    /// The work item is updated with review feedback and reset to Ready state.
    pub fn re_enqueue(&mut self, item: WorkItem) -> std::result::Result<(), String> {
        if item.state != AgentState::Ready {
            return Err(format!(
                "Cannot re-enqueue work item {:?} in state {:?}, must be Ready",
                item.id, item.state
            ));
        }

        // Update or insert the work item
        self.items.insert(item.id.clone(), item);

        Ok(())
    }

    /// Get all ready work items (dependencies satisfied)
    pub fn get_ready_items(&self) -> Vec<&WorkItem> {
        self.items
            .values()
            .filter(|item| {
                item.state == AgentState::Ready && item.dependencies_satisfied(&self.completed)
            })
            .collect()
    }

    /// Get all active work items
    pub fn get_active_items(&self) -> Vec<&WorkItem> {
        self.items
            .values()
            .filter(|item| item.state == AgentState::Active)
            .collect()
    }

    /// Detect deadlocks (items waiting > timeout with no progress)
    pub fn detect_deadlocks(&self) -> Vec<WorkItemId> {
        self.items
            .values()
            .filter(|item| {
                (item.state == AgentState::Waiting || item.state == AgentState::Active)
                    && item.is_timed_out()
            })
            .map(|item| item.id.clone())
            .collect()
    }

    /// Transition to next phase
    pub fn transition_phase(&mut self, next: Phase) -> Result<(), String> {
        if !self.current_phase.can_transition_to(&next) {
            return Err(format!(
                "Invalid phase transition: {:?} -> {:?}",
                self.current_phase, next
            ));
        }

        self.current_phase = next;
        Ok(())
    }

    /// Get current phase
    pub fn current_phase(&self) -> Phase {
        self.current_phase
    }

    /// Get statistics
    pub fn stats(&self) -> WorkQueueStats {
        WorkQueueStats {
            total_items: self.items.len(),
            ready: self.get_ready_items().len(),
            active: self.get_active_items().len(),
            completed: self.completed.len(),
            blocked: self.detect_deadlocks().len(),
        }
    }
}

impl Default for WorkQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// Work queue statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkQueueStats {
    pub total_items: usize,
    pub ready: usize,
    pub active: usize,
    pub completed: usize,
    pub blocked: usize,
}

/// Thread-safe work queue
pub type SharedWorkQueue = Arc<RwLock<WorkQueue>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_transitions() {
        assert_eq!(Phase::PromptToSpec.next(), Some(Phase::SpecToFullSpec));
        assert_eq!(Phase::SpecToFullSpec.next(), Some(Phase::FullSpecToPlan));
        assert_eq!(Phase::FullSpecToPlan.next(), Some(Phase::PlanToArtifacts));
        assert_eq!(Phase::PlanToArtifacts.next(), Some(Phase::Complete));
        assert_eq!(Phase::Complete.next(), None);
    }

    #[test]
    fn test_work_item_dependencies() {
        let mut item = WorkItem::new(
            "Test".to_string(),
            AgentRole::Executor,
            Phase::PromptToSpec,
            5,
        );

        let dep1 = WorkItemId::new();
        let dep2 = WorkItemId::new();

        item.add_dependency(dep1.clone());
        item.add_dependency(dep2.clone());

        // Not satisfied yet
        let completed = HashSet::new();
        assert!(!item.dependencies_satisfied(&completed));

        // One dependency satisfied
        let mut completed = HashSet::new();
        completed.insert(dep1.clone());
        assert!(!item.dependencies_satisfied(&completed));

        // All dependencies satisfied
        completed.insert(dep2.clone());
        assert!(item.dependencies_satisfied(&completed));
    }

    #[test]
    fn test_work_queue() {
        let mut queue = WorkQueue::new();

        let item1 = WorkItem::new(
            "Task 1".to_string(),
            AgentRole::Executor,
            Phase::PromptToSpec,
            5,
        );
        let item1_id = item1.id.clone();

        queue.add(item1);

        // Should be ready
        assert_eq!(queue.get_ready_items().len(), 1);

        // Mark completed
        queue.mark_completed(&item1_id);
        assert_eq!(queue.get_ready_items().len(), 0);
        assert_eq!(queue.stats().completed, 1);
    }
}
