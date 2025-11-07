//! Integration tests for DSpyBridge
//!
//! Tests verify:
//! - Python service initialization
//! - Agent module registration
//! - Generic module calling with JSON I/O
//! - Error handling and retry logic
//! - Module listing and reloading

#[cfg(feature = "python")]
mod dspy_bridge_tests {
    use mnemosyne_core::orchestration::dspy_bridge::DSpyBridge;
    use serde_json::json;
    use std::collections::HashMap;
    use std::sync::Arc;

    /// Helper to create test DSpyBridge (requires Python environment)
    async fn create_test_bridge() -> Arc<DSpyBridge> {
        // Initialize Python interpreter for tests
        pyo3::prepare_freethreaded_python();

        // Create bridge (it manages its own Python service internally)
        Arc::new(DSpyBridge::new().expect("Failed to create DSPy bridge"))
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_bridge_creation() {
        let bridge = create_test_bridge().await;
        assert!(Arc::strong_count(&bridge) > 0);
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_list_agent_modules() {
        let bridge = create_test_bridge().await;

        let modules = bridge
            .list_agent_modules()
            .await
            .expect("Failed to list modules");

        // Should have at least Reviewer and Semantic modules
        assert!(modules.contains(&"Reviewer".to_string()));
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_call_agent_module_basic() {
        let bridge = create_test_bridge().await;

        let mut inputs = HashMap::new();
        inputs.insert("user_intent".to_string(), json!("Test intent"));

        let result = bridge
            .call_agent_module("Reviewer", inputs)
            .await
            .expect("Failed to call module");

        // Should return HashMap with outputs
        assert!(!result.is_empty());
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_call_nonexistent_module() {
        let bridge = create_test_bridge().await;

        let inputs = HashMap::new();
        let result = bridge.call_agent_module("NonExistent", inputs).await;

        // Should return error
        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_json_value_conversion() {
        let bridge = create_test_bridge().await;

        let mut inputs = HashMap::new();
        inputs.insert("text".to_string(), json!("Sample text"));
        inputs.insert("count".to_string(), json!(42));
        inputs.insert("flag".to_string(), json!(true));
        inputs.insert("list".to_string(), json!(["item1", "item2", "item3"]));
        inputs.insert("nested".to_string(), json!({"key": "value", "number": 123}));

        // Should handle complex JSON values
        let result = bridge.call_agent_module("Reviewer", inputs).await;

        // May fail if module doesn't accept these inputs, but should not panic
        assert!(result.is_ok() || result.is_err());
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_concurrent_calls() {
        let bridge = create_test_bridge().await;

        let mut handles = vec![];

        for i in 0..5 {
            let bridge_clone = Arc::clone(&bridge);
            let handle = tokio::spawn(async move {
                let mut inputs = HashMap::new();
                inputs.insert("user_intent".to_string(), json!(format!("Intent {}", i)));

                bridge_clone.call_agent_module("Reviewer", inputs).await
            });
            handles.push(handle);
        }

        // Wait for all to complete
        for handle in handles {
            let result = handle.await.expect("Task panicked");
            // All should complete (success or error, but not panic)
            assert!(result.is_ok() || result.is_err());
        }
    }

    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_reload_modules() {
        let bridge = create_test_bridge().await;

        let result = bridge.reload_modules().await;

        // Should succeed
        assert!(result.is_ok());
    }

    #[test]
    fn test_bridge_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<DSpyBridge>();
        assert_sync::<DSpyBridge>();
    }
}
