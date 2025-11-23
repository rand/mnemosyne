//! Network Layer - Iroh P2P Communication
//!
//! Provides distributed agent communication using Iroh QUIC networking:
//! - Agent endpoints with keypair identity
//! - Protocol handlers for agent messages
//! - Hybrid routing (local Ractor vs remote Iroh)
//! - Peer discovery and connection management

pub mod endpoint;
pub mod protocol;
pub mod router;
pub mod transport;

pub use endpoint::{AgentEndpoint, AgentKeypair};
pub use protocol::AgentProtocol;
pub use router::MessageRouter;

use crate::error::Result;
use crate::launcher::agents::AgentRole;
use iroh::net::endpoint::{Incoming, RecvStream, SendStream};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Network layer managing all distributed communication
pub struct NetworkLayer {
    /// Local agent endpoint
    endpoint: Arc<RwLock<Option<AgentEndpoint>>>,

    /// Message router (WIP)
    #[allow(dead_code)]
    router: Arc<MessageRouter>,

    /// Whether network layer is started
    started: Arc<RwLock<bool>>,
}

impl NetworkLayer {
    /// Get the message router
    pub fn router(&self) -> Arc<MessageRouter> {
        self.router.clone()
    }

    /// Create a new network layer
    pub async fn new() -> Result<Self> {
        Ok(Self {
            endpoint: Arc::new(RwLock::new(None)),
            router: Arc::new(MessageRouter::new()),
            started: Arc::new(RwLock::new(false)),
        })
    }

    /// Start the network layer
    pub async fn start(&self) -> Result<()> {
        let mut started = self.started.write().await;
        if *started {
            return Ok(());
        }

        tracing::debug!("Starting network layer");

        // Create agent endpoint
        let endpoint = AgentEndpoint::new().await?;

        tracing::debug!("Agent endpoint created: {}", endpoint.node_id());

        // Store endpoint
        {
            let mut ep = self.endpoint.write().await;
            *ep = Some(endpoint.clone());
        }

        // Initialize and register transport
        let transport = Arc::new(transport::IrohTransport::new(self.endpoint.clone()));
        self.router.set_transport(transport).await;

        // Start listener loop
        let listener_endpoint = endpoint.clone();
        let listener_router = self.router.clone();
        tokio::spawn(async move {
            run_listener_loop(listener_endpoint, listener_router).await;
        });

        *started = true;

        tracing::debug!("Network layer started");
        Ok(())
    }

    /// Stop the network layer
    pub async fn stop(&self) -> Result<()> {
        let mut started = self.started.write().await;
        if !*started {
            return Ok(());
        }

        tracing::debug!("Stopping network layer");

        // Close endpoint
        // Note: Iroh may log warnings about closed channels during shutdown.
        // These are expected and harmless (background STUN probes being cancelled).
        {
            let mut ep = self.endpoint.write().await;
            if let Some(endpoint) = ep.take() {
                endpoint.close().await?;
            }
        }

        *started = false;

        tracing::debug!("Network layer stopped");
        Ok(())
    }

    /// Get endpoint node ID
    pub async fn node_id(&self) -> Option<String> {
        let ep = self.endpoint.read().await;
        ep.as_ref().map(|e| e.node_id())
    }

    /// Create an invite ticket for this node
    pub async fn create_invite(&self) -> Result<String> {
        let ep = self.endpoint.read().await;
        if let Some(endpoint) = ep.as_ref() {
            endpoint.create_ticket().await
        } else {
            Err(crate::error::MnemosyneError::NetworkError(
                "Network layer not started".to_string(),
            ))
        }
    }

    /// Get local addresses
    pub async fn local_addrs(&self) -> Result<Vec<String>> {
        let ep = self.endpoint.read().await;
        if let Some(endpoint) = ep.as_ref() {
            endpoint.local_addrs()
        } else {
            Ok(vec![])
        }
    }

    /// Join a peer using an invite ticket
    pub async fn join_peer(&self, ticket: &str) -> Result<String> {
        let ep = self.endpoint.read().await;
        if let Some(endpoint) = ep.as_ref() {
            endpoint.add_peer(ticket).await
        } else {
            Err(crate::error::MnemosyneError::NetworkError(
                "Network layer not started".to_string(),
            ))
        }
    }
}

/// Run the network listener loop
async fn run_listener_loop(endpoint: AgentEndpoint, router: Arc<MessageRouter>) {
    tracing::info!("Network listener loop started");
    loop {
        match endpoint.accept().await {
            Some(incoming) => {
                let router = router.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_connection(incoming, router).await {
                        tracing::warn!("Connection error: {}", e);
                    }
                });
            }
            None => {
                tracing::info!("Network listener loop stopped (endpoint closed)");
                break;
            }
        }
    }
}

/// Handle an incoming connection
async fn handle_connection(incoming: Incoming, router: Arc<MessageRouter>) -> Result<()> {
    // Accept connection
    let conn = incoming
        .await
        .map_err(|e| crate::error::MnemosyneError::NetworkError(e.to_string()))?;

    // Note: remote_node_id() seems unavailable or requires different access in this Iroh version
    // For now, we just log that we accepted a connection
    tracing::debug!("Accepted connection from remote peer");

    // Accept bidirectional streams
    loop {
        match conn.accept_bi().await {
            Ok((send, recv)) => {
                let router = router.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_stream(send, recv, router).await {
                        tracing::warn!("Stream error: {}", e);
                    }
                });
            }
            Err(e) => {
                // Connection closed or error
                tracing::debug!("Connection closed: {}", e);
                break;
            }
        }
    }

    Ok(())
}

/// Handle an incoming stream
async fn handle_stream(
    mut send: SendStream,
    mut recv: RecvStream,
    router: Arc<MessageRouter>,
) -> Result<()> {
    // Read message
    let msg = AgentProtocol::recv_message(&mut recv).await?;

    // Determine destination role
    let role = match &msg {
        crate::orchestration::messages::AgentMessage::Orchestrator(_) => AgentRole::Orchestrator,
        crate::orchestration::messages::AgentMessage::Optimizer(_) => AgentRole::Optimizer,
        crate::orchestration::messages::AgentMessage::Reviewer(_) => AgentRole::Reviewer,
        crate::orchestration::messages::AgentMessage::Executor(_) => AgentRole::Executor,
    };

    // Route message
    if let Err(e) = router.route(role, msg).await {
        tracing::warn!("Failed to route message to {:?}: {}", role, e);
        // We could send an error back, but for now just log it
    }

    // Close stream
    // Note: finish() returns Result, not Future in some Iroh versions
    // Check if we need await or not. The error said it's not a future.
    if let Err(e) = send.finish() {
        tracing::warn!("Failed to finish stream: {}", e);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_layer_lifecycle() {
        let layer = NetworkLayer::new().await.unwrap();

        // Test start/stop
        layer.start().await.unwrap();
        assert!(layer.node_id().await.is_some());

        layer.stop().await.unwrap();
    }
}
