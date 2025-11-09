///! Internal CLI commands for automation and hooks
///!
///! These commands are not intended for direct user interaction.
///! They are called by Claude Code hooks and automation scripts.

use mnemosyne_core::orchestration::events::AgentEvent;
use chrono::Utc;
use tracing::debug;

use super::event_bridge;

/// Handle session-started event
/// Called by session-start.sh hook when Claude Code session begins
pub async fn handle_session_started(instance_id: String) -> mnemosyne_core::Result<()> {
    debug!("Emitting SessionStarted event for instance: {}", instance_id);

    // Create SessionStarted event
    let event = AgentEvent::SessionStarted {
        instance_id: instance_id.clone(),
        timestamp: Utc::now(),
    };

    // Emit event via HTTP POST to API server
    if let Err(e) = event_bridge::emit_event(event).await {
        // Log error but don't fail the command
        // API server might not be ready yet (we're starting it)
        debug!("Failed to emit SessionStarted event: {}", e);
    }

    // Output success for hook script
    println!("Session started: {}", instance_id);

    Ok(())
}

/// Handle session-ended event
/// Called by session-end.sh hook when Claude Code session ends
pub async fn handle_session_ended(instance_id: String) -> mnemosyne_core::Result<()> {
    debug!("Emitting SessionEnded event for instance: {}", instance_id);

    // Create SessionEnded event
    let event = AgentEvent::SessionEnded {
        instance_id: instance_id.clone(),
        timestamp: Utc::now(),
    };

    // Emit event via HTTP POST to API server
    if let Err(e) = event_bridge::emit_event(event).await {
        // Log error but don't fail the command
        debug!("Failed to emit SessionEnded event: {}", e);
    }

    // Output success for hook script
    println!("Session ended: {}", instance_id);

    Ok(())
}
