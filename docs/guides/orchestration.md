# Orchestration Command Reference

Multi-agent orchestration for complex software engineering tasks.

## Overview

The `mnemosyne orchestrate` command launches a sophisticated multi-agent system that coordinates four specialized agents to complete complex software tasks following the Work Plan Protocol.

### The Four Agents

1. **Orchestrator**: Central coordinator managing work distribution, dependencies, and phase transitions
2. **Optimizer**: Context budget manager and skill discovery system
3. **Reviewer**: Quality gate enforcer and validation specialist
4. **Executor**: Primary implementation agent that executes tasks

## Command Syntax

```bash
mnemosyne orchestrate [OPTIONS] --plan <PLAN>
```

## Required Arguments

### `--plan <PLAN>`

The work plan defining what to accomplish. Can be:

1. **Plain text prompt** (interpreted as user intent):
   ```bash
   mnemosyne orchestrate --plan "Add user authentication with JWT"
   ```

2. **Structured JSON** (explicit phase breakdown):
   ```bash
   mnemosyne orchestrate --plan '{
     "phases": [
       {"name": "Spec", "tasks": ["Define requirements"]},
       {"name": "Implementation", "tasks": ["Implement auth"]}
     ]
   }'
   ```

3. **File path** (JSON or text file):
   ```bash
   mnemosyne orchestrate --plan @plan.json
   ```

## Optional Arguments

### `--dashboard`

Enable real-time monitoring via dashboard.

```bash
mnemosyne orchestrate --plan "..." --dashboard
```

When enabled:
- Starts embedded API server (http://127.0.0.1:3000)
- Broadcasts events to connected dashboard clients
- Provides REST API for agent states and statistics

**See**: [Dashboard Guide](./dashboard.md) for detailed usage

### `--database <PATH>`

Custom database location (default: `.mnemosyne/project.db`).

```bash
mnemosyne orchestrate --plan "..." --database /path/to/custom.db
```

Use cases:
- Isolate orchestration sessions
- Use shared database across projects
- Preserve orchestration history separately

### `--max-concurrent <N>`

Maximum concurrent agents (default: 4).

```bash
mnemosyne orchestrate --plan "..." --max-concurrent 2
```

Lower values:
- Reduce resource usage
- Simplify debugging
- Avoid context thrashing

Higher values (not recommended):
- May increase coordination overhead
- Can lead to context exhaustion

## Work Plan Formats

### 1. Prompt-Based (Recommended for Quick Tasks)

The simplest approach—provide a natural language description:

```bash
mnemosyne orchestrate --plan "Implement rate limiting for API endpoints"
```

The Orchestrator will:
1. Parse intent (Phase 1: PromptToSpec)
2. Decompose into components (Phase 2: SpecToFullSpec)
3. Create execution plan (Phase 3: FullSpecToPlan)
4. Execute tasks (Phase 4: PlanToArtifacts)

**Best for**:
- Feature requests
- Bug fixes
- Refactoring tasks
- Quick prototypes

### 2. Structured JSON (Recommended for Complex Projects)

Explicitly define phases and tasks:

```json
{
  "phases": [
    {
      "name": "Requirements",
      "tasks": [
        "Define authentication requirements",
        "Document security constraints",
        "Identify integration points"
      ]
    },
    {
      "name": "Design",
      "tasks": [
        "Design JWT token structure",
        "Plan refresh token flow",
        "Design error handling"
      ]
    },
    {
      "name": "Implementation",
      "tasks": [
        "Implement token generation",
        "Implement token validation",
        "Add middleware integration"
      ],
      "dependencies": ["Design"]
    },
    {
      "name": "Testing",
      "tasks": [
        "Write unit tests",
        "Write integration tests",
        "Test token expiration"
      ],
      "dependencies": ["Implementation"]
    }
  ]
}
```

```bash
mnemosyne orchestrate --plan "$(cat work-plan.json)"
```

**Best for**:
- Large features with clear structure
- Multi-step refactorings
- Projects with known dependencies
- Team coordination

### 3. Hybrid Approach

Combine high-level prompt with structured constraints:

```json
{
  "intent": "Implement user authentication",
  "constraints": {
    "tech_stack": ["Rust", "Axum", "PostgreSQL"],
    "test_coverage": "90%",
    "documentation": "required"
  },
  "phases": [
    {"name": "Spec", "tasks": ["auto"]},
    {"name": "Implementation", "tasks": ["auto"]},
    {"name": "Testing", "tasks": ["auto"]}
  ]
}
```

The Orchestrator auto-generates tasks within defined phases.

## Usage Examples

### Basic Feature Implementation

```bash
mnemosyne orchestrate \
  --plan "Add password reset functionality" \
  --dashboard
```

What happens:
1. Orchestrator analyzes intent
2. Creates specification
3. Decomposes into tasks
4. Executor implements with Reviewer validation
5. Dashboard shows real-time progress

### Complex Multi-Phase Project

```bash
# 1. Create detailed plan
cat > api-redesign.json <<EOF
{
  "phases": [
    {
      "name": "Analysis",
      "tasks": [
        "Analyze existing API usage",
        "Identify breaking changes",
        "Document migration path"
      ]
    },
    {
      "name": "Design",
      "tasks": [
        "Design new API schema",
        "Plan backward compatibility",
        "Design deprecation strategy"
      ]
    },
    {
      "name": "Implementation",
      "tasks": [
        "Implement new endpoints",
        "Add compatibility layer",
        "Update documentation"
      ]
    },
    {
      "name": "Migration",
      "tasks": [
        "Create migration guide",
        "Update client examples",
        "Plan rollout strategy"
      ]
    }
  ]
}
EOF

# 2. Execute with monitoring
mnemosyne orchestrate \
  --plan "$(cat api-redesign.json)" \
  --dashboard

# 3. Connect dashboard (separate terminal)
mnemosyne-dash --api http://127.0.0.1:3000
```

### Parallel Orchestration Sessions

Run multiple independent tasks:

```bash
# Terminal 1: Feature A
mnemosyne orchestrate --plan "Feature A" --dashboard
# Uses port 3000

# Terminal 2: Feature B
mnemosyne orchestrate --plan "Feature B" --dashboard
# Auto-selects port 3001

# Terminal 3: Bug fix
mnemosyne orchestrate --plan "Fix bug #123"
# No dashboard needed
```

### Custom Database Isolation

```bash
# Experiment branch
mnemosyne orchestrate \
  --plan "Experimental refactoring" \
  --database .mnemosyne/experiment.db

# Main branch
mnemosyne orchestrate \
  --plan "Production fix" \
  --database .mnemosyne/main.db
```

## Work Plan Protocol

The orchestration follows a structured 4-phase protocol:

### Phase 1: Prompt → Spec

**Goal**: Transform user request into clear specification

**Activities**:
- Orchestrator analyzes intent
- Optimizer discovers relevant skills
- Clarifies ambiguities
- Reviewer validates understanding

**Output**: Specification document (`spec.md`)

### Phase 2: Spec → Full Spec

**Goal**: Decompose into components with dependencies

**Activities**:
- Orchestrator breaks down components
- Identifies dependencies
- Defines typed holes (interfaces)
- Creates test plan

**Output**:
- Component breakdown
- Dependency graph
- `test-plan.md`

### Phase 3: Full Spec → Plan

**Goal**: Create execution plan with parallelization

**Activities**:
- Orders tasks by dependencies
- Identifies parallel work streams
- Computes critical path
- Plans checkpoints

**Output**: `plan.md` with execution strategy

### Phase 4: Plan → Artifacts

**Goal**: Execute plan, create code/tests/docs

**Activities**:
- Executor implements tasks
- Reviewer validates each completion
- Tests run after each commit
- Orchestrator tracks progress

**Output**:
- Implemented code
- Passing tests
- Documentation
- Traceability matrix

### Phase Transitions

Transitions require Reviewer approval:

```
PromptToSpec → (review) → SpecToFullSpec → (review) →
FullSpecToPlan → (review) → PlanToArtifacts
```

Invalid transitions are rejected (e.g., skipping phases).

## Agent Coordination

### Work Item Lifecycle

1. **Submission**: Orchestrator creates work item
2. **Assignment**: Orchestrator assigns to agent
3. **Execution**: Agent processes work item
4. **Validation**: Reviewer checks completion
5. **Completion**: Work item marked done

### Dependency Handling

Work items can depend on others:

```json
{
  "tasks": [
    {"id": "task-1", "description": "Design API"},
    {"id": "task-2", "description": "Implement API", "depends_on": ["task-1"]},
    {"id": "task-3", "description": "Test API", "depends_on": ["task-2"]}
  ]
}
```

Orchestrator ensures:
- Dependencies execute before dependents
- No circular dependencies (deadlock detection)
- Parallel execution of independent tasks

### Error Handling

When work items fail:

1. **Retry**: Up to 3 attempts with same agent
2. **Reassign**: Try different agent if retry fails
3. **Escalate**: Orchestrator reviews error pattern
4. **Abort**: Mark unrecoverable after all options exhausted

Dashboard shows:
- `WorkItemFailed` events
- `WorkItemRetried` events
- Final failure reason if unrecoverable

## Context Management

### Context Budget

Default allocation:
- **Critical**: 40% (current work, recent commits)
- **Skills**: 30% (loaded skill documentation)
- **Project**: 20% (codebase context)
- **General**: 10% (conversation history)

### Optimizer Actions

When context exceeds 75%:
1. **Checkpoint**: Save current state
2. **Compress**: Summarize non-critical content
3. **Unload**: Remove low-priority skills
4. **Alert**: Broadcast `ContextCheckpoint` event

### Skill Discovery

Optimizer discovers skills based on:
- Task keywords (e.g., "authentication" → auth skills)
- File patterns (e.g., `*.rs` → Rust skills)
- Error messages (e.g., "E0425" → Rust error skills)

Skills are loaded on-demand and cached per session.

## Output Artifacts

Orchestration creates several artifacts:

### Specifications

- `spec.md`: Initial specification
- `test-plan.md`: Testing strategy
- `plan.md`: Execution plan
- `traceability.md`: Requirements → implementation mapping

### Logs

- `.mnemosyne/logs/orchestration.log`: Detailed execution log
- `.mnemosyne/logs/agent-{role}.log`: Per-agent logs

### Metadata

- `.mnemosyne/project.db`: Orchestration history
- `.mnemosyne/context-snapshots/`: Context checkpoints
- `.mnemosyne/work-items.json`: Work queue state

## Monitoring and Debugging

### Real-Time Monitoring

Use dashboard for live visibility:

```bash
# Start with dashboard
mnemosyne orchestrate --plan "..." --dashboard

# Connect dashboard
mnemosyne-dash --api http://127.0.0.1:3000
```

Watch:
- Agent states and transitions
- Work item progress
- Phase transitions
- Context usage
- Error events

### Debug Logs

Enable detailed logging:

```bash
RUST_LOG=debug mnemosyne orchestrate --plan "..."
```

Log levels:
- `error`: Only errors
- `warn`: Warnings and errors
- `info`: Key events (default)
- `debug`: Detailed execution
- `trace`: Everything (very verbose)

### API Inspection

Query orchestration state:

```bash
# Health check
curl http://127.0.0.1:3000/health

# Agent states
curl http://127.0.0.1:3000/state/agents | jq

# Statistics
curl http://127.0.0.1:3000/state/stats | jq

# Stream events
curl -N http://127.0.0.1:3000/events
```

## Performance Tips

### Optimize for Your Task

**Small tasks** (< 5 subtasks):
```bash
mnemosyne orchestrate --plan "Quick fix" --max-concurrent 2
```

**Large projects** (> 20 subtasks):
```bash
mnemosyne orchestrate --plan "$(cat complex-plan.json)" --dashboard
```

### Resource Management

**Low memory systems**:
- Reduce `--max-concurrent`
- Use prompt-based (less context overhead)
- Disable dashboard if not monitoring

**High-performance systems**:
- Use default `--max-concurrent 4`
- Enable dashboard for visibility
- Use structured plans for efficiency

### Database Optimization

**Fast SSD**:
- Use default database location
- Enable WAL mode (automatic)

**Network storage**:
- Use local database: `--database /tmp/orchestrate.db`
- Rsync results after completion

## Troubleshooting

### "No agents responding"

**Symptom**: Orchestration starts but no progress

**Solutions**:
1. Check logs: `tail -f .mnemosyne/logs/orchestration.log`
2. Verify agents started: Check dashboard or API `/state/agents`
3. Increase timeout: May need more time for initialization

### "Dependency deadlock detected"

**Symptom**: All agents in "Waiting" state

**Cause**: Circular task dependencies

**Solution**:
1. Review work plan dependencies
2. Remove circular dependencies
3. Restart orchestration with fixed plan

### "Context budget exhausted"

**Symptom**: "No context available" errors

**Solutions**:
1. Optimizer will automatically compress context
2. Wait for `ContextOptimized` event in dashboard
3. If persistent: Reduce scope or split into smaller tasks

### Orchestration hangs

**Symptom**: No progress for > 5 minutes

**Debug steps**:
1. Check agent states in dashboard
2. Review last events in stream
3. Check for errors in logs
4. Send SIGTERM for graceful shutdown: `pkill -TERM -f "mnemosyne orchestrate"`

## Best Practices

### 1. Start Simple

Begin with prompt-based plans:
```bash
mnemosyne orchestrate --plan "Your task here"
```

Graduate to structured plans as needed.

### 2. Use Dashboard for Complex Tasks

Enable monitoring for:
- Multi-phase projects
- Long-running tasks
- Debugging issues

### 3. Explicit Dependencies

In structured plans, always specify dependencies:
```json
{"task": "Test API", "depends_on": ["Implement API"]}
```

Don't rely on implicit ordering.

### 4. Checkpoint Frequently

For long-running orchestrations:
- Optimizer auto-checkpoints at 75% context
- Manual checkpoint: Pause and let context settle
- Resume from checkpoint if interrupted

### 5. Validate Incrementally

Don't wait for full completion:
- Monitor dashboard events
- Check partial artifacts
- Abort early if misaligned with intent

## See Also

- [Dashboard Monitoring Guide](./dashboard.md)
- [Multi-Agent Architecture](../specs/multi-agent-architecture.md)
- [Work Plan Protocol](../specs/work-plan-protocol.md)
- [Context Management](../features/CONTEXT_MANAGEMENT.md)
- [API Server Architecture](../features/API_SERVER.md)
