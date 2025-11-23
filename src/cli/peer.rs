//! Peer management commands

use clap::Subcommand;
use mnemosyne_core::daemon::{ipc, orchestration::OrchestrationDaemonConfig};
use mnemosyne_core::error::Result;

/// Peer management commands
#[derive(Subcommand, Debug, Clone)]
pub enum PeerAction {
    /// Create an invite ticket for this node
    Invite,

    /// Join a peer using an invite ticket
    Join {
        /// The invite ticket
        ticket: String,
    },
}

/// Handle peer commands
pub async fn handle(action: PeerAction) -> Result<()> {
    // Get the daemon socket path
    // We assume the default configuration for now, which uses standard paths
    let config = OrchestrationDaemonConfig::default();
    let socket_path = config.socket_path;

    if !socket_path.exists() {
        return Err(mnemosyne_core::error::MnemosyneError::Other(
            "Mnemosyne daemon is not running. Please start it with 'mnemosyne orchestrate --daemon'.".to_string(),
        ));
    }

    match action {
        PeerAction::Invite => {
            println!("Creating invite ticket...");
            match ipc::create_invite(&socket_path).await {
                Ok(ticket) => {
                    println!("Invite Ticket Created:");
                    println!("{}", ticket);
                    println!("\nShare this ticket with another peer to let them join.");
                }
                Err(e) => {
                    eprintln!("Failed to create invite: {}", e);
                }
            }
        }
        PeerAction::Join { ticket } => {
            println!("Joining peer...");
            match ipc::join_peer(&socket_path, ticket).await {
                Ok(node_id) => {
                    println!("Successfully connected to peer: {}", node_id);
                }
                Err(e) => {
                    eprintln!("Failed to join peer: {}", e);
                }
            }
        }
    }

    Ok(())
}
