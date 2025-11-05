//! HTTP API server command

use mnemosyne_core::{
    api::{ApiServer, ApiServerConfig},
    error::Result,
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

    println!();
    println!("🌐 Mnemosyne API Server");
    println!("   Real-time event streaming and state coordination");
    println!();
    println!("   Address: http://{}", socket_addr);
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
    println!("   mnemosyne-dash --api http://{}", socket_addr);
    println!();

    let server = ApiServer::new(config);
    server.serve().await?;

    Ok(())
}
