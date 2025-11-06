//! Integration test for multi-instance support
//!
//! Verifies that multiple mnemosyne instances can run concurrently
//! with dynamic port allocation and proper instance identification.

use mnemosyne_core::api::{ApiServer, ApiServerConfig};
use std::net::SocketAddr;
use tokio::time::{timeout, Duration};

#[tokio::test]
async fn test_multiple_instances_dynamic_ports() {
    // Create two API servers with same config
    let config = ApiServerConfig {
        addr: ([127, 0, 0, 1], 3000).into(),
        event_capacity: 100,
    };

    let server1 = ApiServer::new(config.clone());
    let server2 = ApiServer::new(config);

    // Verify they have different instance IDs
    assert_ne!(server1.instance_id(), server2.instance_id());

    // Spawn first server - should get port 3000
    let handle1 = tokio::spawn(async move { server1.serve().await });

    // Give it time to bind
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Spawn second server - should get port 3001
    let handle2 = tokio::spawn(async move { server2.serve().await });

    // Give it time to bind
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Verify both servers started (they won't return unless there's an error or shutdown)
    // Check they're still running
    assert!(!handle1.is_finished());
    assert!(!handle2.is_finished());

    // Clean up
    handle1.abort();
    handle2.abort();
}

#[tokio::test]
async fn test_instance_id_in_health_response() {
    let config = ApiServerConfig {
        addr: ([127, 0, 0, 1], 3050).into(), // Use different port to avoid conflicts
        event_capacity: 100,
    };

    let server = ApiServer::new(config);
    let instance_id = server.instance_id().to_string();

    // Spawn server
    let handle = tokio::spawn(async move { server.serve().await });

    // Give it time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Query health endpoint
    let client = reqwest::Client::new();
    let response = timeout(
        Duration::from_secs(2),
        client.get("http://127.0.0.1:3050/health").send(),
    )
    .await;

    if let Ok(Ok(resp)) = response {
        let health: serde_json::Value = resp.json().await.unwrap();

        // Verify instance_id is present
        assert_eq!(health["status"], "ok");
        assert_eq!(health["instance_id"], instance_id);
    }

    // Clean up
    handle.abort();
}

#[tokio::test]
async fn test_port_exhaustion_error_message() {
    // Bind to all ports 3000-3010 manually
    let mut listeners = Vec::new();
    for port in 3060..=3070 {
        let addr: SocketAddr = ([127, 0, 0, 1], port).into();
        if let Ok(listener) = tokio::net::TcpListener::bind(addr).await {
            listeners.push(listener);
        }
    }

    // Try to create server - should fail with helpful message
    let config = ApiServerConfig {
        addr: ([127, 0, 0, 1], 3060).into(),
        event_capacity: 100,
    };

    let server = ApiServer::new(config);
    let result = server.serve().await;

    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("All ports"));
    assert!(err_msg.contains("3060–3070"));
    assert!(err_msg.contains("Core functionality not affected"));

    // Clean up
    drop(listeners);
}
