# Mnemosyne Agent Guide

**Last Updated**: 2025-11-02
**Version**: 2.1.1
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
- **gRPC Remote Access**: Production-ready RPC server with full CRUD, search, and streaming APIs

### Current Status
- **Version**: 2.1.2 (stable release)
- **Test Status**: 610 tests passing, 10 known failures (storage backend tests)
- **Build Time**: ~1-2m clean build, ~1-3s incremental (fast-release), ~2-3m production build
- **Language**: Rust 1.75+, Python 3.10-3.13 (via PyO3)

---

## Quick Start for Agents

### Build Commands
```bash
# Check compilation
cargo check

# Fast rebuild (recommended for development - ~1-3s incremental)
./scripts/rebuild-and-update-install.sh

# Production build (full optimizations, slower)
./scripts/rebuild-and-update-install.sh --full-release

# Full clean build and install (first-time setup)
./scripts/build-and-install.sh

# Run tests
cargo test --lib                    # Unit + integration tests
cargo test --test '*'               # All tests including e2e

# Run specific binary
cargo run --bin mnemosyne -- --help
cargo run --bin mnemosyne-ics
cargo run --bin mnemosyne-dash
cargo run --bin mnemosyne-rpc --features rpc

# Build RPC server (requires --features rpc)
cargo build --release --features rpc --bin mnemosyne-rpc
cargo test --features rpc --test rpc_services_test
```

**Build Performance Comparison**:
| Script | Incremental | Clean Build | Use Case |
|--------|-------------|-------------|----------|
| `rebuild-and-update-install.sh` (default) | ~1s | ~1-2m | **Development iterations** (recommended) |
| `rebuild-and-update-install.sh --full-release` | ~40-50s | ~2-3m | Production builds, performance testing |
| `build-and-install.sh` | ~60s | ~3-4m | First-time setup, config changes |

**Technical Details**:
- **Fast-release profile**: Uses thin LTO, parallel codegen (16 units), incremental compilation
- **Direct binary copy**: Skips `cargo install` overhead (~30-40s saved)
- **macOS code signing**: Automatically preserves xattr + codesign for Gatekeeper compatibility

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
- **RPC** (`src/rpc/`): gRPC server for remote access (feature-gated: `--features rpc`)
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

## Repository Organization

### Organization Principles

Mnemosyne follows strict organization standards to maintain clarity and navigability:

**Core Principles**:
- **Always Tidy**: Repository should be clean, well-organized, and navigable
- **Non-Destructive**: Preserve git history, verify before deleting
- **Reference Integrity**: Update all links and imports when moving/renaming
- **Logical Structure**: Files belong in appropriate directories
- **Test After Changes**: Verify functionality after structural reorganization

### Directory Structure

```
mnemosyne/
├── src/                    # Source code organized by module
│   ├── storage/           # Storage backends
│   ├── services/          # External service integrations
│   ├── mcp/               # Model Context Protocol
│   ├── orchestration/     # Multi-agent system
│   ├── ics/               # Interactive Context Studio
│   ├── evaluation/        # Privacy-preserving evaluation
│   └── evolution/         # Memory evolution
│
├── docs/                   # All documentation (130+ files)
│   ├── INDEX.md           # Documentation navigation hub (KEEP UPDATED)
│   ├── features/          # Feature-specific documentation
│   ├── guides/            # How-to guides
│   ├── specs/             # Technical specifications
│   ├── historical/        # Archived session reports and decisions
│   ├── archive/           # Deprecated documentation
│   └── v2/                # Version-specific planning docs
│
├── tests/                  # Test suites
│   ├── e2e/               # End-to-end tests (shell scripts)
│   ├── integration/       # Integration tests
│   └── *.rs               # Unit tests (also in src/ modules)
│
├── scripts/                # Automation scripts
│   ├── install/           # Installation and uninstallation
│   ├── testing/           # Test runners
│   ├── build-and-install.sh           # First-time setup (full build)
│   └── rebuild-and-update-install.sh  # Fast rebuild (recommended)
│
├── migrations/             # Database migrations
│   └── libsql/            # LibSQL schema migrations
│
└── Root Level              # Key documentation (25 files)
    ├── README.md          # Main project overview
    ├── AGENT_GUIDE.md     # Agent development guide (this file)
    ├── CLAUDE.md          # Claude Code guidelines
    ├── ARCHITECTURE.md    # Technical deep dive
    ├── CHANGELOG.md       # Version history
    ├── ROADMAP.md         # Future plans
    └── ...                # Other root-level docs
```

### Repository Tidying Protocol

**Regular Maintenance Tasks**:

#### 1. Remove Temporary Files
```bash
# Check for temporary files
find . -name "*.bak" -o -name "*.tmp" -o -name "*.swp" -o -name ".DS_Store"

# Remove after verification
find . -name "*.bak" -delete
find . -name ".DS_Store" -delete
```

#### 2. Clean Up Stale Branches
```bash
# List merged branches
git branch --merged main | grep -v "main"

# Delete stale local branches
git branch -d branch-name

# Prune remote tracking branches
git remote prune origin
```

#### 3. Archive Completed Work
```bash
# Move completed session reports to docs/historical/
git mv docs/session-report-xyz.md docs/historical/session-reports/

# Move deprecated features docs
git mv docs/features/deprecated-feature.md docs/archive/

# Update docs/INDEX.md to reflect changes
vim docs/INDEX.md
```

#### 4. Tidying Checklist
Before committing structural changes:

- [ ] Remove temporary files (.bak, .tmp, .swp, .DS_Store)
- [ ] Clean up stale git branches
- [ ] Archive completed work to docs/historical/
- [ ] Update docs/INDEX.md when adding/moving docs
- [ ] Verify all cross-references after moves
- [ ] Run tests after structural changes
- [ ] Commit with descriptive message (e.g., "chore: Clean up temporary files")

### File Operations Protocol

#### Moving Files

**Protocol**:
```bash
# 1. Identify all references to the file
grep -r "old-name" . --exclude-dir={target,.git}
rg "old-name"  # Faster with ripgrep

# 2. Use git mv to preserve history
git mv old-path/file.rs new-path/file.rs

# 3. Update all references
# - Update module imports in Rust code
# - Update documentation links
# - Update docs/INDEX.md
# - Update README.md if referenced

# 4. Verify with grep again
grep -r "old-name" . --exclude-dir={target,.git}

# 5. Test to ensure functionality preserved
cargo check
cargo test --lib

# 6. Commit with descriptive message
git add .
git commit -m "refactor: Move X to Y for better organization

- Moved old-path/file.rs → new-path/file.rs
- Updated all imports and documentation references
- Verified tests pass"
```

**Example**:
```bash
# Move feature doc to correct location
git mv docs/ics-feature.md docs/features/ICS_README.md

# Update references
grep -r "ics-feature.md" docs/
# Update all found references

# Update INDEX.md
vim docs/INDEX.md

# Test
cargo test --lib

# Commit
git commit -m "docs: Move ICS feature doc to features/ directory"
```

#### Renaming Files

**Same protocol as moving**:
```bash
# 1. Identify references
grep -r "OldName" . --exclude-dir={target,.git}

# 2. Rename with git mv
git mv src/old_name.rs src/new_name.rs

# 3. Update code references (imports, use statements)
vim src/lib.rs  # Update module declarations
vim src/other.rs  # Update use statements

# 4. Update documentation references
grep -r "old_name" docs/
# Update all found references

# 5. Test
cargo check
cargo test --lib

# 6. Commit
git commit -m "refactor: Rename old_name to new_name for clarity"
```

#### Deleting Files

**Verification Protocol**:
```bash
# 1. Verify no references exist
grep -r "filename" . --exclude-dir={target,.git}

# 2. Check if file is imported/used
rg "filename" src/

# 3. If safe to delete, use git rm
git rm path/to/file.rs

# 4. Update documentation to remove references
grep -r "filename" docs/
# Remove all found references

# 5. Test
cargo check
cargo test --lib

# 6. Commit with explanation
git commit -m "chore: Remove unused file X

File was no longer referenced after refactoring in commit abc123.
Verified no imports or documentation references remain."
```

**Before Deleting, Ask**:
- Is this file referenced anywhere?
- Is it imported by other modules?
- Is it mentioned in documentation?
- Does removing it break tests?
- Should it be archived instead of deleted?

### Preventing Repository Clutter

**Good Practices**:
- Commit completed work promptly (don't accumulate)
- Delete stale branches after merging
- Archive instead of delete when historical value exists
- Use .gitignore for build artifacts and temporary files
- Regular tidying sessions (weekly for active development)

**Anti-Patterns**:
```
❌ Leaving .bak files in repository
❌ Accumulating stale branches (10+ merged branches)
❌ Moving files without updating references
❌ Deleting files without checking for imports
❌ Ignoring docs/INDEX.md when restructuring docs/
❌ Committing without testing after structural changes
```

### Recovery from Accidental Changes

**If you accidentally delete or move incorrectly**:
```bash
# Undo uncommitted changes
git checkout HEAD -- path/to/file

# Undo last commit (keep changes)
git reset --soft HEAD~1

# Undo last commit (discard changes)
git reset --hard HEAD~1

# Restore file from specific commit
git checkout <commit-hash> -- path/to/file
```

### Cross-Reference Maintenance

**When moving/renaming files, update**:
- [ ] Code imports (`use`, `mod` declarations)
- [ ] Documentation links (relative paths)
- [ ] docs/INDEX.md (navigation hub)
- [ ] README.md (if file is referenced)
- [ ] AGENT_GUIDE.md (if file is mentioned)
- [ ] ARCHITECTURE.md (if architectural component)
- [ ] Test file paths (in test code)

**Verification Command**:
```bash
# After moving file, verify no broken references
grep -r "old-name" . --exclude-dir={target,.git} | grep -v ".git/logs"

# Check all markdown links (if markdown-link-check installed)
find docs -name "*.md" -exec markdown-link-check {} \;
```

---

## Documentation Management

### Documentation Hierarchy

Mnemosyne maintains comprehensive documentation across three tiers:

#### Tier 1: Root Level Documentation (25 files)
**Purpose**: Primary entry points and high-level overviews

| Document | Purpose | Target Audience |
|----------|---------|-----------------|
| README.md | Project overview, features, quick start | Users, potential contributors |
| AGENT_GUIDE.md | Comprehensive development guide | Agents, developers |
| CLAUDE.md | Claude Code workflow guidelines | Claude Code agents |
| ARCHITECTURE.md | Technical deep dive (1460 lines) | Developers, architects |
| CHANGELOG.md | Version history and changes | All stakeholders |
| ROADMAP.md | Future plans and milestones | Users, contributors |
| MCP_SERVER.md | MCP integration details | MCP developers |
| ORCHESTRATION.md | Multi-agent coordination | Orchestration developers |
| INSTALL.md | Detailed installation guide | New users |
| TROUBLESHOOTING.md | Common issues and solutions | All users |
| CONTRIBUTING.md | Contribution guidelines | Contributors |
| QUICK_START.md | 5-minute getting started | New users |

#### Tier 2: Detailed Documentation (docs/ - 130+ files)
**Purpose**: In-depth technical documentation

| Directory | Contents | Update Frequency |
|-----------|----------|------------------|
| docs/INDEX.md | **Navigation hub** (KEEP UPDATED) | Every doc addition/move |
| docs/features/ | Feature-specific documentation | Per feature release |
| docs/guides/ | How-to guides and workflows | As workflows evolve |
| docs/specs/ | Technical specifications | During design phase |
| docs/historical/ | Archived reports and decisions | Post-milestone |
| docs/archive/ | Deprecated documentation | When deprecating features |

**Key Documents**:
- docs/TYPES_REFERENCE.md - Complete type system reference
- docs/STORAGE_SCHEMA.md - Database schema and queries
- docs/BUILD_OPTIMIZATION.md - Build performance tuning

#### Tier 3: GitHub Pages (https://github.com/yourusername/mnemosyne)
**Purpose**: Public-facing documentation website

- Automatically built from docs/ directory on push to main
- No manual deployment required
- Provides searchable, navigable documentation site
- Updates within minutes of git push

### Documentation Update Triggers

**ALWAYS update documentation when**:

| Change Type | Primary Docs | Secondary Docs | Notes |
|-------------|--------------|----------------|-------|
| **New Feature** | README.md, docs/features/X.md | CHANGELOG.md, ROADMAP.md | Add to feature list, update status |
| **Architecture Change** | ARCHITECTURE.md, AGENT_GUIDE.md | README.md | Update diagrams, code examples |
| **API Modification** | docs/TYPES_REFERENCE.md, MCP_SERVER.md | AGENT_GUIDE.md | Update type signatures, examples |
| **Workflow Change** | CLAUDE.md, docs/guides/workflows.md | AGENT_GUIDE.md | Update command sequences |
| **Dependency Addition** | README.md, INSTALL.md | CONTRIBUTING.md | Update installation steps |
| **Release** | CHANGELOG.md, ROADMAP.md | README.md (version) | Follow release management protocol |
| **File Move/Rename** | docs/INDEX.md, all cross-references | - | Critical for navigation |
| **Configuration Change** | Relevant docs, TROUBLESHOOTING.md | - | Update examples, add troubleshooting |
| **Bug Fix** | TROUBLESHOOTING.md | CHANGELOG.md | Document fix, prevent recurrence |
| **Performance Improvement** | README.md, ARCHITECTURE.md | CHANGELOG.md | Update benchmarks, metrics |

### Documentation Update Workflow

#### Standard Update Process
```bash
# 1. Identify affected documentation
# Consult "Documentation Update Triggers" table above

# 2. Update primary documents
vim README.md                      # User-facing changes
vim AGENT_GUIDE.md                 # Developer/agent guidance
vim CHANGELOG.md                   # Version history entry

# 3. Update detailed documentation
vim docs/INDEX.md                  # Navigation changes
vim docs/features/X.md             # Feature-specific details
vim docs/guides/workflow.md        # Workflow updates

# 4. Verify cross-references
grep -r "old-reference" docs/ --exclude-dir=historical
rg "old-reference" docs/

# 5. Update cross-references
# Use Edit tool or vim to update all found references

# 6. Test links (if markdown-link-check available)
find docs -name "*.md" -exec markdown-link-check {} \;

# 7. Commit documentation updates
git add README.md CHANGELOG.md docs/
git commit -m "docs: Update documentation for feature X

- Added feature X to README.md feature list
- Updated AGENT_GUIDE.md with new workflow
- Added docs/features/X.md for detailed documentation
- Updated docs/INDEX.md navigation
- Updated CHANGELOG.md for v2.2.0"

# 8. Verify GitHub Pages deployment
# Wait 2-3 minutes, then visit:
# https://yourusername.github.io/mnemosyne/
```

#### Quick Documentation Update (Minor Changes)
```bash
# For typos, clarifications, small improvements
vim docs/features/X.md
git add docs/features/X.md
git commit -m "docs: Fix typo in feature X documentation"
git push
```

#### Major Documentation Overhaul
```bash
# 1. Create feature branch for documentation work
git checkout -b docs/major-update

# 2. Make all documentation updates
# (multiple files, restructuring, etc.)

# 3. Verify all links
find docs -name "*.md" -exec markdown-link-check {} \;

# 4. Test documentation locally (if preview available)
# mdbook serve docs/  # If using mdbook
# Or open markdown files in viewer

# 5. Commit and create PR
git add .
git commit -m "docs: Major documentation overhaul

- Reorganized docs/ structure
- Updated all cross-references
- Added missing API documentation
- Improved navigation in INDEX.md"

git push -u origin docs/major-update
gh pr create --title "Major Documentation Overhaul"
```

### Documentation Standards

#### Frontmatter and Metadata
Every documentation file should include:

```markdown
# Document Title

**Last Updated**: 2025-11-07
**Version**: 2.2.0 (if version-specific)
**For**: Target audience (e.g., "Developers", "Users", "Agents")

---

## Quick Links
- [Section 1](#section-1)
- [Section 2](#section-2)

---
```

#### Heading Hierarchy
```markdown
# H1: Document Title (ONE per file)

## H2: Major Sections

### H3: Subsections

#### H4: Sub-subsections (use sparingly)
```

#### Code Examples
**Always include**:
- Syntax highlighting (```rust, ```bash, ```json)
- Comments explaining non-obvious parts
- Expected output or result
- Error handling examples

**Good Example**:
````markdown
```bash
# Store memory with explicit namespace
mnemosyne remember "Database uses LibSQL" \
  --namespace "project:mnemosyne" \
  --importance 9

# Expected output:
# ✓ Memory stored: mem_abc123
```
````

#### Cross-References
**Use relative links**:
```markdown
<!-- Good -->
See [ARCHITECTURE.md](ARCHITECTURE.md) for details.
See [Type Reference](docs/TYPES_REFERENCE.md) for complete list.

<!-- Bad - avoid absolute URLs to same repo -->
See https://github.com/user/mnemosyne/blob/main/ARCHITECTURE.md
```

**Test links after moving/renaming**:
```bash
# After moving file
grep -r "old-filename" docs/
# Update all found references
```

#### Context Efficiency
**Guidelines**:
- Concise but complete - no unnecessary words
- Use tables for comparisons and references
- Use bullet lists for sequences and options
- Use code examples over prose explanations
- Cross-reference instead of duplicating content

**Example of Context-Efficient Writing**:
```markdown
<!-- Good: Concise, table format -->
| Command | Purpose |
|---------|---------|
| `mnemosyne init` | Initialize database |
| `mnemosyne remember` | Store memory |

<!-- Bad: Verbose prose -->
The command `mnemosyne init` is used to initialize the database,
while the command `mnemosyne remember` is utilized for the purpose
of storing a new memory in the system...
```

### docs/INDEX.md Maintenance

**CRITICAL**: docs/INDEX.md is the documentation navigation hub. **ALWAYS** update when:

#### 1. Adding New Documentation
```markdown
<!-- Add to appropriate section in INDEX.md -->
### Features
- [**New Feature X**](features/new-feature.md) - Description
```

#### 2. Moving Documentation
```markdown
<!-- Update path in INDEX.md -->
<!-- Old -->
- [**Feature X**](specs/feature-x.md)

<!-- New -->
- [**Feature X**](features/feature-x.md)
```

#### 3. Archiving Documentation
```markdown
<!-- Move from main section to historical -->
### Historical Context
- [**Feature X Design**](historical/feature-x-design.md) - Archived 2025-11-07
```

#### 4. Reorganizing Documentation Structure
```bash
# After reorganization
vim docs/INDEX.md
# Update all paths and navigation structure
# Verify completeness:
# - All new files listed
# - All moved files updated
# - All sections organized logically
```

**INDEX.md Update Checklist**:
- [ ] New files added to appropriate section
- [ ] Moved files have updated paths
- [ ] Archived files moved to historical section
- [ ] Section organization still makes sense
- [ ] No broken links (test with markdown-link-check)
- [ ] "Last Updated" date updated

### Synchronizing Documentation Across Tiers

**Consistency Requirements**:
- Version numbers match across README.md, CHANGELOG.md, Cargo.toml
- Feature lists consistent between README.md and docs/features/
- API examples consistent between root docs and docs/specs/
- Architecture diagrams consistent between ARCHITECTURE.md and docs/specs/

**Synchronization Workflow**:
```bash
# After major feature release
vim README.md                   # Update feature list, status
vim CHANGELOG.md                # Add version entry
vim ROADMAP.md                  # Mark milestone complete
vim docs/features/X.md          # Add detailed documentation
vim docs/INDEX.md               # Update navigation

# Verify consistency
diff <(grep "^- " README.md | grep Feature) \
     <(ls docs/features/*.md | xargs basename -s .md)
```

### GitHub Pages Deployment

**Automatic Deployment**:
- Triggered on every push to `main`
- Builds from `docs/` directory
- Typically completes in 2-3 minutes
- No manual build or deployment required

**Verification**:
```bash
# After pushing documentation updates
git push origin main

# Wait 2-3 minutes, then verify
curl -I https://yourusername.github.io/mnemosyne/ | grep "200 OK"

# Or visit in browser
open https://yourusername.github.io/mnemosyne/
```

**Troubleshooting**:
If GitHub Pages doesn't update:
1. Check GitHub Actions tab for build errors
2. Verify docs/ directory structure is correct
3. Ensure markdown files are valid (no syntax errors)
4. Check GitHub repository settings → Pages → Build and deployment

### Documentation Quality Checklist

Before committing documentation changes:

**Content Quality**:
- [ ] Information is accurate and up-to-date
- [ ] Code examples are tested and work
- [ ] No outdated version numbers or references
- [ ] Technical terms are defined or linked
- [ ] Assumptions are stated clearly

**Structure & Navigation**:
- [ ] Headings follow hierarchy (H1 → H2 → H3)
- [ ] Table of contents for long documents
- [ ] Cross-references use relative links
- [ ] docs/INDEX.md updated if structure changed
- [ ] "See also" links to related documents

**Formatting & Style**:
- [ ] Code blocks have syntax highlighting
- [ ] Tables are well-formatted
- [ ] Lists are properly formatted (bullets or numbers)
- [ ] No spelling errors (run spell checker)
- [ ] Consistent terminology throughout

**Completeness**:
- [ ] All public APIs documented
- [ ] Edge cases and gotchas mentioned
- [ ] Troubleshooting section included (if applicable)
- [ ] Examples cover common use cases
- [ ] "Last Updated" date is current

### Documentation Anti-Patterns

**Avoid**:
```
❌ Outdated version numbers in examples
❌ Broken cross-references after file moves
❌ Code examples that don't compile/run
❌ Duplicating content instead of cross-referencing
❌ Absolute GitHub URLs to same repo
❌ Missing docs/INDEX.md updates
❌ Forgetting to update CHANGELOG.md
❌ Inconsistent terminology (e.g., "memory" vs "note")
❌ Long prose paragraphs (use lists and tables)
❌ Committing documentation without testing links
```

**Good Practices**:
```
✅ Use tables for comparisons and references
✅ Cross-reference with relative links
✅ Test code examples before committing
✅ Update docs/INDEX.md with every structural change
✅ Keep CHANGELOG.md current
✅ Use consistent terminology project-wide
✅ Verify GitHub Pages deployment after push
✅ Archive old docs instead of deleting
✅ Include "See also" sections for navigation
✅ Run markdown-link-check before committing
```

### Emergency Documentation Fixes

**If critical documentation error is discovered**:
```bash
# 1. Fix immediately (don't wait for feature branch)
git checkout main
git pull
vim README.md  # Fix critical error

# 2. Commit with clear message
git add README.md
git commit -m "docs(urgent): Fix critical error in installation instructions

Previous instructions would fail on macOS due to incorrect path.
Updated to use correct ~/.local/bin/ path."

# 3. Push immediately
git push origin main

# 4. Verify GitHub Pages update
# Wait 2-3 minutes, check deployment

# 5. Notify team (if applicable)
# Post in team chat or create issue to track
```

---

## Release Management

### Semantic Versioning

Mnemosyne follows [Semantic Versioning 2.0.0](https://semver.org/) specification:

**Version Format**: `MAJOR.MINOR.PATCH` (e.g., `2.1.2`)

| Component | Increment When | Example |
|-----------|----------------|---------|
| **MAJOR** | Breaking changes, incompatible API | `1.x.x` → `2.0.0` |
| **MINOR** | New features, backward-compatible | `2.0.x` → `2.1.0` |
| **PATCH** | Bug fixes, backward-compatible | `2.1.0` → `2.1.1` |

**Current Version**: 2.1.2 (as of 2025-11-07)

### Release Triggers

#### MAJOR Version Release (X.0.0)

**Trigger when**:
- Breaking API changes (incompatible with previous MAJOR version)
- Incompatible storage schema changes (requires migration)
- Major architecture redesign (significant refactoring)
- Removal of deprecated features (after deprecation period)
- Changes to core abstractions (e.g., `StorageBackend` trait signature)

**Examples**:
- v1.x → v2.0.0: LibSQL migration (incompatible schema)
- v2.x → v3.0.0: Breaking MCP protocol changes

**Process**:
1. Announce breaking changes in advance
2. Provide migration guide
3. Update major documentation (ARCHITECTURE.md, MIGRATION.md)
4. Extensive testing (all test suites)
5. Consider release candidate (v3.0.0-rc.1)

#### MINOR Version Release (x.Y.0)

**Trigger when**:
- New features added (non-breaking)
- New MCP tools (backward-compatible)
- New orchestration capabilities
- Significant performance improvements (non-breaking)
- New CLI commands (backward-compatible)
- Deprecations (with backward compatibility)

**Examples**:
- v2.0.0 → v2.1.0: ICS integration, semantic highlighting
- v2.1.0 → v2.2.0: New DSPy integration module

**Process**:
1. Update CHANGELOG.md with feature descriptions
2. Update README.md feature list
3. Add feature documentation (docs/features/)
4. Update ROADMAP.md milestones
5. Full test suite pass

#### PATCH Version Release (x.y.Z)

**Trigger when**:
- Bug fixes (non-breaking)
- Security patches
- Documentation improvements
- Minor performance tweaks
- Dependency updates (no API changes)
- Build/tooling improvements

**Examples**:
- v2.1.0 → v2.1.1: Python bridge production hardening
- v2.1.1 → v2.1.2: Compiler warnings cleanup

**Process**:
1. Update CHANGELOG.md with fix descriptions
2. Verify tests pass
3. Update README.md version number
4. Quick release (minimal ceremony)

### Pre-Release Versions

For testing before stable release:

| Suffix | Purpose | Example |
|--------|---------|---------|
| `-alpha.N` | Early development, unstable | `v3.0.0-alpha.1` |
| `-beta.N` | Feature complete, testing | `v3.0.0-beta.1` |
| `-rc.N` | Release candidate, final testing | `v3.0.0-rc.1` |

**Usage**:
```bash
git tag -a v3.0.0-rc.1 -m "Release candidate 1 for v3.0.0"
gh release create v3.0.0-rc.1 --prerelease --title "v3.0.0 RC1"
```

### Release Process

#### Standard Release Workflow

```bash
# 1. Verify all tests pass
cargo test --lib                    # Unit tests
cargo test --test '*'               # Integration tests
bash tests/e2e/run_all.sh           # E2E tests
cargo clippy --all-targets          # No warnings

# 2. Update version in Cargo.toml
vim Cargo.toml
# [package]
# version = "2.2.0"

# 3. Update CHANGELOG.md
vim CHANGELOG.md
# ## [2.2.0] - 2025-11-07
# ### Added
# - New DSPy integration module with OptimizerModule
# - Skills discovery integration with SkillsDiscovery
# ### Fixed
# - Memory persistence edge case in consolidation
# ### Changed
# - Improved error messages for storage failures

# 4. Update README.md status section
vim README.md
# **Current Version**: 2.2.0
# **v2.2.0 Release (2025-11-07)** - DSPy Integration:
# - ✅ DSPy OptimizerModule integration complete
# - ✅ Skills discovery with dynamic loading
# ...

# 5. Update ROADMAP.md
vim ROADMAP.md
# **Completed (v2.2.0)**:
# - ✅ DSPy integration (OptimizerModule, ReviewerModule)
# - ✅ Skills discovery system
# **In Progress** (v2.3):
# - 🔄 ...

# 6. Commit version bump
git add Cargo.toml Cargo.lock CHANGELOG.md README.md ROADMAP.md
git commit -m "chore: Bump version to 2.2.0

Prepare for v2.2.0 release with DSPy integration and skills discovery."

# 7. Create annotated tag
git tag -a v2.2.0 -m "Release v2.2.0: DSPy Integration and Skills Discovery

Major features:
- DSPy OptimizerModule integration
- Skills discovery system with dynamic loading
- Enhanced error messages
- Bug fixes for memory persistence

See CHANGELOG.md for complete list of changes."

# 8. Build release binary
cargo build --release
# Verify binary
./target/release/mnemosyne --version
# Expected: mnemosyne 2.2.0

# 9. Push tag to GitHub
git push origin main        # Push version bump commit
git push origin v2.2.0      # Push tag

# 10. Create GitHub release with binary
gh release create v2.2.0 \
  --title "v2.2.0: DSPy Integration" \
  --notes-file <(cat <<'EOF'
# v2.2.0: DSPy Integration and Skills Discovery

## Highlights
- **DSPy Integration**: OptimizerModule and ReviewerModule for AI-powered optimization
- **Skills Discovery**: Dynamic skill loading based on task context
- **Enhanced Errors**: Improved error messages with context

## Added
- DSPy OptimizerModule integration with SkillsDiscovery
- Skills discovery system with relevance scoring
- Enhanced error messages for storage operations

## Fixed
- Memory persistence edge case in consolidation
- Rare race condition in agent coordination

## Changed
- Improved performance of vector search (10% faster)
- Updated dependencies (libsql 0.6.0)

See [CHANGELOG.md](https://github.com/yourusername/mnemosyne/blob/main/CHANGELOG.md) for complete details.

---
**Installation**:
```bash
cargo install --path .
# or download binary from release assets
```
EOF
) \
  ./target/release/mnemosyne#mnemosyne-v2.2.0-$(uname -s)-$(uname -m)

# 11. Verify release
gh release view v2.2.0
open https://github.com/yourusername/mnemosyne/releases/tag/v2.2.0

# 12. Verify GitHub Pages updated
# Wait 2-3 minutes for automatic deployment
open https://yourusername.github.io/mnemosyne/
```

### Pre-Release Checklist

**Before creating release** (all must pass):

#### Code Quality
- [ ] All tests passing (unit, integration, E2E)
- [ ] No compiler warnings (`cargo clippy`)
- [ ] Code formatted (`cargo fmt`)
- [ ] No TODOs or FIXMEs in committed code
- [ ] Dependencies up to date (check `cargo outdated`)

#### Documentation
- [ ] CHANGELOG.md updated with all changes
- [ ] README.md version updated
- [ ] README.md status section updated
- [ ] ROADMAP.md milestones updated
- [ ] Feature documentation added (if new features)
- [ ] API documentation updated (if API changes)
- [ ] Migration guide added (if MAJOR version)

#### Version Management
- [ ] Version bumped in Cargo.toml
- [ ] Cargo.lock updated (`cargo build`)
- [ ] Version consistent across all files
- [ ] Tag message prepared with highlights
- [ ] Release notes prepared

#### Build Verification
- [ ] Release build succeeds (`cargo build --release`)
- [ ] Binary runs and shows correct version
- [ ] Installation script tested (`./scripts/install/install.sh`)
- [ ] Uninstallation script tested (`./scripts/install/uninstall.sh`)

#### Testing
- [ ] Fresh install on clean system (if possible)
- [ ] Database migrations tested (if schema changes)
- [ ] MCP server integration tested
- [ ] E2E tests pass in release mode

### Post-Release Checklist

**After creating release**:

#### Verification
- [ ] Tag created and pushed to GitHub
- [ ] GitHub release created with binary
- [ ] Release notes published
- [ ] Binary downloadable from release page
- [ ] GitHub Pages updated with new docs
- [ ] Version number correct in all locations

#### Communication
- [ ] Announcement prepared (if public release)
- [ ] Team notified (if applicable)
- [ ] CHANGELOG.md link shared
- [ ] Known issues documented (if any)

#### Follow-Up
- [ ] Monitor for critical bugs (first 24 hours)
- [ ] Respond to user feedback
- [ ] Create hotfix branch if critical bug found
- [ ] Update project board/milestones

### Hotfix Release Process

For critical bugs in production that cannot wait for next release:

```bash
# 1. Create hotfix branch from latest release tag
git checkout -b hotfix/2.1.3 v2.1.2

# 2. Fix the critical bug
vim src/storage/libsql.rs  # Apply fix
vim tests/fix_test.rs      # Add test for bug

# 3. Test thoroughly
cargo test --lib
cargo test --all
bash tests/e2e/relevant_test.sh

# 4. Update CHANGELOG.md (PATCH version)
vim CHANGELOG.md
# ## [2.1.3] - 2025-11-08 (Hotfix)
# ### Fixed
# - Critical bug causing data loss in edge case

# 5. Update version
vim Cargo.toml
# version = "2.1.3"

# 6. Commit hotfix
git add .
git commit -m "fix(critical): Prevent data loss in edge case

Fixes #issue-number

Bug occurred when concurrent writes happened during consolidation.
Added mutex protection and comprehensive tests."

# 7. Create tag
git tag -a v2.1.3 -m "Hotfix v2.1.3: Critical data loss bug

Fixes critical bug that could cause data loss under concurrent writes during consolidation."

# 8. Merge back to main
git checkout main
git merge --no-ff hotfix/2.1.3 -m "Merge hotfix v2.1.3 into main"

# 9. Push everything
git push origin main
git push origin v2.1.3

# 10. Create GitHub release (mark as important)
gh release create v2.1.3 \
  --title "v2.1.3 (Hotfix): Critical Data Loss Fix" \
  --notes "🚨 **CRITICAL HOTFIX**: This release fixes a critical bug that could cause data loss under concurrent writes during memory consolidation. All users on v2.1.x should upgrade immediately.

See CHANGELOG.md for details." \
  ./target/release/mnemosyne

# 11. Clean up hotfix branch
git branch -d hotfix/2.1.3
```

### Release Cadence

**Suggested Schedule**:
- **MAJOR**: Annually or when breaking changes accumulate
- **MINOR**: Every 2-3 months or when features complete
- **PATCH**: As needed (bug fixes, documentation)
- **HOTFIX**: Immediately for critical bugs

**Current Pattern** (observed):
- v2.0.0: 2025-10-15 (major LibSQL migration)
- v2.1.0: 2025-11-02 (minor ICS and semantic highlighting)
- v2.1.1: 2025-11-06 (patch production hardening)
- v2.1.2: 2025-11-06 (patch clean build)

### Version Numbering Rules

**When in doubt**:
1. Breaking change? → MAJOR
2. New feature? → MINOR
3. Bug fix? → PATCH
4. Documentation only? → PATCH (or no version bump)

**Special Cases**:
- **Security fixes**: Always release immediately (PATCH or MINOR)
- **Performance improvements**: MINOR if significant (>20%), PATCH otherwise
- **Dependency updates**: PATCH unless breaking change
- **Refactoring**: No version bump unless behavior changes

### Release Announcement Template

**For MINOR/MAJOR releases**:
```markdown
# Mnemosyne v2.2.0 Released 🎉

We're excited to announce the release of Mnemosyne v2.2.0, featuring DSPy integration and skills discovery!

## Highlights
- **DSPy Integration**: AI-powered optimization with OptimizerModule
- **Skills Discovery**: Dynamic skill loading based on task context
- **Enhanced Errors**: Clearer error messages with context

## Installation
\```bash
cargo install --path .
# or download binary from GitHub releases
\```

## Full Changelog
See [CHANGELOG.md](https://github.com/yourusername/mnemosyne/blob/main/CHANGELOG.md) for complete list of changes.

## Upgrade Notes
No breaking changes. Drop-in replacement for v2.1.x.

## Known Issues
None at this time.

## What's Next
Check out our [ROADMAP.md](https://github.com/yourusername/mnemosyne/blob/main/ROADMAP.md) for upcoming features!
```

### GitHub Release Management

**Creating Release on GitHub**:
```bash
# Using GitHub CLI (recommended)
gh release create v2.2.0 \
  --title "v2.2.0: DSPy Integration" \
  --notes-file RELEASE_NOTES.md \
  ./target/release/mnemosyne#mnemosyne-v2.2.0-$(uname -s)-$(uname -m)

# List releases
gh release list

# View specific release
gh release view v2.2.0

# Edit release notes
gh release edit v2.2.0 --notes "Updated notes"

# Delete release (if needed)
gh release delete v2.2.0 --yes
git push --delete origin v2.2.0  # Also delete tag
```

**Release Assets**:
- Always include compiled binary for common platforms
- Include checksums (SHA256)
- Include installation script (if applicable)
- Include CHANGELOG excerpt in release notes

### Rolling Back a Release

**If critical issue discovered after release**:

```bash
# 1. DO NOT delete the release/tag (preserve history)

# 2. Create hotfix immediately (see Hotfix Process above)

# 3. Mark problematic release as superseded
gh release edit v2.2.0 --notes "⚠️ **SUPERSEDED**: This release has been superseded by v2.2.1 due to critical bug. Please upgrade immediately.

Original release notes below:
---
..."

# 4. Announce on all channels
# - Update README.md to point to new version
# - Post in discussions/announcements
# - Update documentation site

# 5. Monitor adoption of fix
```

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
