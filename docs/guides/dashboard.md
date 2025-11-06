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

### Agent Status Panel

The dashboard displays real-time status for all 4 agents:

- **Orchestrator**: Coordinates work distribution and phase transitions
- **Optimizer**: Manages context budget and skill discovery
- **Reviewer**: Validates completeness and quality gates
- **Executor**: Implements code, tests, and documentation

Each agent shows:
- Current state (Idle, Active, Waiting, Completed, Failed)
- Active task (when working)
- Last updated timestamp

### Event Stream

Live feed of orchestration events:

- **Agent Events**: Started, completed, failed
- **Work Item Events**: Assigned, started, completed, retried
- **Phase Transitions**: PromptToSpec → SpecToFullSpec → FullSpecToPlan → PlanToArtifacts
- **Context Events**: Checkpoints, skill discoveries
- **System Events**: Heartbeats (every 10s)

### System Statistics

Real-time metrics:
- Active agents count
- Total agents in session
- Completed tasks
- Failed tasks
- Context usage percentage

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
