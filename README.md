# Mnemosyne

**High-performance agentic memory system for Claude Code's multi-agent orchestration**

Mnemosyne provides persistent semantic memory with sub-millisecond retrieval, built in Rust with LibSQL vector search and PyO3 Python bindings.

---

## Features

### Core Memory System
- **Project-Aware**: Automatic context detection from git repositories and CLAUDE.md
- **Semantic Search**: LibSQL vector embeddings + full-text search (FTS5)
- **Type System**: Insight, Architecture, Decision, Task, Reference memory types
- **Graph Linking**: Automatic bidirectional relationship management
- **Privacy-First**: Local-only storage with optional privacy-preserving evaluation

### Multi-Agent Orchestration
- **Ractor Actors**: 4 specialized agents (Orchestrator, Optimizer, Reviewer, Executor)
- **Work Queue**: Dependency-aware scheduling with priority management
- **Quality Gates**: Automated test verification, anti-pattern detection, constraint validation
- **Deadlock Resolution**: Priority-based preemption (60s timeout)
- **Sub-Agent Spawning**: Parallel work execution across child actors
- **Event Persistence**: Complete audit trail of orchestration events

### Evolution System
- **Consolidation**: Detect and merge duplicate/similar memories
- **Importance Scoring**: Graph-based importance recalibration
- **Archival**: Automatic cleanup of low-value memories
- **Supersede**: Track memory replacements with audit trail

### Branch Coordination
- **Git Integration**: Git-wrapper with agent identity tracking
- **File Tracking**: Conflict detection for overlapping write intents
- **Isolation Modes**: Isolated vs. coordinated branch work
- **Cross-Process**: Shared state via file-based coordination

### Interactive Collaborative Space (ICS)
- **CRDT Editing**: Automerge-based collaborative text editor
- **Undo/Redo**: Transaction-based history with Automerge
- **Panels**: Memory browser, agent status, attribution, proposals
- **Vim Mode**: Full vi/vim keybindings with modal editing

---

## Quick Start

### Prerequisites
- **Rust 1.75+** (core library)
- **Python 3.10-3.14** (orchestration agents)
- **uv** package manager

### Build
```bash
# Install dependencies
uv venv .venv && source .venv/bin/activate
uv sync

# Build Rust library
cargo build --release

# Build PyO3 bindings
maturin develop --release

# Run tests
cargo test --lib
```

### Run MCP Server
```bash
# Start memory server
cargo run --bin mnemosyne-server

# Or with debug logging
RUST_LOG=debug cargo run --bin mnemosyne-server
```

### Run ICS (Interactive Collaborative Space)
```bash
cargo run --bin ics
```

---

## Architecture

### Storage Layer
- **LibSQL**: SQLite-compatible with native vector search
- **Embedding**: fastembed (BGE-small-en-v1.5, 384d)
- **Search Config**: Hybrid (semantic 70%, FTS 20%, graph 10%)
- **Performance**: 2.25ms avg operations, 0.88ms list, 1.61ms search

### Multi-Agent System
```
┌─────────────────────────────────────────────────────┐
│           Multi-Agent Orchestration                  │
│                                                      │
│  ┌──────────────┐    ┌──────────────┐              │
│  │ Orchestrator │◄──►│  Optimizer   │              │
│  │  (Ractor)    │    │  (Ractor)    │              │
│  └──────┬───────┘    └──────────────┘              │
│         │                                            │
│         ▼                                            │
│  ┌──────────────┐    ┌──────────────┐              │
│  │   Executor   │◄──►│   Reviewer   │              │
│  │  (Ractor)    │    │  (Ractor)    │              │
│  │  + Sub-agents│    │ Quality Gates│              │
│  └──────────────┘    └──────────────┘              │
└─────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────┐
│              Storage + Evolution                     │
│                                                      │
│  LibSQL  ◄──►  Consolidation  ◄──►  Importance     │
│  Vector       (Deduplication)      (Graph-based)    │
└─────────────────────────────────────────────────────┘
```

**Actor Responsibilities**:
- **Orchestrator**: Work queue, deadlock detection/resolution, phase transitions
- **Optimizer**: Context management, skill discovery, memory loading
- **Reviewer**: Quality gates, test verification, anti-pattern detection
- **Executor**: Work execution, sub-agent spawning for parallel work

---

## Documentation

### Core System
- [ARCHITECTURE.md](ARCHITECTURE.md) - System architecture and design decisions
- [ORCHESTRATION.md](ORCHESTRATION.md) - Multi-agent coordination guide
- [MCP_SERVER.md](MCP_SERVER.md) - MCP protocol integration

### Features
- [docs/EVOLUTION.md](docs/EVOLUTION.md) - Memory evolution system
- [docs/VECTOR_SEARCH.md](docs/VECTOR_SEARCH.md) - Semantic search implementation
- [docs/PRIVACY.md](docs/PRIVACY.md) - Privacy-preserving evaluation
- [docs/ICS_README.md](docs/ICS_README.md) - Interactive Collaborative Space

### Branch Coordination
- [docs/BRANCH_ISOLATION.md](docs/BRANCH_ISOLATION.md) - Branch isolation design
- [docs/COORDINATION_WORKFLOWS.md](docs/COORDINATION_WORKFLOWS.md) - Multi-agent workflows
- [docs/BRANCH_ISOLATION_TROUBLESHOOTING.md](docs/BRANCH_ISOLATION_TROUBLESHOOTING.md) - Troubleshooting guide

### Development
- [CHANGELOG.md](CHANGELOG.md) - Version history
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md) - Common issues and solutions
- [ARCHITECTURE_OVERVIEW.md](ARCHITECTURE_OVERVIEW.md) - High-level overview

---

## Phase 5.1: Latest Features

### Orchestrator
- **Deadlock Resolution**: Timeout-based detection (60s) with priority-based preemption
- Sorts deadlocked items by priority, preempts lower 50%
- Resets preempted items to Ready state for retry
- Persists DeadlockDetected and DeadlockResolved events

### Executor
- **Sub-Agent Spawning**: Parallel work execution via child actors
- Spawns ExecutorActor children with ractor Actor::spawn()
- Registers orchestrator reference for completion reporting
- Tracks max 4 concurrent sub-agents, falls back to inline at capacity
- Added RegisterOrchestrator message for child initialization

### Reviewer
- **Test Verification**: Checks work success and scans memories for test failures
- **Anti-Pattern Detection**: Flags TODO/FIXME/STUB/MOCK/PLACEHOLDER markers
- **Constraint Verification**: Validates memory structure, importance (1-10), confidence (0.0-1.0)
- All three gates must pass for work approval

### Evolution
- **Supersede Operation**: mark_superseded() archives old memory, links to replacement
- **Link Counting**: count_incoming_links() for accurate importance scoring
- Updates audit_log with supersede events

### Branch Coordination
- **CLI Integration**: status, join, conflicts, switch, release commands
- **Status Line**: Real-time conflict count display
- **Registry Access**: Active branches and assignments exposed via coordinator

---

## Performance

**Storage Operations** (PyO3 vs subprocess):
- Store: 2.25ms avg (was 20-50ms) - **10-20x faster**
- List: 0.88ms avg (<1ms target) - **22-56x faster**
- Search: 1.61ms avg (was 30-60ms) - **18-37x faster**

**Memory**:
- Rust memory management (no GC pauses)
- Zero-copy data passing for agent messages
- Efficient vector storage with LibSQL

**Scalability**:
- Sub-agent spawning for parallel work
- Deadlock prevention via dependency-aware scheduling
- Context preservation at 75% utilization threshold

---

## Testing

```bash
# Unit tests
cargo test --lib

# Integration tests (ICS)
cargo test --test integration_ics --features test-utils

# Evolution tests
cargo test --package mnemosyne --lib evolution::

# Orchestration tests
cargo test --package mnemosyne --lib orchestration::actors::
```

---

## Configuration

### Storage
```rust
ConnectionMode::Local(path)        // Local SQLite file
ConnectionMode::LibsqlRemote(url)  // Remote LibSQL/Turso
```

### Search
```rust
SearchConfig {
    semantic_weight: 0.7,  // 70% semantic similarity
    fts_weight: 0.2,       // 20% keyword match
    graph_weight: 0.1,     // 10% link connectivity
}
```

### Evolution
```bash
# Run consolidation
cargo run --bin mnemosyne-cli -- evolution consolidate

# Recalibrate importance
cargo run --bin mnemosyne-cli -- evolution importance

# Archive old memories
cargo run --bin mnemosyne-cli -- evolution archive
```

---

## Contributing

1. Follow Work Plan Protocol (Phases 1-4: Prompt → Spec → Plan → Artifacts)
2. Use Beads for task tracking: `bd import -i .beads/issues.jsonl`
3. Quality gates: Tests pass, no anti-patterns, constraints maintained
4. Commit before testing (never test uncommitted code)
5. Run `cargo fix` and `cargo clippy` before PRs

---

## License

See LICENSE file for details.

---

## Status

**Current Version**: 2.0.0 (in development)

**Completed**:
- ✅ Phase 1: Core storage and memory system
- ✅ Phase 2: ICS (Interactive Collaborative Space)
- ✅ Phase 3: Branch coordination and conflict detection
- ✅ Phase 5.1: Orchestration enhancements (deadlock, sub-agents, quality gates)
- ✅ Phase 5.2: Code quality (warnings, clippy)

**In Progress**:
- 🔄 Phase 6: Documentation updates
- ⏳ Phase 7: Final validation and testing

**Deferred** (post-MVP):
- Phase 4: Dynamic agent scaling and advanced observability

---

For detailed technical documentation, see [ARCHITECTURE.md](ARCHITECTURE.md).
For troubleshooting, see [TROUBLESHOOTING.md](TROUBLESHOOTING.md).
For MCP server integration, see [MCP_SERVER.md](MCP_SERVER.md).
