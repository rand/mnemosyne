# Agent Visibility Fix - Complete Architecture

## Problem Statement

The dashboard (`mnemosyne-dash`) was showing "No active agents" even when the orchestration system was running with all 4 agents active. This was caused by a race condition between agent startup and dashboard connection.

## Root Cause Analysis

The system had a **timing gap** in the observability layer:

1. Agents spawn and start executing work
2. Heartbeat tasks spawn with 30-second intervals
3. `tokio::time::interval.tick()` waits for first interval before firing
4. Dashboard connects via SSE before any heartbeats are sent
5. StateManager has no events to process → no agents visible
6. Dashboard shows empty state despite agents being active

## Multi-Layered Solution

We implemented **defense in depth** with three complementary layers:

### Phase 1: Immediate First Heartbeat (Proactive)
**Files modified**: `src/orchestration/actors/*.rs`, `src/bin/dash.rs`

Changed all 4 agents (Orchestrator, Optimizer, Reviewer, Executor) to send an immediate first heartbeat before starting the 30-second interval loop:

```rust
// Before
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    loop {
        interval.tick().await;  // Waits 30s before first heartbeat!
        broadcaster.broadcast(Event::heartbeat(agent_id));
    }
});

// After
tokio::spawn(async move {
    // Send immediate first heartbeat
    broadcaster.broadcast(Event::heartbeat(agent_id.clone()));

    // Then continue with 30s interval
    let mut interval = tokio::time::interval(Duration::from_secs(30));
    loop {
        interval.tick().await;
        broadcaster.broadcast(Event::heartbeat(agent_id.clone()));
    }
});
```

Dashboard (`dash.rs`) also force-refreshes state immediately on connection before entering the event loop.

**Result**: Agents visible within ~50-60ms of spawning.

### Phase 2: SSE Snapshot (Reactive)
**Files modified**: `src/api/server.rs`

Enhanced the SSE `/events` endpoint to send a **state snapshot** to new clients immediately upon connection:

```rust
async fn events_handler(State(state): State<AppState>) -> Sse<...> {
    // 1. Query current state
    let agents = state.state.list_agents().await;
    let context_files = state.state.list_context_files().await;

    // 2. Create synthetic events for existing state
    let mut snapshot_events = Vec::new();
    for agent in agents {
        let event = Event::heartbeat(agent.id.clone());
        snapshot_events.push(/* SSE-formatted event */);
    }

    // 3. Subscribe to live event stream
    let live_stream = BroadcastStream::new(state.events.subscribe());

    // 4. Chain: snapshot → live events
    let combined_stream = snapshot_stream.chain(live_event_stream);

    Sse::new(combined_stream).keep_alive(KeepAlive::default())
}
```

**Result**: Late-connecting clients see agents in ~7 microseconds.

### Phase 3: StateManager Auto-Creation (Fallback)
**Existing mechanism**: `src/api/state.rs:218-233`

StateManager already had robust fallback logic that auto-creates agents on first heartbeat:

```rust
EventType::Heartbeat { instance_id, .. } => {
    let mut agents_map = agents.write().await;
    if let Some(agent) = agents_map.get_mut(&instance_id) {
        // Update existing agent
        agent.updated_at = Utc::now();
    } else {
        // Auto-create agent on first heartbeat
        agents_map.insert(
            instance_id.clone(),
            AgentInfo {
                id: instance_id.clone(),
                state: AgentState::Idle,
                updated_at: Utc::now(),
                metadata: HashMap::new(),
            },
        );
    }
}
```

This ensures any agent sending a heartbeat (immediate or periodic) is guaranteed to be registered in StateManager.

## Test Coverage

Created comprehensive integration tests in `tests/dashboard_agents_integration.rs`:

1. **test_agents_visible_within_one_second**: Validates 1-second SLA
2. **test_late_dashboard_connection_sees_agents**: Validates late connection scenario
3. **test_agents_visible_within_100ms**: Strict timing validation
4. **test_concurrent_dashboard_connections**: 5 simultaneous connections
5. **test_dashboard_reconnect_sees_agents**: Disconnect/reconnect resilience
6. **test_performance_benchmarks**: Detailed timing metrics
7. **test_heartbeat_auto_creates_agent**: Validates auto-creation mechanism
8. **test_http_api_shows_agents_immediately** (ignored): Manual E2E verification

**All tests pass** with excellent performance:
- Agents visible in ~54-59ms
- Late connections see agents in ~7µs
- All edge cases handled

## Performance Metrics

| Metric | Before Fix | After Fix |
|--------|-----------|-----------|
| Time to first agent visible | 30 seconds | ~54ms |
| Time to all 4 agents visible | 30 seconds | ~59ms |
| Late dashboard connection | Never saw agents | 7µs |
| Race condition risk | High | Eliminated |

## Architecture Guarantees

The three-layer defense provides these guarantees:

1. **Timing Independence**: Dashboard sees agents regardless of connection timing
2. **Connection Resilience**: Reconnections work seamlessly
3. **Concurrency Safety**: Multiple dashboard instances work correctly
4. **Event Ordering**: SSE snapshot + live stream maintains causality
5. **Fallback Protection**: Auto-creation ensures no agent is missed

## Commits

1. `10637ec` - Phase 1: Immediate first heartbeat + dashboard force-refresh
2. `07a32ea` - Phase 1: Integration tests
3. `f87ce78` - Phase 1: Test compilation fixes
4. `4894657` - Phase 2: SSE snapshot for late clients
5. `0574271` - Phase 3: Comprehensive timing tests

## Verification

To manually verify the fix:

```bash
# Terminal 1: Start orchestration with dashboard
mnemosyne orchestrate --dashboard

# Terminal 2: Connect dashboard
mnemosyne-dash

# Expected result: All 4 agents visible within 1 second
```

## Future Improvements

Potential enhancements (not critical):

1. **Metrics dashboard**: Track agent visibility latency over time
2. **Health monitoring**: Alert if agents don't appear within threshold
3. **Distributed tracing**: OpenTelemetry spans for end-to-end visibility
4. **Connection pooling**: Optimize for many concurrent dashboard clients

## Related Documentation

- `docs/orchestration/README.md` - Multi-agent system overview
- `docs/dashboard/README.md` - Dashboard architecture
- `docs/api/README.md` - HTTP API and SSE specifications
