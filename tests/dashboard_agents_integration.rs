//! Dashboard-Agent Integration Tests
//!
//! Tests to verify that agents are immediately visible to the dashboard
//! via the API endpoints, ensuring no race conditions between agent startup
//! and dashboard connection.

use mnemosyne_core::{
    api::{EventBroadcaster, StateManager},
    orchestration::*,
    storage::StorageBackend,
    ConnectionMode, LibsqlStorage,
};
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::timeout;

/// Helper to create test storage with temporary database
async fn create_test_storage() -> (Arc<LibsqlStorage>, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let storage = LibsqlStorage::new_with_validation(
        ConnectionMode::Local(db_path.to_str().unwrap().to_string()),
        true, // create_if_missing
    )
    .await
    .expect("Failed to create test storage");

    (Arc::new(storage), temp_dir)
}

/// Test that agents become visible via API within 1 second of spawning
#[tokio::test]
async fn test_agents_visible_within_one_second() {
    let (storage, _temp) = create_test_storage().await;

    // Create event broadcaster and state manager
    let event_broadcaster = EventBroadcaster::default(); // 1000 event capacity
    let state_manager = Arc::new(StateManager::new());

    // Subscribe state manager to event stream
    let event_rx = event_broadcaster.subscribe();
    state_manager.subscribe_to_events(event_rx);

    // Create and start orchestration engine with event broadcasting
    let config = SupervisionConfig::default();
    let mut engine = OrchestrationEngine::new_with_state(
        storage,
        config,
        Some(event_broadcaster),
        Some(state_manager.clone()),
    )
    .await
    .expect("Failed to create engine");

    // Start timer
    let start = std::time::Instant::now();

    // Start engine (spawns all 4 agents)
    engine.start().await.expect("Failed to start engine");

    // Poll state manager until all 4 agents are visible or timeout
    let result = timeout(Duration::from_secs(1), async {
        loop {
            let agents = state_manager.list_agents().await;
            if agents.len() >= 4 {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;

    let elapsed = start.elapsed();

    // Assert all agents are visible
    assert!(
        result.is_ok(),
        "Agents should be visible within 1 second, but timed out after {:?}",
        elapsed
    );

    let agents = state_manager.list_agents().await;
    assert_eq!(
        agents.len(),
        4,
        "All 4 agents should be visible, found {}",
        agents.len()
    );

    println!("✅ All 4 agents visible in {:?}", elapsed);

    // Verify agent IDs contain expected roles
    let agent_ids: Vec<String> = agents.iter().map(|a| a.id.clone()).collect();
    assert!(
        agent_ids
            .iter()
            .any(|id| id.to_lowercase().contains("orchestrator")),
        "Should have orchestrator agent"
    );
    assert!(
        agent_ids
            .iter()
            .any(|id| id.to_lowercase().contains("optimizer")),
        "Should have optimizer agent"
    );
    assert!(
        agent_ids
            .iter()
            .any(|id| id.to_lowercase().contains("reviewer")),
        "Should have reviewer agent"
    );
    assert!(
        agent_ids
            .iter()
            .any(|id| id.to_lowercase().contains("executor")),
        "Should have executor agent"
    );

    engine.stop().await.expect("Failed to stop engine");
}

/// Test that dashboard connecting AFTER agents spawn still sees them immediately
#[tokio::test]
async fn test_late_dashboard_connection_sees_agents() {
    let (storage, _temp) = create_test_storage().await;

    // Create event broadcaster and state manager
    let event_broadcaster = EventBroadcaster::default();
    let state_manager = Arc::new(StateManager::new());

    // Create and start orchestration engine
    let config = SupervisionConfig::default();
    let mut engine = OrchestrationEngine::new_with_state(
        storage,
        config,
        Some(event_broadcaster.clone()), // Clone broadcaster for engine
        Some(state_manager.clone()),
    )
    .await
    .expect("Failed to create engine");

    // Start agents FIRST
    engine.start().await.expect("Failed to start engine");

    // Wait 500ms to simulate agents already running
    tokio::time::sleep(Duration::from_millis(500)).await;

    // NOW connect state manager (simulating late dashboard connection)
    let event_rx = event_broadcaster.subscribe();
    state_manager.subscribe_to_events(event_rx);

    // State manager should see agents immediately (from querying state)
    let start = std::time::Instant::now();

    let result = timeout(Duration::from_millis(500), async {
        loop {
            let agents = state_manager.list_agents().await;
            if agents.len() >= 4 {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await;

    let elapsed = start.elapsed();

    assert!(
        result.is_ok(),
        "Late-connecting dashboard should see agents within 500ms, took {:?}",
        elapsed
    );

    let agents = state_manager.list_agents().await;
    assert_eq!(
        agents.len(),
        4,
        "Late dashboard should see all 4 agents"
    );

    println!(
        "✅ Late dashboard saw all 4 agents in {:?} after connection",
        elapsed
    );

    engine.stop().await.expect("Failed to stop engine");
}

/// Test that HTTP API endpoint returns agents immediately
#[tokio::test]
#[ignore] // Requires API server running - run manually with `cargo test --ignored`
async fn test_http_api_shows_agents_immediately() {
    // This test assumes mnemosyne orchestrate --dashboard is running on localhost:3000
    let client = Client::new();

    // Query /state/agents endpoint
    let response = client
        .get("http://localhost:3000/state/agents")
        .send()
        .await
        .expect("Failed to connect to API");

    assert!(
        response.status().is_success(),
        "API should return 200 OK"
    );

    let agents: serde_json::Value = response.json().await.expect("Failed to parse JSON");

    // Should have 4 agents
    let agents_obj = agents.as_object().expect("Should be object");
    assert!(
        agents_obj.len() >= 4,
        "Should have at least 4 agents, found {}",
        agents_obj.len()
    );

    println!("✅ HTTP API shows {} agents", agents_obj.len());
}
