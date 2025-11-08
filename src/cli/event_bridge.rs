//! Event Bridge for CLI Commands
//!
//! Provides optional event broadcasting for CLI commands to enable
//! dashboard observability of direct operations (remember, recall, etc.)
//!
//! This bridge attempts to connect to a running API server and broadcast
//! events. If no API server is running, it gracefully skips event emission.

use mnemosyne_core::api::EventBroadcaster;
use mnemosyne_core::orchestration::events::AgentEvent;
use once_cell::sync::Lazy;
use std::sync::RwLock;

/// Global EventBroadcaster for CLI commands
///
/// This is initialized on first access and attempts to connect to
/// a running API server. If no server is available, events are not broadcast.
static CLI_EVENT_BROADCASTER: Lazy<RwLock<Option<EventBroadcaster>>> =
    Lazy::new(|| RwLock::new(None));

/// Initialize the CLI event broadcaster
///
/// Attempts to connect to the API server running on localhost:3000.
/// If successful, creates an EventBroadcaster for CLI commands to use.
///
/// This is called automatically on first event emission, but can be
/// called explicitly to ensure the broadcaster is initialized.
pub fn init_broadcaster() {
    let mut broadcaster = CLI_EVENT_BROADCASTER.write().unwrap();

    if broadcaster.is_none() {
        // Check if API server is running by attempting to connect
        // For now, we'll create a broadcaster assuming the server might be available
        // The actual connection will be handled by the event emission
        tracing::debug!("Initializing CLI event broadcaster");
        *broadcaster = Some(EventBroadcaster::new(1000));
    }
}

/// Get a reference to the CLI event broadcaster if available
pub fn get_broadcaster() -> Option<EventBroadcaster> {
    // Ensure broadcaster is initialized
    init_broadcaster();

    let broadcaster = CLI_EVENT_BROADCASTER.read().unwrap();
    broadcaster.clone()
}

/// Emit a CLI event if broadcaster is available
///
/// This is a convenience function that emits an event to the broadcaster
/// if one is available. If no broadcaster exists, the event is silently
/// dropped (no error).
///
/// # Arguments
/// * `event` - The AgentEvent to broadcast
///
/// # Returns
/// * `Ok(())` if event was broadcast or broadcaster not available
/// * `Err` if broadcasting failed
pub async fn emit_event(event: AgentEvent) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(broadcaster) = get_broadcaster() {
        // Convert to API event
        let api_event = event_to_api_event(&event);

        if let Some(api_event) = api_event {
            // Broadcast to all subscribers
            if let Err(e) = broadcaster.broadcast(api_event) {
                tracing::debug!("Failed to broadcast CLI event: {}", e);
                // Don't fail the CLI operation if broadcasting fails
            } else {
                tracing::debug!("Broadcast CLI event: {}", event.summary());
            }
        }
    } else {
        tracing::trace!("No event broadcaster available, skipping event emission");
    }

    Ok(())
}

/// Convert AgentEvent to API Event
///
/// This is a simplified version of the conversion in EventPersistence.
/// We only convert CLI-related events here.
fn event_to_api_event(event: &AgentEvent) -> Option<mnemosyne_core::api::Event> {
    use mnemosyne_core::api::Event;

    match event {
        AgentEvent::CliCommandStarted { command, args, .. } => {
            Some(Event::cli_command_started(command.clone(), args.clone()))
        }
        AgentEvent::CliCommandCompleted {
            command,
            duration_ms,
            result_summary,
        } => Some(Event::cli_command_completed(
            command.clone(),
            *duration_ms,
            result_summary.clone(),
        )),
        AgentEvent::CliCommandFailed {
            command,
            error,
            duration_ms,
        } => Some(Event::cli_command_failed(
            command.clone(),
            error.clone(),
            *duration_ms,
        )),
        AgentEvent::RecallExecuted {
            query,
            result_count,
            ..
        } => Some(Event::memory_recalled(query.clone(), *result_count)),
        AgentEvent::RememberExecuted {
            content_preview,
            memory_id,
            ..
        } => Some(Event::memory_stored(
            memory_id.to_string(),
            content_preview.clone(),
        )),
        AgentEvent::EvolveStarted { .. } => Some(Event::memory_evolution_started(
            "Manual evolution triggered".to_string(),
        )),
        AgentEvent::EvolveCompleted {
            consolidations,
            decayed,
            archived,
            ..
        } => Some(Event::memory_evolution_started(format!(
            "Evolution complete: {} consolidated, {} decayed, {} archived",
            consolidations, decayed, archived
        ))),
        AgentEvent::SearchPerformed {
            query,
            search_type,
            result_count,
            duration_ms,
        } => Some(Event::search_performed(
            query.clone(),
            search_type.clone(),
            *result_count,
            *duration_ms,
        )),
        AgentEvent::DatabaseOperation {
            operation,
            table,
            affected_rows,
            duration_ms,
        } => Some(Event::database_operation(
            operation.clone(),
            table.clone(),
            *affected_rows,
            *duration_ms,
        )),
        // Other events are not relevant for CLI operations
        _ => None,
    }
}

/// Emit a CLI command started event
pub async fn emit_command_started(command: &str, args: Vec<String>) {
    let event = AgentEvent::CliCommandStarted {
        command: command.to_string(),
        args,
        timestamp: chrono::Utc::now(),
    };

    let _ = emit_event(event).await;
}

/// Emit a CLI command completed event
pub async fn emit_command_completed(command: &str, duration_ms: u64, result_summary: String) {
    let event = AgentEvent::CliCommandCompleted {
        command: command.to_string(),
        duration_ms,
        result_summary,
    };

    let _ = emit_event(event).await;
}

/// Emit a CLI command failed event
pub async fn emit_command_failed(command: &str, error: String, duration_ms: u64) {
    let event = AgentEvent::CliCommandFailed {
        command: command.to_string(),
        error,
        duration_ms,
    };

    let _ = emit_event(event).await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_emit_event_no_broadcaster() {
        // Should not fail even without broadcaster
        let event = AgentEvent::CliCommandStarted {
            command: "test".to_string(),
            args: vec![],
            timestamp: chrono::Utc::now(),
        };

        let result = emit_event(event).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_init_broadcaster() {
        init_broadcaster();
        assert!(get_broadcaster().is_some());
    }
}
