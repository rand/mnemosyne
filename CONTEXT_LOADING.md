# Context Loading in Mnemosyne

This guide explains how Mnemosyne provides intelligent context loading to Claude Code sessions through a three-layer strategy.

## Overview

Mnemosyne ensures that relevant project knowledge is available to agents throughout the session lifecycle:

1. **Pre-Launch** (Layer 1): Context loaded before Claude Code starts
2. **Post-Launch** (Layer 2): Session start hook displays context
3. **In-Session** (Layer 3): Optimizer dynamically manages context

Each layer serves a distinct purpose and operates at different times.

---

## Layer 1: Pre-Launch Context

### When
Before Claude Code process starts

### How
Via `--append-system-prompt` flag passed to `claude` CLI

### Owner
Launcher (`src/launcher/context.rs`)

### Purpose
Baseline project knowledge immediately available to all agents

### What's Loaded
- Top 10 memories with importance ≥7
- Sorted by importance, then recency
- Filtered by project namespace
- Grouped into two tiers:
  - **Critical** (importance ≥8): Detailed format with content preview
  - **Important** (importance ==7): Compact format with summary only

### Format Example

```markdown
# Project Context: project:mnemosyne

## Critical Memories (Importance ≥ 8)

**Rust chosen for performance and safety** — Architecture — 2 days ago
Decided to use Rust for the Mnemosyne memory system due to its performance characteristics and memory safety guarantees. This is a fundamental architectural decision...

**Vector search with sqlite-vec** — Architecture — 1 week ago
Implemented native vector search using sqlite-vec extension for semantic similarity. Dual storage approach: rusqlite for vectors, libsql for memories...

## Important Memories (Importance 7)

- PyO3 bindings for Python integration — Pattern
- RBAC system with 4 agent roles — Architecture

## Knowledge Graph
15 semantic connections across 10 memories

---
*Context from Mnemosyne • 10 memories loaded*
```

### Performance Characteristics
- **Query time**: <100ms (indexed database)
- **Format time**: <50ms (string building)
- **Total time**: <200ms typical, 500ms hard timeout
- **Size**: ~10KB (fits in 20% context budget)

### Configuration

```rust
// In launcher/mod.rs LauncherConfig
pub context_config: ContextLoadConfig {
    max_memories: 10,           // Maximum memories to load
    min_importance: 7,          // Importance threshold
    max_size_bytes: 10 * 1024,  // 10KB hard limit
    include_metadata: true,     // Include graph summary
}
```

### Error Handling

**Graceful Degradation**:
- If database doesn't exist → session launches without context (warning logged)
- If query fails → session launches without context (error logged)
- If timeout (>500ms) → session launches without context (warning logged)
- **Never blocks session launch**

---

## Layer 2: Post-Launch Context (Hook)

### When
After Claude Code launches (triggered by Claude's hook system)

### How
`.claude/hooks/session-start.sh` script

### Owner
User (customizable)

### Purpose
- User-visible context displayed in chat
- User can customize query, filters, formatting
- Provides transparency about loaded memories

### What's Different from Layer 1
- **Layer 1**: Injected into system prompt (invisible to user)
- **Layer 2**: Displayed as chat message (visible to user)
- **Layer 1**: Fixed at launch
- **Layer 2**: Can be customized per project

### Hook Example

```bash
#!/bin/bash
# .claude/hooks/session-start.sh

# Query high-importance memories
mnemosyne recall \
    --query "architecture OR decision OR constraint" \
    --namespace "project:$(basename $(pwd))" \
    --limit 5 \
    --min-importance 7 \
    --format json
```

### Why Keep Both?

**Layer 1 (Pre-Launch)**:
- ✅ Guaranteed to be available
- ✅ Built-in, no configuration needed
- ✅ Always runs before agents activate

**Layer 2 (Hook)**:
- ✅ User can customize
- ✅ Visible in chat for transparency
- ✅ Can query different information

**Recommendation**: Keep both enabled for best results.

---

## Layer 3: In-Session Context (Optimizer)

### When
Throughout session as context needs evolve

### How
Optimizer agent uses MCP tools programmatically

### Owner
Optimizer agent (automated)

### Purpose
Dynamic context adaptation based on task evolution

### Triggers

The Optimizer agent monitors for:
1. **Context utilization >75%** → Preserve critical info, compact non-critical
2. **Task domain shifts** → Load memories relevant to new domain
3. **Phase transitions** → Refresh context for new phase
4. **Agent requests** → Fetch specific knowledge on demand
5. **New decisions made** → Store for future recall

### MCP Tools Available

```
mnemosyne.recall(query, limit, namespace)
  → Search memories by keyword/semantic query

mnemosyne.context(namespace, importance_threshold)
  → Get full project context snapshot

mnemosyne.graph(seed_ids, max_hops)
  → Traverse memory relationships

mnemosyne.list(namespace, limit, sort_by)
  → Browse memories by importance/recency

mnemosyne.remember(content, importance, namespace)
  → Store new memory for future recall

mnemosyne.update(memory_id, updates)
  → Update existing memory
```

### Example Workflow

```
Scenario: Executor starts working on authentication feature

1. Optimizer detects domain shift
   - Previous domain: "database schema"
   - New domain: "authentication"

2. Optimizer queries relevant memories
   Tool: mnemosyne.recall("authentication OR security OR auth", limit=5)

3. Results returned
   - Past auth architecture decisions
   - Security constraints
   - Related code patterns
   - Previous auth bugs and fixes

4. Optimizer integrates into context
   - Provides focused context update to Executor
   - Removes stale database schema context
   - Maintains 20% project context budget

5. Work proceeds with new decision
   - Executor implements JWT-based auth
   - Makes architectural decision

6. Optimizer stores decision
   Tool: mnemosyne.remember(decision, importance=8, namespace="project:myapp")
```

### Context Budget Management

The Optimizer enforces strict context allocation:

| Category | Allocation | Purpose |
|----------|------------|---------|
| Critical | 40% (~20KB) | Active task, work plan, phase state |
| Skills | 30% (~15KB) | Loaded skills for current domain |
| **Project** | **20%** (~10KB) | **Memories from Mnemosyne** |
| General | 10% (~5KB) | Session metadata, git state |

**Total**: ~50KB context budget (typical for Claude Code sessions)

### Dynamic Loading Protocol

The Optimizer follows this protocol:

1. **Monitor**
   - Track context usage percentage
   - Identify active task domains
   - Note recent tool calls and patterns

2. **Analyze**
   - Determine what context is needed but missing
   - Identify stale context that can be removed
   - Calculate available budget

3. **Query**
   - Use MCP tools to fetch relevant memories
   - Constrain results to available budget
   - Prioritize by importance and relevance

4. **Integrate**
   - Add to working memory
   - Inform relevant agents of context update
   - Update context tracking

5. **Compact**
   - Remove stale context to make room
   - Preserve critical information
   - Maintain budget allocation

---

## Configuration

### Launcher Configuration

```rust
// src/launcher/mod.rs
LauncherConfig {
    load_context_on_start: true,  // Enable pre-launch loading
    context_config: ContextLoadConfig {
        max_memories: 10,
        min_importance: 7,
        max_size_bytes: 10 * 1024,
        include_metadata: true,
    },
    // ... other config
}
```

### Runtime Configuration

Environment variables (optional):
```bash
# Override database path
export MNEMOSYNE_DB_PATH=/custom/path/mnemosyne.db

# Override namespace (normally auto-detected)
export MNEMOSYNE_NAMESPACE=project:myapp
```

---

## Performance

### Pre-Launch Context Loading

| Metric | Target | Actual |
|--------|--------|--------|
| Query time | <100ms | ~50ms |
| Format time | <50ms | ~30ms |
| Total time | <200ms | ~80ms |
| Timeout | 500ms | Hard limit |
| Context size | <10KB | ~8KB typical |

### In-Session Context Loading

| Operation | Time | Notes |
|-----------|------|-------|
| `recall` query | <100ms | With proper indexes |
| `context` full | <200ms | Cached after first load |
| `graph` traversal | <150ms | Depends on hop count |
| `remember` store | <50ms | Async, doesn't block |

---

## Troubleshooting

### Context Not Loading

**Symptom**: No memories appear in pre-launch context

**Possible Causes**:
1. Database doesn't exist → Run `mnemosyne init`
2. No memories with importance ≥7 → Lower `min_importance` threshold
3. Wrong namespace → Check auto-detection with `mnemosyne status`
4. Timeout → Check database performance

**Debug**:
```bash
# Run with debug logging
mnemosyne --log-level debug

# Check loaded context size
# Look for: "Loaded startup context (XXX bytes)"
```

### Context Loading Timeout

**Symptom**: Warning "Context loading timed out (>500ms)"

**Possible Causes**:
1. Database not indexed → Rebuild indexes
2. Too many memories → Reduce `max_memories`
3. Slow disk I/O → Move database to faster storage

**Fix**:
```bash
# Rebuild database indexes
sqlite3 ~/.local/share/mnemosyne/mnemosyne.db "REINDEX"

# Reduce memory count in config
# Edit launcher config to set max_memories: 5
```

### In-Session Loading Not Working

**Symptom**: Optimizer not loading memories during session

**Possible Causes**:
1. MCP server not connected → Check `mnemosyne --serve` is running
2. Optimizer not detecting domain shifts → May be working correctly
3. Context budget full → Optimizer waiting for space

**Verify**:
```
# In Claude Code, ask:
"What Mnemosyne tools do you have access to?"

# Should see: mnemosyne.recall, mnemosyne.context, etc.
```

---

## Best Practices

### For Users

1. **Store important decisions immediately**
   ```
   /memory-store "Decided to use PostgreSQL for user data" importance:9
   ```

2. **Use consistent importance scoring**
   - 9-10: Critical architectural decisions
   - 7-8: Important patterns and constraints
   - 5-6: Useful insights
   - 3-4: Reference information
   - 1-2: Low-priority notes

3. **Review loaded context periodically**
   - Session start hook shows what's loaded
   - Verify critical information is preserved

### For Developers

1. **Respect context budget**
   - Never exceed 20% allocation for project memories
   - Remove stale context before adding new

2. **Query efficiently**
   - Use specific queries, not broad ones
   - Limit results to what's needed
   - Cache frequently-used results

3. **Store strategically**
   - Don't store everything
   - Focus on decisions and constraints
   - Use appropriate importance levels

---

## Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────┐
│ Claude Code Session                                         │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ Orchestrator │  │  Optimizer   │  │   Reviewer   │     │
│  └──────────────┘  └──────┬───────┘  └──────────────┘     │
│                           │                                 │
│                    [MCP Tools]                              │
│                           │                                 │
│  ┌──────────────┐  ┌──────▼───────┐                        │
│  │   Executor   │  │ Layer 3:     │                        │
│  │ (Primary)    │  │ In-Session   │                        │
│  └──────────────┘  │ Loading      │                        │
│                    └──────────────┘                         │
│                                                              │
│  [System Prompt with Layer 1 Context]                      │
│  [Chat with Layer 2 Context Display]                       │
└─────────────────────────────────────────────────────────────┘
                           │
                    [MCP Protocol]
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│ Mnemosyne MCP Server                                        │
│                                                              │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │   Storage    │  │  Embeddings  │  │     LLM      │     │
│  │   (libsql)   │  │  (Voyage)    │  │   (Claude)   │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
└─────────────────────────────────────────────────────────────┘
                           ▲
                           │
                    [Pre-Launch]
                           │
┌─────────────────────────────────────────────────────────────┐
│ Launcher (mnemosyne binary)                                 │
│                                                              │
│  ┌──────────────────────────────────────────────────┐      │
│  │ Layer 1: Pre-Launch Context Loading              │      │
│  │                                                    │      │
│  │ 1. Initialize storage (eager)                     │      │
│  │ 2. Query memories (importance ≥7)                 │      │
│  │ 3. Format as markdown                             │      │
│  │ 4. Inject via --append-system-prompt              │      │
│  │ 5. Launch Claude Code                             │      │
│  └──────────────────────────────────────────────────┘      │
└─────────────────────────────────────────────────────────────┘
```

---

## Future Enhancements

- [ ] **LLM-powered summarization**: Ultra-compact context mode
- [ ] **Multi-namespace merging**: Load from project + global
- [ ] **Context freshness scoring**: Recency + importance + relevance
- [ ] **Streaming context**: Progressive loading for large contexts
- [ ] **Agent-specific filtering**: Different context per agent role
- [ ] **Context caching**: TTL-based cache for repeated queries

---

## References

- [Launcher Implementation](src/launcher/context.rs)
- [Optimizer Agent](src/launcher/agents.rs)
- [Session Start Hook](.claude/hooks/session-start.sh)
- [MCP Server](src/mcp/mod.rs)
- [Storage Backend](src/storage/mod.rs)
