//! HTTP API server command

use super::event_helpers;
use mnemosyne_core::{
    api::{ApiServer, ApiServerConfig},
    error::Result,
    orchestration::events::AgentEvent,
};
use std::net::SocketAddr;
use tracing::debug;

/// Handle API server startup command
pub async fn handle(addr: String, capacity: usize) -> Result<()> {
    debug!("Starting HTTP API server...");

    let socket_addr: SocketAddr = addr
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid address '{}': {}", addr, e))?;
    let config = ApiServerConfig {
        addr: socket_addr,
        event_capacity: capacity,
    };

    let start_time = std::time::Instant::now();
    let instance_id = uuid::Uuid::new_v4().to_string();
    let listen_addr = format!("http://{}", socket_addr);

    // Emit server started event
    event_helpers::emit_domain_event(AgentEvent::ServerStarted {
        server_type: "api".to_string(),
        listen_addr: listen_addr.clone(),
        instance_id: instance_id.clone(),
    })
    .await;

    println!();
    println!("🌐 Mnemosyne API Server");
    println!("   Real-time event streaming and state coordination");
    println!();
    println!("   Address: {}", listen_addr);
    println!("   Event capacity: {}", capacity);
    println!();
    println!("   Endpoints:");
    println!("   - GET  /events - Server-Sent Events stream");
    println!("   - GET  /state/agents - List active agents");
    println!("   - POST /state/agents - Update agent state");
    println!("   - GET  /state/context-files - List context files");
    println!("   - POST /state/context-files - Update context file");
    println!("   - GET  /state/stats - System statistics");
    println!("   - GET  /health - Health check");
    println!();
    println!("   Dashboard:");
    println!("   mnemosyne-dash --api {}", listen_addr);
    println!();

    let server = ApiServer::new(config);
    let result = server.serve().await;

    // Emit server stopped event
    let uptime_ms = start_time.elapsed().as_millis() as u64;
    event_helpers::emit_domain_event(AgentEvent::ServerStopped {
        server_type: "api".to_string(),
        uptime_ms,
        requests_handled: 0, // TODO: Track request count if ApiServer provides metrics
    })
    .await;

    result.map_err(Into::into)
}
