# API Server Architecture

Embedded HTTP API server for real-time orchestration monitoring and integration.

## Overview

The mnemosyne API server provides real-time access to orchestration state through:
- **Server-Sent Events (SSE)** for live event streaming
- **REST endpoints** for state queries
- **EventBroadcaster** for pub/sub event distribution
- **StateManager** for event-driven state projection

## Architecture Diagram

```
┌──────────────────────────────────────────────────────────┐
│ OrchestrationEngine (Ractor Actors)                     │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │ Orchestrator │  │  Optimizer   │  │   Reviewer   │  │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  │
│         │                 │                 │           │
│         └────────────┬────┴────────────┬────┘           │
│                      ▼                 ▼                 │
│              ┌──────────────┐  ┌──────────────┐         │
│              │   Executor   │  │EventPersist  │         │
│              └──────────────┘  └──────┬───────┘         │
└────────────────────────────────────────┼────────────────┘
                                         │ AgentEvent
                                         ▼
                         ┌───────────────────────────────┐
                         │      EventBroadcaster         │
                         │  (tokio::broadcast::channel)  │
                         └───────┬──────────────┬────────┘
                                 │              │
          ┌──────────────────────┼──────────────┼───────────────┐
          │                      │              │               │
          ▼                      ▼              ▼               ▼
    ┌──────────┐          ┌─────────────┐  ┌──────────┐  ┌──────────┐
    │   SSE    │          │StateManager │  │Dashboard │  │Custom    │
    │ Clients  │          │ (in-process)│  │ Client   │  │Integration│
    └────┬─────┘          └──────┬──────┘  └──────────┘  └──────────┘
         │                       │
         │ GET /events           │ Subscribes to events
         │                       │ Maintains agent states
         ▼                       ▼
    ┌──────────────────────────────────────┐
    │         HTTP API Server              │
    │                                       │
    │  GET /health                          │
    │  GET /events (SSE stream)             │
    │  GET /state/agents                    │
    │  GET /state/stats                     │
    │  GET /state/context-files             │
    └───────────────────────────────────────┘
```

## Core Components

### 1. EventBroadcaster

**Purpose**: Pub/sub distribution of events to multiple subscribers

**Implementation**: `src/api/events.rs`

```rust
pub struct EventBroadcaster {
    tx: broadcast::Sender<Arc<Event>>,
}

impl EventBroadcaster {
    pub fn new(capacity: usize) -> Self;
    pub fn broadcast(&self, event: Event) -> Result<()>;
    pub fn subscribe(&self) -> broadcast::Receiver<Arc<Event>>;
}
```

**Key Features**:
- **Non-blocking**: Uses tokio broadcast channel
- **Multi-subscriber**: Unlimited subscribers via `subscribe()`
- **Backpressure**: Lagging subscribers automatically catch up or skip
- **Zero-copy**: Events wrapped in `Arc<Event>` for efficient sharing

**Usage**:
```rust
// Create broadcaster (typically in ApiServer::new)
let broadcaster = EventBroadcaster::new(1000); // 1000 event capacity

// Subscribe to events
let mut receiver = broadcaster.subscribe();

// Broadcast event
let event = Event::agent_started("executor".to_string());
broadcaster.broadcast(event)?;

// Receive event
let event = receiver.recv().await?;
```

### 2. StateManager

**Purpose**: Event-driven projection of agent states

**Implementation**: `src/api/state.rs`

```rust
pub struct StateManager {
    agents: Arc<RwLock<HashMap<String, AgentInfo>>>,
    context_files: Arc<RwLock<Vec<ContextFileInfo>>>,
    event_rx: Option<broadcast::Receiver<Arc<Event>>>,
}
```

**Architecture Pattern**: Event Sourcing
- **Single source of truth**: Events drive all state changes
- **Auto-updating**: Subscribes to EventBroadcaster at creation
- **Concurrent access**: RwLock for high-throughput reads
- **Immutable events**: Events are append-only

**State Transitions**:
```
AgentStarted → agent.state = Active { task: "..." }
WorkItemAssigned → agent.state = Active { task: "..." }
WorkItemCompleted → agent.state = Idle
AgentFailed → agent.state = Failed { error: "..." }
Heartbeat → agent.last_heartbeat = now
```

**Query Methods**:
```rust
// Get all agents
pub async fn get_agents(&self) -> Vec<AgentInfo>;

// Get specific agent
pub async fn get_agent(&self, id: &str) -> Option<AgentInfo>;

// Get statistics
pub async fn get_stats(&self) -> SystemStats;

// Get context files
pub async fn get_context_files(&self) -> Vec<ContextFileInfo>;
```

**Initialization**:
```rust
// StateManager automatically subscribes to events
let state_manager = Arc::new(StateManager::new());
state_manager.subscribe_to_events(broadcaster.subscribe());

// Background task processes events
tokio::spawn(async move {
    state_manager.process_events().await;
});
```

### 3. ApiServer

**Purpose**: HTTP server exposing REST and SSE endpoints

**Implementation**: `src/api/server.rs`

```rust
pub struct ApiServer {
    config: ApiServerConfig,
    events: EventBroadcaster,
    state: Arc<StateManager>,
    instance_id: String,
}
```

**Initialization**:
```rust
let config = ApiServerConfig {
    addr: ([127, 0, 0, 1], 3000).into(),
    event_capacity: 1000,
};

let api_server = ApiServer::new(config);

// Get references for sharing
let broadcaster = api_server.broadcaster().clone();
let state_manager = Arc::clone(api_server.state_manager());

// Start serving
api_server.serve().await?;
```

**Dynamic Port Allocation**:
```rust
// Tries ports 3000-3010 automatically
let mut port = 3000;
loop {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    match TcpListener::bind(addr).await {
        Ok(listener) => break listener,
        Err(_) if port < 3010 => port += 1,
        Err(e) => return Err(e.into()),
    }
}
```

## Event Flow

### 1. Event Creation (Orchestration Layer)

```rust
// In orchestration engine
let event = AgentEvent::WorkItemStarted {
    agent: AgentRole::Executor,
    item_id: WorkItemId::new(),
    description: "Implement authentication".to_string(),
};
```

### 2. Event Persistence (with Broadcasting)

```rust
// EventPersistence stores to Mnemosyne AND broadcasts
let persistence = EventPersistence::new_with_broadcaster(
    storage,
    namespace,
    Some(broadcaster), // Optional broadcaster
);

persistence.persist(event).await?;
// → Stores to database
// → Converts to API Event
// → Broadcasts to subscribers
```

### 3. Event Mapping (Orchestration → API)

```rust
// In src/orchestration/events.rs
fn to_api_event(&self, event: &AgentEvent) -> Option<crate::api::Event> {
    match event {
        AgentEvent::WorkItemStarted { agent, description, .. } => {
            Some(Event::agent_started_with_task(
                self.agent_role_to_id(agent),
                description.clone(),
            ))
        }
        // ... other mappings
    }
}
```

**Supported Mappings**:
| Orchestration Event | API Event | State Change |
|---------------------|-----------|--------------|
| `WorkItemStarted` | `AgentStarted` | Idle → Active |
| `WorkItemAssigned` | `WorkItemAssigned` | Idle → Active |
| `WorkItemCompleted` | `WorkItemCompleted` | Active → Idle |
| `WorkItemFailed` | `AgentFailed` | Active → Failed |
| `PhaseTransition` | `PhaseChanged` | - |
| `Heartbeat` | `AgentHeartbeat` | Update timestamp |
| `ContextCheckpoint` | `ContextCheckpointed` | - |
| `ReviewFailed` | `ReviewFailed` | - |
| `WorkItemRequeued` | `WorkItemRetried` | - |
| `DeadlockDetected` | `DeadlockDetected` | - |

### 4. Event Broadcasting

```rust
// EventBroadcaster distributes to all subscribers
broadcaster.broadcast(api_event)?;

// All subscribers receive
// - StateManager (updates agent states)
// - SSE clients (dashboard, custom integrations)
```

### 5. State Update (StateManager)

```rust
// StateManager processes event
match event.event_type {
    EventType::AgentStarted { agent_id, task, .. } => {
        let mut agents = self.agents.write().await;
        agents.entry(agent_id).and_modify(|agent| {
            agent.state = AgentState::Active { task: task.clone() };
            agent.updated_at = Utc::now();
        });
    }
    // ... other handlers
}
```

### 6. Client Consumption

```rust
// SSE client receives via /events endpoint
let mut event_stream = connect_sse("http://127.0.0.1:3000/events");

while let Some(event) = event_stream.next().await {
    match event.event_type {
        EventType::AgentStarted { agent_id, task, .. } => {
            println!("Agent {} started: {}", agent_id, task.unwrap());
        }
        _ => {}
    }
}
```

## REST API Endpoints

### GET /health

Health check and server information.

**Response**:
```json
{
  "status": "ok",
  "version": "2.1.1",
  "instance_id": "a3f9d2e1",
  "subscribers": 3
}
```

**Usage**:
```bash
curl http://127.0.0.1:3000/health
```

### GET /events

Server-Sent Events stream.

**Response**: SSE stream
```
event: message
data: {"id":"evt_123","event_type":{"AgentStarted":{"agent_id":"executor","task":"Auth"}},"timestamp":"..."}

event: message
data: {"id":"evt_124","event_type":{"WorkItemCompleted":{"agent_id":"executor","item_id":"wi_789"}},"timestamp":"..."}
```

**Usage**:
```bash
curl -N http://127.0.0.1:3000/events
```

**Client Implementation**:
```rust
use eventsource_client::{Client, SSE};

let client = Client::for_url("http://127.0.0.1:3000/events")?.build();
let mut stream = client.stream();

while let Some(event) = stream.next().await {
    match event {
        Ok(SSE::Event(e)) => {
            let api_event: Event = serde_json::from_str(&e.data)?;
            // Process event
        }
        Ok(SSE::Comment(_)) => {} // Ignore comments
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

### GET /state/agents

Current state of all agents.

**Response**:
```json
[
  {
    "id": "executor",
    "role": "executor",
    "state": {
      "Active": {
        "task": "Implement JWT authentication"
      }
    },
    "last_heartbeat": "2025-11-06T19:30:00Z",
    "updated_at": "2025-11-06T19:29:45Z"
  },
  {
    "id": "orchestrator",
    "role": "orchestrator",
    "state": "Idle",
    "last_heartbeat": "2025-11-06T19:30:00Z",
    "updated_at": "2025-11-06T19:29:50Z"
  }
]
```

**Usage**:
```bash
curl http://127.0.0.1:3000/state/agents | jq
```

### GET /state/stats

System-wide statistics.

**Response**:
```json
{
  "active_agents": 1,
  "total_agents": 4,
  "idle_agents": 3,
  "failed_agents": 0,
  "completed_tasks": 12,
  "failed_tasks": 1,
  "context_usage_pct": 67.5
}
```

**Usage**:
```bash
curl http://127.0.0.1:3000/state/stats | jq
```

### GET /state/context-files

Tracked context files.

**Response**:
```json
[
  {
    "path": "/project/src/auth.rs",
    "size_bytes": 4096,
    "last_modified": "2025-11-06T19:25:00Z",
    "in_context": true
  },
  {
    "path": "/project/docs/api.md",
    "size_bytes": 2048,
    "last_modified": "2025-11-06T19:20:00Z",
    "in_context": false
  }
]
```

**Usage**:
```bash
curl http://127.0.0.1:3000/state/context-files | jq
```

## Integration Patterns

### 1. Embedded Server (Orchestrate Command)

The most common pattern - API server embedded in orchestration:

```rust
// src/cli/orchestrate.rs
let (event_broadcaster, state_manager, api_task) = if dashboard {
    let api_config = ApiServerConfig {
        addr: ([127, 0, 0, 1], 3000).into(),
        event_capacity: 1000,
    };

    let api_server = ApiServer::new(api_config);
    let broadcaster = api_server.broadcaster().clone();
    let state_manager = Arc::clone(api_server.state_manager());

    // Spawn API server in background
    let api_task = tokio::spawn(async move {
        api_server.serve().await
    });

    (Some(broadcaster), Some(state_manager), Some(api_task))
} else {
    (None, None, None)
};

// Pass to launcher
launcher::launch_orchestrated_session(
    db_path,
    initial_prompt,
    event_broadcaster,
    state_manager,
).await?;
```

### 2. Standalone Server (Daemon Mode)

API server as independent process:

```rust
// Example: mnemosyne api-server command
let config = ApiServerConfig {
    addr: ([0, 0, 0, 0], 3000).into(), // Listen on all interfaces
    event_capacity: 10000,
};

let api_server = ApiServer::new(config);

// Server runs indefinitely
api_server.serve().await?;
```

### 3. Custom Integration

Integrate API server in custom applications:

```rust
use mnemosyne_core::api::{ApiServer, ApiServerConfig};

#[tokio::main]
async fn main() -> Result<()> {
    // Create API server
    let config = ApiServerConfig {
        addr: "127.0.0.1:8080".parse()?,
        event_capacity: 1000,
    };
    let api_server = ApiServer::new(config);

    // Get broadcaster for your events
    let broadcaster = api_server.broadcaster().clone();

    // Spawn server
    let api_task = tokio::spawn(async move {
        api_server.serve().await
    });

    // Your application logic
    // Broadcast events as needed
    broadcaster.broadcast(Event::custom(...))?;

    api_task.await??;
    Ok(())
}
```

## Event Types Reference

See `src/api/events.rs` for complete definitions:

```rust
pub enum EventType {
    // Agent lifecycle
    AgentStarted { agent_id: String, task: Option<String>, ... },
    AgentCompleted { agent_id: String, result: String, ... },
    AgentFailed { agent_id: String, error: String, ... },
    AgentHeartbeat { agent_id: String, ... },

    // Work items
    WorkItemAssigned { agent_id: String, item_id: String, task: String, ... },
    WorkItemCompleted { agent_id: String, item_id: String, ... },
    WorkItemRetried { item_id: String, reason: String, attempt: u32, ... },

    // Orchestration
    PhaseChanged { from: String, to: String, ... },
    DeadlockDetected { blocked_items: Vec<String>, ... },
    ReviewFailed { item_id: String, issues: Vec<String>, ... },

    // Context
    ContextModified { file_path: String, change_summary: String, ... },
    ContextCheckpointed { agent_id: String, usage_pct: f32, ... },

    // Memory
    MemoryStored { memory_id: String, content: String, ... },
    MemoryRecalled { memory_id: String, relevance_score: f32, ... },
}
```

## Performance Characteristics

### EventBroadcaster

- **Throughput**: ~100K events/sec (single producer)
- **Latency**: <1ms (non-blocking send)
- **Memory**: ~8 bytes/event in channel (Arc overhead)
- **Scalability**: Unlimited subscribers (broadcast channel)

### StateManager

- **Read latency**: <1μs (RwLock read)
- **Write latency**: <10μs (RwLock write)
- **Concurrency**: High read throughput, serialized writes
- **Memory**: ~1KB per agent

### SSE Streaming

- **Latency**: <5ms (event to client delivery)
- **Bandwidth**: ~1-5 KB/s per client
- **Concurrency**: 100+ concurrent clients
- **Buffering**: 1000 events (configurable)

## Testing

### Unit Tests

```bash
# API module tests
cargo test --lib api

# Specific test
cargo test --lib api::events::tests
```

### Integration Tests

```bash
# Phase 2 event integration
cargo test --test phase2_event_integration_test
```

### Manual Testing

```bash
# 1. Start orchestration with dashboard
mnemosyne orchestrate "Test task" --dashboard

# 2. Test health endpoint
curl http://127.0.0.1:3000/health

# 3. Stream events
curl -N http://127.0.0.1:3000/events

# 4. Query agents
curl http://127.0.0.1:3000/state/agents | jq

# 5. Get stats
curl http://127.0.0.1:3000/state/stats | jq
```

## Troubleshooting

### "Address already in use"

**Solution**: API server tries ports 3000-3010 automatically. If all occupied:
```bash
pkill -f "mnemosyne orchestrate"
```

### "No subscribers" in health endpoint

**Expected**: Subscribers only counted when dashboard clients connect

### Events not appearing in SSE stream

**Debug**:
1. Verify broadcaster is shared: `Arc::clone(api_server.broadcaster())`
2. Check events are being broadcast: Add logging
3. Verify SSE client connected: Check `/health` subscribers count

### StateManager not updating

**Debug**:
1. Verify StateManager subscribed: `subscribe_to_events()` called
2. Check event processing task running: `process_events().await`
3. Verify events match expected types: Check event mapping

## See Also

- [Dashboard Monitoring Guide](../guides/dashboard.md)
- [Orchestration Command Reference](../guides/orchestration.md)
- [Multi-Agent Architecture](../specs/multi-agent-architecture.md)
- [Event Sourcing Pattern](https://martinfowler.com/eaaDev/EventSourcing.html)
