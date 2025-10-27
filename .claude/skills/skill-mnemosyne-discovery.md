---
name: skill-mnemosyne-discovery
description: Working on Mnemosyne memory system or related tasks
---

# Mnemosyne Discovery Gateway

**Scope**: Auto-discovers when working on Mnemosyne project or using memory system
**Lines**: ~150
**Last Updated**: 2025-10-27

## When to Use This Skill

**Auto-activates when**:
- Working in Mnemosyne repository
- Using memory tools (mnemosyne.recall, mnemosyne.remember, etc.)
- Implementing MCP protocol features
- Developing Rust storage backend
- Creating Python orchestration agents
- Testing memory system functionality
- Writing documentation for Mnemosyne

**Keywords that trigger activation**:
- mnemosyne, memory, recall, remember, semantic memory
- mcp server, json-rpc, protocol
- sqlite, fts5, full-text search
- namespace, importance, consolidation
- ooda loop, context preservation
- orchestrator, optimizer, reviewer, executor
- pyo3, rust bindings, tokio async

## Relevant Skills to Load

### Always Load

**1. mnemosyne-memory-management.md**
- Core memory operations (store, recall, consolidate)
- Importance scoring and namespace strategies
- OODA loop integration
- Best practices for memory usage

### Conditional Loading

**2. mnemosyne-context-preservation.md** (if working on context/session management)
- Context budget allocation
- Session handoff protocols
- Pre-emptive snapshots at 75% utilization
- ACE framework integration

**3. mnemosyne-rust-development.md** (if working on Rust code)
- Codebase structure and patterns
- SQLite + FTS5 implementation
- Tokio async patterns
- PyO3 bindings
- Testing strategies

**4. mnemosyne-mcp-protocol.md** (if working on MCP server)
- JSON-RPC 2.0 protocol
- Tool implementation patterns
- Request/response handling
- Error codes and schema validation

### From cc-polymath (Global)

**Load based on specific task**:

**Rust Development**:
- `rust/async-programming.md` - Tokio patterns
- `rust/error-handling.md` - Result types
- `database/sqlite-optimization.md` - Query optimization
- `testing/rust-testing.md` - Test patterns

**API/Protocol Design**:
- `api/json-rpc.md` - JSON-RPC 2.0 specifics
- `api/rest-principles.md` - API design patterns
- `api/error-handling.md` - Error responses

**Testing**:
- `testing/integration-testing.md` - Integration test patterns
- `testing/property-based-testing.md` - Property testing
- `testing/test-fixtures.md` - Test data management

**Python/Orchestration**:
- `python/async-patterns.md` - Python asyncio
- `ml/claude-sdk.md` - Claude Agent SDK patterns
- `collaboration/agent-coordination.md` - Multi-agent systems

## Project Context

### Mnemosyne is:
- **Rust-based** memory system with SQLite + FTS5 storage
- **MCP server** exposing 8 OODA-aligned tools via JSON-RPC 2.0
- **Python orchestration** layer with multi-agent coordination
- **Claude Code plugin** for persistent semantic memory

### Architecture:
```
Claude Code (Multi-Agent System)
  ↓ MCP Protocol (JSON-RPC over stdio)
Mnemosyne MCP Server (Rust + Tokio)
  ↓
Storage Layer (SQLite + FTS5)
  ↓
LLM Service (Claude Haiku for enrichment)
```

### Key Components:
- **Storage**: SQLite with FTS5 full-text search and graph traversal
- **LLM Service**: Claude Haiku for memory enrichment
- **Namespace Detector**: Git + CLAUDE.md for project context
- **Orchestration**: Python multi-agent system using Claude Agent SDK

## Common Tasks and Relevant Skills

### Memory Operations (User-facing)
**Skills**: mnemosyne-memory-management.md

Tasks:
- Storing new memories
- Searching existing memories
- Building context from memory graph
- Consolidating similar memories

### Context Management (Agent-facing)
**Skills**: mnemosyne-context-preservation.md, mnemosyne-memory-management.md

Tasks:
- Session start/end protocols
- Context budget allocation
- Pre-emptive snapshots
- Agent coordination

### Rust Development
**Skills**: mnemosyne-rust-development.md + cc-polymath Rust skills

Tasks:
- Implementing new storage features
- Optimizing database queries
- Adding PyO3 bindings
- Writing Rust tests

### MCP Server Development
**Skills**: mnemosyne-mcp-protocol.md + cc-polymath API skills

Tasks:
- Implementing new tools
- Handling JSON-RPC requests
- Schema validation
- Error handling

### Python Orchestration
**Skills**: All Mnemosyne skills + cc-polymath Python/ML skills

Tasks:
- Creating new agents
- Implementing coordination logic
- Context optimization
- Quality validation

## Skill Loading Strategy

### Priority Order:
1. **Project-local** Mnemosyne skills (in `.claude/skills/`)
2. **Global** cc-polymath skills (in `~/.claude/plugins/cc-polymath/skills/`)

### Context Budget:
- **40% Critical**: Essential project context + work plan
- **30% Skills**: Loaded skills (max 7)
  - 2-3 Mnemosyne-specific skills
  - 3-5 cc-polymath skills
- **20% Project**: Files and code
- **10% General**: Miscellaneous

### Load Patterns:

**Memory/OODA work** (most common):
- mnemosyne-memory-management.md
- mnemosyne-context-preservation.md
- (5 slots for cc-polymath as needed)

**Rust backend work**:
- mnemosyne-rust-development.md
- rust/async-programming.md (cc-polymath)
- database/sqlite-optimization.md (cc-polymath)
- testing/rust-testing.md (cc-polymath)
- (3 slots remaining)

**MCP protocol work**:
- mnemosyne-mcp-protocol.md
- mnemosyne-rust-development.md
- api/json-rpc.md (cc-polymath)
- (4 slots remaining)

**Python orchestration work**:
- mnemosyne-memory-management.md
- mnemosyne-context-preservation.md
- python/async-patterns.md (cc-polymath)
- ml/claude-sdk.md (cc-polymath)
- (3 slots remaining)

## Discovery Triggers

### File Patterns:
- `src/**/*.rs` → Load Rust skills
- `src/orchestration/**/*.py` → Load Python/orchestration skills
- `src/mcp/**/*.rs` → Load MCP protocol skills
- `tests/**/*.{rs,py}` → Load testing skills

### Content Patterns:
- "mnemosyne.recall" → Load memory management
- "JSON-RPC" → Load MCP protocol
- "SqliteStorage" → Load Rust development
- "OptimizerAgent" → Load context preservation
- "OODA loop" → Load memory management

### Command Patterns:
- `/memory-*` → Load memory management
- `cargo test` → Load Rust + testing skills
- `pytest` → Load Python + testing skills
- `cargo run -- serve` → Load MCP server skills

## Quick Reference

### Mnemosyne-Specific Skills:
1. mnemosyne-memory-management.md - Memory operations and OODA loop
2. mnemosyne-context-preservation.md - Context budgets and session handoffs
3. mnemosyne-rust-development.md - Rust patterns and architecture
4. mnemosyne-mcp-protocol.md - MCP server implementation

### Key cc-polymath Skills:
- rust/* - Rust language and patterns
- api/* - API and protocol design
- testing/* - Testing strategies
- database/* - Database optimization
- python/* - Python async patterns
- ml/* - Claude Agent SDK

### Documentation:
- `README.md` - Overview and quick start
- `ARCHITECTURE.md` - System architecture
- `MCP_SERVER.md` - MCP tool reference
- `ROADMAP.md` - Development phases
- `CLAUDE.md` - Multi-agent orchestration guidelines

## Next Steps After Discovery

1. **Identify task type** (memory ops, Rust dev, MCP, orchestration)
2. **Load appropriate Mnemosyne skill** (1-2 skills)
3. **Scan cc-polymath catalog** for complementary skills (3-5 skills)
4. **Verify context budget** (stay under 30% for skills)
5. **Begin task** with full skill context loaded
