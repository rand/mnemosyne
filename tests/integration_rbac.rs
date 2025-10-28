//! Integration tests for Role-Based Access Control (RBAC)
//!
//! Tests ownership tracking, permission checks, audit trails, and admin override.

use mnemosyne_core::agents::{
    AgentMemoryView, AgentRole, MemoryAccessControl, MemoryMetadata, MemoryUpdates,
    ModificationType,
};
use mnemosyne_core::storage::libsql::LibsqlStorage;
use mnemosyne_core::types::{MemoryType, Namespace};
use std::sync::Arc;
use tempfile::tempdir;

/// Helper to create test storage
async fn create_test_storage() -> LibsqlStorage {
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test_rbac.db");

    LibsqlStorage::new_local(db_path.to_str().unwrap())
        .await
        .expect("Failed to create storage")
}

/// Helper to create test metadata
fn create_test_metadata() -> MemoryMetadata {
    MemoryMetadata {
        memory_type: MemoryType::CodePattern,
        namespace: Namespace::Global,
        importance: 8,
        confidence: 0.9,
        summary: "Test pattern for RBAC".to_string(),
        keywords: vec!["test".to_string(), "rbac".to_string()],
        tags: vec!["testing".to_string()],
        context: "Integration test context".to_string(),
        related_files: vec![],
        related_entities: vec![],
        expires_at: None,
        visible_to: None,
    }
}

#[tokio::test]
async fn test_agent_can_create_memory() {
    let storage = create_test_storage().await;
    let access_control = MemoryAccessControl::new(AgentRole::Executor, Arc::new(storage.clone()));

    let metadata = create_test_metadata();
    let result = access_control
        .create_memory("Test memory content", metadata)
        .await;

    assert!(result.is_ok(), "Agent should be able to create memory");
    let memory_id = result.unwrap();

    // Verify memory was created
    let memory = storage.get_memory(memory_id).await;
    assert!(memory.is_ok());
    assert_eq!(memory.unwrap().content, "Test memory content");
}

#[tokio::test]
async fn test_agent_can_update_own_memory() {
    let storage = create_test_storage().await;
    let access_control = MemoryAccessControl::new(AgentRole::Executor, Arc::new(storage.clone()));

    // Create a memory
    let metadata = create_test_metadata();
    let memory_id = access_control
        .create_memory("Original content", metadata)
        .await
        .expect("Failed to create memory");

    // Update the memory
    let updates = MemoryUpdates {
        content: Some("Updated content".to_string()),
        summary: Some("Updated summary".to_string()),
        importance: Some(9),
        ..Default::default()
    };

    let result = access_control.update_memory(&memory_id, updates).await;
    assert!(result.is_ok(), "Agent should be able to update own memory");

    // Verify updates were applied
    let memory = storage.get_memory(memory_id).await.unwrap();
    assert_eq!(memory.content, "Updated content");
    assert_eq!(memory.summary, "Updated summary");
    assert_eq!(memory.importance, 9);
}

#[tokio::test]
async fn test_agent_can_delete_own_memory() {
    let storage = create_test_storage().await;
    let access_control = MemoryAccessControl::new(AgentRole::Executor, Arc::new(storage.clone()));

    // Create a memory
    let metadata = create_test_metadata();
    let memory_id = access_control
        .create_memory("Content to delete", metadata)
        .await
        .expect("Failed to create memory");

    // Delete the memory
    let result = access_control.delete_memory(&memory_id).await;
    assert!(result.is_ok(), "Agent should be able to delete own memory");

    // Verify memory was archived
    let memory = storage.get_memory(memory_id).await;
    // Memory should be archived, not deleted
    assert!(memory.is_ok());
    assert!(memory.unwrap().is_archived);
}

#[tokio::test]
async fn test_agent_can_archive_own_memory() {
    let storage = create_test_storage().await;
    let access_control = MemoryAccessControl::new(AgentRole::Executor, Arc::new(storage.clone()));

    // Create a memory
    let metadata = create_test_metadata();
    let memory_id = access_control
        .create_memory("Content to archive", metadata)
        .await
        .expect("Failed to create memory");

    // Archive the memory
    let result = access_control.archive_memory(&memory_id).await;
    assert!(result.is_ok(), "Agent should be able to archive own memory");

    // Verify memory was archived
    let memory = storage.get_memory(memory_id).await.unwrap();
    assert!(memory.is_archived);
}

#[tokio::test]
async fn test_default_visibility_executor() {
    let storage = create_test_storage().await;
    let access_control = MemoryAccessControl::new(AgentRole::Executor, Arc::new(storage));

    let visibility = access_control.default_visibility();

    assert!(visibility.contains(&AgentRole::Executor));
    assert!(visibility.contains(&AgentRole::Reviewer));
    assert_eq!(visibility.len(), 2);
}

#[tokio::test]
async fn test_default_visibility_orchestrator() {
    let storage = create_test_storage().await;
    let access_control = MemoryAccessControl::new(AgentRole::Orchestrator, Arc::new(storage));

    let visibility = access_control.default_visibility();

    assert!(visibility.contains(&AgentRole::Orchestrator));
    assert!(visibility.contains(&AgentRole::Optimizer));
    assert_eq!(visibility.len(), 2);
}

#[tokio::test]
async fn test_default_visibility_optimizer() {
    let storage = create_test_storage().await;
    let access_control = MemoryAccessControl::new(AgentRole::Optimizer, Arc::new(storage));

    let visibility = access_control.default_visibility();

    assert!(visibility.contains(&AgentRole::Optimizer));
    assert!(visibility.contains(&AgentRole::Executor));
    assert_eq!(visibility.len(), 2);
}

#[tokio::test]
async fn test_default_visibility_reviewer() {
    let storage = create_test_storage().await;
    let access_control = MemoryAccessControl::new(AgentRole::Reviewer, Arc::new(storage));

    let visibility = access_control.default_visibility();

    assert!(visibility.contains(&AgentRole::Reviewer));
    assert!(visibility.contains(&AgentRole::Executor));
    assert_eq!(visibility.len(), 2);
}

#[tokio::test]
async fn test_custom_visibility_override() {
    let storage = create_test_storage().await;
    let access_control = MemoryAccessControl::new(AgentRole::Executor, Arc::new(storage));

    // Create memory with custom visibility
    let mut metadata = create_test_metadata();
    metadata.visible_to = Some(vec![
        AgentRole::Orchestrator,
        AgentRole::Optimizer,
        AgentRole::Executor,
    ]);

    let result = access_control
        .create_memory("Custom visibility content", metadata)
        .await;

    assert!(
        result.is_ok(),
        "Should be able to create memory with custom visibility"
    );
}

#[tokio::test]
async fn test_admin_mode_with_env_var() {
    // Save original state
    let original = std::env::var("MNEMOSYNE_ADMIN_MODE").ok();

    // Set admin mode
    std::env::set_var("MNEMOSYNE_ADMIN_MODE", "1");

    let storage = create_test_storage().await;
    let access_control = MemoryAccessControl::new(AgentRole::Executor, Arc::new(storage));

    assert!(access_control.is_admin(), "Admin mode should be detected");

    // Restore original state
    std::env::remove_var("MNEMOSYNE_ADMIN_MODE");
    if let Some(val) = original {
        std::env::set_var("MNEMOSYNE_ADMIN_MODE", val);
    }
}

#[tokio::test]
async fn test_admin_mode_with_human_user() {
    // Save original state
    let original = std::env::var("MNEMOSYNE_USER").ok();

    // Set human user
    std::env::set_var("MNEMOSYNE_USER", "human");

    let storage = create_test_storage().await;
    let access_control = MemoryAccessControl::new(AgentRole::Executor, Arc::new(storage));

    assert!(
        access_control.is_admin(),
        "Human user should be treated as admin"
    );

    // Restore original state
    std::env::remove_var("MNEMOSYNE_USER");
    if let Some(val) = original {
        std::env::set_var("MNEMOSYNE_USER", val);
    }
}

#[tokio::test]
async fn test_update_memory_tracks_changes() {
    let storage = create_test_storage().await;
    let access_control = MemoryAccessControl::new(AgentRole::Executor, Arc::new(storage.clone()));

    // Create a memory
    let metadata = create_test_metadata();
    let memory_id = access_control
        .create_memory("Original", metadata)
        .await
        .expect("Failed to create memory");

    // Update multiple fields
    let updates = MemoryUpdates {
        content: Some("New content".to_string()),
        summary: Some("New summary".to_string()),
        importance: Some(10),
        confidence: Some(0.95),
        keywords: Some(vec!["new".to_string(), "keywords".to_string()]),
        tags: Some(vec!["new-tag".to_string()]),
        ..Default::default()
    };

    let result = access_control.update_memory(&memory_id, updates).await;
    assert!(result.is_ok(), "Update should succeed");

    // Verify all changes were applied
    let memory = storage.get_memory(memory_id).await.unwrap();
    assert_eq!(memory.content, "New content");
    assert_eq!(memory.summary, "New summary");
    assert_eq!(memory.importance, 10);
    assert_eq!(memory.confidence, 0.95);
    assert_eq!(memory.keywords, vec!["new", "keywords"]);
    assert_eq!(memory.tags, vec!["new-tag"]);
}

#[tokio::test]
async fn test_partial_updates() {
    let storage = create_test_storage().await;
    let access_control = MemoryAccessControl::new(AgentRole::Executor, Arc::new(storage.clone()));

    // Create a memory
    let metadata = create_test_metadata();
    let memory_id = access_control
        .create_memory("Original content", metadata)
        .await
        .expect("Failed to create memory");

    let original_memory = storage.get_memory(memory_id).await.unwrap();

    // Update only importance
    let updates = MemoryUpdates {
        importance: Some(10),
        ..Default::default()
    };

    access_control
        .update_memory(&memory_id, updates)
        .await
        .expect("Update should succeed");

    // Verify only importance changed
    let updated_memory = storage.get_memory(memory_id).await.unwrap();
    assert_eq!(updated_memory.importance, 10);
    assert_eq!(updated_memory.content, original_memory.content);
    assert_eq!(updated_memory.summary, original_memory.summary);
    assert_eq!(updated_memory.confidence, original_memory.confidence);
}

#[tokio::test]
async fn test_memory_type_filtering_by_agent_role() {
    let storage = create_test_storage().await;

    // Executor creates a CodePattern memory
    let executor_ac = MemoryAccessControl::new(AgentRole::Executor, Arc::new(storage.clone()));
    let mut metadata = create_test_metadata();
    metadata.memory_type = MemoryType::CodePattern;
    executor_ac
        .create_memory("Code pattern", metadata.clone())
        .await
        .expect("Failed to create");

    // Orchestrator creates an ArchitectureDecision memory
    let orchestrator_ac = MemoryAccessControl::new(AgentRole::Orchestrator, Arc::new(storage.clone()));
    metadata.memory_type = MemoryType::ArchitectureDecision;
    orchestrator_ac
        .create_memory("Architecture decision", metadata)
        .await
        .expect("Failed to create");

    // Test agent views see appropriate types
    let executor_view = AgentMemoryView::new(AgentRole::Executor, storage.clone());
    let executor_types = executor_view.role().memory_types();
    assert!(executor_types.contains(&MemoryType::CodePattern));
    assert!(!executor_types.contains(&MemoryType::ArchitectureDecision));

    let orchestrator_view = AgentMemoryView::new(AgentRole::Orchestrator, storage.clone());
    let orchestrator_types = orchestrator_view.role().memory_types();
    assert!(orchestrator_types.contains(&MemoryType::ArchitectureDecision));
    assert!(!orchestrator_types.contains(&MemoryType::CodePattern));
}

#[tokio::test]
async fn test_multiple_agents_different_memories() {
    let storage = create_test_storage().await;

    // Different agents create memories
    let executor = MemoryAccessControl::new(AgentRole::Executor, Arc::new(storage.clone()));
    let reviewer = MemoryAccessControl::new(AgentRole::Reviewer, Arc::new(storage.clone()));
    let optimizer = MemoryAccessControl::new(AgentRole::Optimizer, Arc::new(storage.clone()));

    let executor_meta = MemoryMetadata {
        memory_type: MemoryType::CodePattern,
        namespace: Namespace::Global,
        importance: 8,
        confidence: 0.9,
        summary: "Executor memory".to_string(),
        keywords: vec!["executor".to_string()],
        tags: vec![],
        context: "Created by executor".to_string(),
        related_files: vec![],
        related_entities: vec![],
        expires_at: None,
        visible_to: None,
    };

    let reviewer_meta = MemoryMetadata {
        memory_type: MemoryType::BugFix,
        namespace: Namespace::Global,
        importance: 9,
        confidence: 0.95,
        summary: "Reviewer memory".to_string(),
        keywords: vec!["reviewer".to_string()],
        tags: vec![],
        context: "Created by reviewer".to_string(),
        related_files: vec![],
        related_entities: vec![],
        expires_at: None,
        visible_to: None,
    };

    let optimizer_meta = MemoryMetadata {
        memory_type: MemoryType::Insight,
        namespace: Namespace::Global,
        importance: 7,
        confidence: 0.85,
        summary: "Optimizer memory".to_string(),
        keywords: vec!["optimizer".to_string()],
        tags: vec![],
        context: "Created by optimizer".to_string(),
        related_files: vec![],
        related_entities: vec![],
        expires_at: None,
        visible_to: None,
    };

    let executor_id = executor
        .create_memory("Executor content", executor_meta)
        .await
        .expect("Failed to create");
    let reviewer_id = reviewer
        .create_memory("Reviewer content", reviewer_meta)
        .await
        .expect("Failed to create");
    let optimizer_id = optimizer
        .create_memory("Optimizer content", optimizer_meta)
        .await
        .expect("Failed to create");

    // Verify all memories were created
    assert!(storage.get_memory(executor_id).await.is_ok());
    assert!(storage.get_memory(reviewer_id).await.is_ok());
    assert!(storage.get_memory(optimizer_id).await.is_ok());
}

#[tokio::test]
async fn test_get_agent_role() {
    let storage = create_test_storage().await;

    let executor = MemoryAccessControl::new(AgentRole::Executor, storage.clone());
    assert_eq!(executor.agent(), AgentRole::Executor);

    let reviewer = MemoryAccessControl::new(AgentRole::Reviewer, Arc::new(storage.clone()));
    assert_eq!(reviewer.agent(), AgentRole::Reviewer);

    let optimizer = MemoryAccessControl::new(AgentRole::Optimizer, Arc::new(storage.clone()));
    assert_eq!(optimizer.agent(), AgentRole::Optimizer);

    let orchestrator = MemoryAccessControl::new(AgentRole::Orchestrator, Arc::new(storage));
    assert_eq!(orchestrator.agent(), AgentRole::Orchestrator);
}

#[tokio::test]
async fn test_audit_trail_placeholder() {
    // This test verifies the audit trail API exists and returns correctly
    // Actual implementation will be added when storage backend supports it
    let storage = create_test_storage().await;
    let access_control = MemoryAccessControl::new(AgentRole::Executor, Arc::new(storage));

    let metadata = create_test_metadata();
    let memory_id = access_control
        .create_memory("Test content", metadata)
        .await
        .expect("Failed to create memory");

    // Get audit trail (currently returns empty)
    let trail = access_control
        .get_audit_trail(&memory_id)
        .await
        .expect("Should return audit trail");

    // Currently empty, but API is in place
    assert_eq!(trail.len(), 0);
}

#[tokio::test]
async fn test_modification_stats_placeholder() {
    // This test verifies the modification stats API exists
    // Actual implementation will be added when storage backend supports it
    let storage = create_test_storage().await;
    let access_control = MemoryAccessControl::new(AgentRole::Executor, Arc::new(storage));

    let stats = access_control
        .get_modification_stats()
        .await
        .expect("Should return stats");

    // Currently empty, but API is in place
    assert_eq!(stats.len(), 0);
}

#[tokio::test]
async fn test_different_namespaces() {
    let storage = create_test_storage().await;
    let access_control = MemoryAccessControl::new(AgentRole::Executor, Arc::new(storage));

    // Create memories in different namespaces
    let global_meta = MemoryMetadata {
        memory_type: MemoryType::CodePattern,
        namespace: Namespace::Global,
        importance: 8,
        confidence: 0.9,
        summary: "Global memory".to_string(),
        keywords: vec!["global".to_string()],
        tags: vec![],
        context: "Global namespace".to_string(),
        related_files: vec![],
        related_entities: vec![],
        expires_at: None,
        visible_to: None,
    };

    let project_meta = MemoryMetadata {
        memory_type: MemoryType::CodePattern,
        namespace: Namespace::Project {
            name: "test-project".to_string(),
        },
        importance: 8,
        confidence: 0.9,
        summary: "Project memory".to_string(),
        keywords: vec!["project".to_string()],
        tags: vec![],
        context: "Project namespace".to_string(),
        related_files: vec![],
        related_entities: vec![],
        expires_at: None,
        visible_to: None,
    };

    let global_id = access_control
        .create_memory("Global content", global_meta)
        .await
        .expect("Failed to create global memory");

    let project_id = access_control
        .create_memory("Project content", project_meta)
        .await
        .expect("Failed to create project memory");

    // Both should be created successfully
    assert_ne!(global_id, project_id);
}
