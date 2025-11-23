//! Integration tests for Rust↔Python orchestration bridge
//!
//! Tests the complete flow:
//! 1. Supervision tree spawns Rust actors
//! 2. Python bridges auto-spawn and register
//! 3. Work delegated to Python agents
//! 4. Results returned to Rust
//! 5. Error handling and recovery

#[cfg(feature = "python")]
mod python_bridge_tests {
    use mnemosyne_core::api::{EventBroadcaster, StateManager};
    use mnemosyne_core::launcher::agents::AgentRole;
    use mnemosyne_core::orchestration::messages::OrchestratorMessage;
    use mnemosyne_core::orchestration::state::{Phase, WorkItem};
    use mnemosyne_core::orchestration::supervision::{SupervisionConfig, SupervisionTree};
    use mnemosyne_core::orchestration::{network, ClaudeAgentBridge};
    use mnemosyne_core::types::Namespace;
    use mnemosyne_core::{ConnectionMode, LibsqlStorage};
    use std::sync::Arc;
    use std::time::Duration;

    /// Test that Python bridges can be spawned and registered with actors
    #[tokio::test]
    #[ignore] // Requires Python environment with dependencies
    async fn test_python_bridge_spawn_and_registration() {
        // Initialize Python interpreter
        pyo3::prepare_freethreaded_python();

        // Setup
        let storage = Arc::new(
            LibsqlStorage::new(ConnectionMode::InMemory)
                .await
                .expect("Failed to create storage"),
        );
        let network = Arc::new(
            network::NetworkLayer::new()
                .await
                .expect("Failed to create network"),
        );

        // Create event broadcaster and state manager for dashboard integration
        let broadcaster = EventBroadcaster::new(100);
        let state_manager = Arc::new(StateManager::new());

        let namespace = Namespace::Session {
            project: "test-bridge".to_string(),
            session_id: "integration-test".to_string(),
        };

        // Create supervision tree with event broadcasting
        let config = SupervisionConfig::default();
        let mut tree = SupervisionTree::new_with_namespace_and_state(
            config,
            storage,
            network,
            namespace,
            Some(broadcaster.clone()),
            Some(state_manager.clone()),
        )
        .await
        .expect("Failed to create supervision tree");

        // Start supervision tree (should auto-spawn Python bridges)
        tree.start()
            .await
            .expect("Failed to start supervision tree");

        // Verify all actors are running
        assert!(tree.is_healthy().await, "Supervision tree not healthy");

        // Verify state manager shows all agents (from heartbeats)
        tokio::time::sleep(Duration::from_millis(500)).await;
        let agents = state_manager.list_agents().await;
        assert!(
            agents.len() >= 4,
            "Expected 4+ agents, got {}",
            agents.len()
        );

        // Cleanup
        tree.stop().await.expect("Failed to stop supervision tree");
    }

    /// Test work delegation to Python agent via bridge
    #[tokio::test]
    #[ignore] // Requires Python environment with ANTHROPIC_API_KEY
    async fn test_work_delegation_to_python_agent() {
        // Initialize Python interpreter
        pyo3::prepare_freethreaded_python();

        // Setup
        let storage = Arc::new(
            LibsqlStorage::new(ConnectionMode::InMemory)
                .await
                .expect("Failed to create storage"),
        );
        let network = Arc::new(
            network::NetworkLayer::new()
                .await
                .expect("Failed to create network"),
        );

        let broadcaster = EventBroadcaster::new(100);
        let state_manager = Arc::new(StateManager::new());

        let namespace = Namespace::Session {
            project: "test-delegation".to_string(),
            session_id: "work-test".to_string(),
        };

        // Create and start supervision tree
        let config = SupervisionConfig::default();
        let mut tree = SupervisionTree::new_with_namespace_and_state(
            config,
            storage.clone(),
            network,
            namespace.clone(),
            Some(broadcaster.clone()),
            Some(state_manager),
        )
        .await
        .expect("Failed to create supervision tree");

        tree.start()
            .await
            .expect("Failed to start supervision tree");

        // Create a simple work item
        let work_item = WorkItem::new(
            "Test work for Python agent".to_string(),
            AgentRole::Executor,
            Phase::PlanToArtifacts,
            5,
        );

        // Submit work to orchestrator
        let orchestrator = tree.orchestrator();
        orchestrator
            .cast(OrchestratorMessage::SubmitWork(Box::new(work_item)))
            .expect("Failed to submit work");

        // Wait for work to be processed
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Verify work was processed (check via storage for completed work items)
        // This is a basic smoke test - full validation would check actual execution

        // Cleanup
        tree.stop().await.expect("Failed to stop supervision tree");
    }

    /// Test bridge error handling and recovery
    #[tokio::test]
    async fn test_bridge_error_handling() {
        // Test that bridge spawn fails gracefully without Python initialization
        let broadcaster = EventBroadcaster::new(10);

        let result = ClaudeAgentBridge::spawn(AgentRole::Executor, broadcaster.sender()).await;

        assert!(
            result.is_err(),
            "Bridge spawn should fail without Python init"
        );

        // Verify error message is informative
        let err = result.unwrap_err();
        let err_msg = format!("{:?}", err);
        assert!(
            err_msg.contains("Python")
                || err_msg.contains("GIL")
                || err_msg.contains("import failed")
                || err_msg.contains("ModuleNotFoundError"),
            "Error should mention Python or import failure: {}",
            err_msg
        );
    }

    /// Test graceful degradation when Python bridge fails
    #[tokio::test]
    async fn test_graceful_degradation_without_python_bridges() {
        // Initialize Python but test graceful degradation when bridge spawn fails
        // (e.g., due to missing API key or module import errors)
        pyo3::prepare_freethreaded_python();

        let storage = Arc::new(
            LibsqlStorage::new(ConnectionMode::InMemory)
                .await
                .expect("Failed to create storage"),
        );
        let network = Arc::new(
            network::NetworkLayer::new()
                .await
                .expect("Failed to create network"),
        );

        let broadcaster = EventBroadcaster::new(100);
        let state_manager = Arc::new(StateManager::new());

        let namespace = Namespace::Session {
            project: "test-degradation".to_string(),
            session_id: "degradation-test".to_string(),
        };

        // Create and start supervision tree
        // Python bridges will likely fail (missing modules/API key), but tree should continue
        let config = SupervisionConfig::default();
        let mut tree = SupervisionTree::new_with_namespace_and_state(
            config,
            storage,
            network,
            namespace,
            Some(broadcaster),
            Some(state_manager.clone()),
        )
        .await
        .expect("Failed to create supervision tree");

        // Should succeed even if Python bridges fail to initialize
        tree.start()
            .await
            .expect("Failed to start supervision tree (should degrade gracefully)");

        // Verify Rust actors are still running
        assert!(
            tree.is_healthy().await,
            "Rust actors should still be healthy"
        );

        // Verify state manager shows Rust agents (heartbeats from actors)
        tokio::time::sleep(Duration::from_millis(500)).await;
        let agents = state_manager.list_agents().await;
        assert!(
            agents.len() >= 4,
            "Expected 4+ Rust agents even if Python bridges fail"
        );

        // Cleanup
        tree.stop().await.expect("Failed to stop supervision tree");
    }

    /// Test concurrent work processing across multiple agents
    #[tokio::test]
    #[ignore] // Requires Python environment
    async fn test_concurrent_work_processing() {
        // Initialize Python interpreter
        pyo3::prepare_freethreaded_python();

        // Setup
        let storage = Arc::new(
            LibsqlStorage::new(ConnectionMode::InMemory)
                .await
                .expect("Failed to create storage"),
        );
        let network = Arc::new(
            network::NetworkLayer::new()
                .await
                .expect("Failed to create network"),
        );

        let broadcaster = EventBroadcaster::new(100);
        let state_manager = Arc::new(StateManager::new());

        let namespace = Namespace::Session {
            project: "test-concurrent".to_string(),
            session_id: "concurrent-test".to_string(),
        };

        // Create and start supervision tree
        let config = SupervisionConfig {
            max_concurrent_agents: 4,
            ..SupervisionConfig::default()
        };
        let mut tree = SupervisionTree::new_with_namespace_and_state(
            config,
            storage,
            network,
            namespace,
            Some(broadcaster),
            Some(state_manager),
        )
        .await
        .expect("Failed to create supervision tree");

        tree.start()
            .await
            .expect("Failed to start supervision tree");

        // Submit multiple work items concurrently
        let orchestrator = tree.orchestrator();
        for i in 0..3 {
            let work_item = WorkItem::new(
                format!("Concurrent work item {}", i),
                AgentRole::Executor,
                Phase::PlanToArtifacts,
                5,
            );
            orchestrator
                .cast(OrchestratorMessage::SubmitWork(Box::new(work_item)))
                .expect("Failed to submit work");
        }

        // Wait for concurrent processing
        tokio::time::sleep(Duration::from_secs(3)).await;

        // Verify all work was dispatched
        // (Full validation would check work queue state)

        // Cleanup
        tree.stop().await.expect("Failed to stop supervision tree");
    }
}
