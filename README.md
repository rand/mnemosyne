# Mnemosyne

**Project-Aware Memory System for Claude Code**

![Version](https://img.shields.io/badge/version-2.0.0-green)
![Status](https://img.shields.io/badge/status-stable-green)
![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)
![Python](https://img.shields.io/badge/python-3.10%2B-blue)
![License](https://img.shields.io/badge/license-MIT-blue)
![Tests](https://img.shields.io/badge/tests-passing-green)

---

## What is Mnemosyne?

Mnemosyne solves a critical problem in AI-assisted development: **memory loss across sessions**. When you restart Claude Code, your AI assistant forgets everything – past decisions, discovered patterns, project context, and hard-won insights.

Mnemosyne is a high-performance memory system that gives Claude Code a multi-agent orchestration system with persistent semantic memory across sessions. It automatically captures, enriches, and retrieves memories so your AI assistant learns and improves over time, just like a human teammate would.

**Key Benefits**:
- Remember architecture decisions and their rationale
- Avoid repeating the same bugs
- Preserve discovered patterns and best practices
- Maintain project context across long development timelines
- Enable smarter, context-aware AI assistance

---

## Status

**Current**: v2.0 Released - Enhanced with vector search, RBAC, and autonomous evolution

**v2.0 Features** (October 2025):
- **Local Vector Search**: Semantic similarity with fastembed + sqlite-vec (768-dim local embeddings, no external API calls)
- **RBAC System**: Agent-based access control with role-based permissions and audit trails
- **Evolution System**: Autonomous importance recalibration, link decay, and memory archival
- **Hybrid Search**: Combined keyword + graph + vector search with weighted ranking

---

## Features

### Core Capabilities
- **Automatic Memory Capture** 🔥: Hooks that auto-load context at session start, preserve decisions before compaction, and link commits to architectural memories—zero manual intervention required
- **Project-Aware Namespacing**: Automatic memory isolation between global, project, and session scopes via git detection
- **Hybrid Search** (v2.0): Combined keyword (FTS5) + graph traversal + vector similarity with weighted ranking
- **LLM-Enriched Storage**: Claude Haiku automatically generates summaries, keywords, classifications, and semantic links
- **OODA Loop Integration**: Explicit Observe-Orient-Decide-Act cycles for humans and agents
- **MCP Protocol**: 8 tools for seamless Claude Code integration
- **Secure Credentials**: Age-encrypted secrets with environment variable and OS keychain fallback
- **Slash Commands**: 6 convenient commands for common memory operations
- **PyO3 Performance**: 10-20x faster operations (<3ms) vs subprocess calls through Rust↔Python bindings

### v2.0 Advanced Features
- **Local Vector Search**: Semantic similarity with fastembed (nomic-embed-text-v1.5, 768-dim) + sqlite-vec extension
- **Global Model Cache**: Shared embedding models at `~/.cache/mnemosyne/models/` (~140MB, reused across projects)
- **RBAC System**: Agent-based access control with 4 roles (Orchestrator, Optimizer, Reviewer, Executor)
- **Audit Trails**: Complete memory modification tracking with agent attribution
- **Evolution System**: Autonomous background jobs for memory optimization
  - **Importance Recalibration**: Logarithmic access-based scoring with exponential recency decay
  - **Link Decay**: Automatic weakening of untraversed connections (90/180-day thresholds)
  - **Memory Archival**: Smart archival of unused memories (never archived: importance ≥7.0)
- **Self-Organizing Knowledge**: Automatic consolidation, link strength evolution, and importance decay
- **Privacy-Preserving Evaluation** 🔒: Adaptive context relevance learning with local-only storage, hashed tasks, and statistical features (no raw content stored)

---

## Quick Start

### One-Command Installation

```bash
./scripts/install/install.sh
```

This automated installer builds the Rust binary, initializes the database, configures your API key, and sets up MCP integration with Claude Code.

**New to Mnemosyne?** See **[Quick Start Guide](QUICK_START.md)** for a 5-minute guided introduction.

**Advanced setup?** See **[Installation Guide](INSTALL.md)** for detailed options.

### Configuration

Mnemosyne uses Claude Haiku for memory intelligence. Set up your Anthropic API key:

```bash
# Interactive setup (recommended)
mnemosyne secrets init

# Or set individual secrets
mnemosyne secrets set ANTHROPIC_API_KEY

# Or via environment variable
export ANTHROPIC_API_KEY=sk-ant-api03-...
```

Secrets are encrypted using [age](https://age-encryption.org/) and stored in `~/.config/mnemosyne/secrets.age`. Environment variables take priority (ideal for CI/CD), with OS keychain as an optional fallback.

For detailed secrets management documentation, see [SECRETS_MANAGEMENT.md](SECRETS_MANAGEMENT.md).

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
mnemosyne.recall     - Hybrid search (keyword + graph + vector)
mnemosyne.list       - List memories with sorting
mnemosyne.graph      - Get memory graph for context
mnemosyne.context    - Get full project context
mnemosyne.consolidate - Merge/supersede memories
mnemosyne.update     - Update existing memory
mnemosyne.delete     - Archive a memory
```

See [MCP_SERVER.md](MCP_SERVER.md) for API documentation and examples.

**CLI Embedding Management**:

```bash
# Generate embeddings for all memories (enables vector search)
mnemosyne embed --all --progress

# Generate embeddings for specific namespace
mnemosyne embed --namespace "project:myapp"

# Manage embedding models
mnemosyne models list      # List available models
mnemosyne models info      # Show cache info
mnemosyne models clear     # Clear model cache (~140MB)
```

For detailed vector search documentation, see [VECTOR_SEARCH.md](docs/VECTOR_SEARCH.md).

### Orchestrated Sessions (Default)

**New in v2.0**: Running `mnemosyne` without any commands launches an orchestrated Claude Code session with full multi-agent coordination:

```bash
# Launch orchestrated Claude Code session (default)
mnemosyne

# Start MCP server only (no Claude Code session)
mnemosyne --serve
```

**Orchestrated sessions include**:
- **4 Active Agents**: Orchestrator (coordinator), Optimizer (context optimization), Reviewer (quality gates), Executor (primary work agent)
- **Automatic Mnemosyne Integration**: MCP server runs in background with proper agent roles
- **Intelligent Context Loading**: Three-layer context strategy
  - **Pre-launch**: Top 10 high-importance memories (≥7) loaded before Claude starts
  - **Post-launch**: Session start hook displays context in chat
  - **In-session**: Optimizer dynamically loads memories as tasks evolve
- **Namespace Detection**: Auto-detects project from git repository
- **Sub-Agent Spawning**: Executor can spawn parallel sub-agents for independent work

This is the recommended way to use Mnemosyne - it provides the full multi-agent orchestration experience with seamless memory integration and intelligent context management.

**Context Loading Details**:
- **Pre-launch context**: Loaded in <200ms, ~10KB, memories with importance ≥7
- **Graceful degradation**: Session launches even if context loading fails
- **Timeout protection**: 500ms hard limit, never blocks session start
- **Budget-aware**: Respects 20% context allocation (~10KB of ~50KB total)
- **Format**: Natural language markdown optimized for LLM consumption

**Use `mnemosyne --serve` when**:
- Integrating with external tools that expect a standalone MCP server
- Running in CI/CD pipelines
- You want manual control over Claude Code session startup

### Skills Integration

Mnemosyne leverages [cc-polymath](https://github.com/rand/cc-polymath) for comprehensive development knowledge:

- **354 atomic skills** across 33+ categories automatically discovered based on task requirements
- **5 Mnemosyne-specific skills** for memory system expertise (memory management, context preservation, Rust development, MCP protocol, discovery gateway)
- **Progressive loading** - Optimizer agent loads only relevant skills per task (max 7 skills)
- **Multi-path discovery** - Project-local skills (`.claude/skills/`) take priority over global cc-polymath skills
- **Context efficient** - ~30% of context budget allocated to skills
- **Adaptive learning** - Evaluation system learns which skills are most relevant over time (privacy-preserving)

The Optimizer agent automatically discovers and loads relevant skills based on task analysis. Project-local Mnemosyne skills get a +10% relevance bonus to ensure project-specific knowledge is prioritized.

**Skills locations**:
- Project-local: `.claude/skills/` (5 Mnemosyne-specific skills)
- Global: `~/.claude/plugins/cc-polymath/skills/` (354 comprehensive skills)

### Privacy-Preserving Evaluation

Mnemosyne's evaluation system helps the Optimizer agent learn which context is most relevant over time **without compromising privacy**:

**Privacy Guarantees**:
- ✅ **Local-Only Storage**: All data in `.mnemosyne/project.db` (gitignored)
- ✅ **Hashed Tasks**: SHA256 hash of task descriptions (16 chars only)
- ✅ **Limited Keywords**: Max 10 generic keywords, no sensitive terms
- ✅ **Statistical Features**: Only computed metrics stored, never raw content
- ✅ **No Additional Network Calls**: Uses existing Anthropic API calls, no separate requests
- ✅ **Graceful Degradation**: System works perfectly when disabled

**How It Works**:
1. Optimizer provides context (skills, memories, files) for a task
2. System tracks implicit feedback (accessed? edited? committed? cited?)
3. Statistical features computed (keyword overlap scores, recency, access patterns)
4. Online learning algorithm updates relevance weights
5. Future context selections improve based on learned patterns

**Hierarchical Learning** (session → project → global):
- **Session-level**: Fast adaptation (α=0.3) for immediate context
- **Project-level**: Moderate adaptation (α=0.1) for project patterns
- **Global-level**: Slow adaptation (α=0.03) for universal patterns

**Example**: If `rust-async.md` is frequently accessed and edited during Rust async tasks, the Optimizer learns to prioritize it for similar future tasks.

**Learn More**:
- **Privacy Policy**: [docs/PRIVACY.md](docs/PRIVACY.md) - Formal privacy guarantees
- **Technical Details**: [EVALUATION.md](EVALUATION.md) - Architecture and examples
- **Disable**: Set `MNEMOSYNE_DISABLE_EVALUATION=1` or `OptimizerConfig(enable_evaluation=False)`

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

```mermaid
%%{init: {'theme':'base', 'themeVariables': {'primaryTextColor':'#000', 'primaryBorderColor':'#333', 'lineColor':'#333', 'fontSize':'14px'}}}%%
flowchart TD
    User([Developer/Agent])

    subgraph Claude["Claude Code"]
        UI[Slash Commands<br/>6 memory operations]
        MCP[MCP Client]
        Agents[Multi-Agent System<br/>4 specialized agents]
    end

    Protocol{{MCP Protocol<br/>JSON-RPC over stdio}}

    subgraph Mnemosyne["Mnemosyne Server"]
        Server[MCP Server<br/>8 OODA Tools]

        subgraph Services["Core Services"]
            Storage[(Dual Storage v2.0<br/>rusqlite + sqlite-vec<br/>libsql for memories)]
            LLM[LLM Service<br/>Claude Haiku]
            NS[Namespace<br/>Git-aware]
            RBAC[Access Control<br/>Agent roles + audit]
            Evolution[Evolution Jobs<br/>Auto-optimization]
        end
    end

    API[/Anthropic API\]

    User --> UI
    UI --> MCP
    Agents --> MCP
    MCP <--> Protocol
    Protocol <--> Server

    Server --> Storage
    Server --> LLM
    Server --> NS

    LLM --> API
    Storage -.->|FTS5 Search| Storage
    Storage -.->|Graph Traversal| Storage

    style User fill:#bbdefb,stroke:#0d47a1,stroke-width:3px,color:#000
    style Claude fill:#e1bee7,stroke:#4a148c,stroke-width:3px,color:#000
    style Mnemosyne fill:#ffe0b2,stroke:#e65100,stroke-width:3px,color:#000
    style Protocol fill:#c8e6c9,stroke:#1b5e20,stroke-width:3px,color:#000
    style Services fill:#e0e0e0,stroke:#212121,stroke-width:2px,color:#000
    style API fill:#c8e6c9,stroke:#2e7d32,stroke-width:3px,color:#000

    style UI fill:#fff,color:#000,stroke:#333,stroke-width:2px
    style MCP fill:#fff,color:#000,stroke:#333,stroke-width:2px
    style Agents fill:#fff,color:#000,stroke:#333,stroke-width:2px
    style Server fill:#fff,color:#000,stroke:#333,stroke-width:2px
    style Storage fill:#fff,color:#000,stroke:#333,stroke-width:2px
    style LLM fill:#fff,color:#000,stroke:#333,stroke-width:2px
    style NS fill:#fff,color:#000,stroke:#333,stroke-width:2px
```

**Key Components**:

| Component | Technology | Purpose |
|-----------|-----------|---------|
| **Storage Layer** | LibSQL/Turso + FTS5 + sqlite-vec | Local vector search (fastembed), hybrid retrieval (keyword + graph + vector) |
| **LLM Service** | Claude Haiku | Auto-generates summaries, tags, semantic links |
| **Namespace Detector** | Git-aware | Project context (global/project/session) |
| **MCP Server** | Rust + Tokio | 8 OODA-aligned tools via JSON-RPC |

For detailed architecture documentation, see [ARCHITECTURE.md](ARCHITECTURE.md).

---

## Development

### Prerequisites

- Rust 1.75+
- LibSQL (bundled via libsql crate)
- Anthropic API key (for LLM operations)
- Python 3.10+ (for orchestration layer, optional)

### Build

```bash
# Rust core
cargo build --release

# Python orchestration (optional)
# See ORCHESTRATION.md for complete PyO3 setup instructions
uv venv .venv && source .venv/bin/activate
uv pip install maturin
export PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1
maturin develop
```

### Test

```bash
# All tests (automatically skips LLM tests if no API key)
./test-all.sh

# Skip LLM tests even if API key is available
./test-all.sh --skip-llm

# Run only LLM tests (requires ANTHROPIC_API_KEY)
./test-all.sh --llm-only

# Or use cargo directly
cargo test              # Regular tests
cargo test -- --ignored # LLM tests (requires API key)
```

For contribution guidelines, see [CONTRIBUTING.md](CONTRIBUTING.md).

---

## Documentation

📚 **[Complete Documentation Index](DOCUMENTATION.md)** - Full documentation navigation and guides

### Quick Links

**Getting Started:**
- 🚀 [Quick Start](QUICK_START.md) - Get running in 5 minutes
- 📦 [Installation Guide](INSTALL.md) - Detailed setup instructions
- 🏗️ [Architecture Overview](ARCHITECTURE_OVERVIEW.md) - How Mnemosyne works

**User Guides:**
- 📖 [Common Workflows](docs/guides/workflows.md) - Practical usage patterns
- 🔧 [Troubleshooting](TROUBLESHOOTING.md) - Fix common issues
- 🔐 [Secrets Management](SECRETS_MANAGEMENT.md) - API key configuration
- 📡 [MCP API Reference](MCP_SERVER.md) - Tool documentation
- 🔒 [Privacy Policy](docs/PRIVACY.md) - Evaluation system privacy guarantees
- 🎓 [Evaluation System](EVALUATION.md) - Adaptive context learning

**Development:**
- 🤝 [Contributing Guide](CONTRIBUTING.md) - How to contribute
- 🏛️ [Architecture Deep Dive](ARCHITECTURE.md) - System internals
- 🧪 [Hooks & Testing](HOOKS_TESTING.md) - Automated capture
- 🗺️ [Roadmap](ROADMAP.md) - Project plans and v2.0 features

**Reference:**
- 📝 [Changelog](CHANGELOG.md) - Version history
- ✅ [Audit Report](AUDIT_REPORT.md) - v1.0 quality assessment
- 🔄 [Migration Guide](docs/guides/migration.md) - Upgrading between versions

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

## Getting Help

- 📖 **[Troubleshooting Guide](TROUBLESHOOTING.md)** - Common issues and solutions
- 💬 **[GitHub Discussions](https://github.com/rand/mnemosyne/discussions)** - Questions and community support
- 🐛 **[Issue Tracker](https://github.com/rand/mnemosyne/issues)** - Bug reports and feature requests
- 📧 **[Contact](mailto:rand.arete@gmail.com)** - Direct support (48h response time)

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
