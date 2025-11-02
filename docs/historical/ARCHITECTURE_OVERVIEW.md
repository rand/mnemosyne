# Architecture Overview

**A user-friendly guide to how Mnemosyne works**

This document explains Mnemosyne's architecture from a user perspective. For detailed technical internals, see [ARCHITECTURE.md](ARCHITECTURE.md).

## Table of Contents

- [How Mnemosyne Works](#how-mnemosyne-works)
- [Key Concepts](#key-concepts)
- [Data Flow](#data-flow)
- [Security Model](#security-model)
- [Performance Characteristics](#performance-characteristics)
- [Integration Points](#integration-points)

---

## How Mnemosyne Works

### The Big Picture

Mnemosyne is a **persistent memory system** for Claude Code that gives your AI assistant the ability to remember things across sessions, just like a human teammate would.

```
You work on a project → Mnemosyne captures decisions
You close Claude Code → Memory is safely stored
You reopen later    → Context automatically loaded
AI assistant knows → Past decisions, patterns, bugs, insights
```

### Three Main Components

1. **Storage Layer**: Where memories live (LibSQL database)
2. **MCP Server**: How Claude Code talks to Mnemosyne
3. **LLM Enrichment**: How memories become searchable and meaningful

---

## Key Concepts

### 1. Memories

A **memory** is a piece of information worth remembering.

**Example Memory:**
```json
{
  "id": "mem_abc123",
  "content": "Decided to use LibSQL for storage because it has native vector search",
  "summary": "Storage decision: LibSQL for vector support",
  "importance": 8,
  "memory_type": "decision",
  "tags": ["storage", "libsql", "architecture"],
  "namespace": "project:mnemosyne",
  "created_at": "2025-10-27T12:00:00Z"
}
```

**Fields explained:**
- **content**: The actual information (what you want to remember)
- **summary**: AI-generated short version (for quick scanning)
- **importance**: How critical is this? (1-10 scale)
- **memory_type**: Classification (decision, pattern, bug, context)
- **tags**: AI-generated keywords (for search)
- **namespace**: Where does this belong? (global/project/session)

### 2. Namespaces

**Namespaces** organize memories into scopes:

- **global**: Cross-project knowledge (conventions, patterns)
- **project:myapp**: Specific to "myapp" project
- **session:abc123**: Temporary, just for one coding session

**Auto-detection:**
```bash
cd /path/to/my-project  # Git repo named "my-project"
mnemosyne remember --content "Fix bug"
# Automatically stored in namespace: "project:my-project"
```

**Why it matters:**
- When you search, you only see relevant memories for your current project
- Global patterns are available everywhere
- No manual management required

### 3. Importance Scores

**Importance** determines how prominent a memory is in search results.

**Scale:**
- **9-10**: Critical architecture decisions, major patterns
- **7-8**: Important conventions, significant bugs
- **5-6**: Useful context, minor decisions
- **3-4**: Temporary notes, reminders
- **1-2**: Trivial information

**Decay over time:**
```
Day 1:  importance = 8
Day 30: importance = 7.5 (automatic decay)
Day 90: importance = 6.8
```

Frequently accessed memories maintain their importance. Rarely accessed ones fade naturally.

### 4. Memory Types

Mnemosyne classifies memories into four types:

| Type | Description | Example |
|------|-------------|---------|
| **decision** | Architecture or design choices | "Use JWT for auth because..." |
| **pattern** | Reusable solutions | "Always wrap DB calls in transactions" |
| **bug** | Problems and their fixes | "Race condition fix in user profile update" |
| **context** | Background information | "Project uses React 18 with Vite" |

**Auto-classification:**
Claude Haiku analyzes your content and picks the right type automatically.

### 5. Links and Relationships

Memories are connected to each other:

```
Memory A: "Use Redis for caching"
    ↓ [implements]
Memory B: "Cache user sessions in Redis"
    ↓ [related_to]
Memory C: "Redis connection pool configuration"
```

**Link types:**
- **implements**: B implements A's decision
- **supersedes**: B replaces outdated A
- **relates_to**: B is relevant to A
- **caused_by**: B is result of A

**Why it matters:**
When you search for one memory, Mnemosyne can find related memories automatically (graph traversal).

---

## Data Flow

### Storing a Memory

```
1. You (or AI) call: mnemosyne.remember
                       ↓
2. Content stored in database
                       ↓
3. Claude Haiku enriches it:
   - Generates summary
   - Extracts keywords
   - Classifies type (decision/pattern/bug/context)
   - Identifies tags
                       ↓
4. LLM finds related memories
                       ↓
5. Creates semantic links
                       ↓
6. Returns enriched memory with ID
```

**Time:** ~300ms typical (includes LLM call)

### Retrieving a Memory

```
1. You search: mnemosyne.recall --query "caching decision"
                      ↓
2. Hybrid search runs:
   - Keyword search (FTS5 full-text search)
   - Graph traversal (find linked memories)
   - Importance weighting
                      ↓
3. Results ranked by relevance
                      ↓
4. Top N results returned
```

**Time:** ~50ms typical

### Automatic Context Loading

```
Session start → Hook runs (.claude/hooks/session-start.sh)
                      ↓
              Loads top 5 important memories for project
                      ↓
              Displays in Claude Code conversation
                      ↓
              AI assistant has context immediately
```

**No manual work required!**

---

## Security Model

### API Keys

**Three-tier security with priority order:**

1. **Environment Variable** (highest priority)
   ```bash
   export ANTHROPIC_API_KEY=sk-ant-api03-...
   ```
   **Security**: Stored in shell, cleared when terminal closes
   **Use case**: CI/CD, temporary testing

2. **Age-encrypted file** (recommended)
   ```bash
   ~/.config/mnemosyne/secrets.age
   ```
   **Security**: Encrypted with X25519 + ChaCha20-Poly1305
   **Use case**: Daily development

3. **OS Keychain** (fallback)
   ```bash
   macOS Keychain / Windows Credential Manager / Linux Secret Service
   ```
   **Security**: OS-managed secure storage
   **Use case**: Backward compatibility

**Why three tiers?**
- Flexibility: Choose what fits your workflow
- Security: Each tier has appropriate protection
- No vendor lock-in: Not dependent on one system

### Data Encryption

| Data | At Rest | In Transit |
|------|---------|------------|
| API Keys | ✅ Age-encrypted or OS keychain | ✅ Never transmitted |
| Memories | ❌ Plain SQLite database | ✅ Local-only (no network) |
| LLM Requests | N/A | ✅ HTTPS to Anthropic API |

**Why memories aren't encrypted at rest:**
- SQLite full-text search requires plaintext
- Database is local-only (not shared)
- File system permissions protect database
- Future: Remote Turso option with encrypted transit

### Permissions

```bash
# Database permissions
~/.local/share/mnemosyne/
└── mnemosyne.db (0600 - owner read/write only)

# Secrets permissions
~/.config/mnemosyne/
└── secrets.age (0600 - owner read/write only)
```

---

## Performance Characteristics

### What's Fast ⚡

| Operation | Latency | Explanation |
|-----------|---------|-------------|
| Keyword search | ~50ms | FTS5 index lookup |
| List recent | ~10ms | Simple SQL query |
| Get by ID | ~5ms | Primary key lookup |
| Graph traversal | ~30ms | Recursive CTE, depth limited |

### What's Slow 🐌

| Operation | Latency | Explanation |
|-----------|---------|-------------|
| Store with LLM | ~300ms | Claude Haiku API call |
| Consolidate | ~500ms per pair | Multiple LLM comparisons |
| Export large set | ~1s+ | Rendering many memories |

### Scaling Characteristics

**Database size:**
- 1,000 memories: ~800KB (~800 bytes/memory)
- 10,000 memories: ~8MB
- 100,000 memories: ~80MB

**Search performance:**
- FTS5 scales well to 100k+ memories
- Graph traversal limited to 3 hops (configurable)
- Importance filtering speeds up large datasets

**Memory usage:**
- Idle: ~30MB (Rust binary + SQLite)
- Active search: +10-20MB temporary
- LLM enrichment: +50MB peak (async buffering)

**Recommendations:**
- < 10,000 memories: No tuning needed
- 10,000 - 100,000: Consider importance threshold filters
- > 100,000: Periodic archival, namespace filtering
- Very large: Use remote Turso database (v2.0+)

---

## Integration Points

### 1. Claude Code via MCP Protocol

```
Claude Code (MCP Client)
         ↕ JSON-RPC over stdio
Mnemosyne MCP Server (Rust)
         ↕ Direct function calls
LibSQL Storage
```

**8 MCP Tools:**
- **OBSERVE**: recall, list (search and browse)
- **ORIENT**: graph, context (understand relationships)
- **DECIDE**: remember, consolidate (store and organize)
- **ACT**: update, delete (maintain and archive)

**Communication:**
- Protocol: JSON-RPC 2.0
- Transport: stdin/stdout
- Format: Structured JSON
- Latency: <10ms overhead (in-process)

### 2. Claude Code Hooks

**Three hooks, zero friction:**

| Hook | Trigger | Action | Time |
|------|---------|--------|------|
| `session-start.sh` | Claude Code opens | Load project context | ~200ms |
| `pre-compact.sh` | Conversation compacted | Save important context | ~150ms |
| `post-commit.sh` | Git commit | Link commit to decisions | ~300ms |

**Configuration:**
```json
// .claude/settings.json
{
  "hooks": {
    "user_prompt_submit": ".claude/hooks/session-start.sh",
    "before_compact": ".claude/hooks/pre-compact.sh",
    "after_git_commit": ".claude/hooks/post-commit.sh"
  }
}
```

### 3. Skills System (Optimizer Agent)

**Dynamic skill loading:**
```
Task request → Optimizer analyzes keywords
            → Scores 354 global skills + 5 Mnemosyne skills
            → Loads top 3-7 most relevant
            → Provides to Executor agent
```

**Mnemosyne-specific skills:**
- `mnemosyne-memory-management.md`: How to use memory system
- `mnemosyne-context-preservation.md`: Context management patterns
- `mnemosyne-rust-development.md`: Mnemosyne codebase knowledge
- `mnemosyne-mcp-protocol.md`: MCP tool usage
- `skill-mnemosyne-discovery.md`: Skill discovery gateway

**Optimization:**
- Skills cached per session
- +10% relevance bonus for project-local skills
- Max 30% of context budget for skills
- Automatic unload when not needed

### 4. Multi-Agent Orchestration (PyO3)

**Four agents collaborate:**

```
Orchestrator → Coordinates workflow, prevents deadlocks
     ↓
Optimizer → Builds optimal context, loads skills
     ↓
Reviewer → Validates quality, checks constraints
     ↓
Executor → Executes tasks, spawns sub-agents
```

**PyO3 Performance:**
- 10-20x faster than subprocess calls
- 2.25ms memory store (vs ~30ms subprocess)
- 0.88ms list operation (vs ~15ms subprocess)
- Zero-copy data passing between Rust ↔ Python

**Why PyO3?**
- Rust: High-performance core (storage, search, LLM)
- Python: Rich AI ecosystem (Claude Agent SDK, orchestration)
- PyO3: Bridge with minimal overhead

---

## Common Questions

### How does Mnemosyne know what to remember?

**Two ways:**

1. **Manual**: You or the AI explicitly store
   ```
   /memory-store Important decision about caching strategy
   ```

2. **Automatic hooks**: Configured triggers
   - Session start: Load context
   - Pre-compact: Save decisions
   - Post-commit: Link to code changes

### Where is my data stored?

```bash
# Database (memories)
~/.local/share/mnemosyne/mnemosyne.db

# Secrets (API keys)
~/.config/mnemosyne/secrets.age

# Configuration (MCP)
~/.claude/mcp_config.json
```

All local. Nothing uploaded to cloud (except LLM API calls for enrichment).

### Can I use Mnemosyne without Claude Code?

**Yes!** Mnemosyne works standalone:

```bash
# Command-line usage
mnemosyne remember --content "Note" --format json
mnemosyne recall --query "search" --format json
mnemosyne list --format json

# Programmatic usage (Python)
import mnemosyne_core
storage = mnemosyne_core.LibsqlStorage("mnemosyne.db")
```

But Claude Code integration provides the best experience (automatic context loading, slash commands, agent access).

### How does search work?

**Hybrid approach:**

1. **FTS5 Keyword Search**: Find memories matching words
   - Supports: phrase matching, prefix search, boolean logic
   - Fast: 50ms typical

2. **Graph Traversal**: Find related memories via links
   - Depth: 1-3 hops (configurable)
   - Finds: Connections you might not have searched for

3. **Combined Ranking**:
   ```
   score = (keyword_match * 0.6) + (graph_relevance * 0.2) + (importance * 0.2)
   ```

4. **Filters**:
   - Namespace: Only project-relevant
   - Importance: Threshold filtering
   - Date: Recency weighting

### What happens if I delete a memory?

**Soft delete (archival):**
- Memory marked as archived (not deleted)
- No longer appears in searches
- Can be recovered if needed
- Immutable audit trail preserved

**Future hard delete:**
- Planned for v2.0
- Permanently removes memory
- Updates linked memories

### How much does LLM enrichment cost?

**Typical usage:**
- Store memory: ~100 tokens ($0.00015 with Claude Haiku)
- Consolidate: ~200 tokens per pair ($0.0003)
- Per-month estimate: $0.10-$1.00 for typical developer

**Cost control:**
- Haiku is cheapest Claude model (~$0.25/million input tokens)
- Enrichment is optional (can disable for cost-sensitive use)
- Caching reduces repeat costs
- Most operations don't use LLM (search, list, retrieve)

---

## Design Principles

1. **Zero-Copy**: Minimize data duplication for speed
2. **Type Safety**: Rust's type system prevents bugs
3. **Async-First**: Non-blocking I/O, scales well
4. **Immutable Audit**: Never delete, only supersede
5. **Fail-Fast**: Explicit errors, not silent failures
6. **Incremental Complexity**: Simple core, rich features optional

---

## Further Reading

- **[ARCHITECTURE.md](ARCHITECTURE.md)** - Technical deep dive (for developers)
- **[MCP_SERVER.md](MCP_SERVER.md)** - Complete API reference
- **[ORCHESTRATION.md](ORCHESTRATION.md)** - Multi-agent system details
- **[ROADMAP.md](ROADMAP.md)** - Future plans (v2.0 features)

---

**Last Updated**: 2025-10-27
**Version**: 1.0.0
