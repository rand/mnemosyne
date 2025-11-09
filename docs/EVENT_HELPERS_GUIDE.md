# Event Helpers Guide

**Location**: `src/cli/event_helpers.rs`

## Overview

The event helpers module provides reusable infrastructure for emitting CLI events with minimal boilerplate. It reduces event emission code from ~10 lines to ~3 lines per command.

## Quick Start

### Pattern 1: Basic Command (Automatic Lifecycle)

For simple commands that just need Started/Completed/Failed events:

```rust
use super::event_helpers;

pub async fn handle_status(...) -> Result<()> {
    event_helpers::with_event_lifecycle("status", vec![], async {
        // Your command logic here
        println!("Status: OK");
        Ok(())
    }).await
}
```

### Pattern 2: Command with Custom Summary

For commands that return data and need a custom success message:

```rust
use super::event_helpers;

pub async fn handle_list(...) -> Result<Vec<Memory>> {
    event_helpers::with_event_lifecycle_and_summary(
        "list",
        vec![format!("--status={}", status)],
        async {
            let memories = storage.list_memories().await?;
            Ok(memories)
        },
        |memories| format!("Listed {} memories", memories.len())
    ).await
}
```

### Pattern 3: Domain-Specific Events

For commands that need additional context-specific events:

```rust
use mnemosyne_core::orchestration::events::AgentEvent;
use super::event_helpers;

pub async fn handle_search(...) -> Result<()> {
    let start = std::time::Instant::now();

    event_helpers::with_event_lifecycle("search", vec![], async {
        let results = perform_search(...).await?;

        // Emit domain-specific event
        let duration_ms = start.elapsed().as_millis() as u64;
        event_helpers::emit_domain_event(AgentEvent::SearchPerformed {
            query: query.clone(),
            search_type: "hybrid".to_string(),
            result_count: results.len(),
            duration_ms,
        }).await;

        Ok(results)
    }).await
}
```

## API Reference

### `with_event_lifecycle`

Wraps command execution with automatic event emission.

**Signature**:
```rust
pub async fn with_event_lifecycle<F, T>(
    command: &str,
    args: Vec<String>,
    handler: F,
) -> Result<T>
where
    F: Future<Output = Result<T>>
```

**Parameters**:
- `command`: Command name (e.g., "status", "list", "graph")
- `args`: Command arguments for tracing (e.g., `vec!["--format=json"]`)
- `handler`: Async function containing command logic

**Events Emitted**:
- `CliCommandStarted` before execution
- `CliCommandCompleted` on success
- `CliCommandFailed` on error

**Example**:
```rust
pub async fn handle_doctor(...) -> Result<()> {
    event_helpers::with_event_lifecycle("doctor", vec![], async {
        check_database().await?;
        check_api_keys().await?;
        println!("All checks passed");
        Ok(())
    }).await
}
```

### `with_event_lifecycle_and_summary`

Like `with_event_lifecycle`, but allows custom success messages.

**Signature**:
```rust
pub async fn with_event_lifecycle_and_summary<F, T, S>(
    command: &str,
    args: Vec<String>,
    handler: F,
    result_summary_fn: S,
) -> Result<T>
where
    F: Future<Output = Result<T>>,
    S: FnOnce(&T) -> String
```

**Parameters**:
- `command`: Command name
- `args`: Command arguments
- `handler`: Async function containing command logic
- `result_summary_fn`: Function that generates summary from result

**Example**:
```rust
pub async fn handle_export(...) -> Result<ExportStats> {
    event_helpers::with_event_lifecycle_and_summary(
        "export",
        vec![format!("--output={}", output_path)],
        async {
            let stats = export_memories(...).await?;
            Ok(stats)
        },
        |stats| format!("Exported {} memories", stats.count)
    ).await
}
```

### `emit_domain_event`

Emits domain-specific events for additional context.

**Signature**:
```rust
pub async fn emit_domain_event(event: AgentEvent)
```

**Parameters**:
- `event`: The AgentEvent to emit

**Available Domain Events**:
- `RememberExecuted`: Memory creation
- `RecallExecuted`: Memory retrieval
- `EvolveStarted`/`EvolveCompleted`: Evolution process
- `SearchPerformed`: Search operations
- `DatabaseOperation`: Database operations

**Example**:
```rust
event_helpers::emit_domain_event(AgentEvent::DatabaseOperation {
    operation: "vacuum".to_string(),
    table: "memories".to_string(),
    affected_rows: 0,
    duration_ms: elapsed.as_millis() as u64,
}).await;
```

## Migration Guide

### Before (Manual Event Emission)

```rust
pub async fn handle_status(...) -> Result<()> {
    let start_time = std::time::Instant::now();

    // Emit started event
    event_bridge::emit_command_started(
        "status",
        vec![],
    ).await;

    // Do work
    let result = perform_status_check().await;

    // Emit completed/failed event
    let duration_ms = start_time.elapsed().as_millis() as u64;
    match &result {
        Ok(_) => {
            event_bridge::emit_command_completed(
                "status",
                duration_ms,
                "Success".to_string(),
            ).await;
        }
        Err(e) => {
            event_bridge::emit_command_failed(
                "status",
                e.to_string(),
                duration_ms,
            ).await;
        }
    }

    result
}
```

### After (With Event Helpers)

```rust
pub async fn handle_status(...) -> Result<()> {
    event_helpers::with_event_lifecycle("status", vec![], async {
        perform_status_check().await
    }).await
}
```

## Commands Status

### Phase 1: Complete (4/22 commands)
- [x] remember.rs
- [x] recall.rs
- [x] evolve.rs
- [x] event_bridge.rs

### Phase 2: Infrastructure Complete
- [x] event_helpers.rs created
- [x] event_bridge.rs helpers added
- [x] Documentation created

### Phase 3: Remaining Commands (18)
- [ ] status.rs
- [ ] export.rs
- [ ] init.rs
- [ ] embed.rs
- [ ] models.rs
- [ ] artifact.rs
- [ ] doctor.rs
- [ ] update.rs
- [ ] interactive.rs
- [ ] edit.rs
- [ ] config.rs
- [ ] secrets.rs
- [ ] orchestrate.rs
- [ ] serve.rs
- [ ] api_server.rs
- [ ] tui.rs
- [ ] helpers.rs (if applicable)
- [ ] internal.rs (if applicable)

## Best Practices

### 1. Always Use Helpers for New Commands

```rust
// ✅ Good
pub async fn handle_new_command(...) -> Result<()> {
    event_helpers::with_event_lifecycle("new_command", vec![], async {
        // logic
        Ok(())
    }).await
}

// ❌ Bad - manual emission
pub async fn handle_new_command(...) -> Result<()> {
    event_bridge::emit_command_started(...).await;
    // logic
    event_bridge::emit_command_completed(...).await;
    Ok(())
}
```

### 2. Provide Meaningful Args

```rust
// ✅ Good - includes important parameters
event_helpers::with_event_lifecycle(
    "recall",
    vec![
        format!("--query={}", query),
        format!("--limit={}", limit),
    ],
    handler
).await

// ❌ Bad - no context
event_helpers::with_event_lifecycle("recall", vec![], handler).await
```

### 3. Use Domain Events for Rich Context

```rust
// ✅ Good - provides search metrics
event_helpers::emit_domain_event(AgentEvent::SearchPerformed {
    query: query.clone(),
    search_type: "vector".to_string(),
    result_count: results.len(),
    duration_ms: elapsed.as_millis() as u64,
}).await;

// ❌ Bad - no additional context
// (lifecycle events alone don't tell us how many results)
```

### 4. Custom Summaries for Rich Results

```rust
// ✅ Good - meaningful summary
with_event_lifecycle_and_summary(
    "export",
    vec![],
    handler,
    |stats| format!("Exported {} memories ({} KB)", stats.count, stats.size_kb)
).await

// ❌ Bad - generic summary
with_event_lifecycle("export", vec![], handler).await
// (just says "Success", no details)
```

## Error Handling

The event helpers follow a critical principle: **CLI commands never fail due to event emission**.

- All event emission errors are caught and logged at `debug` level
- Event emission failures do NOT propagate to the CLI command
- If the API server is unavailable, events are silently dropped
- Commands continue executing normally even if events fail

```rust
// This is built into the helpers - you don't need to handle it
pub async fn emit_domain_event(event: AgentEvent) {
    if let Err(e) = event_bridge::emit_event(event).await {
        tracing::debug!("Failed to emit domain event: {}", e);
        // Command continues normally
    }
}
```

## Testing

The event_helpers module includes unit tests:

```bash
cargo test event_helpers
```

Tests verify:
- Success case emits CliCommandCompleted
- Error case emits CliCommandFailed
- Custom summaries work correctly
- Domain events don't panic

## See Also

- `src/cli/event_bridge.rs`: Low-level event emission
- `src/cli/remember.rs`: Example of manual event emission (Phase 1)
- `src/cli/recall.rs`: Example of domain events (RecallExecuted, SearchPerformed)
- `src/orchestration/events.rs`: AgentEvent definitions
