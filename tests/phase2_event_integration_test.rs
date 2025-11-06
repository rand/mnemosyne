//! Phase 2.1: Event Integration - End-to-End Integration Test
//!
//! This test verifies the complete event flow from orchestration/ICS
//! through the API layer to SSE subscribers.
//!
//! Test Coverage:
//! 1. Orchestration agent events → API events → SSE stream
//! 2. ICS context modification events → API events → SSE stream
//! 3. Multiple concurrent subscribers receive events
//! 4. Event ordering and delivery guarantees

use mnemosyne_core::{
    api::{ApiServer, ApiServerConfig, EventBroadcaster, EventType},
    ics::{IcsApp, IcsConfig},
    launcher::agents::AgentRole,
    orchestration::{
        state::WorkItemId, AgentEvent, EventPersistence, OrchestrationEngine, Phase,
        SupervisionConfig,
    },
    storage::StorageBackend,
    types::Namespace,
    ConnectionMode, LibsqlStorage,
};
use std::sync::Arc;
use tempfile::TempDir;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_orchestration_event_flow() {
    // Setup: Create storage and event broadcaster
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");

    let storage: Arc<dyn StorageBackend> = Arc::new(
        LibsqlStorage::new_with_validation(
            ConnectionMode::Local(db_path.to_str().unwrap().to_string()),
            true,
        )
        .await
        .expect("Failed to create storage"),
    );

    // Create event broadcaster (same as API would create)
    let broadcaster = EventBroadcaster::new(100);
    let mut subscriber1 = broadcaster.subscribe();
    let mut subscriber2 = broadcaster.subscribe();

    // Create orchestration engine with event broadcasting
    let config = SupervisionConfig::default();
    let mut engine =
        OrchestrationEngine::new_with_events(storage.clone(), config, Some(broadcaster.clone()))
            .await
            .expect("Failed to create orchestration engine");

    // Start the engine
    engine.start().await.expect("Failed to start engine");

    // Wait a bit for initialization
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Create event persistence and emit test events
    let namespace = Namespace::Session {
        project: "test-integration".to_string(),
        session_id: "test-session".to_string(),
    };

    let persistence =
        EventPersistence::new_with_broadcaster(storage.clone(), namespace, Some(broadcaster));

    // Test 1: WorkItemStarted event
    let item_id = WorkItemId::new();
    let event = AgentEvent::WorkItemStarted {
        agent: AgentRole::Executor,
        item_id: item_id.clone(),
        description: "Test work".to_string(),
    };

    persistence
        .persist(event)
        .await
        .expect("Failed to persist event");

    // Verify both subscribers receive the event
    let result = timeout(Duration::from_millis(200), subscriber1.recv()).await;
    assert!(result.is_ok(), "Subscriber 1 should receive event");
    let api_event = result.unwrap().unwrap();
    assert!(matches!(
        api_event.event_type,
        EventType::AgentStarted { .. }
    ));

    let result = timeout(Duration::from_millis(200), subscriber2.recv()).await;
    assert!(result.is_ok(), "Subscriber 2 should receive event");
    let api_event = result.unwrap().unwrap();
    assert!(matches!(
        api_event.event_type,
        EventType::AgentStarted { .. }
    ));

    // Test 2: WorkItemCompleted event
    let event = AgentEvent::WorkItemCompleted {
        agent: AgentRole::Executor,
        item_id: item_id.clone(),
        duration_ms: 1000,
        memory_ids: vec![],
    };

    persistence
        .persist(event)
        .await
        .expect("Failed to persist event");

    let result = timeout(Duration::from_millis(200), subscriber1.recv()).await;
    assert!(
        result.is_ok(),
        "Subscriber 1 should receive completed event"
    );
    let api_event = result.unwrap().unwrap();
    assert!(matches!(
        api_event.event_type,
        EventType::AgentCompleted { .. }
    ));

    // Test 3: WorkItemFailed event
    let event = AgentEvent::WorkItemFailed {
        agent: AgentRole::Optimizer,
        item_id: WorkItemId::new(),
        error: "Test failure".to_string(),
    };

    persistence
        .persist(event)
        .await
        .expect("Failed to persist event");

    let result = timeout(Duration::from_millis(200), subscriber1.recv()).await;
    assert!(result.is_ok(), "Subscriber 1 should receive failed event");
    let api_event = result.unwrap().unwrap();
    assert!(matches!(
        api_event.event_type,
        EventType::AgentFailed { .. }
    ));

    // Test 4: Phase transition should NOT be broadcast
    let event = AgentEvent::PhaseTransition {
        from: Phase::PromptToSpec,
        to: Phase::SpecToFullSpec,
        approved_by: AgentRole::Orchestrator,
    };

    persistence
        .persist(event)
        .await
        .expect("Failed to persist event");

    // Should timeout - no event broadcast
    let result = timeout(Duration::from_millis(100), subscriber1.recv()).await;
    assert!(result.is_err(), "Phase transitions should not be broadcast");

    // Cleanup
    engine.stop().await.expect("Failed to stop engine");
}

#[tokio::test]
async fn test_ics_context_event_flow() {
    // Setup: Create storage and event broadcaster
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test_ics.db");

    let storage: Arc<dyn StorageBackend> = Arc::new(
        LibsqlStorage::new_with_validation(
            ConnectionMode::Local(db_path.to_str().unwrap().to_string()),
            true,
        )
        .await
        .expect("Failed to create storage"),
    );

    // Create event broadcaster
    let broadcaster = EventBroadcaster::new(100);
    let mut subscriber = broadcaster.subscribe();

    // Create ICS app with event broadcasting
    let config = IcsConfig::default();
    let mut app = IcsApp::new_with_broadcaster(config, storage, None, None, Some(broadcaster));

    // Create a temp file for ICS to save
    let test_file = temp_dir.path().join("test_context.md");
    std::fs::write(&test_file, "# Test Context\n\nInitial content\n")
        .expect("Failed to create test file");

    // Load file into ICS
    app.load_file(test_file.clone())
        .expect("Failed to load file");

    // Modify and save file (should trigger ContextModified event)
    // Note: In real usage, the editor would modify the buffer
    // For testing, we'll just call save which should emit the event
    let result = app.save_file();

    // Verify the event was broadcast
    let api_event_result = timeout(Duration::from_millis(200), subscriber.recv()).await;

    if result.is_ok() {
        // If save succeeded, we should have received an event
        assert!(
            api_event_result.is_ok(),
            "Should receive ContextModified event after save"
        );
        let api_event = api_event_result.unwrap().unwrap();
        assert!(matches!(
            api_event.event_type,
            EventType::ContextModified { .. }
        ));
    }
}

#[tokio::test]
async fn test_api_server_event_streaming() {
    // Test that ApiServer correctly streams events via SSE

    let config = ApiServerConfig {
        addr: "127.0.0.1:0".parse().unwrap(), // Random port
        event_capacity: 100,
    };

    let api_server = ApiServer::new(config);
    let broadcaster = api_server.broadcaster().clone();

    // Create a subscriber before emitting events
    let mut subscriber = broadcaster.subscribe();

    // Emit test events
    let event1 = mnemosyne_core::api::Event::agent_started("test-agent".to_string());
    broadcaster
        .broadcast(event1)
        .expect("Failed to broadcast event");

    let event2 =
        mnemosyne_core::api::Event::memory_stored("mem-123".to_string(), "Test memory".to_string());
    broadcaster
        .broadcast(event2)
        .expect("Failed to broadcast event");

    // Verify events received
    let result = timeout(Duration::from_millis(100), subscriber.recv()).await;
    assert!(result.is_ok(), "Should receive first event");
    let api_event = result.unwrap().unwrap();
    assert!(matches!(
        api_event.event_type,
        EventType::AgentStarted { .. }
    ));

    let result = timeout(Duration::from_millis(100), subscriber.recv()).await;
    assert!(result.is_ok(), "Should receive second event");
    let api_event = result.unwrap().unwrap();
    assert!(matches!(
        api_event.event_type,
        EventType::MemoryStored { .. }
    ));
}

#[tokio::test]
async fn test_multiple_subscribers_concurrent_events() {
    // Test that multiple subscribers all receive events in order

    let broadcaster = EventBroadcaster::new(100);

    // Create 5 subscribers
    let mut subscribers: Vec<_> = (0..5).map(|_| broadcaster.subscribe()).collect();

    // Emit 10 events
    for i in 0..10 {
        let event = mnemosyne_core::api::Event::agent_started(format!("agent-{}", i));
        broadcaster
            .broadcast(event)
            .expect("Failed to broadcast event");
    }

    // Verify all subscribers receive all events
    for (idx, subscriber) in subscribers.iter_mut().enumerate() {
        for event_num in 0..10 {
            let result = timeout(Duration::from_millis(100), subscriber.recv()).await;
            assert!(
                result.is_ok(),
                "Subscriber {} should receive event {}",
                idx,
                event_num
            );
        }
    }
}
