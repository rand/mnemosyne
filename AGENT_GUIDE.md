# Mnemosyne Agent Guide

**Last Updated**: 2025-11-02
**Version**: 2.1.0+
**For**: Agents working in the Mnemosyne codebase

---

## Quick Links

- [Project Overview](#project-overview)
- [Quick Start](#quick-start-for-agents)
- [Architecture](#architecture)
- [Core Types](#core-types--schemas)
- [Development Workflows](#development-workflows)
- [Testing Strategy](#testing-strategy)
- [Common Tasks](#common-tasks)
- [Resources](#resources)

---

## Project Overview

### Purpose
Mnemosyne is a **high-performance agentic memory system** for Claude Code's multi-agent orchestration. It provides persistent semantic memory with sub-millisecond retrieval, built in Rust with LibSQL vector search and PyO3 Python bindings.

### Key Capabilities
- **Project-Aware Memory**: Automatic namespace detection from git repositories
- **Semantic Search**: LibSQL vector embeddings + FTS5 + graph connectivity
- **Multi-Agent Orchestration**: 4 specialized agents (Orchestrator, Optimizer, Reviewer, Executor)
- **Evolution System**: Memory consolidation, importance scoring, link decay, archival
- **Interactive Context Studio (ICS)**: CRDT-based collaborative editor with semantic highlighting
- **MCP Server**: 8 OODA-aligned tools via JSON-RPC over stdio

### Current Status
- **Version**: 2.1.0+ (unreleased changes on main)
- **Test Status**: 610 tests passing, 10 known failures (storage backend tests)
- **Build Time**: ~2m 46s clean build, ~3-4s incremental
- **Language**: Rust 1.75+, Python 3.10-3.13 (via PyO3)

---

## Quick Start for Agents

### Build Commands
```bash
# Check compilation
cargo check

# Build release binary
cargo build --release

# Run tests
cargo test --lib                    # Unit + integration tests
cargo test --test '*'               # All tests including e2e

# Run specific binary
cargo run --bin mnemosyne -- --help
cargo run --bin mnemosyne-ics
cargo run --bin mnemosyne-dash
```

### Common Development Workflows
```bash
# Start MCP server (for Claude Code integration)
cargo run --bin mnemosyne -- serve

# Start with API server for monitoring
cargo run --bin mnemosyne -- serve --with-api

# Initialize database
cargo run --bin mnemosyne -- init

# Store a memory
cargo run --bin mnemosyne -- remember \
  --content "Decision to use LibSQL" \
  --namespace "project:mnemosyne" \
  --importance 9 \
  --type architecture

# Search memories
cargo run --bin mnemosyne -- recall \
  --query "database decisions" \
  --namespace "project:mnemosyne"
```

### Where to Find Things
- **Source code**: `src/`
- **Tests**: `tests/` (unit tests in `src/` modules)
- **Documentation**: `docs/` (features, guides, specs)
- **Scripts**: `scripts/install/`, `scripts/testing/`
- **Migrations**: `migrations/libsql/`, `migrations/sqlite/`
- **Configuration**: `.cargo/config.toml`, `Cargo.toml`

---

## Architecture

### High-Level Design

```
┌─────────────────────────────────────────────────────────┐
│                     Claude Code                         │
│                   (User Interface)                       │
└────────────────────┬────────────────────────────────────┘
                     │ JSON-RPC over stdio
                     ▼
┌─────────────────────────────────────────────────────────┐
│                   MCP Server                            │
│              (8 OODA-aligned tools)                     │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│              Tool Handler Layer                         │
│     (remember, recall, list, graph, consolidate...)     │
└──────┬──────────────────┬───────────────────┬───────────┘
       │                  │                   │
       ▼                  ▼                   ▼
┌──────────────┐   ┌──────────────┐   ┌─────────────────┐
│   Storage    │   │   Services   │   │ Orchestration   │
│   Backend    │   │   (LLM,      │   │   Engine        │
│  (LibSQL)    │   │  Embeddings) │   │ (4 agents)      │
└──────────────┘   └──────────────┘   └─────────────────┘
```

### Component Diagram

For detailed architecture, see [ARCHITECTURE.md](ARCHITECTURE.md).

**Key Subsystems**:
- **Storage** (`src/storage/`): LibSQL backend with vector search
- **Services** (`src/services/`): LLM and embedding services
- **MCP** (`src/mcp/`): JSON-RPC server and tool handlers
- **Orchestration** (`src/orchestration/`): Multi-agent coordination
- **ICS** (`src/ics/`): Interactive Context Studio editor
- **Evaluation** (`src/evaluation/`): Privacy-preserving relevance scoring
- **Evolution** (`src/evolution/`): Memory consolidation and decay

### Data Flow Overview

**Memory Storage Flow**:
```
User Input → MCP Tool (remember)
          → ToolHandler
          → LlmService (enrichment: summary, tags, keywords)
          → EmbeddingService (vector generation)
          → StorageBackend (LibSQL insert + FTS5 index)
```

**Memory Recall Flow**:
```
User Query → MCP Tool (recall)
          → ToolHandler
          → EmbeddingService (query vector)
          → StorageBackend (hybrid search: FTS5 + vector + graph)
          → Relevance Scoring (evaluation module)
          → Ranked Results → User
```

---

## Module Organization

### Source Code Structure

```
src/
├── main.rs                 # Main binary: MCP server CLI
├── lib.rs                  # Library exports
├── types.rs                # Core data structures
├── config.rs               # Configuration management
├── error.rs                # Error types
├── namespace.rs            # Namespace detection
│
├── storage/                # Storage backends
│   ├── mod.rs              # StorageBackend trait
│   ├── libsql.rs           # LibSQL implementation (primary)
│   └── libsql_workitem_tests.rs  # WorkItem tests
│
├── services/               # External service integrations
│   ├── llm.rs              # LLM service (Claude API)
│   └── embeddings.rs       # Embedding generation
│
├── mcp/                    # Model Context Protocol
│   ├── mod.rs              # MCP exports
│   ├── server.rs           # JSON-RPC server
│   ├── tools.rs            # Tool definitions
│   └── protocol.rs         # JSON-RPC protocol types
│
├── orchestration/          # Multi-agent system
│   ├── mod.rs              # OrchestrationEngine
│   ├── actors/             # Ractor-based agents
│   ├── network/            # Iroh P2P networking
│   ├── state.rs            # WorkItem, WorkQueue, Phase
│   ├── events.rs           # Event persistence
│   └── supervision.rs      # Supervision tree
│
├── ics/                    # Interactive Context Studio
│   ├── mod.rs              # ICS application
│   ├── editor/             # Text editor with CRDT
│   └── semantic_highlighter/ # 3-tier semantic highlighting
│
├── evaluation/             # Privacy-preserving evaluation
│   ├── mod.rs              # Feature extraction
│   ├── feedback.rs         # Implicit feedback collection
│   └── scorer.rs           # Relevance scoring
│
├── evolution/              # Memory evolution
│   ├── mod.rs              # Evolution system
│   ├── consolidation.rs    # Merge/supersede logic
│   ├── importance.rs       # Graph-based recalibration
│   └── decay.rs            # Link strength decay
│
├── agents/                 # Agent-specific views
│   └── access_control.rs   # Memory access control
│
├── api/                    # HTTP API server
│   └── mod.rs              # SSE event streaming
│
├── pty/                    # PTY wrapper
│   └── session.rs          # Claude Code integration
│
├── tui/                    # Shared TUI infrastructure
│   └── widgets/            # Ratatui widgets
│
├── daemon/                 # Background processes
├── launcher/               # Orchestrated session launcher
├── secrets/                # Age-encrypted secrets
├── embeddings/             # Embedding utilities
└── bin/                    # Additional binaries
    ├── ics.rs              # Standalone ICS binary
    └── dash.rs             # Monitoring dashboard
```

### Public API Surface

From `src/lib.rs`, the following types and traits are exported:

**Core Types**:
- `MemoryNote`, `MemoryId`, `MemoryLink`, `MemoryType`
- `Namespace` (Global, Project, Session)
- `SearchQuery`, `SearchResult`
- `LinkType` (Extends, Contradicts, Implements, References, Supersedes)

**Storage**:
- `StorageBackend` trait
- `LibsqlStorage`, `ConnectionMode` (Local, InMemory, Remote)

**Services**:
- `LlmService`, `LlmConfig`
- `EmbeddingService` (Local/Remote)

**Orchestration**:
- `OrchestrationEngine`, `SupervisionConfig`
- `WorkItem`, `WorkQueue`, `Phase`, `AgentEvent`

**MCP**:
- `McpServer`, `ToolHandler`

**Evaluation**:
- `FeatureExtractor`, `FeedbackCollector`, `RelevanceScorer`

**Evolution**:
- `EvolutionConfig`, `ConsolidationJob`, `ImportanceRecalibrator`, `LinkDecayJob`

---

## Core Types & Schemas

### Memory Types

#### `MemoryNote`
Primary data structure for stored memories.

```rust
pub struct MemoryNote {
    pub id: MemoryId,                  // UUID
    pub content: String,                // Main content
    pub summary: Option<String>,        // LLM-generated summary
    pub keywords: Vec<String>,          // LLM-extracted keywords
    pub tags: Vec<String>,              // User/LLM-assigned tags
    pub importance: i32,                // 1-10 importance score
    pub memory_type: MemoryType,        // Classification
    pub namespace: Namespace,           // Isolation scope
    pub context: Option<String>,        // Additional context
    pub created_at: DateTime<Utc>,     // Creation timestamp
    pub last_accessed_at: DateTime<Utc>, // Last retrieval
    pub access_count: i32,              // Retrieval frequency
    pub embedding: Option<Vec<f32>>,    // Vector embedding
    pub links: Vec<MemoryLink>,         // Outgoing relationships
}
```

#### `Namespace`
Hierarchical isolation with priority-based retrieval.

```rust
pub enum Namespace {
    Global,                             // Priority: 1
    Project { name: String },           // Priority: 2
    Session { project: String, session_id: String }, // Priority: 3
}
```

**Serialization**: JSON in database as `{"type": "project", "name": "mnemosyne"}`

**String Format**: `global`, `project:name`, `session:project:id`

#### `MemoryType`
Classification for organizational and filtering.

```rust
pub enum MemoryType {
    ArchitectureDecision,  // System design choices
    CodePattern,           // Implementation approaches
    BugFix,                // Fixes and solutions
    Configuration,         // Settings
    Constraint,            // Requirements
    Entity,                // Domain concepts
    Insight,               // Learnings
    Reference,             // External resources
    Preference,            // User settings
    Task,                  // Action items
    AgentEvent,            // Orchestration events
}
```

**Type Factor**: Used in importance calculations (Architecture: 1.2x, Constraint: 1.1x, etc.)

#### `LinkType`
Typed relationships in knowledge graph.

```rust
pub enum LinkType {
    Extends,       // B builds upon A
    Contradicts,   // B invalidates A
    Implements,    // B implements A
    References,    // B cites A
    Supersedes,    // B replaces A
}
```

### Storage Schema

Complete database schema is in `migrations/libsql/`.

**Key Tables**:
- `memories` - Primary memory storage
- `memories_fts` - FTS5 full-text search index
- `memory_links` - Graph relationships
- `embeddings` - Vector embeddings (if using sqlite-vec)
- `memory_evolution_log` - Consolidation/archival audit trail
- `work_items` - Orchestration work queue
- `agent_events` - Multi-agent coordination events

**Indexes**:
- `idx_memories_namespace_importance` - Fast namespace queries with importance ordering
- `idx_memories_created_at` - Temporal queries
- `idx_memories_type` - Filter by memory type
- `idx_memory_links_source` - Graph traversal from source
- `idx_memory_links_target` - Reverse graph lookup

**JSON Columns**:
- `namespace` - Serialized Namespace enum
- `work_item.spec` - WorkItem specification (orchestration)

For detailed schema documentation, see [docs/STORAGE_SCHEMA.md](docs/STORAGE_SCHEMA.md).

### Orchestration Types

#### `WorkItem`
Unit of work in multi-agent orchestration.

```rust
pub struct WorkItem {
    pub id: WorkItemId,
    pub phase: Phase,                  // Prompt→Spec→FullSpec→Plan→Artifacts
    pub spec: WorkItemSpec,            // Task specification (JSON)
    pub state: WorkItemState,          // Pending/InProgress/Complete/Failed
    pub assigned_agent: Option<AgentId>,
    pub dependencies: Vec<WorkItemId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

#### `Phase`
Work Plan Protocol phases.

```rust
pub enum Phase {
    Prompt,        // User request → specification
    Spec,          // Specification → full decomposition
    FullSpec,      // Decomposition → execution plan
    Plan,          // Execution plan → artifacts
    Artifacts,     // Implementation complete
}
```

---

## Configuration

### Environment Variables

```bash
# API keys (stored via `mnemosyne secrets set`)
ANTHROPIC_API_KEY=sk-...           # For LLM enrichment

# Database configuration
DATABASE_URL=sqlite://path.db      # LibSQL database path
MNEMOSYNE_DB_PATH=/custom/path.db  # Alternative to DATABASE_URL

# Logging
RUST_LOG=info                      # Tracing level
RUST_LOG=mnemosyne=debug           # Debug for mnemosyne only

# Test mode
MNEMOSYNE_TEST_MODE=baseline       # For e2e tests
MNEMOSYNE_TEST_MODE=regression
```

### Cargo Configuration

`.cargo/config.toml`:
- `debug = false` - Faster dev builds (no debug symbols)
- `RUSTC_WRAPPER = "sccache"` - Compilation caching

`Cargo.toml` features:
- `default` - Standard build
- `python` - Enable PyO3 Python bindings
- `keyring-fallback` - OS keychain support (macOS/Windows/Linux)
- `distributed` - Enable ractor_cluster for distributed agents

### MCP Server Configuration

Automatically configured at `~/.claude/mcp_config.json` during installation.

```json
{
  "mnemosyne": {
    "command": "mnemosyne",
    "args": ["serve"],
    "env": {
      "MNEMOSYNE_DB_PATH": "/Users/user/.local/share/mnemosyne/mnemosyne.db"
    }
  }
}
```

---

## Development Workflows

### Building & Testing

#### Fast Type-Checking
```bash
cargo check                        # No codegen, fastest validation
```

#### Building
```bash
cargo build                        # Debug build (~3-4s incremental)
cargo build --release              # Optimized build (~2m 46s clean)
cargo build --profile fast-release # Thin LTO, good for testing
```

#### Testing
```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test '*'

# E2E tests
cd tests/e2e && ./run_all.sh

# Specific test
cargo test --lib test_memory_storage

# With logging
RUST_LOG=debug cargo test --lib -- --nocapture
```

#### Performance
```bash
# Build timings
cargo build --timings
open target/cargo-timings/cargo-timing-*.html

# sccache statistics
sccache --show-stats
```

### Code Organization Principles

**Module Boundaries**:
- `src/storage/` - Pure database operations, no business logic
- `src/services/` - External API calls (LLM, embeddings)
- `src/mcp/` - Protocol handling, delegates to ToolHandler
- `src/orchestration/` - Agent coordination, work distribution

**Error Handling**:
- Use `Result<T, MnemosyneError>` for all fallible operations
- Prefer `.context("message")` from `anyhow` for error chains
- Use `thiserror` for custom error types

**Async Patterns**:
- All I/O operations are `async` with `tokio`
- Use `tokio::spawn` for background tasks
- Use `Arc<T>` for shared state across async boundaries
- Prefer message-passing (via ractor) over shared mutable state

**Testing Conventions**:
- Unit tests in same file as implementation (`#[cfg(test)] mod tests`)
- Integration tests in `tests/` directory
- E2E tests in `tests/e2e/` as shell scripts
- Use `#[tokio::test]` for async tests
- Use `tempfile::tempdir()` for test databases

### Adding Features

#### Adding a New Memory Type
1. Add variant to `MemoryType` enum in `src/types.rs`
2. Update `type_factor()` method with appropriate weight
3. Update CLI parsing in `src/main.rs` (`parse_memory_type` function)
4. Add tests in `src/types.rs`
5. Update documentation in `AGENT_GUIDE.md` (this file)

#### Extending MCP Tools
1. Define tool schema in `src/mcp/tools.rs` (JSON Schema)
2. Add handler method to `ToolHandler` in `src/mcp/tools.rs`
3. Register tool in `McpServer::new()` tool list
4. Add integration test in `tests/mcp_integration_test.rs`
5. Update `MCP_SERVER.md` documentation

#### Adding Orchestration Agents
1. Create actor in `src/orchestration/actors/`
2. Implement `ractor::Actor` trait
3. Define message types in `src/orchestration/messages.rs`
4. Register in `SupervisionTree` (`src/orchestration/supervision.rs`)
5. Add unit tests for actor message handling
6. Add integration test in `tests/orchestration/`
7. Update `ORCHESTRATION.md` documentation

---

## Testing Strategy

### Test Organization

**Unit Tests** (in `src/` modules):
- Test individual functions and methods
- Mock external dependencies
- Fast execution (<1s total)

**Integration Tests** (`tests/` directory):
- Test module interactions
- Real database (in-memory LibSQL)
- Test MCP protocol, storage operations, orchestration

**E2E Tests** (`tests/e2e/` shell scripts):
- Test complete workflows with real binaries
- Database persistence across operations
- Baseline vs. regression modes
- **Current Status**: 15 test suites, focusing on namespace isolation

### Current Test Status

**Passing**: 610 tests
**Failing**: 10 tests (all in `storage::libsql_workitem_tests`)

**Known Failures**:
- WorkItem persistence tests fail with "no such table: work_items"
- Root cause: Test setup doesn't run full migration suite
- Workaround: Tests work in integration context with full migrations

### Running Tests

```bash
# All unit tests
cargo test --lib

# Specific module
cargo test --lib storage::libsql

# Integration tests
cargo test --test mcp_integration_test

# E2E tests (requires release build)
cargo build --release
cd tests/e2e
./run_all.sh

# E2E baseline mode (creates new baselines)
MNEMOSYNE_TEST_MODE=baseline ./storage_1_local_sqlite.sh

# E2E regression mode (compares to baseline)
MNEMOSYNE_TEST_MODE=regression ./storage_1_local_sqlite.sh
```

### Debugging Tests

**Enable Logging**:
```bash
RUST_LOG=debug cargo test --lib -- --nocapture test_name
```

**Inspect Test Database**:
```bash
# E2E tests create databases in /tmp
sqlite3 /tmp/mnemosyne_test_*.db

# Check schema
.schema memories

# Check data
SELECT * FROM memories WHERE namespace LIKE '%test%';
```

**Common Issues**:
- **FTS Trigger Errors**: SQLite 3.43.2+ has stricter FTS virtual table rules. Use conditional trigger from `migrations/*/003_fix_fts_triggers.sql` if needed.
- **Namespace Query Failures**: Ensure using `json_extract(namespace, '$.type')` for JSON-serialized namespaces.
- **Iroh Network Warnings**: Expected during laptop sleep, not actual errors.

---

## Common Tasks

### Adding a New MCP Tool

1. **Define Tool Schema** (`src/mcp/tools.rs`):
```rust
pub fn memory_export_tool() -> Tool {
    Tool {
        name: "mnemosyne.export".to_string(),
        description: "Export memories to JSON".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "namespace": {
                    "type": "string",
                    "description": "Namespace to export"
                },
                "output_path": {
                    "type": "string",
                    "description": "File path for export"
                }
            },
            "required": ["namespace", "output_path"]
        }),
    }
}
```

2. **Implement Handler** (`src/mcp/tools.rs`):
```rust
impl ToolHandler {
    async fn handle_export(&self, params: Value) -> Result<Value> {
        let namespace: String = serde_json::from_value(params["namespace"].clone())?;
        let output_path: String = serde_json::from_value(params["output_path"].clone())?;

        // Implementation
        let memories = self.storage.list_memories(&namespace, 1000).await?;
        let json = serde_json::to_string_pretty(&memories)?;
        std::fs::write(&output_path, json)?;

        Ok(json!({
            "exported": memories.len(),
            "path": output_path
        }))
    }
}
```

3. **Register Tool**:
```rust
// In src/mcp/server.rs or tools.rs
let tools = vec![
    // existing tools...
    memory_export_tool(),
];
```

4. **Add Tests**:
```rust
#[tokio::test]
async fn test_export_memories() {
    // Test implementation
}
```

5. **Update Documentation** (`MCP_SERVER.md`).

### Modifying Storage Schema

1. **Create Migration** (`migrations/libsql/NNN_description.sql`):
```sql
-- Add new column
ALTER TABLE memories ADD COLUMN version INTEGER DEFAULT 1;

-- Create index
CREATE INDEX IF NOT EXISTS idx_memories_version ON memories(version);
```

2. **Update StorageBackend Trait** (`src/storage/mod.rs`):
```rust
#[async_trait]
pub trait StorageBackend: Send + Sync {
    // Add new method
    async fn get_by_version(&self, version: i32) -> Result<Vec<MemoryNote>>;
}
```

3. **Implement in LibsqlStorage** (`src/storage/libsql.rs`):
```rust
async fn get_by_version(&self, version: i32) -> Result<Vec<MemoryNote>> {
    // Implementation
}
```

4. **Add Tests**:
```rust
#[tokio::test]
async fn test_version_filtering() {
    // Test implementation
}
```

5. **Update Documentation** (`docs/STORAGE_SCHEMA.md`, `ARCHITECTURE.md`).

### Debugging Tips

**Enable Detailed Logging**:
```bash
RUST_LOG=mnemosyne=debug,iroh=warn cargo run --bin mnemosyne -- serve
```

**Common Error Patterns**:

1. **"SQLite failure: no such table"**
   - Migration not run
   - Test database not initialized
   - Fix: Ensure migrations run in test setup

2. **"Unsafe use of virtual table 'memories_fts'"**
   - SQLite 3.43.2+ FTS trigger issue
   - Fix: Apply migration `003_fix_fts_triggers.sql`

3. **Iroh "hairpin prep" warnings during laptop sleep**
   - Expected behavior, not an error
   - NAT traversal attempts during network disconnect
   - No action needed

4. **"Failed to connect to LLM service"**
   - Missing or invalid ANTHROPIC_API_KEY
   - Fix: `mnemosyne secrets set anthropic-api-key`

**Debugging with LLDB/GDB**:
```bash
# Build with debug symbols
cargo build --profile dev -Z unstable-options --config 'profile.dev.debug=true'

# Run with debugger
rust-lldb target/debug/mnemosyne
```

---

## Resources

### Essential Documentation

**User-Facing**:
- [README.md](README.md) - Project overview and features
- [QUICK_START.md](QUICK_START.md) - Get started quickly
- [INSTALL.md](INSTALL.md) - Installation guide
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md) - Common issues and solutions

**Developer**:
- [ARCHITECTURE.md](ARCHITECTURE.md) - Technical deep dive (1460 lines)
- [CLAUDE.md](CLAUDE.md) - Development guidelines and Work Plan Protocol
- [CONTRIBUTING.md](CONTRIBUTING.md) - How to contribute
- [CHANGELOG.md](CHANGELOG.md) - Version history and changes

**Technical References**:
- [docs/TYPES_REFERENCE.md](docs/TYPES_REFERENCE.md) - Core types and schemas
- [docs/STORAGE_SCHEMA.md](docs/STORAGE_SCHEMA.md) - Database design
- [MCP_SERVER.md](MCP_SERVER.md) - MCP integration details
- [ORCHESTRATION.md](ORCHESTRATION.md) - Multi-agent system architecture

**Features**:
- [docs/features/](docs/features/) - Feature-specific documentation
  - Semantic Highlighting, ICS, Vector Search, Evolution, Privacy

**Guides**:
- [docs/guides/](docs/guides/) - How-to guides
  - LLM Reviewer Setup, Migration, Workflows, Testing

**Specifications**:
- [docs/specs/](docs/specs/) - Technical specifications
  - Multi-agent architecture, Project plan, Rust implementation, Test plan

**Build & Operations**:
- [docs/BUILD_OPTIMIZATION.md](docs/BUILD_OPTIMIZATION.md) - Build performance tuning
- [SECRETS_MANAGEMENT.md](SECRETS_MANAGEMENT.md) - API key management with age encryption

**Historical Context**:
- [docs/historical/](docs/historical/) - Archived session reports and decisions
- [ROADMAP.md](ROADMAP.md) - Future plans and completed milestones

---

## Contribution Guidelines

### Branch Naming
```
feature/description     # New features
fix/issue-description   # Bug fixes
refactor/component      # Code refactoring
docs/topic              # Documentation updates
cleanup/scope           # Code cleanup
```

### Commit Messages
- Describe what was done, not who did it
- Use imperative mood: "Add feature" not "Added feature"
- Reference issues: "Fix #123: Memory leak in consolidation"
- Keep first line under 72 characters
- **No AI attribution unless explicitly requested by user**

**Good Examples**:
```
Add graceful signal handling to MCP server

Refactor storage backend to use connection pooling

Fix race condition in agent coordination
```

### Pull Request Requirements

**Before Creating PR**:
- [ ] All tests passing (`cargo test --lib`)
- [ ] No compilation warnings (`cargo clippy`)
- [ ] Code formatted (`cargo fmt`)
- [ ] Documentation updated (if public API changed)
- [ ] CHANGELOG.md updated (for user-facing changes)

**PR Description**:
- Summary of changes
- Motivation/background
- Testing performed
- Any breaking changes
- Links to related issues

### Code Review Process

1. **Automated Checks**: CI runs tests and lints
2. **Peer Review**: At least one approval required
3. **Documentation**: Verify docs are updated
4. **Testing**: Confirm adequate test coverage
5. **Merge**: Squash merge to main (keeps history clean)

---

## Principles & Philosophy

### Work Plan Protocol

All development follows the **4-phase Work Plan Protocol** from [CLAUDE.md](CLAUDE.md):

1. **Prompt → Spec**: Transform request into clear specification
2. **Spec → Full Spec**: Decompose into components with dependencies
3. **Full Spec → Plan**: Create execution plan with parallelization
4. **Plan → Artifacts**: Execute plan, create code/tests/docs

**Never skip phases**. Each phase has defined exit criteria.

### Quality Gates

From [CLAUDE.md](CLAUDE.md), all work must pass:

- [ ] Intent satisfied (does it solve the problem?)
- [ ] Tests written and passing
- [ ] Documentation complete
- [ ] No anti-patterns (magic numbers, TODOs, stubs)
- [ ] No compiler warnings
- [ ] Type checking passes (where applicable)

### Memory-First Development

**Before coding**:
1. Recall relevant architectural decisions
2. Check for similar past solutions
3. Store key decisions as you make them
4. Link new memories to related concepts

**After coding**:
1. Store insights about what worked/didn't
2. Document gotchas and edge cases
3. Record performance characteristics
4. Update architecture memories if design changed

Use `mnemosyne remember` to store knowledge as you work.

### Unix Philosophy for Tools

Mnemosyne tools follow Unix principles:

- **Do one thing well**: Each tool has focused purpose
- **Composability**: Tools work together via files and pipes
- **Text streams**: JSON for data exchange
- **No surprising side effects**: Predictable behavior

**Example**:
```bash
# Composable workflow
mnemosyne recall --query "database" --format json | \
  jq -r '.[].content' | \
  mnemosyne remember --namespace "project:analysis" --type reference
```

---

## Appendix: Key Files Quick Reference

| File | Purpose |
|------|---------|
| `src/main.rs` | Main binary CLI, MCP server entry point |
| `src/lib.rs` | Library public API exports |
| `src/types.rs` | Core data structures (MemoryNote, Namespace, etc.) |
| `src/storage/libsql.rs` | Primary storage implementation |
| `src/mcp/server.rs` | JSON-RPC server for MCP protocol |
| `src/mcp/tools.rs` | MCP tool implementations |
| `src/orchestration/mod.rs` | Multi-agent orchestration engine |
| `Cargo.toml` | Dependencies and build configuration |
| `.cargo/config.toml` | Build optimizations (sccache, debug settings) |
| `migrations/libsql/` | Database schema migrations |

---

**End of Agent Guide**

For questions or clarifications, consult [ARCHITECTURE.md](ARCHITECTURE.md) or [CLAUDE.md](CLAUDE.md).
