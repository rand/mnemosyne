# Mnemosyne

**High-performance agentic memory system for Claude Code's multi-agent orchestration**

Mnemosyne provides persistent semantic memory with sub-millisecond retrieval, built in Rust with LibSQL vector search and PyO3 Python bindings.

---

## Features

### Core Memory System
- **Project-Aware**: Automatic namespace detection from git repositories and CLAUDE.md
- **Semantic Search**: LibSQL vector embeddings + full-text search (FTS5) + graph connectivity
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
- **Consolidation**: Detect and merge duplicate/similar memories with LLM-assisted analysis
- **Importance Scoring**: Graph-based importance recalibration
- **Link Decay**: Time-based link strength management
- **Archival**: Automatic cleanup of low-value memories
- **Supersede**: Track memory replacements with audit trail

### Evaluation System *(Privacy-Preserving)*
- **Feedback Collection**: Implicit signals (access, edit, commit) with privacy-preserving task hashing
- **Feature Extraction**: 13 privacy-preserving features (keyword overlap, semantic similarity, recency, etc.)
- **Online Learning**: Hierarchical weight adaptation (session → project → global)
- **Relevance Scoring**: Context-aware ranking with learned weights

### Interactive Collaborative Space (ICS)
- **CRDT Editing**: Automerge-based collaborative text editor
- **Syntax Highlighting**: Tree-sitter based highlighting for 8+ languages (Rust, Python, TypeScript, Go, C/C++, Shell, Markdown, JSON/TOML/YAML)
- **Semantic Highlighting**: Type information, scopes, error overlays
- **Vim Mode**: Full vi/vim keybindings with modal editing
- **Panels**: Memory browser, agent status, attribution, diagnostics
- **Semantic Analysis**: Real-time triple extraction, typed hole detection, dependency graphs
- **Undo/Redo**: Transaction-based history with Automerge

---

## Quick Start

### Installation

**Automated Installation** (Recommended):
```bash
# Clone repository
git clone https://github.com/yourusername/mnemosyne.git
cd mnemosyne

# Run installation script
./scripts/install/install.sh

# Installation will:
# - Build release binary
# - Install to ~/.local/bin
# - Initialize database
# - Configure MCP server
# - Optionally set up API keys
```

**Manual Installation**:
```bash
# Prerequisites: Rust 1.75+, Python 3.10-3.14, uv
cargo build --release

# Copy binary to PATH
cp target/release/mnemosyne ~/.local/bin/

# Initialize database
mnemosyne init

# Configure secrets (optional for LLM enrichment)
mnemosyne secrets set --provider anthropic --key sk-ant-...
```

**Uninstallation**:
```bash
# Remove binary and MCP config (preserves data)
./scripts/install/uninstall.sh

# Remove everything including data
./scripts/install/uninstall.sh --purge
```

### Basic Usage

**Store memories**:
```bash
# Store with automatic namespace detection
mnemosyne remember --content "User prefers concise code reviews" --importance 8

# Store with explicit namespace
mnemosyne remember "Database uses LibSQL with vector search" \
  --namespace "project:mnemosyne" \
  --type architecture \
  --importance 9
```

**Search memories**:
```bash
# Semantic search
mnemosyne recall --query "code review preferences"

# Search with namespace filter
mnemosyne recall "database" --namespace "project:mnemosyne"

# Limit results
mnemosyne recall "architecture decisions" --limit 5
```

**Evolution operations**:
```bash
# Consolidate duplicate memories
mnemosyne evolve consolidate

# Recalibrate importance scores
mnemosyne evolve importance

# Archive old/low-value memories
mnemosyne evolve archive
```

**Interactive Collaborative Space**:
```bash
# Open file in ICS with syntax highlighting and vim mode
mnemosyne ics path/to/file.rs

# Collaborative editing with CRDT sync
mnemosyne ics --collaborate path/to/shared/document.md
```

**Orchestration** (Python agents):
```bash
# Run orchestration workflow
mnemosyne orchestrate --session-id dev-001 --work-items plan.json
```

---

## Architecture

### Storage Layer
- **LibSQL**: SQLite-compatible with native vector search (sqlite-vec)
- **Embeddings**:
  - Local: fastembed (nomic-embed-text-v1.5, 768d)
  - Remote: Voyage AI (voyage-3-large, 1536d)
- **Search Config**: Hybrid scoring (semantic 70%, FTS 20%, graph 10%)
- **Performance**: 2.25ms avg operations, 0.88ms list, 1.61ms search
- **Read-Only Support**: Auto-detects and handles read-only databases gracefully

### Multi-Agent System
```
┌─────────────────────────────────────────────────────┐
│           Multi-Agent Orchestration                  │
│                                                      │
│  ┌──────────────┐    ┌──────────────┐              │
│  │ Orchestrator │◄──►│  Optimizer   │              │
│  │  (Ractor)    │    │  (Ractor)    │              │
│  └──────┬───────┘    └──────┬───────┘              │
│         │                   │                        │
│         │              Skill Discovery               │
│         ▼                   ▼                        │
│  ┌──────────────┐    ┌──────────────┐              │
│  │   Executor   │◄──►│   Reviewer   │              │
│  │  (Ractor)    │    │  (Ractor)    │              │
│  │  + Sub-agents│    │ Quality Gates│              │
│  └──────────────┘    └──────────────┘              │
└─────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────┐
│       Storage + Evolution + Evaluation               │
│                                                      │
│  LibSQL  ◄──►  Consolidation  ◄──►  Evaluation     │
│  Vector       (Deduplication)     (Learning Weights)│
└─────────────────────────────────────────────────────┘
```

**Actor Responsibilities**:
- **Orchestrator**: Work queue, deadlock detection/resolution, phase transitions
- **Optimizer**: Context management, dynamic skill discovery, memory loading
- **Reviewer**: Quality gates, test verification, anti-pattern detection
- **Executor**: Work execution, sub-agent spawning for parallel work

---

## CLI Reference

### Memory Operations
```bash
# Store memory
mnemosyne remember [OPTIONS] <CONTENT>
  --namespace <NS>      Namespace (auto-detected from git/CLAUDE.md)
  --importance <1-10>   Importance score (default: 5)
  --type <TYPE>         Memory type (insight|architecture|decision|task|reference)
  --tags <TAGS>         Comma-separated tags
  --links <IDS>         Link to existing memory IDs

# Search memories
mnemosyne recall [OPTIONS] <QUERY>
  --namespace <NS>      Filter by namespace
  --limit <N>           Max results (default: 10)
  --min-importance <N>  Minimum importance score

# Generate embeddings
mnemosyne embed <TEXT>
  --model <MODEL>       Embedding model (local|remote)
```

### Evolution
```bash
# Run evolution jobs
mnemosyne evolve <OPERATION>
  consolidate           Detect and merge duplicate memories
  importance            Recalibrate importance scores
  archive               Archive low-value memories
  links                 Update link decay scores
```

### Orchestration
```bash
# Run orchestration workflow
mnemosyne orchestrate [OPTIONS]
  --session-id <ID>     Session identifier
  --work-items <FILE>   Work items JSON file
```

### ICS (Interactive Collaborative Space)
```bash
# Open ICS editor
mnemosyne ics [OPTIONS] [FILE]
  --collaborate         Enable CRDT collaborative mode
  --vim-mode            Enable vim keybindings (default: on)
  --theme <THEME>       Color theme (dark|light)
```

### Configuration
```bash
# Initialize database
mnemosyne init [PATH]

# Manage secrets
mnemosyne secrets set --provider <PROVIDER> --key <KEY>
mnemosyne secrets list

# Database info
mnemosyne info
```

---

## Configuration

### Environment Variables
```bash
# Database
export DATABASE_URL="sqlite:///path/to/mnemosyne.db"

# API Keys (for LLM enrichment)
export ANTHROPIC_API_KEY="sk-ant-..."
export VOYAGE_API_KEY="pa-..."   # For remote embeddings

# Logging
export RUST_LOG="info"           # debug|info|warn|error
```

### Search Configuration
```rust
SearchConfig {
    semantic_weight: 0.7,  // 70% semantic similarity
    fts_weight: 0.2,       // 20% keyword match
    graph_weight: 0.1,     // 10% link connectivity
}
```

### Connection Modes
```rust
ConnectionMode::Local(path)              // Local SQLite file
ConnectionMode::LocalReadOnly(path)      // Read-only database
ConnectionMode::Remote { url, token }    // Remote LibSQL/Turso
ConnectionMode::EmbeddedReplica { ... }  // Local replica with sync
```

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

### Development
- [CHANGELOG.md](CHANGELOG.md) - Version history
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md) - Common issues and solutions
- [TODO_TRACKING.md](TODO_TRACKING.md) - Development progress tracking

---

## Testing

```bash
# Unit tests
cargo test --lib

# Integration tests
cargo test --test integration_ics --features test-utils

# E2E tests
bash tests/e2e/human_workflow_1_new_project.sh
bash tests/e2e/agentic_workflow_1_orchestrator.sh
bash tests/e2e/recovery_1_graceful_degradation.sh

# All E2E tests
find tests/e2e -name '*.sh' -executable -exec {} \;

# With coverage
cargo tarpaulin --lib --out Html
```

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

## Contributing

1. Follow Work Plan Protocol (Phases 1-4: Prompt → Spec → Plan → Artifacts)
2. Use Beads for task tracking: `bd import -i .beads/issues.jsonl`
3. Quality gates: Tests pass, no anti-patterns, constraints maintained
4. Commit before testing (never test uncommitted code)
5. Run `cargo clippy` and `cargo fmt` before PRs

**Development Workflow**:
```bash
# Setup
git checkout -b feature/my-feature
bd import -i .beads/issues.jsonl

# Development cycle
cargo build --lib
cargo test --lib
cargo clippy

# E2E testing
bash tests/e2e/relevant_test.sh

# Commit
git add . && git commit -m "Descriptive message"

# Before PR
cargo fmt
cargo clippy --all-targets
cargo test --all

# Export Beads state
bd export -o .beads/issues.jsonl
```

---

## License

See LICENSE file for details.

---

## Status

**Current Version**: 2.0.0

**Completed**:
- ✅ Core storage and memory system with LibSQL vector search
- ✅ Multi-agent orchestration (Ractor-based 4-agent system)
- ✅ Evolution system (consolidation, importance, archival)
- ✅ Evaluation system (privacy-preserving online learning)
- ✅ ICS (Interactive Collaborative Space with CRDT, syntax highlighting, vim mode)
- ✅ CLI commands (remember, recall, evolve, orchestrate, ics)
- ✅ Installation/uninstallation scripts
- ✅ Read-only database support
- ✅ E2E test suite (17 scenarios covering human/agentic workflows)
- ✅ PyO3 bindings for Python orchestration agents
- ✅ MCP server integration

**In Progress** (v2.0 production readiness):
- 🔄 ICS syntax highlighting expansion (8+ languages)
- 🔄 ICS vim mode completion (advanced text objects, motions)
- 🔄 Evaluation system completion (database integration for all features)
- 🔄 Evolution link tracking completion
- 🔄 Test coverage expansion (target: 70%+)
- 🔄 E2E test expansion (30+ scenarios)

**Roadmap** (post-v2.0):
- ⏳ Advanced observability and metrics
- ⏳ Dynamic agent scaling
- ⏳ Distributed orchestration
- ⏳ WebAssembly deployment target

---

For detailed technical documentation, see [ARCHITECTURE.md](ARCHITECTURE.md).
For troubleshooting, see [TROUBLESHOOTING.md](TROUBLESHOOTING.md).
For MCP server integration, see [MCP_SERVER.md](MCP_SERVER.md).
For development progress, see [TODO_TRACKING.md](TODO_TRACKING.md).
