//! Event Helper Module for CLI Commands
//!
//! Provides reusable infrastructure to reduce boilerplate when emitting events
//! from CLI commands. Each command that uses these helpers goes from ~10 lines
//! of event emission code to ~3 lines.
//!
//! # Usage Patterns
//!
//! ## Pattern 1: Automatic Lifecycle (Started/Completed/Failed)
//!
//! ```rust
//! use super::event_helpers;
//!
//! pub async fn handle_status(...) -> Result<()> {
//!     event_helpers::with_event_lifecycle("status", vec![], async {
//!         // Your command logic here
//!         Ok(())
//!     }).await
//! }
//! ```
//!
//! ## Pattern 2: Domain-Specific Events
//!
//! ```rust
//! use mnemosyne_core::orchestration::events::AgentEvent;
//! use super::event_helpers;
//!
//! pub async fn handle_list(...) -> Result<()> {
//!     let results = perform_list();
//!
//!     // Emit domain-specific event
//!     event_helpers::emit_domain_event(AgentEvent::SearchPerformed {
//!         query: query.clone(),
//!         search_type: "list".to_string(),
//!         result_count: results.len(),
//!         duration_ms: elapsed.as_millis() as u64,
//!     }).await;
//!
//!     Ok(())
//! }
//! ```

use mnemosyne_core::error::Result;
use mnemosyne_core::orchestration::events::AgentEvent;
use std::future::Future;

use super::event_bridge;

/// Wraps a CLI command execution with automatic event emission
///
/// Emits CliCommandStarted before execution and CliCommandCompleted/Failed after.
/// Automatically measures execution duration and handles errors gracefully.
///
/// # Arguments
/// * `command` - The command name (e.g., "status", "list", "graph")
/// * `args` - Command arguments for tracing (e.g., vec!["--format=json"])
/// * `handler` - The async function containing the command logic
///
/// # Returns
/// * `Result<T>` - The result of the handler function
///
/// # Example
/// ```rust
/// pub async fn handle_status(...) -> Result<()> {
///     event_helpers::with_event_lifecycle("status", vec![], async {
///         // Command implementation
///         println!("Status: OK");
///         Ok(())
///     }).await
/// }
/// ```
pub async fn with_event_lifecycle<F, T>(command: &str, args: Vec<String>, handler: F) -> Result<T>
where
    F: Future<Output = Result<T>>,
{
    let start = std::time::Instant::now();

    // Emit started event
    event_bridge::emit_command_started(command, args.clone()).await;

    // Execute handler
    let result = handler.await;

    let duration_ms = start.elapsed().as_millis() as u64;

    // Emit completed or failed event
    match &result {
        Ok(_) => {
            event_bridge::emit_command_completed(command, duration_ms, "Success".to_string()).await;
        }
        Err(e) => {
            event_bridge::emit_command_failed(command, e.to_string(), duration_ms).await;
        }
    }

    result
}

/// Wraps a CLI command execution with automatic event emission and custom result summary
///
/// Similar to `with_event_lifecycle`, but allows customizing the success message.
///
/// # Arguments
/// * `command` - The command name
/// * `args` - Command arguments for tracing
/// * `handler` - The async function containing the command logic
/// * `result_summary_fn` - Function that generates a summary from the result
///
/// # Returns
/// * `Result<T>` - The result of the handler function
///
/// # Example
/// ```rust
/// pub async fn handle_list(...) -> Result<Vec<Memory>> {
///     event_helpers::with_event_lifecycle_and_summary(
///         "list",
///         vec!["--status=open".to_string()],
///         async {
///             let memories = fetch_memories().await?;
///             Ok(memories)
///         },
///         |memories| format!("Listed {} memories", memories.len())
///     ).await
/// }
/// ```
pub async fn with_event_lifecycle_and_summary<F, T, S>(
    command: &str,
    args: Vec<String>,
    handler: F,
    result_summary_fn: S,
) -> Result<T>
where
    F: Future<Output = Result<T>>,
    S: FnOnce(&T) -> String,
{
    let start = std::time::Instant::now();

    // Emit started event
    event_bridge::emit_command_started(command, args.clone()).await;

    // Execute handler
    let result = handler.await;

    let duration_ms = start.elapsed().as_millis() as u64;

    // Emit completed or failed event
    match &result {
        Ok(value) => {
            let summary = result_summary_fn(value);
            event_bridge::emit_command_completed(command, duration_ms, summary).await;
        }
        Err(e) => {
            event_bridge::emit_command_failed(command, e.to_string(), duration_ms).await;
        }
    }

    result
}

/// Emit a domain-specific event (e.g., RememberExecuted, RecallExecuted)
///
/// Use this for events that are specific to certain commands and provide
/// additional context beyond Started/Completed/Failed lifecycle events.
///
/// # Arguments
/// * `event` - The AgentEvent to emit
///
/// # Example
/// ```rust
/// // In a search command
/// event_helpers::emit_domain_event(AgentEvent::SearchPerformed {
///     query: query.clone(),
///     search_type: "hybrid".to_string(),
///     result_count: results.len(),
///     duration_ms: elapsed.as_millis() as u64,
/// }).await;
/// ```
///
/// # Error Handling
/// This function never fails. If event emission fails, it logs a debug message
/// and continues. CLI commands should never fail due to event emission issues.
pub async fn emit_domain_event(event: AgentEvent) {
    if let Err(e) = event_bridge::emit_event(event).await {
        tracing::debug!("Failed to emit domain event: {}", e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mnemosyne_core::MnemosyneError;

    #[tokio::test]
    async fn test_with_event_lifecycle_success() {
        // Should not panic or fail
        let result = with_event_lifecycle("test", vec![], async { Ok(42) }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_with_event_lifecycle_failure() {
        // Should propagate error
        let result: Result<i32> = with_event_lifecycle("test", vec![], async {
            Err(MnemosyneError::NotFound("test".to_string()))
        })
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_with_event_lifecycle_and_summary() {
        // Should use custom summary
        let result =
            with_event_lifecycle_and_summary("test", vec![], async { Ok(vec![1, 2, 3]) }, |v| {
                format!("Got {} items", v.len())
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), vec![1, 2, 3]);
    }

    #[tokio::test]
    async fn test_emit_domain_event() {
        // Should not panic
        let event = AgentEvent::SearchPerformed {
            query: "test".to_string(),
            search_type: "test".to_string(),
            result_count: 0,
            duration_ms: 0,
        };

        emit_domain_event(event).await;
    }
}
