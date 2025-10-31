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
- **LLM-Enhanced Reviewer**: Automatic requirement extraction, semantic validation, intent verification with Claude API
- **Work Queue**: Dependency-aware scheduling with priority management
- **Quality Gates**: Automated test verification, anti-pattern detection, constraint validation, requirement traceability
- **Deadlock Resolution**: Priority-based preemption (60s timeout)
- **Sub-Agent Spawning**: Parallel work execution across child actors
- **Event Persistence**: Complete audit trail of orchestration events with SSE broadcasting

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
- **Syntax Highlighting**: Tree-sitter 0.23 based highlighting for 13 languages (Rust, Python, Go, TypeScript, JavaScript, JSON, TOML, YAML, Markdown, Bash, C, C++, Zig)
- **Semantic Highlighting (3-Tier System)**:
  - **Tier 1: Structural** (<5ms real-time) - XML tags, RFC 2119 constraints, modality/hedging, ambiguity detection, domain patterns
  - **Tier 2: Relational** (<200ms incremental) - Named entities, relationships, semantic roles, coreference resolution, anaphora
  - **Tier 3: Analytical** (2s+ background, optional) - Discourse analysis, contradiction detection, pragmatics, LLM-powered
- **ICS Patterns**: `#file`, `@symbol`, `?hole` with color-coded highlighting
- **Hybrid Highlighting**: Combines tree-sitter syntax with semantic pattern detection (3-layer priority system)
- **Vim Mode**: Complete vi/vim keybindings with modal editing (14 movement commands: w/b/e, f/F/t/T, PageUp/Down, gg/G)
- **Panels**: Memory browser, agent status, attribution, diagnostics
- **Semantic Analysis**: Real-time triple extraction, typed hole detection, dependency graphs
- **Undo/Redo**: Transaction-based history with Automerge

### Composable Tools Architecture
- **mnemosyne-ics**: Standalone context editor binary with full terminal ownership (no conflicts)
- **mnemosyne-dash**: Real-time monitoring dashboard with SSE event streaming
- **HTTP API Server** (`:3000`): Optional REST API with `mnemosyne serve --with-api`
- **Unix Philosophy**: Each tool owns its terminal completely, zero conflicts
- **File-Based Handoff**: Context exchange via `.claude/*.md` files
- **Event Streaming**: Real-time coordination via SSE for monitoring

### TUI Wrapper Mode (Deprecated in v2.1.0)
⚠️ **Deprecated**: Use `mnemosyne-ics` + `mnemosyne-dash` instead. See [docs/MIGRATION.md](docs/MIGRATION.md).

- **Command Palette**: Helix-style fuzzy command selector with type-ahead filtering
- **Context-Aware Help**: Modal help overlay (?) with mode-specific shortcuts
- **Status Bar**: Dynamic action hints based on current mode (ICS/Chat)
- **Layout**: Split view with chat, ICS editor, and agent dashboard
- **Keyboard-First**: Complete keyboard navigation with discoverable shortcuts

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

**Interactive Collaborative Space (Standalone)**:
```bash
# Launch standalone ICS context editor
mnemosyne-ics

# Create from template
mnemosyne-ics --template feature

# Open existing file
mnemosyne-ics path/to/context.md

# Read-only mode (view memory dumps)
mnemosyne-ics --read-only path/to/dump.md

# Features:
# - Full terminal ownership (no conflicts)
# - Template system (api, architecture, bugfix, feature, refactor)
# - Storage backend integration
# - Semantic highlighting (3-tier system)
# - Vim mode with modal editing
```

**Real-time Monitoring Dashboard**:
```bash
# Start API server with SSE event streaming
mnemosyne serve --with-api

# In another terminal, launch monitoring dashboard
mnemosyne-dash

# Features:
# - Live agent activity display
# - Color-coded agent states
# - System statistics (memory, CPU, context usage)
# - Event log with scrollback and filtering
# - Auto-reconnect on disconnect
```

**TUI Wrapper Mode** (Deprecated in v2.1.0):
```bash
⚠️ Deprecated: Use mnemosyne-ics + mnemosyne-dash instead
See docs/MIGRATION.md for migration guide

# Launch TUI with command palette, ICS editor, and agent dashboard
mnemosyne tui

# Start with ICS panel visible
mnemosyne tui --with-ics

# Features:
# - Helix-style command palette (Ctrl+P)
# - ICS editor with markdown highlighting (Ctrl+E)
# - Real-time agent dashboard (Ctrl+D)
# - Context-aware help overlay (?)
# - Pattern highlighting: #file.rs @symbol ?hole
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

### ICS (Integrated Context Studio) - Standalone Binary
```bash
# Launch standalone ICS context editor
mnemosyne-ics [OPTIONS] [FILE]
  --template <TEMPLATE>  Use template (api|architecture|bugfix|feature|refactor)
  --read-only            Open in read-only mode
  --vim-mode             Enable vim keybindings (default: on)
  --theme <THEME>        Color theme (dark|light)

# Features:
# • Full terminal ownership (no conflicts with Claude Code)
# • Template system for common contexts
# • 3-tier semantic highlighting (<5ms→<200ms→2s+)
# • Storage backend integration
# • Vim modal editing
# • Pattern syntax: #file.rs @symbol ?hole
```

### Monitoring Dashboard - Standalone Binary
```bash
# Launch real-time monitoring dashboard
mnemosyne-dash [OPTIONS]
  --api-url <URL>       API server URL (default: http://localhost:3000)
  --refresh-rate <MS>   Update interval (default: 100ms)

# Prerequisites:
# Start API server first: mnemosyne serve --with-api

# Features:
# • Live agent activity via SSE
# • Color-coded agent states
# • System statistics (memory, CPU, context)
# • Event log with scrollback
# • Auto-reconnect on disconnect
```

### API Server
```bash
# Start MCP server with HTTP API
mnemosyne serve --with-api [OPTIONS]
  --api-port <PORT>     API server port (default: 3000)
  --cors-origins <URL>  CORS allowed origins

# Endpoints:
# GET  /api/agents          List agent states
# GET  /api/context         Current context state
# GET  /api/events/stream   SSE event stream (real-time)

# Features:
# • REST API with Axum
# • Server-Sent Events (SSE) for real-time updates
# • CORS support for web clients
# • Concurrent with MCP server (tokio::select!)
```

### TUI (Terminal User Interface) - Deprecated
```bash
⚠️ Deprecated in v2.1.0: Use mnemosyne-ics + mnemosyne-dash instead
See docs/MIGRATION.md for migration guide

# Launch enhanced TUI wrapper mode
mnemosyne tui [OPTIONS]
  --with-ics            Start with ICS panel visible
  --no-dashboard        Disable agent dashboard

# TUI Features:
# • Command Palette (Ctrl+P): Helix-style fuzzy command selector
# • ICS Editor (Ctrl+E): Integrated Context Studio with highlighting
# • Agent Dashboard (Ctrl+D): Real-time agent status and work queue
# • Help Overlay (?): Context-aware keyboard shortcuts
# • Status Bar: Dynamic action hints based on current mode

# Keyboard Shortcuts:
# General Navigation:
#   Ctrl+P          Open command palette
#   Ctrl+E          Toggle ICS panel
#   Ctrl+D          Toggle dashboard
#   Ctrl+Q          Quit application
#   ?               Show help overlay

# ICS Mode:
#   Ctrl+Enter      Submit refined context to Claude
#   Ctrl+S          Save edited document
#   Pattern syntax:
#     #file.rs      File reference (blue, bold)
#     @symbol       Symbol reference (green, bold)
#     ?interface    Typed hole (yellow, bold)
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
- [docs/ICS_README.md](docs/ICS_README.md) - Integrated Context Studio

### v2.1.0 Features
- [docs/MIGRATION.md](docs/MIGRATION.md) - Migration from TUI to composable tools (475 lines)
- [docs/guides/llm-reviewer.md](docs/guides/llm-reviewer.md) - LLM reviewer system (533 lines)
- [docs/guides/llm-reviewer-setup.md](docs/guides/llm-reviewer-setup.md) - Setup and troubleshooting (448 lines)
- [SEMANTIC_HIGHLIGHTING.md](SEMANTIC_HIGHLIGHTING.md) - System overview and API reference (423 lines)
- [SEMANTIC_HIGHLIGHTING_INTEGRATION.md](SEMANTIC_HIGHLIGHTING_INTEGRATION.md) - Integration guide (514 lines)
- [SEMANTIC_HIGHLIGHTING_STATUS.md](SEMANTIC_HIGHLIGHTING_STATUS.md) - Implementation status (169 lines)

### Specifications (v2.1.0)
- [docs/background-processing-spec.md](docs/background-processing-spec.md) - Tier 3 background processing (580 lines)
- [docs/ics-integration-spec.md](docs/ics-integration-spec.md) - ICS integration specification (557 lines)
- [docs/incremental-analysis-spec.md](docs/incremental-analysis-spec.md) - Incremental semantic analysis (533 lines)
- [docs/semantic-highlighter-test-plan.md](docs/semantic-highlighter-test-plan.md) - Testing strategy (716 lines)
- [docs/tier3-llm-integration-spec.md](docs/tier3-llm-integration-spec.md) - LLM integration architecture (421 lines)

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

**Current Version**: 2.1.0

**Completed (v2.1.0)**:
- ✅ Core storage and memory system with LibSQL vector search
- ✅ Multi-agent orchestration (Ractor-based 4-agent system)
- ✅ **LLM-Enhanced Reviewer** with requirement extraction and semantic validation
- ✅ Evolution system (consolidation, importance, archival)
- ✅ Evaluation system (privacy-preserving online learning)
- ✅ **ICS Standalone Binary** (`mnemosyne-ics`) with template system
- ✅ **3-Tier Semantic Highlighting** (Structural/Relational/Analytical, 7,500+ lines)
- ✅ **HTTP API Server** (`:3000`) with SSE event streaming
- ✅ **Real-time Monitoring Dashboard** (`mnemosyne-dash`)
- ✅ **Composable Tools Architecture** (Unix philosophy, zero conflicts)
- ✅ **Event Bridging** (orchestration events → SSE → dashboard)
- ✅ TUI wrapper mode (deprecated, use composable tools)
- ✅ CLI commands (remember, recall, evolve, orchestrate, ics, tui)
- ✅ Installation/uninstallation scripts
- ✅ Read-only database support
- ✅ **627 tests passing** (up from 474, +153 new tests)
- ✅ PyO3 bindings for Python orchestration agents
- ✅ MCP server integration
- ✅ **11 new documentation files** (5,000+ lines)

**Known Issues (v2.1.0)**:
- ⚠️ PyO3 0.22.6 doesn't support Python 3.14+ (use Python 3.9-3.13)
- ⚠️ 27 clippy warnings remaining (style/quality, not functional bugs)
- ⚠️ Tier 3 LLM integration is scaffolding only (not fully functional)

**In Progress** (v2.2):
- 🔄 Tier 3 LLM integration completion
- 🔄 Incremental semantic analysis scheduling
- 🔄 ICS-semantic highlighter integration
- 🔄 Clippy warning cleanup
- 🔄 Test coverage expansion (target: 80%+)

**Roadmap** (post-v2.1):
- ⏳ Performance benchmarks for semantic highlighting
- ⏳ Advanced observability and metrics
- ⏳ Dynamic agent scaling
- ⏳ Distributed orchestration
- ⏳ WebAssembly deployment target

---

For detailed technical documentation, see [ARCHITECTURE.md](ARCHITECTURE.md).
For troubleshooting, see [TROUBLESHOOTING.md](TROUBLESHOOTING.md).
For MCP server integration, see [MCP_SERVER.md](MCP_SERVER.md).
For development progress, see [TODO_TRACKING.md](TODO_TRACKING.md).
