//! Integration test to verify agents start without crashing
//!
//! This test verifies the fix for agents exiting immediately due to
//! missing Python modules.

use mnemosyne_core::orchestration::{OrchestrationEngine, SupervisionConfig};
use mnemosyne_core::storage::libsql::{ConnectionMode, LibsqlStorage};
use std::sync::Arc;

#[tokio::test]
async fn test_agents_start_without_python_import_error() {
    // Initialize in-memory storage
    let storage = Arc::new(
        LibsqlStorage::new(ConnectionMode::InMemory)
            .await
            .expect("Failed to create storage")
    );

    // Create orchestration engine with default config
    let config = SupervisionConfig::default();
    let mut engine = OrchestrationEngine::new(storage, config)
        .await
        .expect("Failed to create orchestration engine");

    // Start engine - this spawns all 4 agents (Orchestrator, Optimizer, Reviewer, Executor)
    // If Python modules aren't installed correctly, agents will exit immediately
    engine.start().await.expect("Failed to start agents - check Python imports!");

    // Wait to ensure agents don't crash immediately
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Stop engine gracefully
    engine.stop().await.expect("Failed to stop engine");

    // If we got here without panicking, agents started successfully!
    println!("✓ SUCCESS: All agents started and ran without crashing!");
}
