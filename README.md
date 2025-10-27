# Mnemosyne

**Project-Aware Memory System for Claude Code**

![Status](https://img.shields.io/badge/status-beta-yellow)
![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)
![Python](https://img.shields.io/badge/python-3.10%2B-blue)

---

## What is Mnemosyne?

Mnemosyne solves a critical problem in AI-assisted development: **memory loss across sessions**. When you restart Claude Code, your AI assistant forgets everything—past decisions, discovered patterns, project context, and hard-won insights.

Mnemosyne is a high-performance memory system that gives Claude Code's multi-agent orchestration system persistent semantic memory across sessions. It automatically captures, enriches, and retrieves memories so your AI assistant learns and improves over time, just like a human teammate would.

**Key Benefits**:
- Remember architecture decisions and their rationale
- Avoid repeating the same bugs
- Preserve discovered patterns and best practices
- Maintain project context across long development timelines
- Enable smarter, context-aware AI assistance

---

## Status

**Current**: Beta - Core functionality complete, orchestration layer in active development

The Rust-based memory core, MCP server, and Claude Code integration are fully functional and tested. The Python orchestration layer for multi-agent coordination is being finalized. See [ROADMAP.md](ROADMAP.md) for detailed progress.

---

## Features

- **Project-Aware Namespacing**: Automatic memory isolation between global, project, and session scopes via git detection
- **Hybrid Memory Search**: FTS5 keyword search + graph traversal for semantic retrieval
- **LLM-Enriched Storage**: Claude Haiku automatically generates summaries, keywords, classifications, and semantic links
- **OODA Loop Integration**: Explicit Observe-Orient-Decide-Act cycles for humans and agents
- **MCP Protocol**: 8 tools for seamless Claude Code integration
- **Secure Credentials**: API keys stored in OS-native keychain (macOS/Windows/Linux)
- **Slash Commands**: 6 convenient commands for common memory operations
- **Self-Organizing Knowledge**: Automatic consolidation, link strength evolution, and importance decay

---

## Quick Start

### Installation

```bash
./scripts/install/install.sh
```

This will build the Rust binary, initialize the database, configure your API key, and set up MCP integration with Claude Code.

For detailed installation options, see [INSTALL.md](INSTALL.md).

### Configuration

Mnemosyne uses Claude Haiku for memory intelligence. Set up your Anthropic API key:

```bash
# Interactive setup (recommended)
mnemosyne config set-key

# Or via environment variable
export ANTHROPIC_API_KEY=sk-ant-api03-...
```

Keys are securely stored in your OS keychain (macOS Keychain, Windows Credential Manager, or Linux Secret Service).

### Basic Usage

**In Claude Code**, use slash commands:

```
/memory-store <content>              # Store a new memory
/memory-search <query>               # Search memories
/memory-context                      # Load project context
/memory-list                         # Browse all memories
/memory-export                       # Export to markdown
/memory-consolidate                  # Review duplicates
```

Or use MCP tools programmatically:

```
mnemosyne.remember   - Store a memory with LLM enrichment
mnemosyne.recall     - Hybrid search (keyword + graph)
mnemosyne.list       - List memories with sorting
mnemosyne.graph      - Get memory graph for context
mnemosyne.context    - Get full project context
mnemosyne.consolidate - Merge/supersede memories
mnemosyne.update     - Update existing memory
mnemosyne.delete     - Archive a memory
```

See [MCP_SERVER.md](MCP_SERVER.md) for API documentation and examples.

---

## How It Works

### OODA Loop Integration

Mnemosyne is designed around explicit OODA (Observe-Orient-Decide-Act) loops for both human developers and AI agents.

**Human OODA Loop**:
```
OBSERVE → Session start loads relevant memories
ORIENT  → Review summaries and memory graph
DECIDE  → /memory-store, /memory-search commands
ACT     → Apply patterns, avoid pitfalls
FEEDBACK → Access tracking, importance updates
```

**Agent OODA Loop**:
```
OBSERVE → Phase transitions trigger memory queries
ORIENT  → Build context from memory graph
DECIDE  → Auto-store decisions, consolidate redundant info
ACT     → Apply proven patterns, link new memories
FEEDBACK → Link strength evolution, importance decay
```

### Memory Lifecycle

1. **Capture**: User or agent stores content with context
2. **Enrich**: Claude Haiku generates summary, keywords, tags, and classification
3. **Link**: LLM detects relationships with existing memories
4. **Retrieve**: Hybrid search (keyword + graph) finds relevant memories
5. **Evolve**: Access patterns adjust importance; consolidation merges duplicates

---

## Architecture

```
┌───────────────────────────────────────────────────────────┐
│                      Claude Code                          │
│                                                           │
│  • Slash commands (/memory-search, /memory-store, ...)   │
│  • MCP tools (mnemosyne.recall, mnemosyne.remember, ...) │
│  • Multi-agent skills & orchestration                    │
└─────────────────────────┬─────────────────────────────────┘
                          │
                          │ MCP Protocol
                          │ (JSON-RPC over stdio)
                          │
┌─────────────────────────▼─────────────────────────────────┐
│                  Mnemosyne Server                         │
│                   (Rust + Tokio)                          │
│                                                           │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │
│  │   Storage   │  │ LLM Service │  │  Namespace  │      │
│  │             │  │             │  │  Detector   │      │
│  │   SQLite    │  │   Claude    │  │             │      │
│  │   + FTS5    │  │   Haiku     │  │  Git-aware  │      │
│  │             │  │             │  │  Context    │      │
│  └─────────────┘  └─────────────┘  └─────────────┘      │
│                                                           │
│  • Hybrid search (keyword + graph)                       │
│  • LLM-enriched memory (summaries, tags, links)          │
│  • Project-aware namespacing (global/project/session)    │
└───────────────────────────────────────────────────────────┘
```

**Data Flow**:
1. Claude Code calls MCP tools via JSON-RPC
2. Mnemosyne Server processes requests using Storage, LLM, and Namespace services
3. Storage persists to SQLite with FTS5 indexing
4. LLM Service enriches memories via Claude Haiku API
5. Results flow back to Claude Code for agent decision-making

For detailed architecture documentation, see [ARCHITECTURE.md](ARCHITECTURE.md).

---

## Development

### Prerequisites

- Rust 1.75+
- SQLite 3.43+
- Anthropic API key (for LLM operations)
- Python 3.10+ (for orchestration layer)

### Build

```bash
# Rust core
cargo build --release

# Python orchestration (optional, in development)
pip install maturin
maturin develop
```

### Test

```bash
# Rust tests
cargo test
cargo test --doc

# Python tests (requires ANTHROPIC_API_KEY)
pytest
pytest -m "not integration"  # Skip LLM tests
```

For contribution guidelines, see [CONTRIBUTING.md](CONTRIBUTING.md).

---

## Documentation

- **[INSTALL.md](INSTALL.md)** - Detailed installation guide
- **[MCP_SERVER.md](MCP_SERVER.md)** - MCP API reference and examples
- **[ARCHITECTURE.md](ARCHITECTURE.md)** - System design and implementation details
- **[ROADMAP.md](ROADMAP.md)** - Development phases and progress tracking
- **[CONTRIBUTING.md](CONTRIBUTING.md)** - How to contribute

---

## Design Principles

1. **Zero-Copy**: Minimize allocations for high performance
2. **Type Safety**: Leverage Rust's type system to prevent errors
3. **Async-First**: Non-blocking I/O with Tokio for scalability
4. **Fail-Fast**: Explicit error handling with `Result<T, E>`
5. **Immutable Audit Trail**: Never delete, only supersede
6. **Incremental Complexity**: Start simple, add features progressively

---

## Performance

| Metric | Target | Current |
|--------|--------|---------|
| Retrieval latency (p95) | <200ms | ~50ms |
| Storage latency (p95) | <500ms | ~300ms |
| Memory usage (idle) | <100MB | ~30MB |
| Database size | ~1MB per 1000 memories | ~800KB/1000 |

See [ROADMAP.md](ROADMAP.md) for detailed performance targets and benchmarking plans.

---

## License

MIT

---

## Contributing

This project is in active development. We welcome contributions! See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

---

## Acknowledgments

- Built for [Claude Code](https://claude.ai/claude-code)
- Inspired by the need for persistent memory in AI-assisted development
- Uses [Claude 3.5 Haiku](https://www.anthropic.com/news/claude-3-5-haiku) for memory intelligence
