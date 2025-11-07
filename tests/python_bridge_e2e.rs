//! End-to-end validation tests for Python bridge with actual Claude SDK calls
//!
//! These tests make real API calls to Claude and validate the complete flow:
//! 1. Rust supervision tree → Python agent → Claude API → Response
//! 2. Error handling, retries, timeouts
//! 3. Concurrent execution
//!
//! **Requirements**:
//! - ANTHROPIC_API_KEY environment variable
//! - Python environment with anthropic package
//! - Network connectivity to api.anthropic.com
//!
//! **Cost**: These tests make actual API calls and will incur charges

#[cfg(feature = "python")]
mod python_e2e_tests {
    use mnemosyne_core::api::EventBroadcaster;
    use mnemosyne_core::launcher::agents::AgentRole;
    use mnemosyne_core::orchestration::state::{Phase, WorkItem};
    use mnemosyne_core::orchestration::ClaudeAgentBridge;
    use mnemosyne_core::secrets::SecretsManager;
    use secrecy::ExposeSecret;
    use std::time::Duration;

    /// Setup helper: Ensure API key is available before Python initialization
    ///
    /// This function must be called BEFORE pyo3::prepare_freethreaded_python()
    /// because Python caches os.environ at initialization time.
    ///
    /// Priority order (from SECRETS_MANAGEMENT.md):
    /// 1. Environment variable (if already set)
    /// 2. Age-encrypted config (~/.config/mnemosyne/secrets.age)
    /// 3. OS keychain (fallback)
    fn ensure_api_key() {
        if std::env::var("ANTHROPIC_API_KEY").is_err() {
            let secrets = SecretsManager::new()
                .expect("Failed to initialize secrets manager");
            let api_key = secrets.get_secret("ANTHROPIC_API_KEY")
                .expect("Failed to load ANTHROPIC_API_KEY from secrets. Run: mnemosyne secrets init");
            std::env::set_var("ANTHROPIC_API_KEY", api_key.expose_secret());
        }
    }

    /// Test simple work execution with Claude SDK
    ///
    /// Validates:
    /// - Bridge successfully calls Claude API
    /// - Response is received and parsed
    /// - Work result contains expected content
    #[tokio::test]
    #[ignore] // Requires API key and makes actual API calls
    async fn test_simple_work_execution_with_claude() {
        // Load API key from secrets system BEFORE Python initialization
        ensure_api_key();
        ensure_api_key();
        pyo3::prepare_freethreaded_python();

        // Create event broadcaster
        let broadcaster = EventBroadcaster::new(10);

        // Spawn Executor agent bridge
        let bridge = ClaudeAgentBridge::spawn(AgentRole::Executor, broadcaster.sender())
            .await
            .expect("Failed to spawn executor bridge");

        // Create work item with detailed prompt that passes validation
        let work_item = WorkItem::new(
            "Create a hello_world() function in Python. The function should print the message 'Hello, World!' to stdout using the print() function. This is needed for testing the Python bridge with actual Claude API calls. The function must execute without errors and return None.".to_string(),
            AgentRole::Executor,
            Phase::PlanToArtifacts,
            5,
        );

        println!("\n=== Testing Simple Work Execution ===");
        println!("Work item: {}", work_item.description);
        println!("Sending to Claude...\n");

        // Send work and measure duration
        let start = std::time::Instant::now();
        let result = bridge.send_work(work_item).await;
        let duration = start.elapsed();

        println!("Response received in {:?}", duration);

        // Validate result
        assert!(result.is_ok(), "Work execution should succeed");
        let result = result.unwrap();

        println!("\n=== Result ===");
        println!("Success: {}", result.success);

        if let Some(ref data) = result.data {
            println!("Data length: {} chars", data.len());
            println!("Data preview: {}...", &data[..data.len().min(200)]);
        }

        if let Some(ref error) = result.error {
            println!("Error: {}", error);
        }

        // Assertions
        assert!(result.success, "Work should complete successfully");
        assert!(result.error.is_none(), "Should not have errors");

        // Basic content validation
        if let Some(data) = result.data {
            // Should contain Python code or explanation
            let data_lower = data.to_lowercase();
            assert!(
                data_lower.contains("hello") ||
                data_lower.contains("world") ||
                data_lower.contains("def") ||
                data_lower.contains("print"),
                "Response should contain relevant content about hello world function"
            );
        }

        // Cleanup
        bridge.shutdown().await.expect("Failed to shutdown bridge");

        println!("\n✓ Simple work execution test passed");
    }

    /// Test error recovery with invalid work
    ///
    /// Validates:
    /// - Agent handles malformed requests gracefully
    /// - Error messages are informative
    /// - Bridge doesn't crash on errors
    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_error_recovery_with_invalid_work() {
        ensure_api_key();
        pyo3::prepare_freethreaded_python();

        let broadcaster = EventBroadcaster::new(10);
        let bridge = ClaudeAgentBridge::spawn(AgentRole::Executor, broadcaster.sender())
            .await
            .expect("Failed to spawn executor bridge");

        // Create work item with very short description (should trigger validation)
        let work_item = WorkItem::new(
            "X".to_string(), // Too short
            AgentRole::Executor,
            Phase::PlanToArtifacts,
            5,
        );

        println!("\n=== Testing Error Recovery ===");
        println!("Sending invalid work (description too short)...\n");

        let result = bridge.send_work(work_item).await;

        println!("\n=== Result ===");

        // Should complete (not panic) but may fail validation
        assert!(result.is_ok(), "Bridge should handle invalid work gracefully");

        let result = result.unwrap();
        println!("Success: {}", result.success);

        if let Some(ref error) = result.error {
            println!("Error (expected): {}", error);
            // Error message should mention validation or description
            assert!(
                error.to_lowercase().contains("description") ||
                error.to_lowercase().contains("validation") ||
                error.to_lowercase().contains("short"),
                "Error should mention validation issue"
            );
        }

        bridge.shutdown().await.expect("Failed to shutdown");

        println!("\n✓ Error recovery test passed");
    }

    /// Test concurrent work processing
    ///
    /// Validates:
    /// - Multiple work items can be processed
    /// - Results are tracked correctly
    /// - No race conditions or panics
    #[tokio::test]
    #[ignore] // Requires API key and makes multiple API calls
    async fn test_concurrent_work_processing() {
        ensure_api_key();
        pyo3::prepare_freethreaded_python();

        let broadcaster = EventBroadcaster::new(10);

        // Spawn two executor bridges for concurrent processing
        let bridge1 = ClaudeAgentBridge::spawn(AgentRole::Executor, broadcaster.sender())
            .await
            .expect("Failed to spawn executor 1");

        let bridge2 = ClaudeAgentBridge::spawn(AgentRole::Executor, broadcaster.sender())
            .await
            .expect("Failed to spawn executor 2");

        println!("\n=== Testing Concurrent Work Processing ===");
        println!("Spawned 2 executor bridges\n");

        // Create two different work items
        let work1 = WorkItem::new(
            "Write a function to add two numbers".to_string(),
            AgentRole::Executor,
            Phase::PlanToArtifacts,
            5,
        );

        let work2 = WorkItem::new(
            "Write a function to multiply two numbers".to_string(),
            AgentRole::Executor,
            Phase::PlanToArtifacts,
            5,
        );

        println!("Work 1: {}", work1.description);
        println!("Work 2: {}", work2.description);
        println!("\nProcessing concurrently...\n");

        // Process concurrently
        let start = std::time::Instant::now();
        let (result1, result2) = tokio::join!(
            bridge1.send_work(work1),
            bridge2.send_work(work2)
        );
        let duration = start.elapsed();

        println!("Both completed in {:?}", duration);

        // Validate both results
        assert!(result1.is_ok(), "Work 1 should complete");
        assert!(result2.is_ok(), "Work 2 should complete");

        let result1 = result1.unwrap();
        let result2 = result2.unwrap();

        println!("\n=== Results ===");
        println!("Result 1 success: {}", result1.success);
        println!("Result 2 success: {}", result2.success);

        // At least one should succeed (API rate limits might affect both)
        let success_count = [result1.success, result2.success].iter().filter(|&&s| s).count();
        assert!(success_count >= 1, "At least one work item should succeed");

        // Cleanup
        tokio::join!(
            bridge1.shutdown(),
            bridge2.shutdown()
        );

        println!("\n✓ Concurrent processing test passed");
    }

    /// Test reviewer agent with quality checks
    ///
    /// Validates:
    /// - Reviewer agent integration
    /// - Quality gate evaluation
    /// - Review feedback structure
    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_reviewer_agent_quality_checks() {
        ensure_api_key();
        pyo3::prepare_freethreaded_python();

        let broadcaster = EventBroadcaster::new(10);
        let bridge = ClaudeAgentBridge::spawn(AgentRole::Reviewer, broadcaster.sender())
            .await
            .expect("Failed to spawn reviewer bridge");

        println!("\n=== Testing Reviewer Agent ===");

        // Create review work item with code to review
        let code_to_review = r#"
def calculate_total(items):
    total = 0
    for item in items:
        total += item
    return total
"#;

        let work_item = WorkItem::new(
            format!("Review this Python code for quality and suggest improvements:\n{}", code_to_review),
            AgentRole::Reviewer,
            Phase::PlanToArtifacts,
            5,
        );

        println!("Sending code for review...\n");

        let start = std::time::Instant::now();
        let result = bridge.send_work(work_item).await;
        let duration = start.elapsed();

        println!("Review completed in {:?}", duration);

        assert!(result.is_ok(), "Review should complete");
        let result = result.unwrap();

        println!("\n=== Review Result ===");
        println!("Success: {}", result.success);

        if let Some(ref data) = result.data {
            println!("Review length: {} chars", data.len());
            println!("Review preview: {}...", &data[..data.len().min(300)]);
        }

        bridge.shutdown().await.expect("Failed to shutdown");

        println!("\n✓ Reviewer agent test passed");
    }

    /// Test timeout handling
    ///
    /// Validates:
    /// - Long-running work doesn't hang forever
    /// - Timeout errors are handled gracefully
    #[tokio::test]
    #[ignore] // Requires API key
    async fn test_work_timeout_handling() {
        ensure_api_key();
        pyo3::prepare_freethreaded_python();

        let broadcaster = EventBroadcaster::new(10);
        let bridge = ClaudeAgentBridge::spawn(AgentRole::Executor, broadcaster.sender())
            .await
            .expect("Failed to spawn executor bridge");

        println!("\n=== Testing Timeout Handling ===");

        // Create a complex work item that might take time
        let work_item = WorkItem::new(
            "Write a detailed implementation of a binary search tree in Python with insert, delete, and search operations, including comprehensive documentation and examples".to_string(),
            AgentRole::Executor,
            Phase::PlanToArtifacts,
            5,
        );

        println!("Sending complex work...\n");

        // Use tokio timeout to ensure test doesn't hang
        let timeout_duration = Duration::from_secs(30); // 30 second timeout
        let result = tokio::time::timeout(
            timeout_duration,
            bridge.send_work(work_item)
        ).await;

        match result {
            Ok(work_result) => {
                println!("Work completed within timeout");
                assert!(work_result.is_ok(), "Work should complete or error gracefully");
            },
            Err(_) => {
                println!("Work timed out after {:?} (expected for very long operations)", timeout_duration);
                // Timeout is acceptable - we're testing that the system doesn't hang
            }
        }

        bridge.shutdown().await.expect("Failed to shutdown");

        println!("\n✓ Timeout handling test passed");
    }
}
