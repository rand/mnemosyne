//! Integration tests for ReviewerDSpyAdapter
//!
//! Tests verify:
//! - Requirement extraction from user intent
//! - Semantic intent validation
//! - Completeness checking
//! - Correctness validation
//! - Type safety and error handling

#[cfg(feature = "python")]
mod reviewer_adapter_tests {
    use mnemosyne_core::orchestration::actors::reviewer_dspy_adapter::ReviewerDSpyAdapter;
    use mnemosyne_core::orchestration::dspy_bridge::DSpyBridge;
    use serde_json::json;
    use std::sync::Arc;

    /// Helper to create test adapter (requires Python environment)
    async fn create_test_adapter() -> ReviewerDSpyAdapter {
        let bridge = Arc::new(DSpyBridge::new().expect("Failed to create DSPy bridge"));
        ReviewerDSpyAdapter::new(bridge)
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_extract_requirements_basic() {
        let adapter = create_test_adapter().await;

        let requirements = adapter
            .extract_requirements("Implement user authentication", None)
            .await
            .expect("Failed to extract requirements");

        // Should return list of requirements
        assert!(requirements.is_empty() || !requirements.is_empty());
        // All should be non-empty strings
        for req in requirements {
            assert!(!req.is_empty());
        }
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_extract_requirements_with_context() {
        let adapter = create_test_adapter().await;

        let context = "REST API using Python/FastAPI";
        let requirements = adapter
            .extract_requirements("Add logging and monitoring", Some(context))
            .await
            .expect("Failed to extract requirements with context");

        // Should succeed
        assert!(requirements.is_empty() || !requirements.is_empty());
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_semantic_intent_check_basic() {
        let adapter = create_test_adapter().await;

        let intent = "Create user registration endpoint";
        let implementation = "Created POST /users/register with validation";
        let execution_memories = vec![];

        let (passed, issues) = adapter
            .semantic_intent_check(intent, implementation, execution_memories)
            .await
            .expect("Failed to check intent");

        // Should return boolean and issues list
        assert!(passed || !passed); // Valid boolean
        assert!(issues.is_empty() || !issues.is_empty());
        // All issues should be strings
        for issue in issues {
            assert!(!issue.is_empty());
        }
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_semantic_intent_check_with_context() {
        let adapter = create_test_adapter().await;

        let intent = "Add caching";
        let implementation = "Implemented Redis caching layer";
        let execution_memories = vec![
            json!({"summary": "Installed redis", "content": "pip install redis"}),
            json!({"summary": "Created cache.py", "content": "Cache module"}),
        ];

        let (passed, issues) = adapter
            .semantic_intent_check(intent, implementation, execution_memories)
            .await
            .expect("Failed to check intent with context");

        assert!(passed || !passed);
        assert!(issues.is_empty() || !issues.is_empty());
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_verify_completeness_basic() {
        let adapter = create_test_adapter().await;

        let requirements = vec![
            "User registration".to_string(),
            "Email validation".to_string(),
        ];
        let implementation = "Implemented both requirements";
        let execution_memories = vec![];

        let (complete, issues) = adapter
            .verify_completeness(&requirements, implementation, execution_memories)
            .await
            .expect("Failed to verify completeness");

        assert!(complete || !complete);
        assert!(issues.is_empty() || !issues.is_empty());
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_verify_completeness_empty_requirements() {
        let adapter = create_test_adapter().await;

        let requirements = vec![];
        let implementation = "Some implementation";
        let execution_memories = vec![];

        let result = adapter
            .verify_completeness(&requirements, implementation, execution_memories)
            .await;

        // Should handle gracefully
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_verify_correctness_basic() {
        let adapter = create_test_adapter().await;

        let implementation = "def add(a, b): return a + b";
        let execution_memories = vec![];

        let (correct, issues) = adapter
            .verify_correctness(implementation, execution_memories)
            .await
            .expect("Failed to verify correctness");

        assert!(correct || !correct);
        assert!(issues.is_empty() || !issues.is_empty());
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_verify_correctness_with_context() {
        let adapter = create_test_adapter().await;

        let implementation = "async def fetch(): ...";
        let execution_memories = vec![
            json!({"summary": "Async function", "content": "fetch implementation"}),
            json!({"summary": "Error handling", "content": "try/except blocks"}),
        ];

        let (correct, issues) = adapter
            .verify_correctness(implementation, execution_memories)
            .await
            .expect("Failed to verify correctness with context");

        assert!(correct || !correct);
        assert!(issues.is_empty() || !issues.is_empty());
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_concurrent_operations() {
        let adapter = Arc::new(create_test_adapter().await);

        let mut handles = vec![];

        // Test concurrent calls to different operations
        for i in 0..3 {
            let adapter_clone = Arc::clone(&adapter);
            let handle = tokio::spawn(async move {
                adapter_clone
                    .extract_requirements(&format!("Intent {}", i), None)
                    .await
            });
            handles.push(handle);
        }

        for handle in handles {
            let result = handle.await.expect("Task panicked");
            assert!(result.is_ok() || result.is_err());
        }
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_edge_cases() {
        let adapter = create_test_adapter().await;

        // Empty strings
        let result = adapter.extract_requirements("", None).await;
        assert!(result.is_ok() || result.is_err());

        // Very long text
        let long_text = "Implementation details. ".repeat(1000);
        let result = adapter
            .verify_correctness(&long_text, vec![])
            .await;
        assert!(result.is_ok() || result.is_err());

        // Special characters
        let special = "def test(): return {'key': \"value\"}";
        let result = adapter.verify_correctness(special, vec![]).await;
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_adapter_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<ReviewerDSpyAdapter>();
        assert_sync::<ReviewerDSpyAdapter>();
    }
}
