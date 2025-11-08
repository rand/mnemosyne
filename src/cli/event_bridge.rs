//! Event Bridge for CLI Commands
//!
//! Provides optional event broadcasting for CLI commands to enable
//! dashboard observability of direct operations (remember, recall, etc.)
//!
//! This bridge POSTs events to the API server's /events/emit endpoint.
//! If no API server is running, it gracefully skips event emission.
//!
//! # Configuration
//!
//! - **MNEMOSYNE_DISABLE_EVENTS**: Set this environment variable to disable
//!   all event emission (useful when dashboard is not needed)
//!
//! # Connection Management
//!
//! Uses exponential backoff to avoid spamming connection attempts:
//! - First failure: retry after 60 seconds
//! - Subsequent failures: doubles up to 5 minutes (60s → 120s → 240s → 300s)
//! - Success: resets to quick 5-second checks

use mnemosyne_core::orchestration::events::AgentEvent;
use once_cell::sync::Lazy;
use std::sync::RwLock;

/// API server URL for event emission
const API_SERVER_URL: &str = "http://localhost:3000";

/// HTTP client for emitting events
static HTTP_CLIENT: Lazy<reqwest::Client> = Lazy::new(|| {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(500))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
});

/// Cache for API server availability check
/// Format: (last_check_time, is_available, consecutive_failures)
static API_SERVER_AVAILABLE: Lazy<RwLock<Option<(std::time::Instant, bool, u32)>>> =
    Lazy::new(|| RwLock::new(None));

/// Check if API server is available
///
/// Uses a cache with exponential backoff to avoid spamming connection attempts.
/// - Base cache duration: 60 seconds when unavailable
/// - Exponential backoff: doubles with each failure up to 5 minutes
/// - Quick recheck: 5 seconds when available
async fn is_api_server_available() -> bool {
    // Check if event emission is disabled via environment variable
    if std::env::var("MNEMOSYNE_DISABLE_EVENTS").is_ok() {
        return false;
    }

    // Check cache first
    {
        let cache = API_SERVER_AVAILABLE.read().unwrap();
        if let Some((last_check, is_available, consecutive_failures)) = *cache {
            // Calculate cache duration based on availability and failure count
            let cache_duration = if is_available {
                // Quick recheck when available (5s)
                std::time::Duration::from_secs(5)
            } else {
                // Exponential backoff when unavailable (60s, 120s, 240s, up to 300s)
                let backoff_secs = (60 * 2_u64.pow(consecutive_failures)).min(300);
                std::time::Duration::from_secs(backoff_secs)
            };

            if last_check.elapsed() < cache_duration {
                return is_available;
            }
        }
    }

    // Cache miss or expired - check server health
    let is_available = HTTP_CLIENT
        .get(format!("{}/health", API_SERVER_URL))
        .send()
        .await
        .map(|resp| resp.status().is_success())
        .unwrap_or(false);

    // Update cache with failure tracking
    {
        let mut cache = API_SERVER_AVAILABLE.write().unwrap();
        let consecutive_failures = if let Some((_, was_available, failures)) = *cache {
            if is_available {
                0 // Reset on success
            } else if was_available {
                1 // First failure
            } else {
                (failures + 1).min(4) // Increment up to 4 (max 5 minutes backoff)
            }
        } else {
            if is_available { 0 } else { 1 }
        };

        *cache = Some((std::time::Instant::now(), is_available, consecutive_failures));
    }

    is_available
}

/// Emit a CLI event to the API server
///
/// Posts the event to the API server's /events/emit endpoint.
/// If the server is not running, the event is silently dropped.
///
/// # Arguments
/// * `event` - The AgentEvent to broadcast
///
/// # Returns
/// * `Ok(())` whether event was sent successfully or not (never fails)
pub async fn emit_event(event: AgentEvent) -> Result<(), Box<dyn std::error::Error>> {
    // Check if server is available
    if !is_api_server_available().await {
        tracing::trace!("API server not available, skipping event emission");
        return Ok(());
    }

    // Convert to API event
    let api_event = match event_to_api_event(&event) {
        Some(e) => e,
        None => {
            tracing::trace!("Event type not mapped to API event, skipping");
            return Ok(());
        }
    };

    // POST to API server
    match HTTP_CLIENT
        .post(format!("{}/events/emit", API_SERVER_URL))
        .json(&api_event)
        .send()
        .await
    {
        Ok(response) => {
            if response.status().is_success() {
                tracing::debug!("Emitted CLI event: {}", event.summary());
            } else {
                tracing::debug!("API server rejected event: {}", response.status());
            }
        }
        Err(e) => {
            tracing::debug!("Failed to emit CLI event: {}", e);
            // Mark server as unavailable in cache and increment failure count
            let mut cache = API_SERVER_AVAILABLE.write().unwrap();
            let consecutive_failures = if let Some((_, _, failures)) = *cache {
                (failures + 1).min(4)
            } else {
                1
            };
            *cache = Some((std::time::Instant::now(), false, consecutive_failures));
        }
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

    #[tokio::test]
    async fn test_health_check() {
        // Health check should handle unavailable server gracefully
        let available = is_api_server_available().await;
        // Should return false when server not running, but shouldn't panic
        assert!(!available || available);
    }
}
