# Coordination Workflows

## Overview

This document describes common workflows for coordinating multiple agents working on the same codebase.

## Workflow 1: Independent Feature Development

**Scenario**: Two agents working on separate features

**Setup**:
```bash
# Agent 1
git checkout -b feature/authentication
mnemosyne branch join feature/authentication full --mode isolated

# Agent 2
git checkout -b feature/payment
mnemosyne branch join feature/payment full --mode isolated
```

**Characteristics**:
- Complete isolation between agents
- No conflict detection needed
- Each agent has full branch access
- Independent merge to main

**Best For**:
- Unrelated features
- Different modules
- Minimal dependencies

## Workflow 2: Collaborative Feature Development

**Scenario**: Multiple agents working together on a large feature

**Setup**:
```bash
# Agent 1 (Lead)
git checkout -b feature/dashboard
mnemosyne branch join feature/dashboard full --mode coordinated

# Agent 2 (UI)
git checkout feature/dashboard
mnemosyne branch join feature/dashboard write \
    --files src/ui/dashboard.tsx \
    --mode coordinated

# Agent 3 (Backend)
git checkout feature/dashboard
mnemosyne branch join feature/dashboard write \
    --files src/api/dashboard.rs \
    --mode coordinated
```

**Characteristics**:
- Multiple agents on same branch
- File-scoped work intent minimizes conflicts
- Automatic conflict detection
- Periodic notifications (every 20 minutes)

**Best For**:
- Large features requiring multiple specializations
- Paired programming
- Code review and refinement

## Workflow 3: Code Review and Testing

**Scenario**: One agent implements, another reviews

**Setup**:
```bash
# Agent 1 (Implementation)
git checkout -b feature/api-v2
mnemosyne branch join feature/api-v2 full --mode coordinated

# Agent 2 (Review)
git checkout feature/api-v2
mnemosyne branch join feature/api-v2 read --mode coordinated
```

**Characteristics**:
- Read-only access for reviewer
- Auto-approved (no user prompt)
- No conflict risk
- Reviewer can add tests if needed

**Best For**:
- Code review workflows
- Quality assurance
- Documentation review

## Workflow 4: Hotfix with Coordination

**Scenario**: Critical bug fix while other work continues

**Setup**:
```bash
# Agent 1 (Hotfix)
git checkout main
git checkout -b hotfix/critical-bug
mnemosyne branch join hotfix/critical-bug full --mode isolated

# Agent 2 (Ongoing work - paused)
mnemosyne branch release
# Wait for hotfix completion
```

**Characteristics**:
- Isolated hotfix branch
- Other work temporarily suspended
- Fast turnaround
- Minimal coordination overhead

**Best For**:
- Critical bug fixes
- Security patches
- Production incidents

## Workflow 5: Test-Driven Development

**Scenario**: One agent writes tests, another implements features

**Setup**:
```bash
# Agent 1 (Tests)
git checkout -b feature/user-service
mnemosyne branch join feature/user-service write \
    --files tests/test_user_service.rs \
    --mode coordinated

# Agent 2 (Implementation)
git checkout feature/user-service
mnemosyne branch join feature/user-service write \
    --files src/user_service.rs \
    --mode coordinated
```

**Characteristics**:
- Test isolation (no conflicts on test files)
- Clear separation of concerns
- Parallel work possible
- Tests define contract

**Best For**:
- TDD workflows
- Contract-first development
- API design

## Workflow 6: Refactoring with Orchestrator

**Scenario**: Orchestrator manages large-scale refactoring

**Setup**:
```bash
# Orchestrator (Special permissions)
mnemosyne branch join main full --mode isolated
# Orchestrator bypasses isolation rules automatically

# Spawns sub-agents for specific refactoring tasks
# Each sub-agent works on isolated files
```

**Characteristics**:
- Orchestrator has bypass permissions
- Can work on any branch regardless of assignments
- Coordinates sub-agents
- Maintains consistency

**Best For**:
- Large refactorings
- Cross-cutting changes
- Architecture updates
- Dependency updates

## Conflict Resolution Patterns

### Pattern 1: Sequential Work
```bash
# Agent 1 completes work
git commit -am "Implement feature X"
mnemosyne branch release

# Agent 2 starts
mnemosyne branch join feature/shared full
```

### Pattern 2: File Partitioning
```bash
# Agent 1: Frontend
mnemosyne branch join feature/dashboard write --files "src/ui/**"

# Agent 2: Backend
mnemosyne branch join feature/dashboard write --files "src/api/**"
```

### Pattern 3: Read-First
```bash
# All agents start read-only
mnemosyne branch join feature/complex read

# One agent escalates to write
mnemosyne branch release
mnemosyne branch join feature/complex write --files src/specific.rs
```

## Notification Handling

### On-Save Notifications
Triggered when: Agent saves file that conflicts with another agent's work

**Response**:
1. Review the conflict
2. Communicate with other agent (if external)
3. Coordinate file ownership
4. Consider file partitioning

### Periodic Notifications (Every 20 minutes)
Summary of all active conflicts

**Response**:
1. Review list of conflicts
2. Check if conflicts resolved naturally
3. Plan coordination if conflicts persist
4. Update work intent if needed

### Session-End Notifications
Final conflict report before session ends

**Response**:
1. Review unresolved conflicts
2. Document coordination needs
3. Release assignments
4. Plan next session

## Cross-Process Coordination

### Mnemosyne-Managed Agents
```bash
# Automatically coordinated through internal messaging
# No special setup needed
```

### Directly-Launched Claude Code Agents
```bash
# Use file-based coordination
export MNEMOSYNE_DIR=".mnemosyne"

# Agent registers on start
mnemosyne branch join feature/test write --files src/lib.rs

# Heartbeat sent automatically every 30 seconds
# Stale processes cleaned up after timeout
```

### Mixed Environment
```bash
# Mnemosyne-managed agent
mnemosyne orchestrator start

# External Claude Code agent
mnemosyne branch join feature/shared read
# Coordination messages exchange through .mnemosyne/coordination_queue/
```

## Advanced Patterns

### Progressive Escalation
```bash
# Start read-only
mnemosyne branch join feature/test read

# Escalate to write specific files
mnemosyne branch release
mnemosyne branch join feature/test write --files src/module.rs

# Escalate to full branch
mnemosyne branch release
mnemosyne branch join feature/test full --mode coordinated
```

### Temporary Coordination
```bash
# Request coordinated mode for pairing session
mnemosyne branch join feature/refactor full --mode coordinated

# After session, release and re-join in isolated mode
mnemosyne branch release
mnemosyne branch join feature/refactor full --mode isolated
```

### Conflict Avoidance by Timing
```bash
# Check status before joining
mnemosyne branch status --all

# If branch busy, wait or work elsewhere
mnemosyne branch join feature/other full
```

## Metrics and Monitoring

### Track Coordination Efficiency
```bash
# View coordination history
mnemosyne branch conflicts --all

# Export metrics
mnemosyne branch export --format json > coordination_metrics.json
```

### Identify Bottlenecks
- Frequently conflicting files
- Long-running isolated assignments
- High coordination overhead

### Optimization Opportunities
- Better file partitioning
- Clearer module boundaries
- More granular work intent

## Best Practices Summary

1. **Start Conservative**: Use isolated mode by default
2. **Scope Work Intent**: Be specific about files when using write intent
3. **Monitor Notifications**: Respond to conflict notifications promptly
4. **Release Early**: Release assignments when work is complete
5. **Communicate**: Use commit messages for async coordination
6. **Test Independently**: Leverage test isolation
7. **Use Read-Only**: Request read-only when reviewing or researching
8. **Trust the System**: Auto-approval and intelligent conflict detection work

## Examples Repository

See `examples/coordination-workflows/` for complete runnable examples of each workflow pattern.
