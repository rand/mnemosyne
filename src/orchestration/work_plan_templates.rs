//! Work Plan Protocol Templates
//!
//! Pre-defined work item templates for each phase of the Work Plan Protocol.
//! These templates bootstrap the orchestration system to execute the 4-phase workflow automatically.

use crate::launcher::agents::AgentRole;
use crate::orchestration::state::{Phase, WorkItem};

/// Create initial work items for session initialization
///
/// This creates work items for Phase 1 (Prompt → Spec) that will:
/// 1. Load project context and memories
/// 2. Discover relevant skills for the domain
/// 3. Wait for user prompt (or process queued prompts)
pub fn create_session_init_work_items() -> Vec<WorkItem> {
    vec![
        WorkItem::new(
            "Load project context and recent memories".to_string(),
            AgentRole::Optimizer,
            Phase::PromptToSpec,
            10, // Highest priority
        ),
        WorkItem::new(
            "Discover and load relevant skills".to_string(),
            AgentRole::Optimizer,
            Phase::PromptToSpec,
            9,
        ),
        WorkItem::new(
            "Initialize session state and monitoring".to_string(),
            AgentRole::Orchestrator,
            Phase::PromptToSpec,
            9,
        ),
    ]
}

/// Create work items for Phase 1: Prompt → Spec
///
/// Given a user prompt, create work items to:
/// 1. Clarify ambiguities and requirements
/// 2. Discover domain-specific skills
/// 3. Load relevant memories
/// 4. Generate specification
pub fn create_phase1_work_items(user_prompt: String) -> Vec<WorkItem> {
    vec![
        WorkItem::new(
            format!("Analyze and clarify intent: {}", user_prompt),
            AgentRole::Reviewer,
            Phase::PromptToSpec,
            10,
        ),
        WorkItem::new(
            format!("Discover skills for: {}", user_prompt),
            AgentRole::Optimizer,
            Phase::PromptToSpec,
            9,
        ),
        WorkItem::new(
            format!("Load context for: {}", user_prompt),
            AgentRole::Optimizer,
            Phase::PromptToSpec,
            9,
        ),
        WorkItem::new(
            format!("Generate specification for: {}", user_prompt),
            AgentRole::Executor,
            Phase::PromptToSpec,
            8,
        ),
    ]
}

/// Create work items for Phase 2: Spec → Full Spec
///
/// Decompose specification into components with:
/// 1. Dependencies identified
/// 2. Typed holes (interfaces) defined
/// 3. Test plan created
/// 4. Edge cases documented
pub fn create_phase2_work_items(spec_summary: String) -> Vec<WorkItem> {
    vec![
        WorkItem::new(
            format!("Decompose into components: {}", spec_summary),
            AgentRole::Executor,
            Phase::SpecToFullSpec,
            10,
        ),
        WorkItem::new(
            "Identify component dependencies".to_string(),
            AgentRole::Orchestrator,
            Phase::SpecToFullSpec,
            9,
        ),
        WorkItem::new(
            "Define typed holes (interfaces)".to_string(),
            AgentRole::Executor,
            Phase::SpecToFullSpec,
            8,
        ),
        WorkItem::new(
            "Create test plan with coverage targets".to_string(),
            AgentRole::Reviewer,
            Phase::SpecToFullSpec,
            8,
        ),
        WorkItem::new(
            "Document edge cases and constraints".to_string(),
            AgentRole::Executor,
            Phase::SpecToFullSpec,
            7,
        ),
    ]
}

/// Create work items for Phase 3: Full Spec → Plan
///
/// Create execution plan with:
/// 1. Tasks ordered by dependencies
/// 2. Parallelization opportunities identified
/// 3. Critical path computed
/// 4. Checkpoints planned
pub fn create_phase3_work_items(full_spec_summary: String) -> Vec<WorkItem> {
    vec![
        WorkItem::new(
            format!("Order tasks by dependencies: {}", full_spec_summary),
            AgentRole::Orchestrator,
            Phase::FullSpecToPlan,
            10,
        ),
        WorkItem::new(
            "Identify parallelization opportunities".to_string(),
            AgentRole::Orchestrator,
            Phase::FullSpecToPlan,
            9,
        ),
        WorkItem::new(
            "Compute critical path".to_string(),
            AgentRole::Orchestrator,
            Phase::FullSpecToPlan,
            9,
        ),
        WorkItem::new(
            "Plan checkpoints and rollback points".to_string(),
            AgentRole::Orchestrator,
            Phase::FullSpecToPlan,
            8,
        ),
        WorkItem::new(
            "Generate implementation plan document".to_string(),
            AgentRole::Executor,
            Phase::FullSpecToPlan,
            7,
        ),
    ]
}

/// Create work items for Phase 4: Plan → Artifacts
///
/// Execute plan to create:
/// 1. Code implementation
/// 2. Tests (unit, integration, e2e)
/// 3. Documentation
/// 4. All typed holes filled
pub fn create_phase4_work_items(plan_tasks: Vec<String>) -> Vec<WorkItem> {
    let mut work_items = Vec::new();

    // Create work items for each plan task
    for (index, task_desc) in plan_tasks.iter().enumerate() {
        work_items.push(WorkItem::new(
            format!("Implement: {}", task_desc),
            AgentRole::Executor,
            Phase::PlanToArtifacts,
            10 - (index.min(5) as u8), // Decrease priority for later tasks
        ));
    }

    // Add test execution work items
    work_items.push(WorkItem::new(
        "Write and execute unit tests".to_string(),
        AgentRole::Executor,
        Phase::PlanToArtifacts,
        9,
    ));

    work_items.push(WorkItem::new(
        "Write and execute integration tests".to_string(),
        AgentRole::Executor,
        Phase::PlanToArtifacts,
        8,
    ));

    // Add documentation work items
    work_items.push(WorkItem::new(
        "Generate API documentation".to_string(),
        AgentRole::Executor,
        Phase::PlanToArtifacts,
        7,
    ));

    // Add final verification
    work_items.push(WorkItem::new(
        "Verify all typed holes filled".to_string(),
        AgentRole::Reviewer,
        Phase::PlanToArtifacts,
        10, // High priority verification
    ));

    work_items.push(WorkItem::new(
        "Verify all tests passing".to_string(),
        AgentRole::Reviewer,
        Phase::PlanToArtifacts,
        10,
    ));

    work_items
}

/// Helper to extract task descriptions from a plan
///
/// This is a placeholder - in practice, this would parse
/// the generated plan document to extract task items.
pub fn extract_tasks_from_plan(plan: &str) -> Vec<String> {
    // Simple line-based parsing
    // TODO: Enhance with structured plan parsing
    plan.lines()
        .filter(|line| line.trim().starts_with("- [ ]"))
        .map(|line| line.trim().trim_start_matches("- [ ]").trim().to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_init_creates_work_items() {
        let items = create_session_init_work_items();
        assert_eq!(items.len(), 3);
        assert!(items.iter().all(|i| i.phase == Phase::PromptToSpec));
        assert!(items
            .iter()
            .all(|i| i.state == crate::orchestration::state::AgentState::Ready));
    }

    #[test]
    fn test_phase1_creates_work_items() {
        let items = create_phase1_work_items("Implement JWT auth".to_string());
        assert_eq!(items.len(), 4);
        assert!(items.iter().all(|i| i.phase == Phase::PromptToSpec));
    }

    #[test]
    fn test_phase2_creates_work_items() {
        let items = create_phase2_work_items("JWT auth spec".to_string());
        assert_eq!(items.len(), 5);
        assert!(items.iter().all(|i| i.phase == Phase::SpecToFullSpec));
    }

    #[test]
    fn test_phase3_creates_work_items() {
        let items = create_phase3_work_items("Full JWT spec".to_string());
        assert_eq!(items.len(), 5);
        assert!(items.iter().all(|i| i.phase == Phase::FullSpecToPlan));
    }

    #[test]
    fn test_phase4_creates_work_items() {
        let tasks = vec![
            "Create database schema".to_string(),
            "Implement token generation".to_string(),
            "Add middleware".to_string(),
        ];
        let items = create_phase4_work_items(tasks);

        // 3 implementation + 2 tests + 1 docs + 2 verification
        assert_eq!(items.len(), 8);
        assert!(items.iter().all(|i| i.phase == Phase::PlanToArtifacts));
    }

    #[test]
    fn test_extract_tasks_from_plan() {
        let plan = r#"
# Implementation Plan

## Tasks
- [ ] Create database schema
- [ ] Implement JWT generation
- [ ] Add middleware integration

## Notes
This is a note, not a task.
        "#;

        let tasks = extract_tasks_from_plan(plan);
        assert_eq!(tasks.len(), 3);
        assert_eq!(tasks[0], "Create database schema");
        assert_eq!(tasks[1], "Implement JWT generation");
        assert_eq!(tasks[2], "Add middleware integration");
    }
}
