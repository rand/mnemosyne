# Changelog

All notable changes to Mnemosyne will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - 2025-10-27

### Added

**Multi-Agent Orchestration**
- Complete 4-agent architecture (Orchestrator, Optimizer, Reviewer, Executor)
- PyO3 bindings for 10-20x performance improvement (2.25ms store, 0.88ms list, 1.61ms search)
- Agent coordination with zero-copy data passing and dependency-aware scheduling
- Context preservation at 75% threshold with automatic snapshots
- Work Plan Protocol with 4 phases (Prompt→Spec→Full Spec→Plan→Artifacts)

**Security & Secrets Management**
- Age-encrypted secrets storage (X25519 + ChaCha20-Poly1305)
- Three-tier key lookup: environment variable → age file → OS keychain
- Secure API key management with `mnemosyne secrets` commands
- Cross-platform keychain support (macOS, Windows, Linux)

**Automatic Memory Capture**
- Session-start hook: Auto-loads project context at session beginning
- Pre-compact hook: Preserves important context before conversation compaction
- Post-commit hook: Links git commits to architectural decisions
- Zero-friction memory capture with keyword-based filtering

**Core Memory System**
- LibSQL/Turso storage with native vector support
- Hybrid search combining FTS5 keyword search + graph traversal
- Project-aware namespace isolation (global/project/session)
- Importance decay with exponential recency (30-day half-life)
- Memory consolidation with LLM-guided merge/supersede decisions
- Graph traversal with recursive CTE for relationship discovery

**MCP Server Integration**
- 8 OODA-aligned tools via JSON-RPC over stdio
- Tools: recall, list, graph, context, remember, consolidate, update, delete
- Automatic LLM enrichment (Claude Haiku) for summaries, tags, classifications
- Semantic link generation between related memories

**Claude Code Integration**
- 6 slash commands: /memory-store, /memory-search, /memory-context, /memory-list, /memory-export, /memory-consolidate
- Skills integration with cc-polymath (354 comprehensive skills)
- 5 Mnemosyne-specific skills with +10% relevance bonus
- Progressive skill loading by Optimizer agent (max 7 skills, 30% context budget)

**Installation & Setup**
- Safe, non-destructive installation script with backup creation
- Smart MCP config merging (preserves existing servers)
- Comprehensive uninstallation with data preservation by default
- Support for project-level and global MCP configuration

### Changed

**Storage Migration**
- Migrated from SQLite to LibSQL for better performance and native vectors
- Optimized database schema with proper indexes
- Added migration system for schema evolution

**Security Improvements**
- Updated all scripts to use secure key management system
- Removed direct keychain access in favor of unified secrets management
- Environment variable priority over encrypted files

**Performance Optimizations**
- PyO3 bindings provide 10-20x speedup for Python orchestration
- Optimized hybrid search with weighted scoring
- Efficient graph traversal with depth limits

### Fixed

**Critical Fixes**
- Keychain storage silently failing on macOS (P0-001)
- Shared LLM service instance reduces keychain prompts
- Agent coordination test failures
- Async/await issues in storage operations

**Script Security**
- Test scripts now use secure key management
- No more direct keychain access bypassing encryption
- Cross-platform compatibility improvements

**Documentation**
- README updated with accurate test counts
- Installation instructions clarified
- MCP integration properly documented

### Documentation

- 15 comprehensive markdown files (6000+ lines total)
- Complete architecture documentation (ARCHITECTURE.md)
- Installation guide (INSTALL.md)
- Secrets management guide (SECRETS_MANAGEMENT.md)
- Multi-agent orchestration guide (ORCHESTRATION.md)
- Hooks testing guide (HOOKS_TESTING.md)
- MCP server API reference (MCP_SERVER.md)
- Complete roadmap with 10/10 phases (ROADMAP.md)
- Comprehensive audit report (AUDIT_REPORT.md)
- Implementation plans for v2.0 features (docs/v2/)

### Performance

| Metric | v1.0.0 | Target |
|--------|--------|--------|
| Retrieval latency (p95) | ~50ms | <200ms ✓ |
| Storage latency (p95) | ~300ms | <500ms ✓ |
| Memory usage (idle) | ~30MB | <100MB ✓ |
| Database size | ~800KB/1000 memories | ~1MB/1000 ✓ |
| PyO3 store operations | 2.25ms | <3ms ✓ |
| PyO3 list operations | 0.88ms | <1ms ✓ |
| PyO3 search operations | 1.61ms | <5ms ✓ |

### Testing

- 30+ unit tests (all passing)
- 8 integration tests (all passing)
- 5 LLM tests (optional, require API key)
- 9 Python orchestration tests (all passing)
- 3 E2E workflow tests
- PyO3 performance benchmarks

---

## [0.1.0] - 2025-10-20

### Added

- Initial development release
- Core memory storage and retrieval
- FTS5 keyword search
- Graph traversal
- Basic MCP server integration
- SQLite storage backend
- Namespace detection (git-aware)
- LLM enrichment with Claude Haiku
- CLI commands for memory operations

### Known Issues

- SQLite performance limitations (migrated to LibSQL in v1.0.0)
- Manual memory capture required (hooks added in v1.0.0)
- No multi-agent orchestration (added in v1.0.0)
- Basic secrets management (enhanced in v1.0.0)

---

## Future Releases

See [ROADMAP.md](ROADMAP.md) for planned features:

### v1.1 (Planned - 1 week)
- Configurable database path
- Dead code removal
- Hooks improvements
- Documentation cleanup

### v2.0 (Planned - 22 weeks)
- Vector similarity search with embeddings
- Background memory evolution (auto-consolidation)
- Advanced agent features (role-based views, prefetching)
- VSCode extension with memory browser

---

[1.0.0]: https://github.com/rand/mnemosyne/releases/tag/v1.0.0
[0.1.0]: https://github.com/rand/mnemosyne/releases/tag/v0.1.0
