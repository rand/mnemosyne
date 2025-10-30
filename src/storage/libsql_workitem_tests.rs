//! Tests for work item persistence in LibsqlStorage
//!
//! These tests verify the complete lifecycle of work items:
//! - Store and load round-trip
//! - Update functionality
//! - Query by state
//! - Delete functionality
//! - Complex field handling

#[cfg(test)]
mod work_item_persistence_tests {
    use crate::launcher::agents::AgentRole;
    use crate::orchestration::state::{AgentState, Phase, WorkItem, WorkItemId};
    use crate::storage::StorageBackend;
    use crate::types::MemoryId;
    use crate::{ConnectionMode, LibsqlStorage};
    
    use std::path::PathBuf;
    
    use std::time::Duration;
    use tempfile::TempDir;

    /// Create a test storage backend
    async fn create_test_storage() -> (LibsqlStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test_workitems.db");

        let storage = LibsqlStorage::new_with_validation(
            ConnectionMode::Local(db_path.to_str().unwrap().to_string()),
            true, // create_if_missing
        )
        .await
        .expect("Failed to create test storage");

        (storage, temp_dir)
    }

    /// Create a test work item with all fields populated
    fn create_test_work_item() -> WorkItem {
        let mut item = WorkItem::new(
            "Test work item".to_string(),
            AgentRole::Executor,
            Phase::PlanToArtifacts,
            5,
        );

        // Populate optional fields
        item.dependencies = vec![WorkItemId::new(), WorkItemId::new()];
        item.error = Some("Test error".to_string());
        item.timeout = Some(Duration::from_secs(120));
        item.assigned_branch = Some("feature/test".to_string());
        item.estimated_duration = Some(Duration::from_secs(300));
        item.file_scope = Some(vec![
            PathBuf::from("/test/file1.rs"),
            PathBuf::from("/test/file2.rs"),
        ]);
        item.review_feedback = Some(vec![
            "Issue 1".to_string(),
            "Issue 2".to_string(),
        ]);
        item.suggested_tests = Some(vec![
            "Test async behavior".to_string(),
            "Test error handling".to_string(),
        ]);
        item.review_attempt = 2;
        item.execution_memory_ids = vec![MemoryId::new(), MemoryId::new()];
        item.consolidated_context_id = Some(MemoryId::new());
        item.estimated_context_tokens = 5000;

        item
    }

    #[tokio::test]
    async fn test_store_and_load_round_trip() {
        let (storage, _temp_dir) = create_test_storage().await;

        // Create test work item
        let original_item = create_test_work_item();
        let item_id = original_item.id.clone();

        // Store it
        storage
            .store_work_item(&original_item)
            .await
            .expect("Failed to store work item");

        // Load it back
        let loaded_item = storage
            .load_work_item(&item_id)
            .await
            .expect("Failed to load work item");

        // Verify all fields match
        assert_eq!(loaded_item.id.to_string(), original_item.id.to_string());
        assert_eq!(loaded_item.description, original_item.description);
        assert_eq!(loaded_item.original_intent, original_item.original_intent);
        assert_eq!(
            format!("{:?}", loaded_item.agent),
            format!("{:?}", original_item.agent)
        );
        assert_eq!(
            format!("{:?}", loaded_item.state),
            format!("{:?}", original_item.state)
        );
        assert_eq!(
            format!("{:?}", loaded_item.phase),
            format!("{:?}", original_item.phase)
        );
        assert_eq!(loaded_item.priority, original_item.priority);
        assert_eq!(loaded_item.dependencies.len(), original_item.dependencies.len());
        assert_eq!(loaded_item.error, original_item.error);
        assert_eq!(loaded_item.assigned_branch, original_item.assigned_branch);
        assert_eq!(loaded_item.review_feedback, original_item.review_feedback);
        assert_eq!(loaded_item.suggested_tests, original_item.suggested_tests);
        assert_eq!(loaded_item.review_attempt, original_item.review_attempt);
        assert_eq!(
            loaded_item.execution_memory_ids.len(),
            original_item.execution_memory_ids.len()
        );
        assert_eq!(
            loaded_item.consolidated_context_id.map(|id| id.to_string()),
            original_item.consolidated_context_id.map(|id| id.to_string())
        );
        assert_eq!(
            loaded_item.estimated_context_tokens,
            original_item.estimated_context_tokens
        );
    }

    #[tokio::test]
    async fn test_update_work_item() {
        let (storage, _temp_dir) = create_test_storage().await;

        // Create and store work item
        let mut item = create_test_work_item();
        let item_id = item.id.clone();
        storage
            .store_work_item(&item)
            .await
            .expect("Failed to store work item");

        // Modify fields
        item.state = AgentState::PendingReview;
        item.review_attempt = 3;
        item.review_feedback = Some(vec![
            "Issue 1".to_string(),
            "Issue 2".to_string(),
            "Issue 3".to_string(),
        ]);
        item.error = Some("New error".to_string());

        // Update it
        storage
            .update_work_item(&item)
            .await
            .expect("Failed to update work item");

        // Load it back
        let loaded_item = storage
            .load_work_item(&item_id)
            .await
            .expect("Failed to load work item");

        // Verify changes persisted
        assert_eq!(format!("{:?}", loaded_item.state), "PendingReview");
        assert_eq!(loaded_item.review_attempt, 3);
        assert_eq!(loaded_item.review_feedback.as_ref().unwrap().len(), 3);
        assert_eq!(loaded_item.error, Some("New error".to_string()));
    }

    #[tokio::test]
    async fn test_load_work_items_by_state() {
        let (storage, _temp_dir) = create_test_storage().await;

        // Create work items with different states
        let mut item1 = create_test_work_item();
        item1.state = AgentState::Ready;
        let mut item2 = create_test_work_item();
        item2.state = AgentState::Ready;
        let mut item3 = create_test_work_item();
        item3.state = AgentState::Active;

        // Store all
        storage.store_work_item(&item1).await.unwrap();
        storage.store_work_item(&item2).await.unwrap();
        storage.store_work_item(&item3).await.unwrap();

        // Query for Ready state
        let ready_items = storage
            .load_work_items_by_state(AgentState::Ready)
            .await
            .expect("Failed to load by state");

        // Should get exactly 2 items
        assert_eq!(ready_items.len(), 2);
        for item in ready_items {
            assert_eq!(format!("{:?}", item.state), "Ready");
        }

        // Query for Active state
        let active_items = storage
            .load_work_items_by_state(AgentState::Active)
            .await
            .expect("Failed to load by state");

        // Should get exactly 1 item
        assert_eq!(active_items.len(), 1);
        assert_eq!(format!("{:?}", active_items[0].state), "Active");
    }

    #[tokio::test]
    async fn test_delete_work_item() {
        let (storage, _temp_dir) = create_test_storage().await;

        // Create and store work item
        let item = create_test_work_item();
        let item_id = item.id.clone();
        storage
            .store_work_item(&item)
            .await
            .expect("Failed to store work item");

        // Verify it exists
        storage
            .load_work_item(&item_id)
            .await
            .expect("Work item should exist");

        // Delete it
        storage
            .delete_work_item(&item_id)
            .await
            .expect("Failed to delete work item");

        // Try to load (should fail)
        let result = storage.load_work_item(&item_id).await;
        assert!(result.is_err(), "Loading deleted work item should fail");
    }

    #[tokio::test]
    async fn test_load_nonexistent_work_item() {
        let (storage, _temp_dir) = create_test_storage().await;

        // Try to load non-existent work item
        let fake_id = WorkItemId::new();
        let result = storage.load_work_item(&fake_id).await;
        assert!(result.is_err(), "Loading non-existent work item should error");
    }

    #[tokio::test]
    async fn test_complex_field_handling() {
        let (storage, _temp_dir) = create_test_storage().await;

        // Create work item with complex fields
        let mut item = create_test_work_item();

        // Multiple dependencies
        item.dependencies = vec![
            WorkItemId::new(),
            WorkItemId::new(),
            WorkItemId::new(),
        ];

        // Multiple review feedback entries
        item.review_feedback = Some(vec![
            "Critical issue in module A".to_string(),
            "Missing tests for async functions".to_string(),
            "Documentation incomplete for API endpoints".to_string(),
            "Anti-pattern detected: TODO markers".to_string(),
        ]);

        // Multiple suggested tests
        item.suggested_tests = Some(vec![
            "Add tests for error handling".to_string(),
            "Add tests for boundary conditions".to_string(),
            "Add integration tests".to_string(),
        ]);

        // Multiple execution memory IDs
        item.execution_memory_ids = vec![
            MemoryId::new(),
            MemoryId::new(),
            MemoryId::new(),
            MemoryId::new(),
        ];

        // Multiple file scope paths
        item.file_scope = Some(vec![
            PathBuf::from("/src/module1/file1.rs"),
            PathBuf::from("/src/module1/file2.rs"),
            PathBuf::from("/src/module2/file3.rs"),
            PathBuf::from("/tests/integration_test.rs"),
        ]);

        let item_id = item.id.clone();

        // Store it
        storage
            .store_work_item(&item)
            .await
            .expect("Failed to store complex work item");

        // Load it back
        let loaded_item = storage
            .load_work_item(&item_id)
            .await
            .expect("Failed to load complex work item");

        // Verify all complex fields
        assert_eq!(loaded_item.dependencies.len(), 3);
        assert_eq!(loaded_item.review_feedback.as_ref().unwrap().len(), 4);
        assert_eq!(loaded_item.suggested_tests.as_ref().unwrap().len(), 3);
        assert_eq!(loaded_item.execution_memory_ids.len(), 4);
        assert_eq!(loaded_item.file_scope.as_ref().unwrap().len(), 4);
    }

    #[tokio::test]
    async fn test_work_item_with_minimal_fields() {
        let (storage, _temp_dir) = create_test_storage().await;

        // Create work item with only required fields
        let item = WorkItem::new(
            "Minimal work item".to_string(),
            AgentRole::Optimizer,
            Phase::PromptToSpec,
            1,
        );

        let item_id = item.id.clone();

        // Store it
        storage
            .store_work_item(&item)
            .await
            .expect("Failed to store minimal work item");

        // Load it back
        let loaded_item = storage
            .load_work_item(&item_id)
            .await
            .expect("Failed to load minimal work item");

        // Verify fields
        assert_eq!(loaded_item.description, "Minimal work item");
        assert_eq!(format!("{:?}", loaded_item.agent), "Optimizer");
        assert_eq!(format!("{:?}", loaded_item.phase), "PromptToSpec");
        assert_eq!(loaded_item.priority, 1);
        assert_eq!(loaded_item.dependencies.len(), 0);
        assert_eq!(loaded_item.review_feedback, None);
        assert_eq!(loaded_item.suggested_tests, None);
        assert_eq!(loaded_item.review_attempt, 0);
    }

    #[tokio::test]
    async fn test_work_item_state_transitions() {
        let (storage, _temp_dir) = create_test_storage().await;

        // Create work item
        let mut item = create_test_work_item();
        let item_id = item.id.clone();
        storage.store_work_item(&item).await.unwrap();

        // Transition through states
        let states = vec![
            AgentState::Ready,
            AgentState::Active,
            AgentState::PendingReview,
            AgentState::Complete,
        ];

        for state in states {
            item.state = state;
            storage.update_work_item(&item).await.unwrap();

            let loaded = storage.load_work_item(&item_id).await.unwrap();
            assert_eq!(format!("{:?}", loaded.state), format!("{:?}", state));
        }
    }

    #[tokio::test]
    async fn test_work_item_review_attempts() {
        let (storage, _temp_dir) = create_test_storage().await;

        // Create work item
        let mut item = create_test_work_item();
        item.review_attempt = 0;
        let item_id = item.id.clone();
        storage.store_work_item(&item).await.unwrap();

        // Simulate multiple review failures
        for attempt in 1..=5 {
            item.review_attempt = attempt;
            item.review_feedback = Some(vec![format!("Issue from attempt {}", attempt)]);
            storage.update_work_item(&item).await.unwrap();

            let loaded = storage.load_work_item(&item_id).await.unwrap();
            assert_eq!(loaded.review_attempt, attempt);
            assert_eq!(loaded.review_feedback.as_ref().unwrap().len(), 1);
        }
    }
}
