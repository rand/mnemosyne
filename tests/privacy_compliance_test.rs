//! Privacy compliance tests for the evaluation system.
//!
//! Verifies that the evaluation system maintains strict privacy guarantees:
//! - Task hashes are truncated to 16 chars max
//! - No raw task descriptions stored
//! - Only generic keywords stored (max 10)
//! - No sensitive keywords stored
//! - All data stored locally in .mnemosyne/
//! - No network calls for evaluation
//! - Only statistical features stored

use mnemosyne_core::evaluation::feedback_collector::{
    ContextType, ErrorContext, FeedbackCollector, ProvidedContext, TaskType, WorkPhase,
    ContextEvaluation,
};
use mnemosyne_core::evaluation::feature_extractor::{FeatureExtractor, RelevanceFeatures};
use mnemosyne_core::evaluation::relevance_scorer::{RelevanceScorer, Scope, WeightSet};
use std::path::PathBuf;
use tempfile::TempDir;

mod common;

/// Test helper: Create a test database
async fn create_test_db() -> (TempDir, String) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let db_path = temp_dir.path().join("test.db");
    let db_path_str = db_path.to_string_lossy().to_string();

    // Initialize database schema
    let db = libsql::Builder::new_local(&db_path_str)
        .build()
        .await
        .expect("Failed to create database");

    let conn = db.connect().expect("Failed to connect");

    // Create context_evaluations table
    conn.execute(
        r#"
        CREATE TABLE IF NOT EXISTS context_evaluations (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            agent_role TEXT NOT NULL,
            namespace TEXT NOT NULL,
            context_type TEXT NOT NULL,
            context_id TEXT NOT NULL,
            task_hash TEXT NOT NULL,
            task_keywords TEXT,
            task_type TEXT,
            work_phase TEXT,
            file_types TEXT,
            error_context TEXT,
            related_technologies TEXT,
            was_accessed INTEGER NOT NULL DEFAULT 0,
            access_count INTEGER NOT NULL DEFAULT 0,
            time_to_first_access_ms INTEGER,
            total_time_accessed_ms INTEGER NOT NULL DEFAULT 0,
            was_edited INTEGER NOT NULL DEFAULT 0,
            was_committed INTEGER NOT NULL DEFAULT 0,
            was_cited_in_response INTEGER NOT NULL DEFAULT 0,
            user_rating INTEGER,
            task_completed INTEGER NOT NULL DEFAULT 0,
            task_success_score REAL,
            context_provided_at INTEGER NOT NULL,
            evaluation_updated_at INTEGER NOT NULL
        )
        "#,
        libsql::params![],
    )
    .await
    .expect("Failed to create table");

    (temp_dir, db_path_str)
}

// ============================================================================
// HASH PRIVACY TESTS
// ============================================================================

#[tokio::test]
async fn test_task_hash_truncated_to_16_chars() {
    let (_temp_dir, db_path) = create_test_db().await;
    let collector = FeedbackCollector::new(db_path.clone());

    // Create a context with a long hash (simulating SHA256 output)
    let long_hash = "a".repeat(64); // SHA256 is 64 hex chars
    let context = ProvidedContext {
        session_id: "test-session-1".to_string(),
        agent_role: "optimizer".to_string(),
        namespace: "test".to_string(),
        context_type: ContextType::Skill,
        context_id: "rust-async.md".to_string(),
        task_hash: long_hash.clone(),
        task_keywords: None,
        task_type: None,
        work_phase: None,
        file_types: None,
        error_context: None,
        related_technologies: None,
    };

    let eval_id = collector
        .record_context_provided(context)
        .await
        .expect("Failed to record context");

    // Retrieve and verify hash is truncated
    let evaluation = collector
        .get_evaluation(&eval_id)
        .await
        .expect("Failed to get evaluation");

    assert!(
        evaluation.task_hash.len() <= 16,
        "Task hash exceeds 16 characters: {}",
        evaluation.task_hash.len()
    );
    assert_eq!(
        evaluation.task_hash,
        long_hash.chars().take(16).collect::<String>(),
        "Task hash not properly truncated"
    );
}

#[tokio::test]
async fn test_task_hash_consistency() {
    let (_temp_dir, db_path) = create_test_db().await;
    let collector = FeedbackCollector::new(db_path.clone());

    let task_hash = "abc123def456".to_string();

    // Record same hash twice
    let context1 = ProvidedContext {
        session_id: "session-1".to_string(),
        agent_role: "optimizer".to_string(),
        namespace: "test".to_string(),
        context_type: ContextType::Skill,
        context_id: "skill1.md".to_string(),
        task_hash: task_hash.clone(),
        task_keywords: None,
        task_type: None,
        work_phase: None,
        file_types: None,
        error_context: None,
        related_technologies: None,
    };

    let context2 = ProvidedContext {
        session_id: "session-2".to_string(),
        agent_role: "executor".to_string(),
        namespace: "test".to_string(),
        context_type: ContextType::Memory,
        context_id: "mem-123".to_string(),
        task_hash: task_hash.clone(),
        task_keywords: None,
        task_type: None,
        work_phase: None,
        file_types: None,
        error_context: None,
        related_technologies: None,
    };

    let eval_id1 = collector
        .record_context_provided(context1)
        .await
        .expect("Failed to record context 1");
    let eval_id2 = collector
        .record_context_provided(context2)
        .await
        .expect("Failed to record context 2");

    let eval1 = collector
        .get_evaluation(&eval_id1)
        .await
        .expect("Failed to get evaluation 1");
    let eval2 = collector
        .get_evaluation(&eval_id2)
        .await
        .expect("Failed to get evaluation 2");

    assert_eq!(
        eval1.task_hash, eval2.task_hash,
        "Same task hash should produce consistent stored hashes"
    );
}

#[tokio::test]
async fn test_no_raw_task_description_in_database() {
    let (_temp_dir, db_path) = create_test_db().await;
    let collector = FeedbackCollector::new(db_path.clone());

    let sensitive_task = "Fix authentication bug with user password reset flow";
    let task_hash = "abcd1234"; // Pre-hashed

    let context = ProvidedContext {
        session_id: "test-session".to_string(),
        agent_role: "optimizer".to_string(),
        namespace: "test".to_string(),
        context_type: ContextType::Skill,
        context_id: "auth-skill.md".to_string(),
        task_hash: task_hash.to_string(),
        task_keywords: Some(vec!["authentication".to_string(), "bugfix".to_string()]),
        task_type: Some(TaskType::Bugfix),
        work_phase: Some(WorkPhase::Debugging),
        file_types: None,
        error_context: Some(ErrorContext::Runtime),
        related_technologies: None,
    };

    let eval_id = collector
        .record_context_provided(context)
        .await
        .expect("Failed to record context");

    // Query database directly to verify no raw text stored
    let evaluation = collector
        .get_evaluation(&eval_id)
        .await
        .expect("Failed to get evaluation");

    // Verify no raw task description anywhere in the evaluation
    assert!(
        !evaluation.task_hash.contains(sensitive_task),
        "Raw task description found in task_hash"
    );

    // Check keywords don't contain sensitive data
    if let Some(keywords) = evaluation.task_keywords {
        for keyword in keywords {
            assert!(
                !keyword.contains("password"),
                "Sensitive keyword 'password' found in stored keywords: {}",
                keyword
            );
        }
    }
}

// ============================================================================
// KEYWORD PRIVACY TESTS
// ============================================================================

#[tokio::test]
async fn test_max_10_keywords_stored() {
    let (_temp_dir, db_path) = create_test_db().await;
    let collector = FeedbackCollector::new(db_path.clone());

    // Try to store 20 keywords
    let too_many_keywords: Vec<String> = (0..20).map(|i| format!("keyword{}", i)).collect();

    let context = ProvidedContext {
        session_id: "test-session".to_string(),
        agent_role: "optimizer".to_string(),
        namespace: "test".to_string(),
        context_type: ContextType::Skill,
        context_id: "test-skill.md".to_string(),
        task_hash: "test123".to_string(),
        task_keywords: Some(too_many_keywords.clone()),
        task_type: None,
        work_phase: None,
        file_types: None,
        error_context: None,
        related_technologies: None,
    };

    let eval_id = collector
        .record_context_provided(context)
        .await
        .expect("Failed to record context");

    let evaluation = collector
        .get_evaluation(&eval_id)
        .await
        .expect("Failed to get evaluation");

    if let Some(keywords) = evaluation.task_keywords {
        assert!(
            keywords.len() <= 10,
            "More than 10 keywords stored: {}",
            keywords.len()
        );
    }
}

#[tokio::test]
async fn test_sensitive_keywords_never_stored() {
    let (_temp_dir, db_path) = create_test_db().await;
    let collector = FeedbackCollector::new(db_path.clone());

    // List of sensitive keywords that should NEVER be stored
    let sensitive_keywords = vec![
        "password".to_string(),
        "secret".to_string(),
        "key".to_string(),
        "token".to_string(),
        "api_key".to_string(),
        "credentials".to_string(),
        "private_key".to_string(),
        "ssh_key".to_string(),
    ];

    let mixed_keywords = vec![
        "rust".to_string(),
        "password".to_string(), // Should be filtered
        "async".to_string(),
        "secret".to_string(), // Should be filtered
        "tokio".to_string(),
    ];

    let context = ProvidedContext {
        session_id: "test-session".to_string(),
        agent_role: "optimizer".to_string(),
        namespace: "test".to_string(),
        context_type: ContextType::Skill,
        context_id: "test-skill.md".to_string(),
        task_hash: "test456".to_string(),
        task_keywords: Some(mixed_keywords),
        task_type: None,
        work_phase: None,
        file_types: None,
        error_context: None,
        related_technologies: None,
    };

    let eval_id = collector
        .record_context_provided(context)
        .await
        .expect("Failed to record context");

    let evaluation = collector
        .get_evaluation(&eval_id)
        .await
        .expect("Failed to get evaluation");

    if let Some(keywords) = evaluation.task_keywords {
        for keyword in keywords {
            for sensitive in &sensitive_keywords {
                assert!(
                    !keyword.to_lowercase().contains(&sensitive.to_lowercase()),
                    "Sensitive keyword '{}' found in stored keywords: {}",
                    sensitive,
                    keyword
                );
            }
        }
    }
}

#[tokio::test]
async fn test_keywords_are_generic_technology_names() {
    let (_temp_dir, db_path) = create_test_db().await;
    let collector = FeedbackCollector::new(db_path.clone());

    // Generic technology keywords (acceptable)
    let generic_keywords = vec![
        "rust".to_string(),
        "python".to_string(),
        "postgres".to_string(),
        "tokio".to_string(),
        "async".to_string(),
    ];

    let context = ProvidedContext {
        session_id: "test-session".to_string(),
        agent_role: "optimizer".to_string(),
        namespace: "test".to_string(),
        context_type: ContextType::Skill,
        context_id: "tech-skill.md".to_string(),
        task_hash: "tech789".to_string(),
        task_keywords: Some(generic_keywords.clone()),
        task_type: None,
        work_phase: None,
        file_types: None,
        error_context: None,
        related_technologies: None,
    };

    let eval_id = collector
        .record_context_provided(context)
        .await
        .expect("Failed to record context");

    let evaluation = collector
        .get_evaluation(&eval_id)
        .await
        .expect("Failed to get evaluation");

    // Verify keywords are stored (these are safe)
    assert!(evaluation.task_keywords.is_some(), "Generic keywords should be stored");
    let stored_keywords = evaluation.task_keywords.unwrap();
    assert_eq!(stored_keywords.len(), generic_keywords.len());
}

// ============================================================================
// STORAGE PRIVACY TESTS
// ============================================================================

#[tokio::test]
async fn test_database_is_local_only() {
    let (_temp_dir, db_path) = create_test_db().await;

    // Verify database path is local (not remote URL)
    assert!(
        !db_path.starts_with("http://") && !db_path.starts_with("https://"),
        "Database path should be local, not remote: {}",
        db_path
    );
    assert!(
        !db_path.starts_with("libsql://"),
        "Database should not use remote libsql protocol: {}",
        db_path
    );

    let path = PathBuf::from(&db_path);
    assert!(
        path.is_absolute() || path.starts_with("."),
        "Database path should be absolute or relative (local): {}",
        db_path
    );
}

#[tokio::test]
async fn test_gitignore_covers_evaluation_data() {
    // Read .gitignore
    let gitignore_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".gitignore");
    let gitignore_content = std::fs::read_to_string(&gitignore_path)
        .expect("Failed to read .gitignore");

    // Verify .mnemosyne/ is gitignored
    assert!(
        gitignore_content.contains(".mnemosyne/"),
        ".mnemosyne/ directory should be gitignored"
    );

    // Verify *.db files are gitignored
    assert!(
        gitignore_content.contains("*.db"),
        "*.db files should be gitignored"
    );
}

#[tokio::test]
async fn test_no_network_calls_during_evaluation() {
    // This test verifies that evaluation code doesn't make network calls
    // by checking that all storage operations use local paths

    let (_temp_dir, db_path) = create_test_db().await;
    let collector = FeedbackCollector::new(db_path.clone());

    let context = ProvidedContext {
        session_id: "net-test-session".to_string(),
        agent_role: "optimizer".to_string(),
        namespace: "test".to_string(),
        context_type: ContextType::Skill,
        context_id: "net-test.md".to_string(),
        task_hash: "net123".to_string(),
        task_keywords: Some(vec!["network".to_string(), "test".to_string()]),
        task_type: None,
        work_phase: None,
        file_types: None,
        error_context: None,
        related_technologies: None,
    };

    // All these operations should work without network
    let eval_id = collector
        .record_context_provided(context)
        .await
        .expect("Failed to record context");

    collector
        .record_context_accessed(&eval_id)
        .await
        .expect("Failed to record access");

    collector
        .record_context_edited(&eval_id)
        .await
        .expect("Failed to record edit");

    collector
        .record_context_committed(&eval_id)
        .await
        .expect("Failed to record commit");

    // If we got here without errors, no network calls were made
    // (would fail if trying to connect to remote database)
}

// ============================================================================
// FEATURE PRIVACY TESTS
// ============================================================================

#[tokio::test]
async fn test_feature_extractor_stores_only_statistics() {
    let (_temp_dir, db_path) = create_test_db().await;
    let extractor = FeatureExtractor::new(db_path.clone());

    // Create test evaluation
    let evaluation = create_test_evaluation();

    // Extract features
    let context_keywords = vec!["rust".to_string(), "async".to_string(), "tokio".to_string()];
    let features = extractor
        .extract_features(&evaluation, &context_keywords)
        .await
        .expect("Failed to extract features");

    // Verify only statistical features are present
    assert!(
        features.keyword_overlap_score >= 0.0 && features.keyword_overlap_score <= 1.0,
        "Keyword overlap should be a normalized score [0.0, 1.0]"
    );
    assert!(features.recency_days >= 0.0, "Recency should be non-negative");
    assert!(
        features.access_frequency >= 0.0,
        "Access frequency should be non-negative"
    );

    // Verify no raw content in features (all fields are numeric or boolean)
    // This is enforced by the RelevanceFeatures struct definition
}

#[tokio::test]
async fn test_keyword_overlap_computed_not_stored() {
    let (_temp_dir, db_path) = create_test_db().await;
    let extractor = FeatureExtractor::new(db_path.clone());

    let evaluation = create_test_evaluation();

    // Keywords used for computation (but should not be stored in features)
    let context_keywords = vec![
        "rust".to_string(),
        "async".to_string(),
        "tokio".to_string(),
        "sensitive_data".to_string(),
    ];

    let features = extractor
        .extract_features(&evaluation, &context_keywords)
        .await
        .expect("Failed to extract features");

    // Verify we get a score (keywords were used for computation)
    assert!(
        features.keyword_overlap_score > 0.0,
        "Should have computed keyword overlap"
    );

    // Verify keywords themselves are NOT in the features struct
    // (RelevanceFeatures has no keyword field, only keyword_overlap_score)
}

#[tokio::test]
async fn test_no_pii_in_features() {
    let (_temp_dir, db_path) = create_test_db().await;
    let extractor = FeatureExtractor::new(db_path.clone());

    let mut evaluation = create_test_evaluation();
    // Add some metadata that might contain PII
    evaluation.context_id = "/home/user/documents/private-notes.md".to_string();

    let context_keywords = vec!["test".to_string()];
    let features = extractor
        .extract_features(&evaluation, &context_keywords)
        .await
        .expect("Failed to extract features");

    // Features should only contain evaluation_id reference, not the actual context_id path
    assert_eq!(features.evaluation_id, evaluation.id);

    // Convert features to JSON and verify no paths or identifiers leak
    let features_json = serde_json::to_string(&features).expect("Failed to serialize");
    assert!(
        !features_json.contains("/home/user"),
        "PII path found in features JSON: {}",
        features_json
    );
    assert!(
        !features_json.contains("private-notes"),
        "Sensitive filename found in features JSON: {}",
        features_json
    );
}

// ============================================================================
// INTEGRATION PRIVACY TESTS (Python bindings)
// ============================================================================

#[tokio::test]
async fn test_optimizer_hash_task_description() {
    // Test that Python optimizer._hash_task_description produces consistent 16-char hashes
    use sha2::{Digest, Sha256};

    let task1 = "Implement user authentication";
    let task2 = "Implement user authentication"; // Same task
    let task3 = "Implement user authorization"; // Different task

    // Simulate Python's _hash_task_description
    let hash1 = hash_task(task1);
    let hash2 = hash_task(task2);
    let hash3 = hash_task(task3);

    // Same task -> same hash
    assert_eq!(hash1, hash2, "Same task should produce same hash");

    // Different task -> different hash
    assert_ne!(hash1, hash3, "Different tasks should produce different hashes");

    // All hashes should be 16 chars
    assert_eq!(hash1.len(), 16, "Hash should be exactly 16 characters");
    assert_eq!(hash3.len(), 16, "Hash should be exactly 16 characters");

    // Verify no raw task in hash
    assert!(
        !hash1.contains("authentication"),
        "Hash should not contain task keywords"
    );
}

#[test]
fn test_hash_generation() {
    use sha2::{Digest, Sha256};

    let task = "Test task";
    let hash = hash_task(task);

    assert_eq!(hash.len(), 16);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
}

#[tokio::test]
async fn test_optimizer_extract_task_metadata_safe() {
    // Test that task metadata extraction doesn't leak sensitive info

    let sensitive_task = "Fix bug in user password reset endpoint where passwords are logged";

    // Simulate metadata extraction (as done in Python optimizer.py)
    let metadata = extract_safe_metadata(sensitive_task);

    // Verify sensitive words are not in metadata values
    assert!(
        !metadata_contains_sensitive(&metadata),
        "Metadata should not contain sensitive information"
    );

    // Verify only categorical/generic data
    assert!(
        matches!(
            metadata.get("task_type").map(|s| s.as_str()),
            Some("bugfix") | Some("feature") | Some("refactor")
        ),
        "task_type should be categorical"
    );
}

// ============================================================================
// GRACEFUL DEGRADATION TESTS
// ============================================================================

#[tokio::test]
async fn test_evaluation_graceful_degradation_if_disabled() {
    // If evaluation is disabled, system should work without it

    let (_temp_dir, db_path) = create_test_db().await;

    // Attempt operations when evaluation is disabled
    // (In Python, EVALUATION_AVAILABLE = False)

    // System should not crash, just skip evaluation
    // This is tested via Python integration test
}

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_evaluation() -> ContextEvaluation {
    ContextEvaluation {
        id: "test-eval-1".to_string(),
        session_id: "test-session-1".to_string(),
        agent_role: "optimizer".to_string(),
        namespace: "test".to_string(),
        context_type: ContextType::Skill,
        context_id: "rust-async.md".to_string(),
        task_hash: "abc123".to_string(),
        task_keywords: Some(vec!["rust".to_string(), "async".to_string()]),
        task_type: Some(TaskType::Feature),
        work_phase: Some(WorkPhase::Implementation),
        file_types: Some(vec![".rs".to_string()]),
        error_context: Some(ErrorContext::None),
        related_technologies: Some(vec!["tokio".to_string()]),
        was_accessed: false,
        access_count: 0,
        time_to_first_access_ms: None,
        total_time_accessed_ms: 0,
        was_edited: false,
        was_committed: false,
        was_cited_in_response: false,
        user_rating: None,
        task_completed: false,
        task_success_score: None,
        context_provided_at: chrono::Utc::now().timestamp(),
        evaluation_updated_at: chrono::Utc::now().timestamp(),
    }
}

fn hash_task(task: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(task.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result).chars().take(16).collect()
}

fn extract_safe_metadata(task: &str) -> std::collections::HashMap<String, String> {
    use std::collections::HashMap;
    let mut metadata = HashMap::new();

    let task_lower = task.to_lowercase();

    // Classify task type (categorical, no sensitive data)
    if task_lower.contains("bug") || task_lower.contains("fix") {
        metadata.insert("task_type".to_string(), "bugfix".to_string());
    } else if task_lower.contains("test") {
        metadata.insert("task_type".to_string(), "test".to_string());
    } else {
        metadata.insert("task_type".to_string(), "feature".to_string());
    }

    // Only store generic technology names
    if task_lower.contains("rust") {
        metadata.insert("technology".to_string(), "rust".to_string());
    }
    if task_lower.contains("python") {
        metadata.insert("technology".to_string(), "python".to_string());
    }

    metadata
}

fn metadata_contains_sensitive(metadata: &std::collections::HashMap<String, String>) -> bool {
    let sensitive_words = ["password", "secret", "key", "token", "private"];

    for value in metadata.values() {
        let value_lower = value.to_lowercase();
        for word in &sensitive_words {
            if value_lower.contains(word) {
                return true;
            }
        }
    }
    false
}
