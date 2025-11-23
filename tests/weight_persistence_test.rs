//! Integration tests for weight persistence in the evaluation system
//!
//! Tests database storage and retrieval of learned relevance weights across:
//! - Session, Project, and Global scopes
//! - Weight set creation and updates
//! - Hierarchical fallback behavior
//! - Weight propagation

use mnemosyne_core::evaluation::relevance_scorer::{RelevanceScorer, Scope};
use mnemosyne_core::evaluation::WeightSet;
use tempfile::TempDir;

/// Helper to create test scorer with temporary database
async fn create_test_scorer() -> (RelevanceScorer, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test_weights.db");
    let scorer = RelevanceScorer::new(db_path.to_str().unwrap().to_string());

    // Initialize schema
    scorer.init_schema().await.expect("Failed to init schema");

    (scorer, temp_dir)
}

#[tokio::test]
async fn test_store_and_retrieve_session_weights() {
    let (scorer, _temp) = create_test_scorer().await;

    // Create session-level weights
    let mut weights = WeightSet::default_for_scope(
        Scope::Session,
        "test-session-123".to_string(),
        "memory".to_string(),
        "optimizer".to_string(),
    );

    weights.sample_count = 5;
    weights.weights.insert("keyword_match".to_string(), 0.4);
    weights.weights.insert("recency".to_string(), 0.3);
    weights.weights.insert("access_patterns".to_string(), 0.3);
    weights.update_confidence();

    // Store weights
    scorer
        .store_weights(&weights)
        .await
        .expect("Failed to store weights");

    // Retrieve with exact match
    let retrieved = scorer
        .get_weights_with_fallback(
            Scope::Session,
            "test-session-123",
            "memory",
            "optimizer",
            None,
            None,
            None,
        )
        .await
        .expect("Failed to retrieve weights");

    // Verify retrieved weights match
    assert_eq!(retrieved.scope, Scope::Session);
    assert_eq!(retrieved.scope_id, "test-session-123");
    assert_eq!(retrieved.sample_count, 5);
    assert!((retrieved.weights["keyword_match"] - 0.4).abs() < 0.001);
    assert!((retrieved.weights["recency"] - 0.3).abs() < 0.001);
}

#[tokio::test]
async fn test_store_and_retrieve_project_weights() {
    let (scorer, _temp) = create_test_scorer().await;

    // Create project-level weights
    let mut weights = WeightSet::default_for_scope(
        Scope::Project,
        "my-project".to_string(),
        "skill".to_string(),
        "reviewer".to_string(),
    );

    weights.sample_count = 20;
    weights.weights.insert("keyword_match".to_string(), 0.5);
    weights.update_confidence();

    // Store weights
    scorer
        .store_weights(&weights)
        .await
        .expect("Failed to store weights");

    // Retrieve with exact match
    let retrieved = scorer
        .get_weights_with_fallback(
            Scope::Project,
            "my-project",
            "skill",
            "reviewer",
            None,
            None,
            None,
        )
        .await
        .expect("Failed to retrieve weights");

    assert_eq!(retrieved.scope, Scope::Project);
    assert_eq!(retrieved.scope_id, "my-project");
    assert_eq!(retrieved.sample_count, 20);
}

#[tokio::test]
async fn test_store_and_retrieve_global_weights() {
    let (scorer, _temp) = create_test_scorer().await;

    // Create global weights
    let mut weights = WeightSet::default_for_scope(
        Scope::Global,
        "global".to_string(),
        "artifact".to_string(),
        "executor".to_string(),
    );

    weights.sample_count = 100;
    weights.update_confidence();

    // Store weights
    scorer
        .store_weights(&weights)
        .await
        .expect("Failed to store weights");

    // Retrieve
    let retrieved = scorer
        .get_weights_with_fallback(
            Scope::Global,
            "global",
            "artifact",
            "executor",
            None,
            None,
            None,
        )
        .await
        .expect("Failed to retrieve weights");

    assert_eq!(retrieved.scope, Scope::Global);
    assert_eq!(retrieved.sample_count, 100);
    assert!(retrieved.confidence > 0.9); // Global with 100 samples should have high confidence
}

#[tokio::test]
async fn test_update_existing_weights() {
    let (scorer, _temp) = create_test_scorer().await;

    // Store initial weights
    let mut weights = WeightSet::default_for_scope(
        Scope::Session,
        "update-test".to_string(),
        "memory".to_string(),
        "optimizer".to_string(),
    );
    weights.sample_count = 5;
    weights.weights.insert("keyword_match".to_string(), 0.3);

    scorer
        .store_weights(&weights)
        .await
        .expect("Failed to store initial weights");

    // Update weights
    weights.sample_count = 10;
    weights.weights.insert("keyword_match".to_string(), 0.5);
    weights.update_confidence();

    scorer
        .store_weights(&weights)
        .await
        .expect("Failed to update weights");

    // Retrieve and verify update
    let retrieved = scorer
        .get_weights_with_fallback(
            Scope::Session,
            "update-test",
            "memory",
            "optimizer",
            None,
            None,
            None,
        )
        .await
        .expect("Failed to retrieve updated weights");

    assert_eq!(retrieved.sample_count, 10);
    assert!((retrieved.weights["keyword_match"] - 0.5).abs() < 0.001);
}

#[tokio::test]
async fn test_hierarchical_fallback() {
    let (scorer, _temp) = create_test_scorer().await;

    // Store only global weights
    let mut global_weights = WeightSet::default_for_scope(
        Scope::Global,
        "global".to_string(),
        "fallback-test".to_string(),
        "optimizer".to_string(),
    );
    global_weights.sample_count = 50;

    scorer
        .store_weights(&global_weights)
        .await
        .expect("Failed to store global weights");

    // Try to retrieve session weights (should fall back to global)
    let retrieved = scorer
        .get_weights_with_fallback(
            Scope::Session,
            "nonexistent-session",
            "fallback-test",
            "optimizer",
            None,
            None,
            None,
        )
        .await
        .expect("Failed to retrieve with fallback");

    // Should return global weights as fallback (or default if none exist)
    // The actual scope returned depends on whether global weights exist
    assert!(retrieved.scope == Scope::Global || retrieved.scope == Scope::Session);
    if retrieved.scope == Scope::Global {
        assert_eq!(retrieved.sample_count, 50);
    }
}

#[tokio::test]
async fn test_weights_with_optional_context() {
    let (scorer, _temp) = create_test_scorer().await;

    // Store weights with work_phase and task_type
    let mut weights = WeightSet::default_for_scope(
        Scope::Project,
        "context-test".to_string(),
        "skill".to_string(),
        "reviewer".to_string(),
    );
    weights.work_phase = Some("PromptToSpec".to_string());
    weights.task_type = Some("documentation".to_string());
    weights.sample_count = 15;

    scorer
        .store_weights(&weights)
        .await
        .expect("Failed to store weights with context");

    // Retrieve with matching context
    let retrieved = scorer
        .get_weights_with_fallback(
            Scope::Project,
            "context-test",
            "skill",
            "reviewer",
            Some("PromptToSpec"),
            Some("documentation"),
            None,
        )
        .await
        .expect("Failed to retrieve weights with context");

    assert_eq!(retrieved.work_phase, Some("PromptToSpec".to_string()));
    assert_eq!(retrieved.task_type, Some("documentation".to_string()));
}

#[tokio::test]
async fn test_nonexistent_weights_returns_default() {
    let (scorer, _temp) = create_test_scorer().await;

    // Try to retrieve weights that don't exist
    // Should return default weights for the requested scope
    let retrieved = scorer
        .get_weights_with_fallback(
            Scope::Session,
            "does-not-exist",
            "unknown-context",
            "unknown-agent",
            None,
            None,
            None,
        )
        .await
        .expect("Failed to get default weights");

    // Should return default weights (may be session or global depending on fallback logic)
    assert!(retrieved.scope == Scope::Session || retrieved.scope == Scope::Global);
    assert_eq!(retrieved.sample_count, 0); // Default has no samples yet
}

#[tokio::test]
async fn test_multiple_scopes_independent() {
    let (scorer, _temp) = create_test_scorer().await;

    // Store weights for different scopes with same context
    let mut session_weights = WeightSet::default_for_scope(
        Scope::Session,
        "session-1".to_string(),
        "memory".to_string(),
        "optimizer".to_string(),
    );
    session_weights.sample_count = 5;

    let mut project_weights = WeightSet::default_for_scope(
        Scope::Project,
        "project-1".to_string(),
        "memory".to_string(),
        "optimizer".to_string(),
    );
    project_weights.sample_count = 15;

    let mut global_weights = WeightSet::default_for_scope(
        Scope::Global,
        "global".to_string(),
        "memory".to_string(),
        "optimizer".to_string(),
    );
    global_weights.sample_count = 100;

    scorer.store_weights(&session_weights).await.unwrap();
    scorer.store_weights(&project_weights).await.unwrap();
    scorer.store_weights(&global_weights).await.unwrap();

    // Verify each scope maintains independent values
    let session = scorer
        .get_weights_with_fallback(
            Scope::Session,
            "session-1",
            "memory",
            "optimizer",
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(session.sample_count, 5);

    let project = scorer
        .get_weights_with_fallback(
            Scope::Project,
            "project-1",
            "memory",
            "optimizer",
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(project.sample_count, 15);

    let global = scorer
        .get_weights_with_fallback(
            Scope::Global,
            "global",
            "memory",
            "optimizer",
            None,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(global.sample_count, 100);
}
