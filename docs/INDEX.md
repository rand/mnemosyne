---
title: "Mnemosyne: Semantic Memory and Multi-Agent Orchestration for LLM Systems"
description: "A production-ready semantic memory system with multi-agent orchestration for LLM-based systems. Technical whitepaper."
version: "2.2.0"
date: "November 8, 2025"
---

# Mnemosyne: Semantic Memory and Multi-Agent Orchestration for LLM Systems

**Version 2.2.0** · **November 8, 2025** · [github.com/rand/mnemosyne](https://github.com/rand/mnemosyne)

## Abstract

Large language models face fundamental limitations: context windows bound working memory, coordination between agents lacks persistence, and knowledge evaporates between sessions. Mnemosyne addresses these challenges through a production-ready semantic memory system with multi-agent orchestration.

Built in Rust with LibSQL storage, it provides sub-millisecond retrieval (0.88ms list operations, 1.61ms search), LLM-guided memory evolution, and a four-agent coordination framework composed of Orchestrator, Optimizer, Reviewer, and Executor agents. The system integrates with Claude Code via Model Context Protocol, automatic hooks, and real-time monitoring.

Hybrid search combines keyword matching (FTS5), graph traversal, and vector similarity with weighted scoring. Privacy-preserving evaluation, comprehensive testing, and production deployment enable persistent context across sessions, autonomous agent coordination, and continuous memory optimization.

This paper presents the architecture, validates claims against tagged source code (v2.2.0), compares with existing solutions (MemGPT, Mem0, LangChain Memory), and demonstrates production readiness through comprehensive testing and real-world integration.

## The Challenge

### Context Window Mathematics

Context windows constrain LLM working memory. Modern systems provide 32K-200K tokens, but effective memory drops to 10-15K tokens after system instructions (2-3K), conversation history (3-10K), and code context (10-20K)—roughly 10-15 pages of unique information.

Cost compounds the challenge. GPT-4 charges $0.03/1K input tokens; a 32K context costs $1 per request. Repeated loading across sessions creates financial pressure to minimize context.

### The Re-initialization Tax

Every session starts with zero context. Developers spend 5-15 minutes reconstructing relevant information: identifying files (2-3 min), explaining tasks (1-2 min), providing architectural context (2-5 min), referencing decisions (0-3 min).

For 40 sessions over a 2-week feature (4/day), that's 200-520 minutes (3.3-8.7 hours) spent on context management. At $100/hour, inefficiency costs $330-$870 per feature.

### Multi-Agent Coordination Failures

Without shared memory: Agent A completes work but Agent B can't access results, requiring re-transmission. Race conditions occur when agents duplicate work without knowledge of each other. Deadlocks happen when agents wait on each other circularly. Debugging coordination failures becomes impossible without audit trails.

## Architecture

### Core Memory System

**Hybrid Search**: Three complementary techniques provide multi-modal retrieval:

- **FTS5 Keyword Search** (20% weight): SQLite's full-text search with BM25 ranking, <0.5ms typical latency
- **Graph Expansion** (10% weight): Recursive CTEs traverse memory links with strength weighting, ~5ms for 1-hop traversal
- **Vector Semantics** (70% weight, planned v2.2+): Embedding-based similarity using fastembed or Voyage AI

**Storage**: LibSQL provides ACID guarantees, B-tree indexes on namespace/importance/created_at, FTS5 virtual tables, and ~800KB per 1,000 memories. Performance: 0.5ms get-by-ID, 0.88ms list-recent, 1.61ms hybrid-search.

### System Architecture

The following diagram shows the high-level component architecture and data flow through the system:

![System Architecture](assets/diagrams/01-system-architecture-light.svg#gh-light-mode-only)
![System Architecture](assets/diagrams/01-system-architecture-dark.svg#gh-dark-mode-only)

### Hybrid Search Architecture

Mnemosyne uses a three-strategy hybrid search system combining FTS5, graph traversal, and vector similarity:

![Hybrid Search Architecture](assets/diagrams/04-hybrid-search-light.svg#gh-light-mode-only)
![Hybrid Search Architecture](assets/diagrams/04-hybrid-search-dark.svg#gh-dark-mode-only)

### Data Flow

End-to-end data movement from user input through processing, storage, and retrieval:

![Data Flow](assets/diagrams/09-data-flow-light.svg#gh-light-mode-only)
![Data Flow](assets/diagrams/09-data-flow-dark.svg#gh-dark-mode-only)

### Four-Agent Framework

Specialized agents coordinate through Ractor actor supervision:

- **Orchestrator**: Prioritized work queue (0=highest), dependency tracking, 60s deadlock detection with cycle resolution
- **Optimizer**: Context budget allocation (40% critical, 30% skills, 20% project, 10% general), dynamic skill discovery, prefetching
- **Reviewer**: Quality gates (intent satisfied, tests passing, docs complete, no anti-patterns), DSPy-based semantic validation
- **Executor**: Work execution with timeout/retry, sub-agent spawning for parallel work, graceful failure with rollback

#### Multi-Agent Coordination Flow

The following diagram illustrates how the four agents interact during a typical work session:

![Multi-Agent Coordination](assets/diagrams/02-multi-agent-coordination-light.svg#gh-light-mode-only)
![Multi-Agent Coordination](assets/diagrams/02-multi-agent-coordination-dark.svg#gh-dark-mode-only)

### Autonomous Evolution

LLM-guided optimization runs during idle periods:

- **Consolidation**: Claude Haiku analyzes memory pairs for merge/supersede/keep-both decisions
- **Importance Recalibration**: Recency decay (e^(-age/30)), access boost (+0.1 per retrieval, max +2.0), graph proximity (+0.05 per neighbor)
- **Link Decay**: -1% strength per day inactive, access reinforcement, prune <0.2 strength
- **Archival**: Soft-delete memories with importance <2 AND age >90 days, or superseded memories after 7 days

### Technology Stack

**Core**: Rust 1.75+ (type safety, zero-cost abstractions), Tokio (async runtime), LibSQL (SQLite-compatible with vector support), PyO3 0.22 (Python bindings, 10-20x faster than subprocess)

**LLM**: Claude Haiku 4.5 for enrichment, linking, consolidation (<500ms typical, 4-5x cheaper than Sonnet)

**Protocols**: MCP (JSON-RPC 2.0 over stdio), SSE (real-time events), Ractor message passing

![Technology Stack](assets/diagrams/08-tech-stack-light.svg#gh-light-mode-only)
![Technology Stack](assets/diagrams/08-tech-stack-dark.svg#gh-dark-mode-only)

### gRPC Remote Access (v2.2.0)

The RPC feature provides production-ready gRPC server access enabling external applications to store, search, and manage memories remotely. Built on Tonic with Protocol Buffers for type-safe, high-performance access.

#### RPC Architecture

![gRPC Architecture](assets/diagrams/05-rpc-architecture-light.svg#gh-light-mode-only)
![gRPC Architecture](assets/diagrams/05-rpc-architecture-dark.svg#gh-dark-mode-only)

**Services**:

- **MemoryService**: 13 methods for CRUD operations, hybrid search (Recall, SemanticSearch, GraphTraverse), and streaming (RecallStream, ListMemoriesStream)
- **HealthService**: System monitoring with HealthCheck, GetStats, GetMetrics, GetMemoryUsage, StreamMetrics, and GetVersion

**Language Support**: Python, Go, Rust, JavaScript, or any gRPC-compatible language via Protocol Buffers

### Integrated Context Studio (ICS)

Terminal-based semantic editor with multi-panel UI, CRDT-based collaborative editing, and real-time validation:

![ICS Architecture](assets/diagrams/06-ics-architecture-light.svg#gh-light-mode-only)
![ICS Architecture](assets/diagrams/06-ics-architecture-dark.svg#gh-dark-mode-only)

### Dashboard Monitoring

Real-time web-based monitoring of multi-agent orchestration with 6-panel TUI showing memory metrics, context usage, work progress, and agent coordination:

![Dashboard Architecture](assets/diagrams/07-dashboard-architecture-light.svg#gh-light-mode-only)
![Dashboard Architecture](assets/diagrams/07-dashboard-architecture-dark.svg#gh-dark-mode-only)

## Comparison with Existing Systems

| Feature | Mnemosyne | MemGPT | Mem0 | LangChain Memory |
|---------|-----------|---------|------|------------------|
| Memory Model | Hybrid (FTS5 + Graph, Vector planned) | Virtual context (RAM/disk pages) | Graph nodes | Conversation buffers |
| Multi-Agent Coordination | **4-agent framework** | Single-agent focus | Limited (application layer) | None |
| Evolution System | **Autonomous (LLM-guided)** | Manual management | Limited automation | Manual cleanup |
| Integration | MCP + Hooks + CLI + Dashboard | Python library + API | REST API + SDKs | Python library |
| Implementation | Rust + Python bindings | Python | Python + Go | Python |
| Production Readiness | **702 tests, type safety** | Research/experimental | Beta (production-ready) | Production (stable) |

Mnemosyne treats memory and agents as unified concerns. Where MemGPT provides sophisticated single-agent memory management and Mem0 offers production-grade graph storage, Mnemosyne integrates persistent memory with multi-agent orchestration and autonomous evolution.

## Validation & Evidence

### Test Coverage

**702 passing tests** (100% pass rate) across categories:

- ~250 unit tests: Type system, storage operations, evolution algorithms, serialization
- ~150 integration tests: MCP server, orchestration, DSPy bridge, LLM service
- ~80 E2E tests: Human workflows, agent coordination, recovery scenarios
- ~50 specialized tests: File descriptor safety, process management, ICS integration

### Performance Metrics

Benchmarks from `tests/performance/`:

- Store memory: 2.25ms (includes async LLM enrichment dispatch)
- Get by ID: 0.5ms (direct UUID lookup)
- List recent: 0.88ms (indexed query)
- Hybrid search: 1.61ms (FTS5 + graph on 1K memories)
- Graph traversal: ~5ms (1-hop), ~12ms (2-hop)

### Production Readiness

Stability established through:

- [File descriptor leak prevention](https://github.com/rand/mnemosyne/commit/87b7a33) (commit 87b7a33): Hooks close all FDs, validation in test suite
- [Terminal corruption prevention](https://github.com/rand/mnemosyne/commit/eec1a33) (commit eec1a33): Clean process management, proper signal handling
- Robust error handling: Result<T,E> throughout, custom error types, graceful degradation

### Quality Gates

Multi-layered quality assurance process ensures production reliability. Every change must pass 8 validation gates:

![Quality Gates](assets/diagrams/10-quality-gates-light.svg#gh-light-mode-only)
![Quality Gates](assets/diagrams/10-quality-gates-dark.svg#gh-dark-mode-only)

### Code Validation

Complete validation matrix available: [validation.md](https://github.com/rand/mnemosyne/blob/v2.2.0/docs/whitepaper/validation.md)

Every technical claim maps to v2.2.0 source code and tests. Sample mappings:

- Sub-ms retrieval (0.88ms) → [src/storage/libsql.rs:420-450](https://github.com/rand/mnemosyne/blob/v2.2.0/src/storage/libsql.rs#L420-450) + [tests](https://github.com/rand/mnemosyne/blob/v2.2.0/tests/performance/storage_perf.rs#L89-110)
- 4-agent orchestration → [src/orchestration/mod.rs:89-150](https://github.com/rand/mnemosyne/blob/v2.2.0/src/orchestration/mod.rs#L89-150) + [tests](https://github.com/rand/mnemosyne/blob/v2.2.0/tests/orchestration_e2e.rs)

## Summary

Mnemosyne demonstrates that semantic memory and multi-agent orchestration form a unified system. The architecture delivers persistent context through hybrid search, multi-agent coordination via specialized agents with Ractor supervision, autonomous evolution through LLM-guided consolidation, and production integration via MCP protocol.

The system addresses fundamental challenges: context loss elimination (sessions maintain complete state), coordination infrastructure (shared memory enables debugging), cognitive load reduction (automatic context loading), and long-running workflow support (accumulation over weeks).

### Resources

- [Repository](https://github.com/rand/mnemosyne)
- [Full Whitepaper (Markdown)](https://github.com/rand/mnemosyne/blob/main/docs/whitepaper/whitepaper.md)
- [Validation Matrix](https://github.com/rand/mnemosyne/blob/v2.2.0/docs/whitepaper/validation.md)
- [Documentation](https://github.com/rand/mnemosyne/tree/v2.2.0/docs)

---

Mnemosyne v2.2.0 · November 8, 2025 · [MIT License](https://github.com/rand/mnemosyne/blob/v2.2.0/LICENSE)
