---
name: mnemosyne-context-preservation
description: Preserving context across sessions using Mnemosyne memory system
---

# Mnemosyne Context Preservation

**Scope**: Using Mnemosyne to prevent context loss and maintain continuity across sessions
**Lines**: ~300
**Last Updated**: 2025-10-27

## When to Use This Skill

Activate this skill when:
- Starting a new Claude Code session on an existing project
- Approaching context window limits (>75% utilization)
- Coordinating work across multiple agents
- Building context-aware agent prompts
- Implementing context compaction strategies
- Designing session handoff protocols
- Preventing "amnesia" between sessions

## Core Problem

**Context window limitations create amnesia**:
- Each new session starts with empty context
- Previous decisions forgotten
- Patterns rediscovered multiple times
- Architecture rationale lost
- Bug fixes repeated
- Team agreements ignored

**Mnemosyne solves this** through persistent semantic memory that survives session boundaries.

## Context Budget Model

### Allocation Strategy

**Total Context Budget**: 200K tokens (typical)

**Optimizer allocates**:
- **40% Critical**: Essential project context
  - Architecture decisions (from memory)
  - Current work plan
  - Key constraints
  - Recent discoveries
- **30% Skills**: Loaded skills (like this one)
  - Mnemosyne-specific skills (project-local)
  - cc-polymath skills (global)
  - Dynamically discovered
- **20% Project**: Files and code
  - Currently edited files
  - Related modules
  - Test files
- **10% General**: Miscellaneous
  - Tool outputs
  - Error messages
  - Temporary state

### Memory's Role in Critical Budget

**Memory provides 40% critical context**:
1. **Architecture decisions**: Load high-importance memories
2. **Pattern library**: Discovered code patterns
3. **Constraint catalog**: Requirements and limitations
4. **Bug knowledge**: Past issues and solutions
5. **Team agreements**: Conventions and preferences

**Without memory**:
- Must rediscover patterns
- Re-explain architecture
- Repeat past mistakes
- Lost team knowledge

**With memory**:
- Instant context restoration
- Pattern recognition
- Mistake avoidance
- Preserved team wisdom

## Context Preservation Strategies

### Session Start Protocol

**Automatic context loading**:

```python
async def session_start(project_path: str):
    # 1. Detect namespace (git + CLAUDE.md)
    namespace = detect_namespace(project_path)

    # 2. Load high-importance memories
    critical_memories = await mnemosyne.recall(
        query="architecture decisions constraints",
        namespace=namespace,
        min_importance=7,
        max_results=10
    )

    # 3. Build memory graph
    memory_ids = [m['id'] for m in critical_memories]
    graph = await mnemosyne.graph(
        seed_ids=memory_ids,
        max_hops=2  # Include related memories
    )

    # 4. Load full context
    context = await mnemosyne.context(
        memory_ids=graph['all_ids'],
        include_links=True
    )

    # 5. Provide to Orchestrator
    return {
        "critical_context": context,
        "namespace": namespace,
        "session_id": generate_session_id()
    }
```

**Orchestrator uses this** to bootstrap all agents with shared context.

### Pre-emptive Snapshots

**Trigger at 75% context utilization**:

```python
async def monitor_context_usage():
    current_usage = get_context_usage()  # tokens used
    total_budget = 200000

    if current_usage / total_budget >= 0.75:
        # Snapshot critical context to memory
        await snapshot_to_memory()

        # Compact non-critical context
        await compact_context()

        # Notify Orchestrator
        orchestrator.trigger_context_checkpoint()
```

**Snapshot process**:
1. Identify critical information in current context
2. Store to memory with high importance (8-9)
3. Tag with session ID for recovery
4. Update work plan with checkpoint reference

**Example snapshot**:
```json
{
  "content": "Current progress: Implemented authentication middleware with JWT validation. Next: Add refresh token rotation.",
  "namespace": "session:2025-10-27-143000",
  "importance": 8,
  "tags": ["checkpoint", "authentication", "in-progress"],
  "context": "Session checkpoint at 75% context usage"
}
```

### Phase Transition Checkpoints

**Store context at critical phases**:

```python
async def phase_transition(from_phase: str, to_phase: str, context: Dict):
    # Store phase completion summary
    await mnemosyne.remember(
        content=f"""Completed {from_phase} → {to_phase}

**Decisions made**:
{context['decisions']}

**Key discoveries**:
{context['discoveries']}

**Next steps**:
{context['next_steps']}
""",
        namespace=f"session:{context['session_id']}",
        importance=7,
        context=f"Phase transition: {from_phase} → {to_phase}"
    )
```

**Phases with checkpoints**:
- Prompt → Spec (requirements clarified)
- Spec → Full Spec (decomposition complete)
- Full Spec → Plan (execution strategy defined)
- Plan → Artifacts (implementation finished)

### Agent Coordination Context

**Shared context across agents**:

```python
async def coordinate_agents(work_plan: Dict):
    # Store work plan for all agents
    work_plan_id = await mnemosyne.remember(
        content=serialize_work_plan(work_plan),
        namespace="session:current",
        importance=9,
        context="Multi-agent coordination"
    )

    # Each agent loads shared context
    for agent in [orchestrator, optimizer, reviewer, executor]:
        agent_context = await build_agent_context(
            agent_id=agent.id,
            work_plan_id=work_plan_id
        )
        await agent.load_context(agent_context)
```

**Benefits**:
- All agents see same work plan
- Decisions visible across agents
- No duplicate work
- Coordinated handoffs

### Context Compaction

**When context exceeds 75%**:

1. **Identify compactable context**:
   - Low-importance memories (<5)
   - Temporary session state
   - Redundant information
   - Outdated references

2. **Move to memory**:
   - Store non-critical info to memory
   - Mark with session ID for recovery
   - Assign appropriate importance

3. **Remove from active context**:
   - Clear compacted information
   - Keep only critical context
   - Maintain work plan and current state

4. **Document compaction**:
   ```python
   await mnemosyne.remember(
       content=f"Context compacted: {compacted_items}",
       namespace="session:current",
       importance=6,
       tags=["compaction", "context-management"]
   )
   ```

## Session Handoff Protocol

### Session End Checklist

**Before ending session**:

1. **Export current state**:
   ```bash
   /memory-export
   ```

2. **Store session summary**:
   ```python
   await mnemosyne.remember(
       content="""Session Summary:

       **Completed**:
       - Feature X implemented
       - Tests passing

       **In Progress**:
       - Feature Y (50% complete)

       **Blocked**:
       - Feature Z (waiting for API key)

       **Next Session**:
       - Complete Feature Y
       - Resolve Feature Z blocker
       """,
       namespace="session:2025-10-27-143000",
       importance=8,
       context="Session end summary"
   )
   ```

3. **Tag incomplete work**:
   - Mark in-progress memories
   - Reference related files
   - Document blockers

4. **Commit code**:
   - Ensure all changes committed
   - Push to remote
   - Tag with session reference

### Session Start Checklist

**Beginning new session**:

1. **Load session history**:
   ```bash
   /memory-search "session summary"
   ```

2. **Review last session**:
   - Read session end summary
   - Check in-progress items
   - Note any blockers

3. **Load project context**:
   ```bash
   /memory-context
   ```

4. **Verify environment**:
   - Pull latest code
   - Check dependencies
   - Review beads state

## ACE Framework Integration

**Accumulated Context Enrichment (ACE)**:

### Incremental Updates

**Don't recreate context from scratch**:
```python
# BAD: Rebuild entire context
context = await build_full_context()

# GOOD: Incremental update
context = await update_context_incrementally(
    previous_context=load_from_memory(),
    new_information=current_changes
)
```

### Structured Accumulation

**Organize context hierarchically**:
```
Project Context
├── Architecture (importance: 9)
│   ├── Core decisions
│   ├── Module structure
│   └── Integration patterns
├── Patterns (importance: 7)
│   ├── Code patterns
│   ├── Test patterns
│   └── Deployment patterns
├── Constraints (importance: 8)
│   ├── Performance requirements
│   ├── Security requirements
│   └── Compatibility requirements
└── History (importance: 5)
    ├── Bug fixes
    ├── Refactorings
    └── Team discussions
```

### Strategy Preservation

**Preserve decision rationale**:
```python
await mnemosyne.remember(
    content=f"""Decision: {decision}

Rationale: {why_this_approach}

Alternatives Considered:
- Option A: {why_not_a}
- Option B: {why_not_b}

Constraints:
- {constraint_1}
- {constraint_2}

Expected Impact:
- {impact}
""",
    namespace="project:myapp",
    importance=9
)
```

## Monitoring and Metrics

### Context Health Metrics

**Track via Coordinator**:
```python
coordinator.set_metric("context_usage_pct", 0.65)  # 65%
coordinator.set_metric("memory_count", 127)        # memories loaded
coordinator.set_metric("skill_count", 7)           # skills loaded
coordinator.set_metric("critical_pct", 0.40)       # 40% critical context
```

**Alert thresholds**:
- Context >75%: Trigger pre-emptive snapshot
- Context >90%: Emergency compaction
- Memory count >50: Consider consolidation
- Skill count >10: Reduce skill loading

### Context Preservation Quality

**Measure preservation effectiveness**:
```python
metrics = {
    "session_continuity": 0.85,  # 85% context preserved
    "decision_recall": 0.92,     # 92% decisions recovered
    "pattern_reuse": 0.78,       # 78% patterns applied
    "amnesia_events": 2          # 2 context loss incidents
}
```

## Best Practices

### DO

- Store context at phase transitions
- Snapshot at 75% context usage
- Tag memories with session IDs
- Use memory graph for related context
- Preserve decision rationale
- Monitor context health metrics
- Checkpoint before risky operations

### DON'T

- Wait until 100% context usage
- Store every small detail
- Ignore context budget limits
- Skip session end summaries
- Forget to tag in-progress work
- Rely on agent memory alone
- Assume context will survive

### Context Budget Guidelines

**Critical (40%)**:
- Architecture decisions (importance 8-10)
- Active work plan
- Key constraints
- Current phase state

**Skills (30%)**:
- Task-relevant skills only
- Max 7 skills loaded
- Project-local priority
- Unload unused skills

**Project (20%)**:
- Currently edited files
- Related dependencies
- Relevant tests
- Minimal boilerplate

**General (10%)**:
- Tool outputs
- Error messages
- Temporary notes

## Common Pitfalls

**Context bloat**:
- Loading too many memories
- Not compacting non-critical info
- Keeping stale context
- Over-loading skills

**Context loss**:
- Skipping checkpoints
- Not storing phase transitions
- Forgetting session summaries
- Ignoring 75% threshold

**Poor organization**:
- Flat memory structure
- Missing importance scores
- No namespace strategy
- Weak semantic links

## Further Reading

- `mnemosyne-memory-management.md`: Memory operations and best practices
- `ARCHITECTURE.md`: Multi-agent coordination patterns
- `CLAUDE.md`: Multi-agent orchestration system
- `/memory-context`: Load project context
- `/memory-export`: Export memories for backup
