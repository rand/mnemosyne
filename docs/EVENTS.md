# Event Architecture & Broadcasting

**Comprehensive guide to mnemosyne's event system for observability and multi-agent coordination**

---

## Table of Contents

1. [Overview](#overview)
2. [Event Flow Architecture](#event-flow-architecture)
3. [Event Types Catalog](#event-types-catalog)
4. [Event Broadcasting](#event-broadcasting)
5. [Event Subscription](#event-subscription)
6. [Usage Examples](#usage-examples)
7. [Troubleshooting](#troubleshooting)
8. [Advanced Topics](#advanced-topics)

---

## Overview

Mnemosyne's event system provides **bidirectional event flow** for observability and coordination:

- **CLI → API Server → Dashboard**: Real-time monitoring of all mnemosyne operations
- **CLI → API Server → Orchestrator**: Autonomous multi-agent coordination
- **Orchestrator → Memory**: Event persistence for audit trail

### Key Design Principles

1. **Autonomous**: API server auto-starts with Claude Code sessions via hooks
2. **Comprehensive**: All 22 CLI commands emit structured events
3. **Bidirectional**: Events flow from CLI to orchestrator for coordination
4. **Resilient**: CLI commands work even if API server unavailable
5. **Structured**: 42 event types with consistent schema

### Architecture Components

```
┌─────────────────┐
│   CLI Command   │
│  (22 commands)  │
└────────┬────────┘
         │ HTTP POST (event_bridge)
         ↓
┌─────────────────┐
│   API Server    │
│   (port 3000)   │
└────┬───────┬────┘
     │       │
     │       └─────→ SSE Stream → Dashboard (mnemosyne-dash)
     │
     └─────→ SSE Stream → Orchestrator (SseSubscriber)
                          │
                          ↓
                   ┌──────────────┐
                   │  Event       │
                   │  Persistence │
                   └──────────────┘
```

---

## Event Flow Architecture

### Phase 1: Event Emission (CLI → API)

**Every CLI command emits events via `event_bridge`**:

```rust
// src/cli/event_helpers.rs
pub async fn emit_domain_event(event: AgentEvent) {
    if let Err(e) = event_bridge::emit(event).await {
        debug!("Failed to emit event (non-fatal): {}", e);
    }
}
```

**Events are sent via HTTP POST**:
- URL: `http://localhost:3000/events`
- Timeout: 100ms (non-blocking)
- Graceful degradation: CLI continues if API unavailable

### Phase 2: Event Broadcasting (API → Consumers)

**API Server broadcasts via Server-Sent Events (SSE)**:

```bash
# Dashboard connection
GET http://localhost:3000/events/stream

# Event format
event: MemoryStored
data: {"event_type":"MemoryStored","timestamp":"2024-11-09T...", ...}
```

### Phase 3: Event Subscription (Orchestrator)

**SSE Subscriber receives events autonomously**:

```rust
// src/orchestration/sse_subscriber.rs
pub struct SseSubscriber {
    config: SseSubscriberConfig,
    orchestrator: ActorRef<OrchestratorMessage>,
    shutdown_rx: broadcast::Receiver<()>,
}

impl SseSubscriber {
    pub async fn run(mut self) {
        // Connect to API server SSE stream
        let stream_url = format!("{}/events/stream", self.config.api_url);
        // ... exponential backoff reconnection logic
    }
}
```

**Key features**:
- Auto-start when supervision tree starts
- Exponential backoff reconnection (1s → 60s)
- Event conversion: API EventType → AgentEvent
- Graceful shutdown with 5s timeout

### Phase 4: Event Processing (Orchestrator)

**Orchestrator reacts to CLI events**:

```rust
// src/orchestration/actors/orchestrator.rs
async fn handle_cli_event(
    state: &mut OrchestratorState,
    event: AgentEvent,
) -> Result<()> {
    // Persist for audit trail
    state.events.persist(event.clone()).await?;

    // React to specific events
    match &event {
        AgentEvent::RememberExecuted { .. } => {
            debug!("Memory operation detected");
        }
        AgentEvent::SessionStarted { instance_id, .. } => {
            info!("Claude Code session started: {}", instance_id);
        }
        // ... 40 more event types
    }

    Ok(())
}
```

---

## Event Types Catalog

### Memory Operations (11 events)

| Event | Importance | Description |
|-------|-----------|-------------|
| `RememberExecuted` | 5 | Memory stored via `remember` command |
| `RecallExecuted` | 4 | Memory retrieval via `recall` command |
| `SearchExecuted` | 4 | Full-text search performed |
| `GraphTraversalExecuted` | 4 | Graph traversal completed |
| `EvolutionCycleStarted` | 6 | Memory evolution initiated |
| `EvolutionCycleCompleted` | 6 | Evolution finished (consolidation, decay, archival) |
| `ConsolidationCompleted` | 5 | Duplicate memories merged |
| `ArchivalCompleted` | 3 | Low-value memories archived |
| `LinkDecayApplied` | 3 | Link strengths recalculated |
| `ImportanceRecalibrated` | 4 | Graph-based importance updated |
| `MemoryLinksUpdated` | 3 | Bidirectional links refreshed |

### System Operations (8 events)

| Event | Importance | Description |
|-------|-----------|-------------|
| `StatusCheckExecuted` | 3 | `status` command run |
| `HealthCheckStarted` | 4 | `doctor` command initiated |
| `HealthCheckCompleted` | 4 | Health diagnostics finished |
| `ConfigurationChanged` | 6 | Config updated (API key, model, etc.) |
| `DatabaseInitialized` | 8 | First-time setup completed |
| `DatabaseMigrated` | 7 | Schema migration applied |
| `BackupCreated` | 5 | Database backup generated |
| `BackupRestored` | 7 | Backup restoration completed |

### Session Lifecycle (4 events)

| Event | Importance | Description |
|-------|-----------|-------------|
| `SessionStarted` | 7 | Claude Code session opened |
| `SessionEnded` | 7 | Claude Code session closed |
| `ServerStarted` | 6 | API/RPC server launched |
| `ServerStopped` | 6 | Server gracefully shut down |

### CLI Operations (8 events)

| Event | Importance | Description |
|-------|-----------|-------------|
| `CliCommandStarted` | 2 | Any CLI command initiated |
| `CliCommandCompleted` | 2 | Command finished successfully |
| `CliCommandFailed` | 5 | Command failed with error |
| `InteractiveModeEntered` | 4 | `interactive` session started |
| `InteractiveModeExited` | 3 | Interactive session ended |
| `EditingSessionStarted` | 4 | ICS editor opened |
| `EditingSessionEnded` | 3 | ICS editor closed |
| `ArtifactSaved` | 4 | Context file saved from ICS |

### Orchestration Events (11 events)

| Event | Importance | Description |
|-------|-----------|-------------|
| `WorkItemCreated` | 6 | New task added to work queue |
| `WorkItemStarted` | 5 | Agent began work on task |
| `WorkItemCompleted` | 6 | Task finished successfully |
| `WorkItemFailed` | 8 | Task failed (needs attention) |
| `WorkItemBlocked` | 7 | Task blocked by dependencies |
| `AgentSpawned` | 5 | Sub-agent created for parallel work |
| `AgentCompleted` | 4 | Sub-agent finished task |
| `AgentFailed` | 7 | Sub-agent encountered error |
| `ContextCompacted` | 6 | Context snapshot saved (75% threshold) |
| `DeadlockDetected` | 9 | Circular dependency detected |
| `DeadlockResolved` | 8 | Deadlock resolved via preemption |

---

## Event Broadcasting

### Starting the API Server

**Automatic (via hooks)**:
```bash
# Claude Code session start triggers .claude/hooks/session-start.sh
# API server auto-starts on port 3000
```

**Manual**:
```bash
mnemosyne api --addr 127.0.0.1:3000 --capacity 1000
```

### Monitoring Events

**Dashboard** (recommended):
```bash
mnemosyne-dash --api http://localhost:3000
```

**Raw SSE stream** (debugging):
```bash
curl -N http://localhost:3000/events/stream
```

**Event filtering**:
```bash
# Dashboard keyboard shortcuts
# 1-8: Toggle event categories
# c: Clear history
# h: Show/hide help
```

---

## Event Subscription

### SSE Subscriber (Orchestrator)

**Auto-start configuration**:

```rust
// src/orchestration/supervision.rs
pub async fn start(&mut self) -> Result<()> {
    // ... orchestrator start ...

    // Start SSE subscriber for bidirectional event flow
    if let Some(ref orchestrator) = self.orchestrator {
        let (shutdown_tx, shutdown_rx) = broadcast::channel(1);
        let sse_config = SseSubscriberConfig::default();
        let sse_subscriber = SseSubscriber::new(
            sse_config,
            orchestrator.clone(),
            shutdown_rx,
        );

        let sse_handle = tokio::spawn(async move {
            sse_subscriber.run().await;
        });

        self.sse_shutdown_tx = Some(shutdown_tx);
        self.sse_subscriber_handle = Some(sse_handle);
    }

    Ok(())
}
```

**Reconnection logic**:
- Initial delay: 1s
- Exponential backoff: 2x on each failure
- Maximum delay: 60s
- Infinite retries (orchestrator needs CLI events)

### Event Conversion

**API EventType → AgentEvent**:

```rust
fn convert_api_event_to_agent_event(api_event: &ApiEvent) -> Option<AgentEvent> {
    match &api_event.event_type {
        EventType::MemoryStored { memory_id, summary, .. } => {
            Some(AgentEvent::RememberExecuted {
                content_preview: summary.clone(),
                importance: 5,
                memory_id: MemoryId::from_string(memory_id).unwrap_or_default(),
            })
        }
        EventType::Heartbeat { .. } => None, // Dashboard-only, skip
        // ... 40+ more conversions
    }
}
```

**Filtered events**:
- `Heartbeat`: Dashboard health check (not relevant to orchestrator)
- Other dashboard-only events as needed

---

## Usage Examples

### Example 1: Monitor Memory Operations

```bash
# Terminal 1: Start API server (or rely on auto-start)
mnemosyne api

# Terminal 2: Start dashboard
mnemosyne-dash

# Terminal 3: Perform memory operations
mnemosyne remember -c "Authentication uses JWT tokens" \
  -i 8 -t architecture,auth

mnemosyne recall -q "authentication" -l 5

# Dashboard shows:
# - MemoryStored event (importance: 8)
# - RecallExecuted event (query: "authentication", results: 5)
```

### Example 2: Debug Event Flow

```bash
# Watch raw SSE stream
curl -N http://localhost:3000/events/stream | jq -r '.event_type'

# Output (as commands run):
# SessionStarted
# CliCommandStarted
# RememberExecuted
# CliCommandCompleted
# CliCommandStarted
# RecallExecuted
# CliCommandCompleted
```

### Example 3: Orchestrator Coordination

```bash
# Start orchestration (SSE subscriber auto-starts)
mnemosyne orchestrate

# In another terminal: Perform memory operation
mnemosyne remember -c "Work item: Implement rate limiting" \
  -i 8 -t task,security

# Orchestrator receives RememberExecuted event
# Event persisted in orchestrator's memory
# Orchestrator can react to high-importance task memories
```

### Example 4: Session Lifecycle Tracking

```bash
# Session start event emitted automatically by hook
# (.claude/hooks/session-start.sh)

# Work in Claude Code session...

# Session end event emitted on exit
# API server gracefully shuts down
```

---

## Troubleshooting

### Issue 1: No Events in Dashboard

**Symptoms**: Dashboard shows "No events" or stale data

**Diagnosis**:
```bash
# Check API server status
curl http://localhost:3000/health

# Check SSE stream directly
curl -N http://localhost:3000/events/stream

# Check CLI event emission
RUST_LOG=debug mnemosyne status 2>&1 | grep "emit_domain_event"
```

**Solutions**:
1. **API server not running**: Start with `mnemosyne api`
2. **Port conflict**: Use `--addr 127.0.0.1:3001`
3. **Event emission disabled**: Check `MNEMOSYNE_DISABLE_EVENTS` env var

### Issue 2: Orchestrator Not Receiving Events

**Symptoms**: CLI commands work but orchestrator doesn't react

**Diagnosis**:
```bash
# Check SSE subscriber connection
RUST_LOG=mnemosyne::orchestration::sse_subscriber=debug mnemosyne orchestrate

# Look for:
# - "SSE subscriber: connected to event stream"
# - "Orchestrator received CLI event: ..."
```

**Solutions**:
1. **Connection failed**: Check API server URL in SseSubscriberConfig
2. **Event conversion issue**: Check convert_api_event_to_agent_event logs
3. **Orchestrator not started**: Verify supervision tree initialization

### Issue 3: High Event Latency

**Symptoms**: Events appear in dashboard >1s after CLI command

**Diagnosis**:
```bash
# Measure end-to-end latency
time mnemosyne status
curl http://localhost:3000/events/stream | grep StatusCheckExecuted
```

**Solutions**:
1. **Network latency**: Use `127.0.0.1` instead of `localhost`
2. **Event buffer full**: Increase `--capacity` parameter
3. **Dashboard processing slow**: Check dashboard terminal for errors

### Issue 4: SSE Reconnection Loop

**Symptoms**: Constant "reconnecting..." messages in orchestrator logs

**Diagnosis**:
```bash
# Check API server logs
RUST_LOG=mnemosyne::api=debug mnemosyne api

# Check SSE subscriber backoff
RUST_LOG=mnemosyne::orchestration::sse_subscriber=debug mnemosyne orchestrate
```

**Solutions**:
1. **API server crashing**: Fix server errors first
2. **Incorrect URL**: Verify SseSubscriberConfig.api_url
3. **Firewall blocking**: Check local firewall rules

---

## Advanced Topics

### Custom Event Processing

**Extending orchestrator event handling**:

```rust
// src/orchestration/actors/orchestrator.rs
async fn handle_cli_event(
    state: &mut OrchestratorState,
    event: AgentEvent,
) -> Result<()> {
    // Custom logic for specific events
    match &event {
        AgentEvent::RememberExecuted { importance, .. } if *importance >= 8 => {
            // High-importance memories trigger proactive review
            info!("High-importance memory stored, scheduling review");
            // ... create work item for review ...
        }
        // ... other custom handlers ...
    }

    Ok(())
}
```

### Event Persistence Schema

**Events stored as MemoryNote**:

```rust
// src/orchestration/event_persistence.rs
pub async fn persist(&mut self, event: AgentEvent) -> Result<()> {
    let importance = event.importance_score();
    let note = MemoryNote {
        content: event.summary(),
        context: format!("AgentEvent: {}", event.event_type()),
        memory_type: MemoryType::Task, // Or custom type
        importance,
        tags: event.tags(),
        namespace: Some("orchestrator".to_string()),
    };

    self.storage.create_memory(note).await?;
    Ok(())
}
```

### Event-Driven Work Queue

**Automatically create work items from events**:

```rust
match &event {
    AgentEvent::HealthCheckCompleted { checks_failed, .. } if *checks_failed > 0 => {
        // Create work item to investigate failures
        let work_item = WorkItem {
            description: format!("Investigate {} health check failures", checks_failed),
            priority: Priority::High,
            dependencies: vec![],
        };
        state.work_queue.enqueue(work_item)?;
    }
    // ... other event-driven work creation ...
}
```

### Multi-Instance Coordination

**Future enhancement: Cross-instance event forwarding**:

```
Instance A (owner) → API Server → SSE → Instance B (client)
                                      → Instance C (client)
```

**Use cases**:
- Distributed work queue across multiple Claude Code sessions
- Shared memory updates synchronized in real-time
- Coordinated multi-agent problem solving

---

## See Also

- [AGENT_GUIDE.md](../AGENT_GUIDE.md) - Comprehensive agent development guide
- [DASHBOARD.md](DASHBOARD.md) - Dashboard usage and features
- [EVENT_HELPERS_GUIDE.md](EVENT_HELPERS_GUIDE.md) - CLI event emission patterns
- [API.md](API.md) - HTTP API endpoints and schemas
- [ORCHESTRATION.md](ORCHESTRATION.md) - Multi-agent orchestration details

---

**Last Updated**: November 9, 2024
**Version**: 2.3.1
