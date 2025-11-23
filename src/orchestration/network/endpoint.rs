//! Agent Endpoint - Iroh P2P Networking
//!
//! Each agent has an Iroh endpoint with:
//! - Keypair-based identity
//! - QUIC connections to other agents
//! - Automatic hole-punching + relay fallback

// Allow deprecation warnings for iroh 0.28.x
// We're already using the correct iroh::net::* paths, but they're re-exports
// from deprecated iroh::iroh_net in this version. Will be resolved when
// upgrading to iroh 0.29+.
#![allow(deprecated)]

use crate::error::{MnemosyneError, Result};
use iroh::base::node_addr::NodeAddr;
use iroh::base::ticket::NodeTicket;
use iroh::net::key::SecretKey;
use iroh::net::{Endpoint as IrohEndpoint, NodeId};
use std::str::FromStr;

/// Agent keypair for identity
#[derive(Clone)]
pub struct AgentKeypair {
    secret: SecretKey,
}

impl AgentKeypair {
    /// Generate a new random keypair
    pub fn generate() -> Self {
        Self {
            secret: SecretKey::generate(),
        }
    }

    /// Get the public node ID
    pub fn node_id(&self) -> NodeId {
        self.secret.public()
    }

    /// Get the secret key
    pub fn secret_key(&self) -> &SecretKey {
        &self.secret
    }
}

/// Agent endpoint wrapping Iroh QUIC networking
#[derive(Clone)]
pub struct AgentEndpoint {
    /// Iroh endpoint
    endpoint: IrohEndpoint,

    /// Agent keypair (WIP)
    #[allow(dead_code)]
    keypair: AgentKeypair,

    /// Node ID (cached)
    node_id: NodeId,
}

impl AgentEndpoint {
    /// Create a new agent endpoint
    pub async fn new() -> Result<Self> {
        let keypair = AgentKeypair::generate();
        let node_id = keypair.node_id();

        // Build Iroh endpoint
        let endpoint = IrohEndpoint::builder()
            .secret_key(keypair.secret_key().clone())
            .bind()
            .await
            .map_err(|e| MnemosyneError::NetworkError(e.to_string()))?;

        tracing::debug!("Agent endpoint created with node ID: {}", node_id);

        Ok(Self {
            endpoint,
            keypair,
            node_id,
        })
    }

    /// Get the node ID for this endpoint
    pub fn node_id(&self) -> String {
        self.node_id.to_string()
    }

    /// Get the endpoint's local addresses
    pub fn local_addrs(&self) -> Result<Vec<String>> {
        let (v4, v6) = self.endpoint.bound_sockets();
        let mut addrs = vec![v4.to_string()];
        if let Some(v6_addr) = v6 {
            addrs.push(v6_addr.to_string());
        }
        Ok(addrs)
    }

    /// Connect to another agent by node ID
    pub async fn connect(&self, node_addr: &NodeAddr) -> Result<iroh::net::endpoint::Connection> {
        tracing::info!("Connecting to agent: {}", node_addr.node_id);

        let conn = self
            .endpoint
            .connect(node_addr.clone(), b"mnemosyne-agent")
            .await
            .map_err(|e| MnemosyneError::NetworkError(e.to_string()))?;

        tracing::info!("Connected to agent: {}", node_addr.node_id);

        Ok(conn)
    }

    /// Open a bidirectional stream on a connection
    pub async fn open_stream(
        &self,
        conn: &iroh::net::endpoint::Connection,
    ) -> Result<(
        iroh::net::endpoint::SendStream,
        iroh::net::endpoint::RecvStream,
    )> {
        conn.open_bi()
            .await
            .map_err(|e| MnemosyneError::NetworkError(e.to_string()))
    }

    /// Accept incoming connections
    pub async fn accept(&self) -> Option<iroh::net::endpoint::Incoming> {
        self.endpoint.accept().await
    }

    /// Close the endpoint
    pub async fn close(self) -> Result<()> {
        // Close with graceful shutdown (code 0, no reason)
        self.endpoint
            .close(0u32.into(), b"shutdown")
            .await
            .map_err(|e| MnemosyneError::NetworkError(e.to_string()))?;
        Ok(())
    }

    /// Get the underlying Iroh endpoint
    pub fn inner(&self) -> &IrohEndpoint {
        &self.endpoint
    }

    /// Create a ticket for others to join
    pub async fn create_ticket(&self) -> Result<String> {
        let addr = self.endpoint.node_addr().await.map_err(|e| MnemosyneError::NetworkError(e.to_string()))?;
        let ticket = NodeTicket::new(addr);
        Ok(ticket.to_string())
    }

    /// Add a peer from a ticket
    pub async fn add_peer(&self, ticket_str: &str) -> Result<String> {
        let ticket = NodeTicket::from_str(ticket_str)
            .map_err(|e| MnemosyneError::NetworkError(format!("Invalid ticket: {}", e)))?;
        let addr = ticket.node_addr().clone();
        let node_id = addr.node_id.to_string();
        self.endpoint.add_node_addr(addr).map_err(|e| MnemosyneError::NetworkError(e.to_string()))?;
        Ok(node_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_endpoint_creation() {
        let endpoint = AgentEndpoint::new().await.unwrap();
        let node_id = endpoint.node_id();

        assert!(!node_id.is_empty());
        assert!(endpoint.local_addrs().is_ok());
    }

    #[tokio::test]
    async fn test_keypair_generation() {
        let keypair1 = AgentKeypair::generate();
        let keypair2 = AgentKeypair::generate();

        // Different keypairs should have different node IDs
        assert_ne!(keypair1.node_id(), keypair2.node_id());
    }

    #[tokio::test]
    async fn test_ticket_generation() {
        let endpoint = AgentEndpoint::new().await.unwrap();
        
        // Generate ticket
        let ticket_str = endpoint.create_ticket().await.unwrap();
        assert!(!ticket_str.is_empty());
        
        // Parse ticket manually to verify it's valid
        // We can't use add_peer on ourselves because iroh prevents adding our own address
        let ticket = NodeTicket::from_str(&ticket_str).expect("Failed to parse ticket");
        let addr = ticket.node_addr();
        
        assert_eq!(addr.node_id.to_string(), endpoint.node_id());
    }
}
