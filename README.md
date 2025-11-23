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

### Distributed Coordination
- **Peer Discovery**: Automatic peer discovery on local network via `mnemosyne peer invite/join`
- **Work Delegation**: Seamless offloading of tasks to available peers
- **Iroh Networking**: P2P encrypted communication layer for secure direct connections

### Network Visualization
- **Network Graph**: Visual representation of connected peers and topology via `mnemosyne graph`
- **Real-time Status**: Connection latency, bandwidth, and peer health monitoring
- **Topology Awareness**: Automatic detection of network partitions and routing paths

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
**Integrated context editor accessible via `mnemosyne edit` or `/ics` slash command**

- **CRDT Editing**: Automerge-based collaborative text editor
- **Template System**: 5 built-in templates (API, Architecture, Bugfix, Feature, Refactor)
- **Panels**: Memory browser, diagnostics, proposals, typed holes
- **Syntax Highlighting**: Tree-sitter 0.23 based highlighting for 13 languages (Rust, Python, Go, TypeScript, JavaScript, JSON, TOML, YAML, Markdown, Bash, C, C++, Zig)
- **Semantic Highlighting (3-Tier System)**:
  - **Tier 1: Structural** (<5ms real-time) - XML tags, RFC 2119 constraints, modality/hedging, ambiguity detection, domain patterns
  - **Tier 2: Relational** (<200ms incremental) - Named entities, relationships, semantic roles, coreference resolution, anaphora
  - **Tier 3: Analytical** (2s+ background, optional) - Discourse analysis, contradiction detection, pragmatics, LLM-powered
- **ICS Patterns**: `#file`, `@symbol`, `?hole` with color-coded highlighting
- **Hybrid Highlighting**: Combines tree-sitter syntax with semantic pattern detection (3-layer priority system)
- **Vim Mode**: Complete vi/vim keybindings with modal editing (14 movement commands: w/b/e, f/F/t/T, PageUp/Down, gg/G)
- **Semantic Analysis**: Real-time triple extraction, typed hole detection, dependency graphs
- **Undo/Redo**: Transaction-based history with Automerge
- **Claude Code Integration**: Seamless handoff via file-based coordination protocol

**Usage**:
```bash
# From Claude Code session
/ics context.md
/ics --template feature new-feature.md
/ics --panel memory --template api auth.md

# Command-line
mnemosyne edit context.md
mnemosyne edit --template architecture decision.md
mnemosyne ics --readonly --panel diagnostics review.md
```

See [docs/guides/ICS_INTEGRATION.md](docs/guides/ICS_INTEGRATION.md) for complete guide.

### Dashboard & Monitoring
- **mnemosyne-dash**: Real-time monitoring dashboard with clean 4-panel layout (redesigned from "static wall of garbage")
- **Panels**: System Overview (health metrics), Activity Stream (filtered event log), Agent Details (per-agent status), Operations (CLI command history)
- **Smart Filtering**: Intelligent noise reduction (heartbeats hidden by default), 8 event categories, compound filter logic
- **Event Correlation**: Links start→complete events with duration tracking, automatic slow operation detection
- **Real-time Updates**: Server-Sent Events (SSE) streaming from API server with zero-latency event delivery
- **Interactive Controls**: Full keyboard navigation (panel toggles, clear history, focus modes)
- **HTTP API Server** (`:3000`): Automatic REST API with owner/client mode for multiple instances
- **Event Streaming**: Real-time coordination via SSE for monitoring and cross-instance event forwarding
- **Production Quality**: 124+ tests, 6,100+ lines of code, comprehensive error handling

See [docs/DASHBOARD.md](docs/DASHBOARD.md) for complete documentation.

### gRPC Remote Access (RPC Server)
**Production-ready gRPC server for remote access to mnemosyne's memory system**

- **Full CRUD Operations**: Store, retrieve, update, delete memories via gRPC
- **Advanced Search**: Semantic search (vector embeddings), graph traversal, hybrid recall
- **Streaming APIs**: Progressive results for large datasets, progress tracking for slow operations
- **Type-Safe Protocol**: Protocol Buffers ensure schema validation and backward compatibility
- **Multi-Language Support**: Client libraries for Python, Rust, Go, and any gRPC-compatible language
- **Production Features**: Comprehensive error handling, input validation, rate limiting

**Usage**:
```bash
# Start RPC server on default port (50051)
mnemosyne-rpc

# Custom configuration
mnemosyne-rpc --host 0.0.0.0 --port 9090 --enable-llm

# With custom database
mnemosyne-rpc --db-path /path/to/mnemosyne.db
```

**Client Example (Python)**:
```python
import grpc
from mnemosyne.v1 import memory_pb2, memory_pb2_grpc

# Connect and store a memory
channel = grpc.insecure_channel('localhost:50051')
stub = memory_pb2_grpc.MemoryServiceStub(channel)

response = stub.StoreMemory(memory_pb2.StoreMemoryRequest(
    content="Important architectural decision",
    namespace=memory_pb2.Namespace(
        project=memory_pb2.ProjectNamespace(name="my-project")
    ),
    importance=9,
    tags=["architecture", "decision"]
))
print(f"Stored memory: {response.memory_id}")
```

See [src/rpc/README.md](src/rpc/README.md) for complete API documentation, deployment guides, and client examples.

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
# - Detect and optionally install Nerd Fonts for icon support
```

**Icon System**: Mnemosyne uses Nerd Font icons (Font Awesome) for a polished CLI experience with automatic fallback to ASCII. For best results, install [JetBrainsMono Nerd Font](https://www.nerdfonts.com/). See [docs/ICONS.md](docs/ICONS.md) for details.

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
# API server starts automatically with first MCP instance (owner mode)
# Launch monitoring dashboard (connects to http://localhost:3000 by default)
mnemosyne-dash

# Custom configuration
mnemosyne-dash --api http://localhost:3000 --refresh 500

# Features:
# - Clean 4-panel layout (System Overview, Activity Stream, Agent Details, Operations)
# - Smart event filtering (heartbeats hidden by default, 8 categories)
# - Event correlation (links start→complete with durations)
# - Real-time SSE updates with zero latency
# - Full keyboard control (0-3 panel toggles, c to clear, q to quit)
# - Automatic slow operation and failure detection
# - 124+ tests, production-ready monitoring

# See docs/DASHBOARD.md for keyboard shortcuts and advanced usage
```

**TUI Wrapper Mode** (Deprecated in v2.1.0):
```bash
⚠️ Deprecated: Use mnemosyne-ics + mnemosyne-dash instead
See docs/guides/migration.md for migration guide

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

# API server starts automatically with first MCP instance
# No manual startup required

# Features:
# • Live agent activity via SSE across all MCP instances
# • Color-coded agent states
# • System statistics (memory, CPU, context)
# • Event log with scrollback
# • Auto-reconnect on disconnect
```

### API Server (Automatic)
```bash
# MCP server automatically starts HTTP API on first instance (owner mode)
# Subsequent instances connect as clients and forward events via HTTP
mnemosyne serve

# Owner mode (first instance):
# • Binds port 3000 (or 3001-3010 if 3000 unavailable)
# • Starts API server with SSE event streaming
# • Broadcasts events locally

# Client mode (subsequent instances):
# • Detects existing API server via health check
# • Forwards events via POST /events/emit
# • No port conflicts - seamless multi-instance support

# Endpoints:
# GET  /health                Health check (used for auto-detection)
# GET  /events                SSE event stream (real-time)
# POST /events/emit           Event forwarding (client mode)
# GET  /state/agents          List agent states
# GET  /state/context-files   Context files across instances

# Features:
# • Automatic owner/client mode detection
# • Zero-configuration multi-instance support
# • Event forwarding via HTTP POST (100ms timeout, fire-and-forget)
# • REST API with Axum + Server-Sent Events (SSE)
# • CORS support for web clients
```

### TUI (Terminal User Interface) - Deprecated
```bash
⚠️ Deprecated in v2.1.0: Use mnemosyne-ics + mnemosyne-dash instead
See docs/guides/migration.md for migration guide

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

### Getting Started
- [README.md](README.md) - Project overview and quick start (this file)
- [QUICK_START.md](QUICK_START.md) - Get up and running in 5 minutes
- [INSTALL.md](INSTALL.md) - Detailed installation guide

### For Agents/Developers
- **[AGENT_GUIDE.md](AGENT_GUIDE.md)** - **START HERE** - Comprehensive development guide
- [docs/INDEX.md](docs/INDEX.md) - Documentation navigation hub
- [docs/TYPES_REFERENCE.md](docs/TYPES_REFERENCE.md) - Complete type system reference
- [docs/STORAGE_SCHEMA.md](docs/STORAGE_SCHEMA.md) - Database schema and query patterns

### Core System
- [ARCHITECTURE.md](ARCHITECTURE.md) - System architecture and design decisions
- [ORCHESTRATION.md](ORCHESTRATION.md) - Multi-agent coordination guide
- [MCP_SERVER.md](MCP_SERVER.md) - MCP protocol integration

### Features
- [docs/features/EVOLUTION.md](docs/features/EVOLUTION.md) - Memory evolution system
- [docs/features/VECTOR_SEARCH.md](docs/features/VECTOR_SEARCH.md) - Semantic search implementation
- [docs/features/PRIVACY.md](docs/features/PRIVACY.md) - Privacy-preserving evaluation
- [docs/features/ICS_README.md](docs/features/ICS_README.md) - Integrated Context Studio
- [docs/features/semantic_highlighting.md](docs/features/semantic_highlighting.md) - 3-tier highlighting system

### Guides
- [docs/guides/migration.md](docs/guides/migration.md) - Migration from TUI to composable tools
- [docs/guides/llm-reviewer.md](docs/guides/llm-reviewer.md) - LLM reviewer system
- [docs/guides/llm-reviewer-setup.md](docs/guides/llm-reviewer-setup.md) - Setup and troubleshooting
- [docs/guides/workflows.md](docs/guides/workflows.md) - Common development workflows

### Specifications
- [docs/specs/background-processing-spec.md](docs/specs/background-processing-spec.md) - Tier 3 background processing
- [docs/specs/ics-integration-spec.md](docs/specs/ics-integration-spec.md) - ICS integration specification
- [docs/specs/incremental-analysis-spec.md](docs/specs/incremental-analysis-spec.md) - Incremental semantic analysis
- [docs/specs/semantic-highlighter-test-plan.md](docs/specs/semantic-highlighter-test-plan.md) - Testing strategy
- [docs/specs/tier3-llm-integration-spec.md](docs/specs/tier3-llm-integration-spec.md) - LLM integration architecture

### Development
- [CHANGELOG.md](CHANGELOG.md) - Version history
- [TROUBLESHOOTING.md](TROUBLESHOOTING.md) - Common issues and solutions
- [TODO_TRACKING.md](TODO_TRACKING.md) - Development progress tracking
- [docs/BUILD_OPTIMIZATION.md](docs/BUILD_OPTIMIZATION.md) - Build performance tuning

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

## Troubleshooting

### macOS "killed" Error

If you see `zsh: killed  mnemosyne` when trying to run the binary:

**Quick Fix**:
```bash
xattr -d com.apple.provenance ~/.cargo/bin/mnemosyne
codesign --force --sign - ~/.cargo/bin/mnemosyne
```

**Root Cause**: macOS Gatekeeper invalidates code signatures when binaries are relocated (e.g., by `cargo install`). The binary in `target/release/` works fine, but the installed copy in `~/.cargo/bin/` gets killed by taskgated.

**Permanent Fix**: Always use the install script, which handles re-signing automatically:
```bash
./scripts/install/install.sh
```

**Quick rebuild during development**:
```bash
./scripts/build-and-install.sh
```

For more troubleshooting help, see [TROUBLESHOOTING.md](TROUBLESHOOTING.md).

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

**Current Version**: 2.3.1

**v2.4.0 Release (2025-11-23)** - Distributed Coordination:
- ✅ **Peer Discovery**: Iroh-based P2P discovery and connection management
- ✅ **Work Delegation**: Distributed task execution across connected peers
- ✅ **Network Visualization**: TUI-based network graph and status monitoring
- ✅ **Documentation**: Updated guides for distributed setup and usage

**v2.3.1 Release (2025-11-09)** - Dashboard Crash Fix:
- ✅ **Critical Bug Fix**: Fixed dashboard crash from NaN values in health metrics
- ✅ **Terminal Corruption Prevention**: Added panic handler to restore terminal state
- ✅ **Floating-Point Safety**: Fixed unsafe `partial_cmp().unwrap()` patterns
- ✅ **Enhanced Error Handling**: SSE bounds checking, graceful error recovery
- ✅ **Comprehensive Tests**: 11 sparkline tests + 8 anaphora tests passing
- ✅ **Documentation**: Troubleshooting guide + development best practices

**v2.3.0 Release (2025-11-08)** - Dashboard Redesign & CLI Operations Tracking:
- ✅ **Dashboard Redesign**: 4-panel layout replacing 7-panel "wall of garbage"
  - System Overview (top): At-a-glance health metrics
  - Activity Stream (left, 60%): Intelligent event log with filtering
  - Agent Details (right-top, 40%): Per-agent status and work queues
  - Operations (right-bottom, 40%): CLI command history with outcomes
- ✅ **Smart Event Filtering**: 8 categories, compound AND/OR/NOT logic, filter presets
- ✅ **Event Correlation Engine**: Links start→complete events, duration tracking, slow operation detection
- ✅ **CLI Operations Tracking**: Real-time CLI command visibility in dashboard
- ✅ **Full Keyboard Control**: Interactive navigation (q/Esc, 0-3 panel toggles, c to clear)
- ✅ **Production Quality**: 124+ tests, 6,122 lines of code, comprehensive error handling
- ✅ **Documentation**: `docs/DASHBOARD.md` (300+ lines) with architecture, features, troubleshooting

**v2.2.0 Release (2025-11-08)** - gRPC Remote Access:
- ✅ **gRPC Server**: Production-ready gRPC server for remote memory access
- ✅ **MemoryService**: 13 RPC methods (CRUD, search, streaming operations)
- ✅ **HealthService**: System monitoring, metrics, statistics
- ✅ **Language-Agnostic**: Protocol buffer API for Rust, Python, Go, TypeScript, etc.
- ✅ **Feature-Gated**: Optional `rpc` feature, no impact on default builds
- ✅ **Test Suite**: 11 RPC integration tests passing, 728 library tests passing
- ✅ **Documentation**: 1,868 lines of comprehensive RPC documentation

**v2.1.2 Release (2025-11-06)** - Clean Build & Repository Cleanup:
- ✅ **Clean Build**: Fixed all 6 compiler warnings (unused variables, imports, fields)
- ✅ **Repository Cleanup**: Removed temporary files (.bak, .DS_Store) and stale branches
- ✅ **Documentation Updates**: Updated ROADMAP, README, CHANGELOG for v2.1.2
- ✅ **Test Suite**: 715 unit tests passing, 0 failures
- ✅ **Build**: 0 warnings, 0 errors

**v2.1.1 Release (2025-11-06)** - Python Bridge Architecture & Production Hardening:
- ✅ **Python Bridge Complete**: PyO3 integration with Claude SDK agents
- ✅ **Phase 5 Production Hardening**: 8/8 tasks complete (100%)
  - Structured logging, enhanced errors, validation, metrics
  - E2E validation with actual Claude API calls (5/5 tests passing)
  - Comprehensive troubleshooting guide (628 lines)
- ✅ **Test Suite**: 715 unit tests + 10 integration/E2E tests passing
- ✅ **Documentation**: 2,200+ lines across 5 major documents
- ✅ **Clean Build**: 0 warnings, 0 errors
- ✅ **Stability Fixes**: File descriptor leak prevention, robust process management
- ✅ **Production-ready**: Fully validated with actual Claude API calls

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
- ✅ **715 tests passing** (up from 474, +241 new tests)
- ✅ MCP server integration
- ✅ **11 new documentation files** (5,000+ lines)

**Known Issues (v2.3.1)**:
- ⚠️ PyO3 0.22.6 doesn't support Python 3.14+ (use Python 3.9-3.13)
- ⚠️ Tier 3 LLM integration is scaffolding only (not fully functional)

**Roadmap** (post-v2.3.1):
- ⏳ Tier 3 LLM integration completion
- ⏳ Incremental semantic analysis scheduling
- ⏳ ICS-semantic highlighter integration
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
