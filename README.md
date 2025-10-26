# Mnemosyne

**Project-Aware Agentic Memory System for Claude Code**

![Status](https://img.shields.io/badge/status-in%20development-yellow)
![Phase](https://img.shields.io/badge/phase-1%20of%2010-blue)
![Rust](https://img.shields.io/badge/rust-1.75%2B-orange)

---

## Overview

Mnemosyne is a high-performance, Rust-based memory system designed to provide Claude Code's multi-agent orchestration system with persistent semantic memory across sessions. The system features:

- **Project-Aware Namespacing**: Automatic isolation between global, project, and session scopes
- **Hybrid Memory Retrieval**: Vector similarity + keyword search + graph traversal
- **LLM-Guided Intelligence**: Automatic note construction and semantic linking via Claude Haiku
- **OODA Loop Integration**: Explicit Observe-Orient-Decide-Act cycles for both human and agent users
- **Self-Organizing Knowledge Graphs**: Memory evolution through consolidation and link strength adjustment

---

## Current Status

### ✅ Phase 1: Core Memory System (IN PROGRESS)

**Completed**:
- [x] Rust project foundation with Cargo workspace
- [x] Core data structures (`types.rs`)
  - MemoryId, Namespace (Global/Project/Session)
  - MemoryType, LinkType
  - MemoryNote with full metadata
  - SearchQuery and SearchResult
  - Consolidation decisions
- [x] Comprehensive error handling (`error.rs`)
- [x] CLI framework with clap
- [x] Code compiles successfully ✓

**In Progress**:
- [ ] SQLite storage backend with sqlite-vec
- [ ] Database migrations
- [ ] Embedding service (fastembed integration pending)

**Not Started**:
- Remaining phases 2-10 (see [Implementation Plan](#implementation-plan))

---

## Architecture

```
┌─────────────────────────────────────────────────┐
│         Claude Code + Multi-Agent System        │
│  ┌──────────────────────────────────────────┐   │
│  │  Orchestrator  Optimizer  Reviewer       │   │
│  │  Executor      (with memory skills)      │   │
│  └───────────────────┬──────────────────────┘   │
└────────────────────┼─────────────────────────────┘
                     │ MCP Protocol
          ┌──────────▼──────────┐
          │  Mnemosyne Server   │
          │  (Rust + MCP)       │
          └──────────┬──────────┘
                     │
     ┌───────────────┼───────────────┐
     │               │               │
┌────▼────┐    ┌────▼────┐    ┌────▼────┐
│ Storage │    │   LLM   │    │Embedding│
│(SQLite) │    │(Claude) │    │(Local)  │
└─────────┘    └─────────┘    └─────────┘
```

### Key Components

1. **Core Types** (`src/types.rs`)
   - Namespace hierarchy for project isolation
   - Memory classification and linking
   - OODA-aware importance decay

2. **Storage Layer** (planned: `src/storage/`)
   - SQLite backend with sqlite-vec for vector search
   - Graph traversal for semantic connections
   - Atomic transactions and audit trails

3. **Intelligence Layer** (planned: `src/services/`)
   - LLM service for note construction
   - Embedding generation (local via fastembed)
   - Memory consolidation decisions

4. **MCP Server** (planned: `src/mcp_server.rs`)
   - JSON-RPC over stdio
   - 8 core tools: remember, recall, list, update, delete, consolidate, switch_context, export

---

## OODA Loop Integration

Mnemosyne is designed around explicit OODA (Observe-Orient-Decide-Act) loops for both human developers and AI agents.

### Human OODA Loop

```
OBSERVE → Session start loads relevant memories
ORIENT  → Review summaries and memory graph
DECIDE  → /memory-store, /memory-search commands
ACT     → Apply patterns, avoid pitfalls
FEEDBACK → Access tracking, importance updates
```

### Agent OODA Loop

```
OBSERVE → Phase transitions trigger memory queries
ORIENT  → Build context from memory graph
DECIDE  → Auto-store decisions, consolidate redundant info
ACT     → Apply proven patterns, link new memories
FEEDBACK → Link strength evolution, importance decay
```

---

## Installation (Coming Soon)

```bash
# Once implemented:
./install.sh

# This will:
# - Build mnemosyne binary
# - Initialize database
# - Configure MCP for Claude Code
# - Install skills, commands, hooks
```

---

## Quick Start (Coming Soon)

```bash
# Start MCP server
mnemosyne serve

# Initialize database
mnemosyne init

# Check status
mnemosyne status
```

In Claude Code:
```
# Store a memory
/memory-store "Decision: Using PostgreSQL for scalability"

# Search memories
/memory-search "database choices"

# Export project knowledge
/memory-export --project myapp
```

---

## Development

### Prerequisites

- Rust 1.75+
- SQLite 3.43+
- Anthropic API key (for LLM operations)

### Build

```bash
cargo build --release
```

### Test

```bash
cargo test
cargo test --doc
```

### Benchmark

```bash
cargo bench
```

---

## Implementation Plan

**Total Timeline**: 7-8 weeks

### Phase 1: Core Memory System (Weeks 1-2) ← YOU ARE HERE
- Rust foundation ✓
- Core data structures ✓
- Error handling ✓
- SQLite storage (in progress)
- Database migrations
- Embedding service

### Phase 2: Memory Intelligence (Weeks 2-3)
- LLM service (Claude Haiku)
- Note construction
- Semantic linking
- Consolidation logic
- Hybrid search

### Phase 3: Project-Aware Context (Week 3)
- Namespace detection
- Priority system
- Permission model

### Phase 4: MCP Server (Week 4)
- JSON-RPC protocol
- 8 core tools
- Claude Code integration

### Phase 5: Multi-Agent Integration (Week 5)
- Memory skills
- Slash commands
- Enhanced hooks

### Phase 6: Agent Orchestration (Week 5-6)
- Agent-specific views
- Background evolution

### Phase 7: Installation (Week 6)
- Install/uninstall scripts
- Configuration system

### Phase 8: CLAUDE.md Integration (Week 6)
- Documentation updates
- Decision trees

### Phase 9: Testing (Week 7)
- Unit, integration, E2E tests
- Performance benchmarks

### Phase 10: Documentation (Week 7-8)
- API docs
- Architecture guide
- Examples

---

## Performance Targets

- **Retrieval Latency**: <200ms p95
- **Embedding Generation**: <100ms
- **Search Accuracy**: 70-80%
- **Context Compression**: 85-95%
- **Zero Data Loss**: Immutable audit trail
- **Namespace Isolation**: 100% effective

---

## Design Principles

1. **Zero-Copy**: Minimize allocations
2. **Type Safety**: Leverage Rust's type system
3. **Async-First**: Non-blocking I/O
4. **Fail-Fast**: Explicit error handling with `Result<T, E>`
5. **Immutable Audit Trail**: Never delete, only supersede
6. **Incremental Complexity**: Start simple, add features progressively

---

## Contributing

This project is currently in early development. Contribution guidelines will be added once the core implementation stabilizes.

---

## License

MIT

---

## Related Projects

- [Claude Code](https://claude.ai/claude-code) - AI-powered development environment
- [Multi-Agent Design Spec](./multi-agent-design-spec.md) - Original multi-agent architecture
- [Mnemosyne Rust Spec](./mnemosyne-rust-spec.md) - Detailed implementation specification

---

**Status**: Phase 1 in progress (core types complete, storage layer next)
**Next Milestone**: SQLite storage backend with vector search
**Estimated Completion**: 6-7 weeks remaining
