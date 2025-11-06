//! Integration tests for LLM-enhanced reviewer
//!
//! These tests verify:
//! - Requirement tracking and persistence
//! - Requirement status management
//! - Implementation evidence tracking
//! - Database persistence of requirement fields
//! - Review retry workflow

use mnemosyne_core::{
    launcher::agents::AgentRole,
    orchestration::{
        messages::WorkResult,
        state::{AgentState, Phase, RequirementStatus, WorkItem, WorkItemId},
    },
    storage::StorageBackend,
    types::MemoryId,
    ConnectionMode, LibsqlStorage,
};
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

/// Helper to create test storage with temporary database
async fn create_test_storage() -> (Arc<LibsqlStorage>, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let storage = LibsqlStorage::new_with_validation(
        ConnectionMode::Local(db_path.to_str().unwrap().to_string()),
        true, // create_if_missing
    )
    .await
    .expect("Failed to create test storage");

    (Arc::new(storage), temp_dir)
}

/// Helper to create a test work item
fn create_test_work_item(description: &str, intent: &str) -> WorkItem {
    WorkItem {
        id: WorkItemId::new(),
        description: description.to_string(),
        original_intent: intent.to_string(),
        agent: AgentRole::Executor,
        state: AgentState::PendingReview,
        phase: Phase::PlanToArtifacts,
        priority: 1,
        dependencies: Vec::new(),
        created_at: chrono::Utc::now(),
        started_at: Some(chrono::Utc::now()),
        completed_at: None,
        error: None,
        timeout: Some(Duration::from_secs(300)),
        assigned_branch: None,
        estimated_duration: None,
        file_scope: None,
        review_feedback: None,
        suggested_tests: None,
        review_attempt: 0,
        execution_memory_ids: Vec::new(),
        consolidated_context_id: None,
        estimated_context_tokens: 0,
        requirements: Vec::new(), // Start empty, use add_requirement to populate
        requirement_status: std::collections::HashMap::new(),
        implementation_evidence: std::collections::HashMap::new(),
    }
}

// =============================================================================
// Test: Work Result Creation
// =============================================================================

#[tokio::test]
async fn test_work_result_with_memory_ids() {
    let item_id = WorkItemId::new();
    let memory_id1 = MemoryId::new();
    let memory_id2 = MemoryId::new();

    let work_result = WorkResult {
        item_id: item_id.clone(),
        success: true,
        data: Some("Implementation complete".to_string()),
        error: None,
        duration: Duration::from_secs(10),
        memory_ids: vec![memory_id1, memory_id2],
    };

    assert_eq!(work_result.memory_ids.len(), 2);
    assert!(work_result.success);
    assert_eq!(work_result.item_id, item_id);
}

// =============================================================================
// Test: Requirement Tracking
// =============================================================================

#[tokio::test]
async fn test_requirement_tracking_persistence() {
    let (storage, _temp) = create_test_storage().await;

    // Create work item with requirements
    let mut work_item = create_test_work_item(
        "Implement user authentication",
        "Add JWT authentication with refresh tokens",
    );

    work_item.add_requirement("JWT token generation".to_string());
    work_item.add_requirement("Refresh token rotation".to_string());
    work_item.add_requirement("Token validation middleware".to_string());

    // Store work item
    storage
        .store_work_item(&work_item)
        .await
        .expect("Failed to store work item");

    // Load work item
    let loaded_item = storage
        .load_work_item(&work_item.id)
        .await
        .expect("Failed to load work item");

    // Verify requirements persisted
    assert_eq!(loaded_item.requirements.len(), 3);
    assert!(loaded_item
        .requirements
        .contains(&"JWT token generation".to_string()));
    assert!(loaded_item
        .requirements
        .contains(&"Refresh token rotation".to_string()));
    assert!(loaded_item
        .requirements
        .contains(&"Token validation middleware".to_string()));
}

#[tokio::test]
async fn test_requirement_status_tracking() {
    let (storage, _temp) = create_test_storage().await;

    // Create work item with requirements
    let mut work_item = create_test_work_item(
        "Implement authentication",
        "Add authentication with proper error handling",
    );

    work_item.add_requirement("JWT token generation".to_string());
    work_item.add_requirement("Error handling".to_string());
    work_item.add_requirement("Unit tests".to_string());

    // Mark some requirements as satisfied
    work_item.update_requirement_status(
        "JWT token generation",
        mnemosyne_core::orchestration::state::RequirementStatus::Satisfied,
    );
    work_item.update_requirement_status(
        "Error handling",
        mnemosyne_core::orchestration::state::RequirementStatus::InProgress,
    );

    // Store and reload
    storage
        .store_work_item(&work_item)
        .await
        .expect("Failed to store work item");

    let loaded_item = storage
        .load_work_item(&work_item.id)
        .await
        .expect("Failed to load work item");

    // Verify status persisted
    assert_eq!(
        loaded_item.requirement_status.get("JWT token generation"),
        Some(&mnemosyne_core::orchestration::state::RequirementStatus::Satisfied)
    );
    assert_eq!(
        loaded_item.requirement_status.get("Error handling"),
        Some(&mnemosyne_core::orchestration::state::RequirementStatus::InProgress)
    );
    assert_eq!(
        loaded_item.requirement_status.get("Unit tests"),
        Some(&mnemosyne_core::orchestration::state::RequirementStatus::NotStarted)
    );

    // Check unsatisfied requirements
    let unsatisfied = loaded_item.unsatisfied_requirements();
    assert_eq!(unsatisfied.len(), 2);
    assert!(unsatisfied.contains(&"Error handling".to_string()));
    assert!(unsatisfied.contains(&"Unit tests".to_string()));
}

#[tokio::test]
async fn test_implementation_evidence_tracking() {
    let (storage, _temp) = create_test_storage().await;

    // Create work item
    let mut work_item = create_test_work_item("Implement auth", "Add authentication");

    work_item.add_requirement("JWT generation".to_string());

    // Add implementation evidence
    let memory_id1 = MemoryId::new();
    let memory_id2 = MemoryId::new();

    work_item.add_implementation_evidence("JWT generation", memory_id1.clone());
    work_item.add_implementation_evidence("JWT generation", memory_id2.clone());

    // Store and reload
    storage
        .store_work_item(&work_item)
        .await
        .expect("Failed to store work item");

    let loaded_item = storage
        .load_work_item(&work_item.id)
        .await
        .expect("Failed to load work item");

    // Verify evidence persisted
    let evidence = loaded_item
        .implementation_evidence
        .get("JWT generation")
        .expect("Evidence not found");

    assert_eq!(evidence.len(), 2);
    assert!(evidence.contains(&memory_id1));
    assert!(evidence.contains(&memory_id2));
}

// =============================================================================
// Test: Review Retry Workflow
// =============================================================================

#[tokio::test]
async fn test_review_retry_increments_attempt() {
    let (storage, _temp) = create_test_storage().await;

    // Create work item
    let mut work_item = create_test_work_item("Implement feature", "Implement feature");

    // Simulate first review failure
    work_item.review_attempt = 0;
    work_item.state = AgentState::PendingReview;

    storage
        .store_work_item(&work_item)
        .await
        .expect("Failed to store work item");

    // Simulate review failure - increment attempt
    work_item.review_attempt += 1;
    work_item.state = AgentState::Ready; // Back to ready for retry
    work_item.review_feedback = Some(vec!["Implementation incomplete".to_string()]);

    storage
        .update_work_item(&work_item)
        .await
        .expect("Failed to update work item");

    // Load and verify
    let loaded_item = storage
        .load_work_item(&work_item.id)
        .await
        .expect("Failed to load work item");

    assert_eq!(loaded_item.review_attempt, 1);
    assert_eq!(loaded_item.state, AgentState::Ready);
    assert!(loaded_item.review_feedback.is_some());
}

// =============================================================================
// Test: Requirement Helpers
// =============================================================================

#[tokio::test]
async fn test_all_requirements_satisfied() {
    let mut work_item = create_test_work_item("Implement feature", "Complete implementation");

    work_item.add_requirement("Feature A".to_string());
    work_item.add_requirement("Feature B".to_string());

    // Initially not satisfied
    assert!(!work_item.all_requirements_satisfied());

    // Mark all as satisfied
    work_item.update_requirement_status("Feature A", RequirementStatus::Satisfied);
    work_item.update_requirement_status("Feature B", RequirementStatus::Satisfied);

    // Now all satisfied
    assert!(work_item.all_requirements_satisfied());
}

#[tokio::test]
async fn test_unsatisfied_requirements_list() {
    let mut work_item = create_test_work_item("Implement feature", "Complete implementation");

    work_item.add_requirement("Feature A".to_string());
    work_item.add_requirement("Feature B".to_string());
    work_item.add_requirement("Feature C".to_string());

    // Mark one as satisfied
    work_item.update_requirement_status("Feature A", RequirementStatus::Satisfied);

    let unsatisfied = work_item.unsatisfied_requirements();
    assert_eq!(unsatisfied.len(), 2);
    assert!(unsatisfied.contains(&"Feature B".to_string()));
    assert!(unsatisfied.contains(&"Feature C".to_string()));
    assert!(!unsatisfied.contains(&"Feature A".to_string()));
}
