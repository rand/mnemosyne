# Mnemosyne Dashboard - Real-time Monitoring System

## Table of Contents
- [Overview](#overview)
- [Why the Redesign?](#why-the-redesign)
- [Architecture](#architecture)
- [Features](#features)
- [Panel Reference](#panel-reference)
- [Keyboard Shortcuts](#keyboard-shortcuts)
- [Usage Guide](#usage-guide)
- [Technical Details](#technical-details)
- [Testing](#testing)
- [Troubleshooting](#troubleshooting)

---

## Overview

The Mnemosyne Dashboard (`mnemosyne-dash`) is a real-time terminal UI for monitoring the multi-agent orchestration system. It provides live visibility into:

- **System health** and performance metrics
- **Agent activity** across all MCP instances
- **Event streams** with intelligent filtering
- **CLI operations** and their outcomes

The dashboard connects to the Mnemosyne API server (automatically started with the first MCP instance) via Server-Sent Events (SSE) for zero-latency updates.

**Quick Start**:
```bash
# Dashboard connects to http://localhost:3000 by default
mnemosyne-dash

# Custom configuration
mnemosyne-dash --api http://localhost:3000 --refresh 500
```

---

## Why the Redesign?

### Before: "Static Wall of Garbage"
The old dashboard suffered from several critical issues:
- **7-panel layout** with rigid, non-interactive splits
- **Static data** requiring manual refresh
- **Information overload** with no filtering or prioritization
- **Poor signal-to-noise** ratio (heartbeats drowning real events)
- **No correlation** between related events
- **Limited interactivity** - mostly read-only views

### After: "Clean 4-Panel Real-Time System"
The redesign addresses every pain point:
- **4-panel layout** with logical information hierarchy
- **Real-time updates** via SSE event streaming
- **Smart filtering** (heartbeats hidden by default)
- **Event correlation** (link start→complete with durations)
- **Interactive controls** (panel toggles, focus modes)
- **Actionable signals** (slow ops, failures, anomalies highlighted)

**Result**: Transformation from a "static wall of garbage" to a production-grade monitoring tool.

---

## Architecture

### Panel Layout

The dashboard uses a clean 4-panel layout optimized for information density and clarity:

```
┌───────────────────────────────────────────────────────────────┐
│ System Overview (6-8 lines, fixed height)                    │
│ • Health indicators  • Active agents  • Performance metrics   │
└───────────────────────────────────────────────────────────────┘
┌──────────────────────────────┬────────────────────────────────┐
│                              │                                │
│ Activity Stream (60%)        │ Agent Details (40%, top)       │
│ • Filtered event log         │ • Per-agent status             │
│ • Color-coded categories     │ • Work queue                   │
│ • Correlated durations       │ • Health metrics               │
│                              ├────────────────────────────────┤
│                              │                                │
│                              │ Operations (40%, bottom)       │
│                              │ • CLI command history          │
│                              │ • Outcomes & timings           │
│                              │ • Error details                │
└──────────────────────────────┴────────────────────────────────┘
```

### Event Flow

```
┌─────────────────┐
│  MCP Instances  │
│  (owner/client) │
└────────┬────────┘
         │ Events
         ▼
┌─────────────────┐
│   API Server    │
│  (port 3000)    │
└────────┬────────┘
         │ SSE Stream
         ▼
┌─────────────────┐      ┌──────────────┐
│   Dashboard     │─────→│  Filters     │
│   (SSE Client)  │      │  Correlation │
└─────────────────┘      └──────────────┘
         │
         ▼
┌─────────────────┐
│  Panel Widgets  │
│  (ratatui TUI)  │
└─────────────────┘
```

**Key Components**:
1. **SSE Client** (`spawn_sse_client`): Maintains persistent connection, auto-reconnects
2. **Event Router** (`process_events`): Routes events to appropriate panels
3. **Filter Engine** (`filters.rs`): Smart filtering with 8 category types
4. **Correlation Tracker** (`correlation.rs`): Links start/complete events, calculates durations
5. **Panel Manager** (`panel_manager.rs`): Visibility controls, layout logic

---

## Features

### 1. Smart Event Filtering

**Default Behavior**: Hide heartbeats, show everything else.

**8 Event Categories**:
- **CLI**: Command-line operations (`remember`, `recall`, `evolve`)
- **Memory**: Storage operations (stored, recalled, consolidated)
- **Agent**: Lifecycle events (started, completed, failed)
- **Skill**: Skill loading/usage tracking
- **Work**: Orchestration (work items, phases, deadlocks)
- **Context**: Context modifications and validations
- **System**: Health updates, heartbeats, sessions
- **Error**: All failures and degraded states (highest priority)

**Filter Presets**:
```rust
FilterPresets::default()         // Hide heartbeats
FilterPresets::errors_only()     // Show failures only
FilterPresets::cli_focus()       // CLI commands only
FilterPresets::memory_focus()    // Memory operations only
FilterPresets::agent_focus("id") // Specific agent
FilterPresets::activity_focus()  // Hide system noise
FilterPresets::recent(5)         // Last 5 minutes
FilterPresets::high_priority()   // Errors + work events
```

**Compound Filters** (AND/OR/NOT logic):
```rust
EventFilter::All(vec![
    EventFilter::HideHeartbeats,
    EventFilter::Category(EventCategory::Memory),
])
```

### 2. Event Correlation

**Purpose**: Link related events to show operation lifecycles with durations.

**Supported Correlations**:
| Start Event | End Event | Key |
|-------------|-----------|-----|
| `CliCommandStarted` | `CliCommandCompleted`/`Failed` | Command name |
| `AgentStarted` | `AgentCompleted`/`Failed` | Agent ID |
| `MemoryEvolutionStarted` | (implicit completion) | Instance-wide |
| `WorkItemAssigned` | `WorkItemCompleted` | Work item ID |

**Automatic Detection**:
- **Slow operations**: >1s for CLI, >5s for agents, >10s for evolution
- **Failures**: Tracks failed vs. completed operations
- **Success rate**: Percentage of successful operations

**Example**:
```
[14:23:45] CLI: remember started
[14:23:46] CLI: remember completed (1.2s) ✓
```

### 3. Real-Time Updates

**SSE Streaming**:
- Zero-latency event delivery from API server
- Auto-reconnect with 5-second retry on disconnect
- Multiplexed stream across all MCP instances

**Update Frequency**:
- Events: Real-time (SSE push)
- State polling: Configurable (default 1000ms)
- Rendering: 16ms (60 FPS)

### 4. Keyboard Shortcuts

**Full interactive control** without mouse:

| Key | Action | Description |
|-----|--------|-------------|
| `q` / `Esc` | Quit | Exit dashboard |
| `0` | Toggle All | Show/hide all panels |
| `h` | Toggle Header | Show/hide System Overview |
| `1` | Toggle Activity | Show/hide Activity Stream |
| `2` | Toggle Agents | Show/hide Agent Details |
| `3` | Toggle Operations | Show/hide Operations panel |
| `c` | Clear Stream | Clear Activity Stream history |
| `v` | Cycle View | Cycle Operations view mode (planned) |
| `e` | Error Focus | Show errors only (planned) |
| `a` | Agent Focus | Filter by agent (planned) |
| `?` | Help | Show help overlay (planned) |

**Navigation** (planned):
- `↑`/`↓`: Scroll active panel
- `PgUp`/`PgDn`: Page navigation

### 5. Color-Coded Events

**Visual Hierarchy**:
- **Red**: Errors, failures, degraded health
- **Yellow**: Warnings, slow operations
- **Green**: Success, completions
- **Blue**: CLI operations, work items
- **Cyan**: Agent activity
- **Magenta**: Memory operations
- **Gray**: System events (heartbeats, health)

**Category Badges**: Each event shows category tag (`[CLI]`, `[Agent]`, `[Error]`)

---

## Panel Reference

### Panel 1: System Overview (Top, 6-8 lines)

**Purpose**: At-a-glance system health snapshot.

**Displays**:
- **Connection Status**: Connected/disconnected to API server
- **Active Agents**: Count and health status
- **Event Metrics**: Events/sec, filter pass rate
- **Performance**: Memory usage, CPU, context utilization (planned)

**Data Source**: Aggregated from event stream + periodic `/state/agents` polling.

**Example**:
```
╭─ System Overview ──────────────────────────────────────────╮
│ Status: Connected | Agents: 4/4 healthy | Events: 12/sec  │
│ Filter: 85% pass rate | Memory: 45MB | Context: 62%       │
╰────────────────────────────────────────────────────────────╯
```

### Panel 2: Activity Stream (Left, 60%)

**Purpose**: Intelligent event log with filtering and correlation.

**Displays**:
- **Filtered Events**: Smart hiding of noise (heartbeats default off)
- **Correlated Durations**: Shows operation timings when available
- **Color Coding**: Visual categorization
- **Scrollback**: Configurable history depth

**Controls**:
- `c`: Clear history
- `e`: Error focus mode (planned)

**Example**:
```
╭─ Activity Stream ──────────────────────────────────────────╮
│ 14:23:45 [CLI] remember started                           │
│ 14:23:46 [CLI] remember completed (1.2s) ✓                │
│ 14:23:47 [Agent] executor started: implement-auth         │
│ 14:23:50 [Error] executor failed: missing dependency ✗    │
╰────────────────────────────────────────────────────────────╯
```

### Panel 3: Agent Details (Right-Top, 40%)

**Purpose**: Deep-dive into agent activity and work queues.

**Displays**:
- **Per-Agent Status**: Running/idle/failed states
- **Current Work**: What each agent is executing
- **Work Queue**: Pending items with priorities
- **Health Metrics**: Error counts, restart counts

**Data Source**: Periodic `/state/agents` polling + agent events.

**Example**:
```
╭─ Agent Details ────────────────────────────────────────────╮
│ orchestrator   RUNNING  work_item_7  (2 queued)           │
│ optimizer      IDLE     -            (0 queued)           │
│ executor       RUNNING  implement-auth                     │
│ reviewer       BLOCKED  waiting-for-tests                  │
╰────────────────────────────────────────────────────────────╯
```

### Panel 4: Operations (Right-Bottom, 40%)

**Purpose**: CLI command history and outcomes.

**Displays**:
- **Command History**: Recent CLI invocations
- **Status**: Running/completed/failed
- **Durations**: Time to complete
- **Results**: Summary or error message

**View Modes** (cycle with `v`, planned):
- **List**: Chronological command list
- **Grouped**: Group by command type
- **Statistics**: Success rates, avg durations

**Example**:
```
╭─ Operations ───────────────────────────────────────────────╮
│ Time     Command   Status      Duration  Result           │
│ 14:23:46 remember  Completed   1.2s      Stored mem-a3f8  │
│ 14:24:10 recall    Completed   0.5s      5 results        │
│ 14:24:30 evolve    Running     -         consolidating... │
╰────────────────────────────────────────────────────────────╯
```

---

## Keyboard Shortcuts

### Current (Implemented)

| Shortcut | Action | Panel | Description |
|----------|--------|-------|-------------|
| `q` | Quit | Global | Exit dashboard cleanly |
| `Esc` | Quit | Global | Alternative quit key |
| `0` | Toggle All Panels | Global | Show/hide all panels at once |
| `h` | Toggle System Overview | System Overview | Show/hide header panel |
| `1` | Toggle Activity Stream | Activity Stream | Show/hide event log |
| `2` | Toggle Agent Details | Agent Details | Show/hide agent panel |
| `3` | Toggle Operations | Operations | Show/hide CLI operations |
| `c` | Clear Activity Stream | Activity Stream | Clear event history |

### Planned (Future Enhancements)

| Shortcut | Action | Panel | Description |
|----------|--------|-------|-------------|
| `v` | Cycle View Mode | Operations | List → Grouped → Statistics |
| `e` | Error Focus Mode | Activity Stream | Show errors only |
| `a` | Agent Focus Mode | Activity Stream | Filter by agent ID |
| `?` | Help Overlay | Global | Show keyboard shortcuts |
| `↑`/`↓` | Scroll | Active Panel | Line-by-line scroll |
| `PgUp`/`PgDn` | Page | Active Panel | Page navigation |

---

## Usage Guide

### Basic Usage

**1. Start the Dashboard**:
```bash
# Default (connects to localhost:3000)
mnemosyne-dash

# Custom API URL
mnemosyne-dash --api http://localhost:3000

# Faster refresh rate
mnemosyne-dash --refresh 500
```

**2. Understand the Display**:
- **Top panel**: System health overview
- **Left panel**: Filtered event stream (heartbeats hidden)
- **Right-top**: Agent status and work queues
- **Right-bottom**: CLI command history

**3. Interactive Controls**:
- Press `0` to hide all panels for minimal view
- Press `1`, `2`, `3` to toggle individual panels
- Press `c` to clear cluttered event history
- Press `q` or `Esc` to exit

### Monitoring Scenarios

**Scenario 1: Watch Agent Orchestration**:
1. Keep all panels visible (`0` if hidden)
2. Watch **Activity Stream** for agent handoffs
3. Check **Agent Details** for work queue progress
4. Monitor **System Overview** for overall health

**Scenario 2: Debug CLI Command Failures**:
1. Toggle to **Operations** panel (`3`)
2. Look for red `Failed` status
3. Check error message in Result column
4. Cross-reference with **Activity Stream** for context

**Scenario 3: Monitor Memory Operations**:
1. Use error focus mode (`e`, planned) or watch **Activity Stream**
2. Look for `[Memory]` category tags
3. Check correlation for slow `recall` operations (>1s)
4. Verify `evolve` completes successfully

**Scenario 4: Troubleshoot Connection Issues**:
1. Check **System Overview** connection status
2. If "Connecting...", verify API server is running:
   ```bash
   curl http://localhost:3000/health
   ```
3. Dashboard auto-reconnects every 5 seconds
4. Check logs: `tail -f /tmp/mnemosyne-dash.log`

### Advanced Usage

**Custom Refresh Rate**:
```bash
# High-frequency monitoring (100ms)
mnemosyne-dash --refresh 100

# Low-frequency monitoring (5s)
mnemosyne-dash --refresh 5000
```

**Debug Logging**:
```bash
# Enable debug logs
mnemosyne-dash --log-level debug

# View logs
tail -f /tmp/mnemosyne-dash.log
```

**Multi-Instance Monitoring**:
The dashboard aggregates events from **all MCP instances** (both owner and clients). No special configuration needed - events are automatically forwarded via the API server.

---

## Technical Details

### Event-Driven Architecture

**Components**:
1. **SSE Client** (`spawn_sse_client`):
   - Tokio async task, persistent HTTP connection
   - Parses SSE protocol (`data:`, empty line delimiters)
   - Deserializes JSON events, sends to app via `mpsc::unbounded_channel`
   - Auto-reconnects on disconnect with 5s backoff

2. **Event Router** (`App::process_events`):
   - Drains all available events from channel (`try_recv` loop)
   - Routes to panels based on event type
   - Updates correlation tracker for operation timing

3. **Rendering Loop** (`main`):
   - 16ms event polling for 60 FPS responsiveness
   - Periodic state updates via HTTP GET (default 1s)
   - `tokio::select!` for concurrent I/O

### Smart Filtering System

**Implementation** (`filters.rs`):
- **Category-based**: 8 high-level categories (CLI, Memory, Agent, etc.)
- **Compound logic**: `All` (AND), `Any` (OR), `Not` (negation)
- **Regex search**: Full-text search across event JSON
- **Time ranges**: Filter by event age
- **Statistics**: Track pass/filter rates

**Default Filter**:
```rust
EventFilter::HideHeartbeats
```

**Custom Filters**:
```rust
// Show errors OR recent work events
EventFilter::Any(vec![
    EventFilter::ErrorsOnly,
    EventFilter::All(vec![
        EventFilter::Category(EventCategory::Work),
        EventFilter::TimeRange(Duration::from_secs(300)),
    ]),
])
```

### Event Correlation Engine

**Implementation** (`correlation.rs`):
- **Correlation Keys**: CliCommand, Agent, MemoryEvolution, WorkItem
- **State Tracking**: HashMap of pending operations
- **History**: Circular buffer (configurable max, default 100)
- **Duration Calc**: `ended_at - started_at` in milliseconds

**Slow Operation Detection**:
- CLI commands: >1 second
- Agent operations: >5 seconds
- Memory evolution: >10 seconds
- Work items: >2 seconds

**Statistics**:
- Pending/completed/failed counts
- Success rate percentage
- Operation type breakdown

### Panel Management

**Implementation** (`panel_manager.rs`):
- **Visibility Tracking**: 4 boolean flags (SystemOverview, ActivityStream, AgentDetails, Operations)
- **Dynamic Layout**: Calculates constraints based on visible panels
- **Keyboard Mapping**: Centralized toggle logic

**Layout Algorithm**:
1. If SystemOverview visible: allocate 8 lines (fixed)
2. Remaining space: horizontal split 60/40
3. Left (60%): ActivityStream
4. Right (40%): vertical split 50/50 for AgentDetails/Operations

### Time Series Visualization

**Implementation** (`time_series.rs`):
- **Unicode Sparklines**: Uses `▁▂▃▄▅▆▇█` characters
- **Circular Buffer**: Fixed-size (e.g., 50 points)
- **Auto-scaling**: Maps values to 0-7 range
- **Real-time Updates**: Add data points, render on demand

**Usage** (planned for System Overview):
```rust
let mut events_per_sec = TimeSeriesBuffer::new(50);
events_per_sec.add(12.5);
events_per_sec.add(15.0);
let sparkline = events_per_sec.render(); // "▂▃▄▅"
```

---

## Testing

### Test Coverage

**Total Test Stats** (as of redesign completion):
- **Source Lines**: 6,122 total (all dashboard code)
- **Test Functions**: 124+ comprehensive tests
- **Test Modules**: 12 modules with `#[cfg(test)]`

**Test Breakdown**:
- `filters.rs`: 8 tests (category, compound, presets, stats)
- `correlation.rs`: 11 tests (correlation, history, stats, edge cases)
- `panel_manager.rs`: ~10 tests (visibility, toggles, layout)
- `panels/activity_stream.rs`: ~30 tests (filtering, rendering)
- `panels/operations.rs`: ~25 tests (command tracking, status)
- `panels/agents.rs`: ~20 tests (agent state, work queues)
- `panels/system_overview.rs`: ~15 tests (metrics, aggregation)
- `time_series.rs`: ~5 tests (sparklines, auto-scaling)

**Key Test Scenarios**:
1. **Filtering**: Hide heartbeats, errors-only, category filters
2. **Correlation**: Start/complete matching, orphaned events, duration calc
3. **Edge Cases**: Empty state, disconnects, rapid events
4. **Panel Logic**: Visibility toggles, dynamic layout
5. **SSE Client**: Reconnection, malformed events, channel closure

### Running Tests

```bash
# All dashboard tests
cargo test --bin mnemosyne-dash

# Specific module
cargo test --bin mnemosyne-dash filters

# With output
cargo test --bin mnemosyne-dash -- --nocapture

# Coverage (requires tarpaulin)
cargo tarpaulin --bin mnemosyne-dash --out Html
```

### Manual Testing

**Checklist**:
- [ ] Dashboard connects to API server
- [ ] Events appear in Activity Stream in real-time
- [ ] Heartbeats are hidden by default
- [ ] Panel toggles (0, h, 1, 2, 3) work correctly
- [ ] CLI operations show in Operations panel with status/duration
- [ ] Agent states update in Agent Details panel
- [ ] Correlation links start/complete events with durations
- [ ] Slow operations highlighted in yellow
- [ ] Failed operations highlighted in red
- [ ] Dashboard auto-reconnects after API server restart
- [ ] Clean exit with `q` or `Esc`

---

## Troubleshooting

### Dashboard Won't Start

**Symptom**: `mnemosyne-dash` exits immediately or shows error.

**Solutions**:
1. **Check API server**:
   ```bash
   curl http://localhost:3000/health
   # Expected: {"status":"ok"}
   ```
2. **Verify MCP instance running**: API server starts with first MCP instance
3. **Check port conflicts**: API tries ports 3000-3010 sequentially
4. **Review logs**:
   ```bash
   tail -f /tmp/mnemosyne-dash.log
   ```

### No Events Appearing

**Symptom**: Dashboard shows "Connected" but Activity Stream is empty.

**Solutions**:
1. **Trigger events**: Run `mnemosyne remember "test"` or other CLI commands
2. **Check filters**: Press `e` for error-focus to verify filtering isn't too aggressive
3. **Verify SSE endpoint**:
   ```bash
   curl -N http://localhost:3000/events
   # Should stream events in SSE format
   ```
4. **Clear and retry**: Press `c` to clear stream, wait for new events

### Connection Keeps Dropping

**Symptom**: "Connecting..." message flashes frequently.

**Solutions**:
1. **Check API server stability**: Look for crashes in MCP logs
2. **Network issues**: If remote API, verify firewall/proxy settings
3. **Timeout too aggressive**: Increase refresh rate:
   ```bash
   mnemosyne-dash --refresh 2000
   ```
4. **Restart API server**: Kill all MCP instances, restart

### Performance Issues

**Symptom**: Dashboard is slow or unresponsive.

**Solutions**:
1. **High event rate**: Increase filter aggressiveness (error-only mode)
2. **Clear history**: Press `c` to clear Activity Stream
3. **Reduce refresh rate**:
   ```bash
   mnemosyne-dash --refresh 2000
   ```
4. **Limit concurrent operations**: Too many agents spawning events

### Keyboard Shortcuts Not Working

**Symptom**: Pressing keys has no effect.

**Solutions**:
1. **Terminal compatibility**: Ensure terminal supports raw mode
2. **SSH sessions**: May have input lag or key mapping issues
3. **Restart dashboard**: `q` to quit, restart `mnemosyne-dash`
4. **Check logs**: May show event handling errors

---

## Design Decisions

### Why 4 Panels Instead of 7?

**Old Layout (7 panels)**:
- Memory, Context, Work Progress, Active Agents, Beads Tasks, Events, System Overview
- Too fragmented, cognitive overload
- Redundant information (Memory/Context overlap)

**New Layout (4 panels)**:
- System Overview: Aggregated top-level metrics
- Activity Stream: Unified event log (replaces 3+ event panels)
- Agent Details: Focused agent monitoring
- Operations: Dedicated CLI visibility

**Benefits**:
- 30% more space per panel
- Clearer information hierarchy
- Less visual clutter
- Faster scanning for critical info

### Why Hide Heartbeats by Default?

**Problem**: Heartbeats (every 1-5 seconds) create noise, drowning actionable events.

**Solution**: Smart default filter hides heartbeats, shows everything else.

**User Control**: Can show heartbeats if needed (future filter toggle).

### Why SSE Instead of WebSockets?

**Reasons**:
1. **Simplicity**: HTTP-based, no handshake complexity
2. **Auto-reconnect**: Built into browser/client implementations
3. **One-way**: Dashboard only needs to receive events, not send
4. **Firewall-friendly**: Standard HTTP, no special ports

**Tradeoff**: No bidirectional communication (acceptable for monitoring use case).

---

## Future Enhancements

### Phase 5: Interactivity (Planned)

**Focus Modes**:
- `e`: Error focus (errors only)
- `a`: Agent focus (specific agent)
- `m`: Memory focus (memory ops only)
- `/`: Search mode (regex filter)

**Navigation**:
- `↑`/`↓`: Scroll active panel
- `PgUp`/`PgDn`: Page navigation
- `Home`/`End`: Jump to top/bottom

**Help System**:
- `?`: Show help overlay with all shortcuts
- Context-aware hints in status bar

### Phase 6: Anomaly Detection (Planned)

**Automatic Detection**:
- Slow operations (>threshold for type)
- Failure spikes (>5 errors/minute)
- Event floods (>100 events/second)
- Agent deadlocks (blocked >60s)

**Visual Alerts**:
- Flash red border on System Overview
- Badge counts on panels (e.g., "3 errors")
- Audio alerts (optional, configurable)

### Phase 7: Visual Polish (In Progress)

**Sparklines** (System Overview):
- Events/sec trend over 50-point window
- Agent activity heatmap
- Memory usage over time

**Responsive Layout**:
- 80 columns: Minimal (1 panel visible)
- 120 columns: Standard (2 panels)
- 160+ columns: Full (all 4 panels)

**Color Palette Consistency**:
- Unified color scheme across all panels
- High-contrast mode for accessibility
- Theme customization (dark/light)

---

## References

- **Source Code**: `/Users/rand/src/mnemosyne/src/bin/dash/`
- **API Spec**: See `src/api/` for event definitions
- **SSE Protocol**: [MDN SSE Guide](https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events)
- **Ratatui Docs**: [ratatui.rs](https://ratatui.rs/)

---

## Conclusion

The redesigned Mnemosyne Dashboard transforms real-time monitoring from a "static wall of garbage" into a clean, actionable, production-grade tool. With 124+ tests, 6,100+ lines of code, smart filtering, event correlation, and full keyboard control, it's ready for serious orchestration monitoring.

**Next Steps**:
1. Try it: `mnemosyne-dash`
2. Explore keyboard shortcuts: `0`, `1`, `2`, `3`, `c`, `q`
3. Monitor your multi-agent workflows in real-time
4. Provide feedback for future enhancements

**Questions?** Check [TROUBLESHOOTING.md](/Users/rand/src/mnemosyne/TROUBLESHOOTING.md) or open an issue.
