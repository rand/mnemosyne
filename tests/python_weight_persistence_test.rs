//! Verification test for Python bindings weight persistence logic
//!
//! This test verifies the exact flow used by the Python bindings:
//! 1. Initialize RelevanceScorer
//! 2. Store weights (mimicking what update_weights does internally)
//! 3. Retrieve weights (mimicking get_weights)

use mnemosyne_core::evaluation::relevance_scorer::{RelevanceScorer, Scope};
use mnemosyne_core::evaluation::WeightSet;
use tempfile::TempDir;

#[tokio::test]
async fn test_python_bindings_weight_persistence_flow() {
    // 1. Initialize Scorer (simulating PyRelevanceScorer::new)
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("python_test.db");
    let scorer = RelevanceScorer::new(db_path.to_str().unwrap().to_string());
    
    // Initialize schema (Python bindings might do this or expect it done)
    scorer.init_schema().await.expect("Failed to init schema");

    // 2. Store weights (simulating the persistence part of update_weights)
    // In Python: update_weights -> scorer.update_weights -> store_weights
    let mut weights = WeightSet::default_for_scope(
        Scope::Session,
        "test-session-py".to_string(),
        "skill".to_string(),
        "optimizer".to_string(),
    );
    
    // Modify some weights to ensure we're getting back what we wrote
    weights.weights.insert("keyword_match".to_string(), 0.42);
    weights.weights.insert("recency".to_string(), 0.17);
    weights.sample_count = 1;
    weights.update_confidence();

    scorer.store_weights(&weights).await
        .expect("Failed to store weights");

    // 3. Retrieve weights (simulating PyRelevanceScorer::get_weights)
    // In Python: get_weights -> scorer.get_weights_with_fallback
    let retrieved = scorer
        .get_weights_with_fallback(
            Scope::Session,
            "test-session-py",
            "skill",
            "optimizer",
            None,
            None,
            None,
        )
        .await
        .expect("Failed to retrieve weights");

    // Verify
    assert_eq!(retrieved.scope, Scope::Session);
    assert_eq!(retrieved.scope_id, "test-session-py");
    assert_eq!(retrieved.context_type, "skill");
    assert_eq!(retrieved.agent_role, "optimizer");
    
    // Verify specific values persisted
    assert!((retrieved.weights["keyword_match"] - 0.42).abs() < 0.001);
    assert!((retrieved.weights["recency"] - 0.17).abs() < 0.001);
    
    println!("Successfully verified weight persistence flow!");
}
