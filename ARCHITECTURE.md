# Mnemosyne Architecture

This document describes the system architecture, design decisions, and implementation details of Mnemosyne.

## Table of Contents

1. [System Overview](#system-overview)
2. [Architecture Layers](#architecture-layers)
3. [Core Components](#core-components)
4. [Data Flow](#data-flow)
5. [Storage Architecture](#storage-architecture)
6. [Memory Intelligence](#memory-intelligence)
7. [Namespace System](#namespace-system)
8. [MCP Integration](#mcp-integration)
9. [Design Decisions](#design-decisions)
10. [Security](#security)
11. [Performance](#performance)

---

## System Overview

Mnemosyne is a high-performance, project-aware agentic memory system built in Rust that provides persistent semantic memory for Claude Code's multi-agent orchestration system.

### Key Design Goals

1. **Project Awareness**: Automatic context detection from git repositories and CLAUDE.md
2. **Performance**: Sub-200ms retrieval latency for p95
3. **Intelligence**: LLM-guided note construction and semantic linking
4. **Safety**: Type-safe Rust with comprehensive error handling
5. **Integration**: Seamless Claude Code integration via MCP protocol

### System Diagram

```
┌─────────────────────────────────────────────────────────────┐
│                    Claude Code Environment                   │
│  ┌───────────────────────────────────────────────────────┐  │
│  │         Multi-Agent Orchestration System              │  │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  │  │
│  │  │Orchestrator │  │ Optimizer   │  │  Reviewer   │  │  │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  │  │
│  │  ┌─────────────┐                                     │  │
│  │  │  Executor   │  (with mnemosyne skills)            │  │
│  │  └─────────────┘                                     │  │
│  └─────────────────────────┬─────────────────────────────┘  │
└──────────────────────────┼───────────────────────────────────┘
                           │
                           │ JSON-RPC 2.0 over stdio (MCP)
                           │
              ┌────────────▼────────────┐
              │   Mnemosyne Server      │
              │    (Rust + Tokio)       │
              └────────────┬────────────┘
                           │
         ┌─────────────────┼─────────────────┐
         │                 │                 │
    ┌────▼─────┐     ┌────▼─────┐     ┌────▼─────┐
    │ Storage  │     │   LLM    │     │Namespace │
    │ (SQLite) │     │ (Claude) │     │ Detector │
    │  +FTS5   │     │  Haiku   │     │          │
    └──────────┘     └──────────┘     └──────────┘
```

---

## Architecture Layers

### 1. Presentation Layer (MCP Server)

**Location**: `src/mcp/`

**Responsibilities**:
- JSON-RPC 2.0 protocol handling
- Request routing and validation
- Response serialization
- Error handling and reporting
- Stdio-based communication

**Key Files**:
- `protocol.rs`: JSON-RPC types and structures
- `server.rs`: Async server implementation
- `tools.rs`: 8 OODA-aligned tool implementations

### 2. Service Layer

**Location**: `src/services/`

**Responsibilities**:
- Business logic and orchestration
- LLM integration for memory intelligence
- Namespace detection and management
- Memory consolidation decisions

**Key Files**:
- `llm.rs`: Claude Haiku integration
- `namespace.rs`: Project context detection

### 3. Storage Layer

**Location**: `src/storage/`

**Responsibilities**:
- SQLite database operations
- FTS5 keyword search
- Graph traversal (recursive CTE)
- Transaction management
- Migration handling

**Key Files**:
- `sqlite.rs`: Storage implementation
- `migrations/`: SQL schema migrations

### 4. Core Layer

**Location**: `src/`

**Responsibilities**:
- Type definitions and domain models
- Error types and handling
- Configuration management
- Common utilities

**Key Files**:
- `types.rs`: Core data structures
- `error.rs`: Error types and conversions
- `config.rs`: Secure credential management
- `lib.rs`: Public API exports

---

## Core Components

### Type System (`src/types.rs`)

#### MemoryId
```rust
pub struct MemoryId(Uuid);
```
- Globally unique identifier
- Used for memory identity and linking
- Immutable once created

#### Namespace
```rust
pub enum Namespace {
    Global,
    Project(String),
    Session { project: String, session_id: String },
}
```
- Three-tier hierarchy: Global → Project → Session
- Automatic isolation between contexts
- Priority-based retrieval

#### MemoryType
```rust
pub enum MemoryType {
    ArchitectureDecision,
    CodePattern,
    BugFix,
    Configuration,
    Constraint,
    Entity,
    Insight,
    Reference,
    Preference,
}
```
- 9 classifications for memories
- Used for filtering and organization
- LLM automatically assigns during enrichment

#### LinkType
```rust
pub enum LinkType {
    Extends,
    Contradicts,
    Implements,
    References,
    Supersedes,
}
```
- 5 semantic relationship types
- Directed links with strength (0.0-1.0)
- Automatic generation via LLM

#### MemoryNote
```rust
pub struct MemoryNote {
    pub id: MemoryId,
    pub namespace: Namespace,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // Content
    pub content: String,
    pub summary: String,
    pub keywords: Vec<String>,
    pub tags: Vec<String>,

    // Metadata
    pub context: String,
    pub memory_type: MemoryType,
    pub importance: u8,          // 1-10
    pub confidence: f32,         // 0.0-1.0

    // Relationships
    pub links: Vec<MemoryLink>,
    pub related_files: Vec<String>,
    pub related_entities: Vec<String>,

    // Access tracking
    pub access_count: u64,
    pub last_accessed_at: DateTime<Utc>,

    // Lifecycle
    pub expires_at: Option<DateTime<Utc>>,
    pub is_archived: bool,
    pub superseded_by: Option<MemoryId>,

    // Embeddings
    pub embedding: Option<Vec<f32>>,
    pub embedding_model: String,
}
```

### Error Handling (`src/error.rs`)

Comprehensive error types with conversions:

```rust
pub enum MnemosyneError {
    Storage(String),
    Serialization(String),
    LlmApi(String),
    Namespace(String),
    Config(config::ConfigError),
    NotFound(MemoryId),
    Database(sqlx::Error),
    Io(std::io::Error),
    Http(reqwest::Error),
}
```

All errors implement `std::error::Error` and can be converted via `From` traits.

---

## Data Flow

### 1. Memory Creation Flow

```
User/Agent → mnemosyne.remember
    ↓
Tool Handler validates input
    ↓
Namespace Detector determines context
    ↓
LLM Service enriches content
    ├── Generate summary
    ├── Extract keywords
    ├── Assign tags
    ├── Classify type
    └── Score importance
    ↓
LLM Service generates links
    ├── Find candidate memories
    ├── Detect relationships
    ├── Assign link types
    └── Score strengths
    ↓
Storage Layer persists
    ├── Insert memory row
    ├── Update FTS5 index
    └── Create link rows
    ↓
Response with MemoryId
```

### 2. Memory Retrieval Flow

```
User/Agent → mnemosyne.recall
    ↓
Tool Handler validates query
    ↓
Storage Layer searches
    ├── Keyword search (FTS5)
    ├── Filter by namespace
    ├── Filter by type/tags
    └── Sort by relevance/importance
    ↓
Storage Layer expands graph
    ├── Follow links (optional)
    ├── Recursive CTE traversal
    └── Respect max_depth
    ↓
Response with ranked results
```

### 3. Context Building Flow

```
Session Start
    ↓
Namespace Detector
    ├── Find git root
    ├── Parse CLAUDE.md
    └── Generate session ID
    ↓
mnemosyne.context
    ├── Get recent memories
    ├── Get important memories
    ├── Build memory graph
    └── Aggregate metadata
    ↓
Context payload for agents
```

---

## Storage Architecture

### SQLite Schema

#### memories table
```sql
CREATE TABLE memories (
    memory_id TEXT PRIMARY KEY,
    namespace TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,

    content TEXT NOT NULL,
    summary TEXT NOT NULL,
    keywords TEXT NOT NULL,  -- JSON array
    tags TEXT NOT NULL,      -- JSON array

    context TEXT NOT NULL,
    memory_type TEXT NOT NULL,
    importance INTEGER NOT NULL,
    confidence REAL NOT NULL,

    related_files TEXT NOT NULL,     -- JSON array
    related_entities TEXT NOT NULL,  -- JSON array

    access_count INTEGER NOT NULL DEFAULT 0,
    last_accessed_at INTEGER NOT NULL,

    expires_at INTEGER,
    is_archived BOOLEAN NOT NULL DEFAULT 0,
    superseded_by TEXT,

    embedding BLOB,
    embedding_model TEXT
);
```

#### memory_links table
```sql
CREATE TABLE memory_links (
    source_id TEXT NOT NULL,
    target_id TEXT NOT NULL,
    link_type TEXT NOT NULL,
    strength REAL NOT NULL,
    reason TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    PRIMARY KEY (source_id, target_id),
    FOREIGN KEY (source_id) REFERENCES memories(memory_id),
    FOREIGN KEY (target_id) REFERENCES memories(memory_id)
);
```

#### memories_fts (FTS5 virtual table)
```sql
CREATE VIRTUAL TABLE memories_fts USING fts5(
    content,
    summary,
    keywords,
    tags,
    content='memories',
    content_rowid='rowid'
);
```

### FTS5 Synchronization

Triggers maintain FTS5 index consistency:

```sql
-- Insert trigger
CREATE TRIGGER memories_ai AFTER INSERT ON memories BEGIN
    INSERT INTO memories_fts(rowid, content, summary, keywords, tags)
    VALUES (NEW.rowid, NEW.content, NEW.summary, NEW.keywords, NEW.tags);
END;

-- Update trigger (DELETE + INSERT for FTS5 compatibility)
CREATE TRIGGER memories_au AFTER UPDATE ON memories BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, content, summary, keywords, tags)
    VALUES ('delete', OLD.rowid, OLD.content, OLD.summary, OLD.keywords, OLD.tags);
    INSERT INTO memories_fts(rowid, content, summary, keywords, tags)
    VALUES (NEW.rowid, NEW.content, NEW.summary, NEW.keywords, NEW.tags);
END;

-- Delete trigger
CREATE TRIGGER memories_ad AFTER DELETE ON memories BEGIN
    INSERT INTO memories_fts(memories_fts, rowid, content, summary, keywords, tags)
    VALUES ('delete', OLD.rowid, OLD.content, OLD.summary, OLD.keywords, OLD.tags);
END;
```

### Graph Traversal

Recursive CTE for efficient graph walking:

```sql
WITH RECURSIVE memory_graph(id, depth) AS (
    -- Base case: starting memory
    SELECT memory_id, 0 FROM memories WHERE memory_id = ?

    UNION ALL

    -- Recursive case: follow links
    SELECT ml.target_id, mg.depth + 1
    FROM memory_graph mg
    JOIN memory_links ml ON ml.source_id = mg.id
    WHERE mg.depth < ?
)
SELECT DISTINCT m.* FROM memories m
JOIN memory_graph mg ON m.memory_id = mg.id
ORDER BY mg.depth;
```

---

## Memory Intelligence

### LLM Integration (`src/services/llm.rs`)

#### Enrichment Pipeline

**Input**: Raw content + context string

**Process**:
1. Construct structured prompt
2. Call Claude Haiku API
3. Parse structured response
4. Extract fields and validate

**Output**: Enriched MemoryNote with:
- Concise summary (1-2 sentences)
- 3-5 keywords for indexing
- 2-3 tags for categorization
- Memory type classification
- Importance score (1-10)

**Prompt Template**:
```
You are helping construct a structured memory note.

Given this raw observation:
{raw_content}

Context: {context}

Provide:
1. A concise summary (1-2 sentences)
2. 3-5 keywords for indexing
3. 2-3 tags for categorization
4. Memory type (ArchitectureDecision|CodePattern|BugFix|...)
5. Importance score (1-10)

Format EXACTLY as:
SUMMARY: <summary>
KEYWORDS: <keyword1>, <keyword2>, ...
TAGS: <tag1>, <tag2>, ...
TYPE: <memory_type>
IMPORTANCE: <score>
```

#### Link Generation

**Input**: New memory + candidate memories (similar by tags/keywords)

**Process**:
1. Format candidate descriptions
2. Construct relationship analysis prompt
3. Call Claude Haiku API
4. Parse link specifications
5. Create MemoryLink structures

**Output**: Vec<MemoryLink> with:
- Target memory ID
- Relationship type
- Strength (0.0-1.0)
- Reason for relationship

**Prompt Template**:
```
You are analyzing semantic relationships between memories.

New memory:
Summary: {summary}
Content: {content}
Type: {type}
Tags: {tags}

Candidate memories:
[0] {summary} (Type: {type}, Tags: {tags})
[1] ...

For each meaningful relationship, specify:
1. Candidate index
2. Relationship type (Extends|Contradicts|Implements|References|Supersedes)
3. Link strength (0.0 - 1.0)
4. Brief reason

Format EXACTLY as (one per line):
LINK: <index>, <type>, <strength>, <reason>

If no relationships exist:
NO_LINKS
```

#### Consolidation Decisions

**Input**: Two candidate memories for consolidation

**Process**:
1. Format both memories
2. Construct decision prompt
3. Call Claude Haiku API
4. Parse decision type
5. Create ConsolidationDecision

**Output**: One of:
- `Merge { into, content }`: Combine memories
- `Supersede { kept, superseded }`: One replaces other
- `KeepBoth`: Maintain separately

### API Configuration

**Model**: `claude-3-5-haiku-20241022`
**Max Tokens**: 1024
**Temperature**: 0.7
**Endpoint**: `https://api.anthropic.com/v1/messages`

**Authentication**:
- Header: `x-api-key`
- API version: `2023-06-01`

---

## Namespace System

### Automatic Detection (`src/namespace.rs`)

#### Git Root Detection

Algorithm:
```
1. Start from current directory
2. Walk up directory tree
3. Check for .git/ directory
4. If found: return as project root
5. If reach filesystem root: no git repo
```

Implementation uses recursive directory traversal with depth limits.

#### CLAUDE.md Parsing

Two-phase parser:
1. **YAML Frontmatter** (optional)
   ```yaml
   ---
   project: myproject
   description: My awesome project
   ---
   ```

2. **Markdown Content** (fallback)
   - First H1 heading as project name
   - First paragraph as description

Parser is lenient and works with partial content.

#### Session ID Generation

Format: `session_{timestamp}_{random}`

Example: `session_20241026_a8f3b2`

Ensures uniqueness across parallel sessions.

### Namespace Hierarchy

```
Global
├── Project: myproject
│   ├── Session: session_20241026_a8f3b2
│   └── Session: session_20241026_x9k1m4
└── Project: otherproject
    └── Session: session_20241026_p5r7t3
```

### Priority-Based Retrieval

When searching memories:

1. **Session scope**: Query only current session memories
2. **Project scope**: Query project + global memories
3. **Global scope**: Query all memories

Default: Start narrow (session), expand as needed.

---

## MCP Integration

### JSON-RPC 2.0 Protocol

#### Message Format

**Request**:
```json
{
  "jsonrpc": "2.0",
  "method": "method_name",
  "params": { ... },
  "id": 1
}
```

**Success Response**:
```json
{
  "jsonrpc": "2.0",
  "result": { ... },
  "id": 1
}
```

**Error Response**:
```json
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32600,
    "message": "Invalid request"
  },
  "id": 1
}
```

### Communication Model

- **Transport**: stdio (stdin/stdout)
- **Format**: Newline-delimited JSON
- **Concurrency**: Async/await (Tokio)
- **Logging**: Stderr only (no stdout pollution)

### 8 OODA-Aligned Tools

#### OBSERVE Phase

1. **mnemosyne.recall** - Search memories by query
   - Keyword search (FTS5)
   - Namespace filtering
   - Tag/type filtering
   - Relevance ranking

2. **mnemosyne.list** - List recent/important memories
   - Time-based filtering
   - Importance thresholds
   - Namespace scoping

#### ORIENT Phase

3. **mnemosyne.graph** - Get memory graph
   - Starting point memory
   - Configurable depth
   - Link traversal
   - Related memories

4. **mnemosyne.context** - Get full project context
   - Recent memories
   - Important memories
   - Memory graph overview
   - Project metadata

#### DECIDE Phase

5. **mnemosyne.remember** - Store new memory
   - LLM enrichment
   - Auto-classification
   - Link generation
   - Namespace assignment

6. **mnemosyne.consolidate** - Merge/supersede memories
   - LLM-guided decisions
   - Content merging
   - Link preservation
   - Audit trail

#### ACT Phase

7. **mnemosyne.update** - Update existing memory
   - Content modification
   - Importance adjustment
   - Link management
   - Version tracking

8. **mnemosyne.delete** - Archive memory
   - Soft delete (is_archived flag)
   - Link preservation
   - Supersession tracking
   - Audit trail

---

## Design Decisions

### Why Rust?

1. **Performance**: Zero-cost abstractions, no GC pauses
2. **Safety**: Type system prevents memory errors
3. **Concurrency**: Async/await with Tokio for high throughput
4. **Reliability**: Comprehensive error handling via `Result`
5. **Ecosystem**: Excellent crates (sqlx, tokio, serde, etc.)

### Why SQLite?

1. **Simplicity**: Single-file database, no server required
2. **Performance**: Excellent for read-heavy workloads
3. **FTS5**: Built-in full-text search with stemming
4. **Portability**: Works everywhere, easy backups
5. **Reliability**: Battle-tested, ACID compliant

### Why Claude Haiku?

1. **Cost**: 4-5x cheaper than Sonnet for simple tasks
2. **Speed**: <500ms latency for enrichment
3. **Quality**: Sufficient for note construction and linking
4. **Consistency**: Structured output with reliable parsing

### Why OS Keychain?

1. **Security**: OS-native credential storage
2. **Integration**: Works with system security policies
3. **Encryption**: Automatic at rest and in transit
4. **Auditability**: System logs and access controls
5. **Platform Support**: macOS, Windows, Linux via libsecret

### Why Deferred Vector Search?

**Original Plan**: Local embeddings via fastembed + onnxruntime

**Issues Encountered**:
- `onnxruntime` compilation failures on macOS
- Large binary size (100+ MB with embeddings)
- Complexity vs benefit for v1.0

**Decision**: Defer to v2.0, use FTS5 keyword search

**Benefits**:
- Faster implementation timeline
- Smaller binary size
- Sufficient accuracy for initial use
- Can add later without breaking changes

---

## Security

### API Key Management

**Storage**:
- macOS: Keychain (encrypted by system)
- Windows: Credential Manager (encrypted by DPAPI)
- Linux: Secret Service (libsecret + keyring daemon)

**Retrieval Priority**:
1. `ANTHROPIC_API_KEY` environment variable (CI/CD)
2. OS keychain (interactive use)
3. Interactive prompt (first-time setup)

**Security Properties**:
- Never written to disk in plaintext
- Not logged or displayed (masked)
- Protected by OS-level encryption
- Scoped to user account

### Database Security

**File Permissions**:
- Created with 0600 (user read/write only)
- Respects umask for group/other

**Content**:
- No automatic encryption (add via SQLite extensions if needed)
- Sensitive content should be avoided or explicitly encrypted
- Audit trail prevents accidental data loss

**Backup**:
- Single-file design enables easy secure backup
- Recommended: Encrypt backups at rest
- Consider `.gitignore` for `mnemosyne.db`

### Network Security

**HTTPS Only**:
- All Anthropic API calls over HTTPS
- Certificate verification enabled
- No certificate pinning (trust system CA store)

**No Inbound Connections**:
- Server reads stdin only
- No network listeners
- No remote access

---

## Performance

### Targets

| Metric | Target | Current |
|--------|--------|---------|
| Retrieval latency (p95) | <200ms | ~50ms (keyword) |
| Storage latency (p95) | <500ms | ~300ms (with LLM) |
| Memory usage | <100MB | ~30MB (idle) |
| Database size | ~1MB per 1000 memories | ~800KB/1000 |
| Concurrent requests | 100+ | Untested |

### Optimizations

**Zero-Copy Reads**:
- SQLite row data mapped directly to Rust structs
- No intermediate allocations for large result sets
- Streaming responses for large queries

**Async I/O**:
- Tokio runtime for concurrency
- Non-blocking database operations via sqlx
- Parallel LLM calls when appropriate

**Indexing Strategy**:
- FTS5 for keyword search (indexed on insert)
- B-tree indexes on namespace, created_at, importance
- Covering indexes for common queries

**Connection Pooling**:
- SQLite connection pool (size: 5)
- Reuse connections across requests
- Automatic health checks

**Caching** (future):
- In-memory cache for hot memories
- LRU eviction policy
- Cache invalidation on updates

### Benchmarks

TODO: Add benchmark results from Phase 9

---

## Future Enhancements

### Phase 2 Completion: Hybrid Search

**Vector Similarity**:
- Add embeddings via fastembed (when stable)
- Store in `embedding` BLOB column
- Use `sqlite-vec` extension for similarity search

**Hybrid Ranking**:
```rust
score = 0.4 * vector_similarity
      + 0.3 * keyword_match
      + 0.2 * graph_proximity
      + 0.1 * importance
```

### Phase 6: Agent Orchestration

**Agent-Specific Views**:
- Filter memories by agent role
- Customize importance scoring
- Role-based access control

**Background Evolution**:
- Periodic consolidation jobs
- Link strength decay over time
- Importance recalculation
- Dead memory archival

### Phase 8: CLAUDE.md Integration

**Decision Trees**:
- Document memory workflow in CLAUDE.md
- Add memory considerations to planning phases
- Integrate with multi-agent protocols

**Hooks**:
- `session-start`: Load context automatically
- `pre-compact`: Checkpoint critical memories
- `post-commit`: Store decisions made

---

## Appendix: File Structure

```
mnemosyne/
├── Cargo.toml
├── Cargo.lock
├── README.md
├── INSTALL.md
├── MCP_SERVER.md
├── ARCHITECTURE.md          # This file
├── CONTRIBUTING.md
├── LICENSE
├── install.sh
├── uninstall.sh
├── .claude/
│   └── mcp_config.json
├── src/
│   ├── lib.rs               # Public API
│   ├── main.rs              # CLI entry point
│   ├── types.rs             # Core types
│   ├── error.rs             # Error types
│   ├── config.rs            # Credential management
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── sqlite.rs        # Storage implementation
│   │   └── migrations/      # SQL migrations
│   ├── services/
│   │   ├── mod.rs
│   │   ├── llm.rs           # LLM integration
│   │   └── namespace.rs     # Context detection
│   └── mcp/
│       ├── mod.rs
│       ├── protocol.rs      # JSON-RPC types
│       ├── server.rs        # MCP server
│       └── tools.rs         # Tool implementations
├── tests/
│   └── integration/
└── benches/
    └── memory_ops.rs
```

---

**Version**: 0.1.0
**Last Updated**: 2025-10-26
**Status**: 5 of 10 phases complete
