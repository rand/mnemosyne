# Dashboard Monitoring

Real-time monitoring of multi-agent orchestration via web dashboard.

## Overview

The mnemosyne dashboard provides live visibility into the orchestration system's multi-agent workflow. Watch as the Orchestrator coordinates work, the Optimizer manages context, the Reviewer validates quality, and the Executor implements tasks—all in real-time.

## Quick Start

1. **Start orchestration with dashboard enabled:**
   ```bash
   mnemosyne orchestrate "Implement user authentication" --dashboard
   ```

2. **Note the API URL** displayed in the output:
   ```
   🔗 Dashboard API: http://127.0.0.1:3000
      Connect dashboard: mnemosyne-dash --api http://127.0.0.1:3000
   ```

3. **Connect the dashboard client** (in a new terminal):
   ```bash
   mnemosyne-dash --api http://127.0.0.1:3000
   ```

## Dashboard Features

The dashboard provides a **7-panel layout** with real-time sparkline visualizations showing time-series trends for all metrics.

### Panel Keyboard Shortcuts

- `1` - Memory Panel
- `2` - Context Panel
- `3` - Work Panel
- `4` - Active Agents Panel
- `5` - Beads Panel (Task Tracking)
- `6` - Operations Panel (CLI Commands)
- `7` - Event Log Panel
- `q` - Quit dashboard

### Panel 1: Memory Panel

Monitors memory usage and recall performance:

- **Memory Count**: Total memories in current namespace
- **Recent Recalls**: Number of recent memory queries (with sparkline trend)
- **Cache Hit Rate**: Percentage of cache hits vs misses (with sparkline trend)
- **Recent Writes**: Number of recent memory stores (with sparkline trend)

**Time-series visualization**: Each metric includes a compact sparkline (Unicode block characters: ▁▂▃▄▅▆▇█) showing the last 50 data points.

### Panel 2: Context Panel

Tracks context budget and utilization:

- **Context Usage**: Current context utilization percentage (with sparkline trend)
- **File Tracking**: Number of context files being monitored (with sparkline trend)
- **Budget Status**: Remaining context budget with color-coded warning levels:
  - Green: < 75% usage
  - Yellow: 75-90% usage
  - Red: > 90% usage (triggers automatic compression)

**Features**:
- Real-time sparklines show context pressure over time
- Automatic checkpoint detection when usage exceeds 75%
- Visual warnings before context exhaustion

### Panel 3: Work Panel

Displays orchestration progress with critical path tracking:

- **Progress Bar**: Overall completion (completed/total tasks)
- **Current Phase**: Active Work Plan Protocol phase
- **Completion Trend**: Sparkline showing task completion velocity
- **Critical Path Progress**: Percentage complete on critical path (with sparkline)
- **Parallel Streams**: Active parallel work streams (up to 3 shown, with count)

**Color-coded progress**:
- Red: < 33% complete
- Yellow: 33-66% complete
- Green: > 66% complete

### Panel 4: Active Agents Panel

Real-time status for all 4 orchestration agents:

- **Orchestrator**: Coordinates work distribution and phase transitions
- **Optimizer**: Manages context budget and skill discovery
- **Reviewer**: Validates completeness and quality gates
- **Executor**: Implements code, tests, and documentation

**Each agent displays**:
- State indicator with icon (●=Active, ○=Idle, ◐=Waiting, ✓=Completed, ✗=Failed)
- Current task or status message (truncated to 40 chars)
- Health indicator (❤=Healthy, ⚠=Degraded with error count)
- Activity sparkline showing agent active/idle pattern over time

**Agent count sparkline**: Shows total active agents over time at top of panel.

### Panel 5: Beads Panel (Task Tracking)

Integrates with Beads task management:

- **Open Issues**: Number of unresolved tasks
- **Ready Tasks**: Unblocked tasks available for work (with sparkline trend)
- **Recent Completions**: Number of recently closed tasks (with sparkline trend)
- **Task Velocity**: Rate of task completion over time

**Note**: Requires Beads to be initialized in the project.

### Panel 6: Operations Panel (CLI Commands)

Real-time visibility into CLI command activity:

- **Command**: The CLI command being executed (remember, recall, evolve, etc.)
- **Arguments**: Command arguments (truncated for display)
- **Status**: Color-coded execution status
  - **RUNNING** (blue): Command currently executing
  - **DONE** (green): Command completed successfully
  - **FAIL** (red): Command encountered an error
- **Duration**: Execution time (ms, seconds, or minutes)
- **Result**: Summary of the operation result or error message

**Features**:
- Table view with 5 columns: Time, Command, Status, Duration, Result
- Shows most recent operations first (reverse chronological)
- Scrollable history (last 100 operations)
- Automatic updates as CLI commands are executed
- Works independently of orchestration (shows direct CLI usage)

**Event Types**:
- `CliCommandStarted`: Command initiated
- `CliCommandCompleted`: Command finished successfully
- `CliCommandFailed`: Command encountered error
- `RecallExecuted`: Memory search performed
- `RememberExecuted`: Memory stored
- `EvolveStarted`: Memory evolution job started
- `EvolveCompleted`: Memory evolution finished
- `SearchPerformed`: Database search executed
- `DatabaseOperation`: Direct database operation

**Example Display**:
```
┌─ CLI Operations (3 total) ───────────────────────────────┐
│ Time     Command       Status   Duration  Result         │
│ 14:23:45 remember      DONE     1.9s      Stored memo... │
│ 14:23:30 recall        DONE     453ms     Found 5 mem... │
│ 14:23:15 evolve        RUNNING  -         ...            │
└──────────────────────────────────────────────────────────┘
```

**Integration**: This panel shows CLI activity even when not using `mnemosyne orchestrate`. It provides visibility into direct command usage via the event bridge system.

### Panel 7: Event Log Panel

Scrollable event stream with real-time updates:

- **Agent Events**: Started, completed, failed
- **Work Item Events**: Assigned, started, completed, retried
- **Phase Transitions**: PromptToSpec → SpecToFullSpec → FullSpecToPlan → PlanToArtifacts
- **Context Events**: Checkpoints, skill discoveries, optimizations
- **System Events**: Heartbeats (every 30s maintain connection)

**Features**:
- Auto-scroll with latest events at bottom
- Color-coded event types
- Timestamp for each event
- Scrollable history (last 100 events)

## Command Options

### Orchestrate with Dashboard

```bash
mnemosyne orchestrate [OPTIONS] --plan <PLAN>
```

**Required**:
- `--plan <text|json>`: Work plan (prompt or structured JSON)

**Optional**:
- `--dashboard`: Enable real-time monitoring (default: disabled)
- `--database <path>`: Custom database location
- `--max-concurrent <n>`: Max concurrent agents (default: 4)

### Dashboard Client

```bash
mnemosyne-dash --api <URL>
```

**Required**:
- `--api <URL>`: API server URL (e.g., `http://127.0.0.1:3000`)

**Optional**:
- `--update-interval <ms>`: Stats refresh interval (default: 1000ms)

## Usage Examples

### Simple Prompt-Based Orchestration

```bash
# Start with dashboard
mnemosyne orchestrate "Add rate limiting to API endpoints" --dashboard

# In another terminal
mnemosyne-dash --api http://127.0.0.1:3000
```

### Structured Work Plan

```bash
# Create work plan JSON
cat > plan.json <<EOF
{
  "phases": [
    {"name": "Spec", "tasks": ["Define requirements", "Design API"]},
    {"name": "Implementation", "tasks": ["Implement endpoints", "Add tests"]}
  ]
}
EOF

# Run with dashboard
mnemosyne orchestrate --plan "$(cat plan.json)" --dashboard

# Connect dashboard
mnemosyne-dash --api http://127.0.0.1:3000
```

### Monitoring CLI Commands

The dashboard can monitor direct CLI usage without orchestration:

```bash
# Terminal 1: Start API server
mnemosyne api-server

# Terminal 2: Start dashboard
mnemosyne-dash --api http://127.0.0.1:3000

# Terminal 3: Run CLI commands - visible in dashboard Operations panel
mnemosyne remember -c "Testing dashboard visibility" -n "test" -i 7
mnemosyne recall -q "dashboard" -l 5
mnemosyne evolve
```

The Operations panel (keyboard shortcut `6`) will show real-time status for each command.

### Multiple Concurrent Orchestrations

When running multiple orchestration sessions, the API server automatically selects available ports (3000-3010):

```bash
# Terminal 1
mnemosyne orchestrate "Task A" --dashboard
# Uses http://127.0.0.1:3000

# Terminal 2
mnemosyne orchestrate "Task B" --dashboard
# Uses http://127.0.0.1:3001

# Terminal 3 (dashboard for Task A)
mnemosyne-dash --api http://127.0.0.1:3000

# Terminal 4 (dashboard for Task B)
mnemosyne-dash --api http://127.0.0.1:3001
```

## Understanding Agent States

### Idle
Agent is waiting for work assignment. Default state after initialization.

### Active
Agent is actively working on a task. The dashboard shows:
- Task description
- Time since task started

### Waiting
Agent is blocked waiting for:
- Another agent to complete work
- Context availability
- Resource allocation

### Completed
Agent finished its assigned work successfully. Transitions back to Idle.

### Failed
Agent encountered an error. Orchestrator will:
- Retry the task (up to 3 attempts)
- Reassign to different agent
- Mark work item as failed if unrecoverable

## Event Types Explained

### Work Item Lifecycle

1. **WorkItemAssigned**: Orchestrator assigns task to agent
2. **WorkItemStarted**: Agent begins working (Idle → Active)
3. **WorkItemCompleted**: Agent finishes successfully (Active → Idle)

Alternative flows:
- **WorkItemFailed**: Agent encounters error
- **WorkItemRetried**: Orchestrator retries failed work

### Phase Transitions

The Work Plan Protocol progresses through 4 phases:

1. **PromptToSpec**: Transform user request into specification
2. **SpecToFullSpec**: Decompose into components with dependencies
3. **FullSpecToPlan**: Create execution plan with parallelization
4. **PlanToArtifacts**: Execute plan, create code/tests/docs

Each transition requires Reviewer approval.

### Context Management

- **ContextCheckpoint**: Optimizer saves context snapshot when usage > 75%
- **ContextOptimized**: Context compression applied
- **SkillDiscovered**: New skill loaded for current task

## Troubleshooting

### Dashboard Won't Connect

**Symptom**: `mnemosyne-dash` shows "Connection failed"

**Solutions**:
1. Verify API server is running:
   ```bash
   curl http://127.0.0.1:3000/health
   ```
   Should return: `{"status":"ok"}`

2. Check correct port number in orchestrate output
3. Ensure no firewall blocking localhost:3000

### No Events Appearing

**Symptom**: Dashboard connected but event stream empty

**Solutions**:
1. Verify orchestration is actually running (check terminal output)
2. Wait for heartbeat events (arrive every 10 seconds)
3. Check that `--dashboard` flag was used when starting orchestrate

### "Port Already in Use" Error

**Symptom**: `Address already in use` when starting orchestrate

**Solutions**:
1. Use different port (automatic: tries 3000-3010)
2. Kill existing orchestration:
   ```bash
   pkill -f "mnemosyne orchestrate"
   ```
3. Wait a few seconds for port to be released

### Agents Stuck in "Waiting" State

**Symptom**: All agents show "Waiting" but no progress

**Possible causes**:
1. **Dependency deadlock**: Check for circular work item dependencies
2. **Context exhaustion**: Optimizer needs to free context
3. **Resource contention**: Reduce `--max-concurrent` value

**Debug**:
```bash
# Check orchestration logs
tail -f .mnemosyne/logs/orchestration.log

# Check for deadlock detection events in dashboard
```

### High Context Usage Warning

**Symptom**: Context usage > 90% shown in stats

**Actions**:
1. Optimizer will automatically checkpoint and compress context
2. Non-critical context will be unloaded
3. Skills may be temporarily unloaded and reloaded on demand

## Performance Considerations

### Event Stream Bandwidth

- Dashboard uses Server-Sent Events (SSE) for low-latency streaming
- Typical bandwidth: 1-5 KB/s during active orchestration
- Events are JSON-formatted for easy parsing

### Dashboard Updates

- Agent states: Updated on every event
- Statistics: Recomputed on every state change
- Heartbeats: Every 10 seconds keep connection alive

### Multiple Dashboards

You can connect multiple dashboard clients to the same API server:
```bash
# Multiple terminals all connected to same orchestration
mnemosyne-dash --api http://127.0.0.1:3000  # Terminal 1
mnemosyne-dash --api http://127.0.0.1:3000  # Terminal 2
mnemosyne-dash --api http://127.0.0.1:3000  # Terminal 3
```

All clients receive the same event stream in real-time.

## API Endpoints

For custom integrations or debugging:

### Health Check
```bash
GET http://127.0.0.1:3000/health
```
Returns: `{"status":"ok","version":"2.1.1","instance_id":"..."}`

### Event Stream (SSE)
```bash
GET http://127.0.0.1:3000/events
```
Returns: Server-Sent Events stream

### Agent States
```bash
GET http://127.0.0.1:3000/state/agents
```
Returns: JSON array of all agent states

### System Statistics
```bash
GET http://127.0.0.1:3000/state/stats
```
Returns: JSON object with system metrics

### Context Files
```bash
GET http://127.0.0.1:3000/state/context-files
```
Returns: JSON array of tracked context files

## See Also

- [Orchestration Command Reference](./orchestration.md)
- [Multi-Agent Architecture](../specs/multi-agent-architecture.md)
- [API Server Architecture](../features/API_SERVER.md)
- [Work Plan Protocol](../specs/work-plan-protocol.md)
