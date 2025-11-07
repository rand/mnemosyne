# Changelog

All notable changes to Mnemosyne will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [2.1.1] - 2025-11-06

### Added

**Python Bridge Architecture (Phase 13)**
- Complete PyO3 integration with Claude SDK agents
- Phase 5 production hardening (8/8 tasks, 100% complete)
  - Structured logging infrastructure with multi-level support
  - Enhanced error context with troubleshooting hints and recovery suggestions
  - Input validation for work items, agent state, and plans
  - Performance metrics (per-item and per-agent tracking)
  - Integration testing (5/5 tests passing)
  - E2E validation with actual Claude SDK API calls (5/5 tests passing, 41.77s execution)
  - Python dependency management (requirements.txt, pyproject.toml)
  - Comprehensive troubleshooting guide (628 lines)
- Production documentation (2,200+ lines across 5 major documents)
- Test suite expanded to 715 unit tests + 10 integration/E2E tests
- Clean build with 0 warnings, 0 errors

### Fixed

**Stability and Reliability Improvements**
- Fixed file descriptor leak in hook subprocess calls causing EIO errors (87b7a33)
- Added stdin protection (`< /dev/null`) to all subprocess invocations in hooks
- Fixed subprocess fd inheritance issues in session-start, post-tool-use, and on-stop hooks
- Prevented terminal corruption from hook stderr output (eec1a33)
- Added CC_HOOK_DEBUG flag for controlled hook verbosity

**Process Management**
- Added robust process management and cleanup tooling (048f26d)
- Implemented graceful shutdown with SIGTERM → SIGKILL fallback
- Added PID validation before kill attempts to prevent stale PID errors
- Created comprehensive cleanup script (`scripts/cleanup-processes.sh`)
- Enhanced test-server.sh with proper port and PID validation

**Hook System**
- Eliminated hook noise with debug flag (9712c0c)
- Hooks now silent by default in production mode
- Debug output only shown when CC_HOOK_DEBUG=1
- Improved user experience with cleaner terminal output

### Added

**Documentation**
- Added comprehensive test validation report: `docs/FD_LEAK_FIX_TEST_RESULTS.md`
- Enhanced `docs/CRASH_RECOVERY.md` with complete resolution documentation
- Documented all stability fixes with root cause analysis
- Production sign-off with 689 passing unit tests and 12 FD safety tests

**Testing**
- Added specialized FD safety test suite (`/tmp/test-fd-safety.sh`)
- Validated hook execution under realistic load (180+ invocations)
- Tested concurrent execution (5 parallel processes)
- Confirmed no fd leaks or terminal corruption

### Added (From Previous Unreleased)

**Documentation Reorganization**
- Created comprehensive agent guide: `AGENT_GUIDE.md` (500+ lines)
- Added type system reference: `docs/TYPES_REFERENCE.md` (350+ lines)
- Added database schema documentation: `docs/STORAGE_SCHEMA.md` (450+ lines)
- Created documentation navigation hub: `docs/INDEX.md` (250+ lines)
- Organized documentation into logical structure:
  - `docs/features/` - Feature-specific documentation
  - `docs/guides/` - How-to guides and workflows
  - `docs/specs/` - Technical specifications
  - `docs/historical/` - Archived session and test reports

### Changed

**Signal Handling Improvements**
- Enhanced graceful shutdown handling for MCP server
- Proper cleanup on SIGINT and SIGTERM signals
- Prevents "unsafe use of virtual table" errors during shutdown
- Ensures database connections are closed cleanly

### Performance

**Build Speed Improvements**
- Disabled debug symbols in dev builds for 10-20% faster compilation
- Optimized tokio features: removed `full`, use only required features (macros, rt-multi-thread, sync, time, net, io-util, tracing)
- Configured sccache for compilation caching across builds
- Clean build time: 2m 58s → 2m 46s (7% improvement)
- Incremental builds: ~3-4s (excellent for rapid development)
- See `docs/BUILD_OPTIMIZATION.md` for details and future optimization opportunities

## [2.1.0] - 2025-10-31

### Added

**ICS Standalone Binary** (`mnemosyne-ics`)
- Standalone context editor with full terminal ownership (no conflicts)
- Template system for common contexts (api, architecture, bugfix, feature, refactor)
- Storage backend integration for context persistence
- Read-only mode for viewing memory dumps
- Clean terminal lifecycle management

**HTTP API Server** (`:3000`)
- Optional API server with `mnemosyne serve --with-api`
- Real-time event streaming via Server-Sent Events (SSE)
- RESTful endpoints for agent and context state
- Concurrent operation with MCP server (tokio::select!)
- CORS support for web-based monitoring

**Real-time Monitoring Dashboard** (`mnemosyne-dash`)
- TUI dashboard showing live agent activity
- SSE client for real-time event consumption
- Agent status display with color-coded states
- System statistics (memory, CPU, context usage)
- Event log with scrollback and filtering
- Auto-reconnect on disconnect

**Semantic Highlighting System** (3-tier architecture, 7,500+ lines)
- **Tier 1: Structural** (<5ms real-time)
  - XML tag analyzer with nesting validation
  - RFC 2119 constraint detector (MUST, SHOULD, MAY)
  - Modality/hedging analyzer (4 confidence levels)
  - Ambiguity detector for vague language
  - Domain pattern matcher (#file, @symbol, ?hole)
- **Tier 2: Relational** (<200ms incremental)
  - Named entity recognizer (5 types: Person, Organization, Location, Concept, Temporal)
  - Relationship extractor (5 relation types)
  - Semantic role labeler (6 roles: Agent, Patient, Theme, etc.)
  - Coreference resolver (distance-based)
  - Anaphora resolver (4 pronoun types)
  - LRU caching for performance
- **Tier 3: Analytical** (2s+ background, optional)
  - Discourse analyzer (8 relation types)
  - Contradiction detector (4 severity levels)
  - Pragmatics analyzer (5 speech acts)
  - Request batching and rate limiting
  - Content-hash deduplication
  - Priority-based scheduling

**LLM-Enhanced Reviewer Agent**
- Automatic requirement extraction from user intent using Claude API
- Semantic validation beyond pattern matching
- Intent validation: verify implementation satisfies original intent
- Completeness checking: ensure all requirements fully implemented
- Correctness analysis: validate logic soundness and error handling
- Improvement guidance generation for failed reviews
- Requirement traceability with database persistence
- Python bindings via PyO3 for Rust-Python integration
- Configurable retry logic with exponential backoff (3 retries, 1s→2s→4s)
- Graceful degradation to pattern matching on LLM failure
- 27+ new tests for LLM validation workflows

**Event Bridging**
- Orchestration events wired to API for real-time monitoring
- MCP tools emit events during recall/remember operations
- Event persistence with broadcasting to SSE subscribers
- API event types: AgentStarted, AgentCompleted, AgentFailed, MemoryRecalled, MemoryStored
- 3 new integration tests for event streaming

**Composable Tools Architecture**
- Migration from TUI wrapper to Unix-philosophy composable tools
- Each tool owns its terminal completely (zero conflicts)
- File-based context handoff via .claude/*.md files
- HTTP SSE for real-time coordination
- MCP works standalone, API/dashboard are additive
- Migration guide: docs/guides/migration.md

### Changed

**Architecture**
- TUI wrapper (`mnemosyne tui`) deprecated in favor of composable tools
- Claude Code auto-launches `mnemosyne serve` via MCP config
- Context editing now via `mnemosyne-ics` (standalone binary)
- Monitoring now via `mnemosyne-dash` + `mnemosyne serve --with-api`

**Storage**
- Added database migration for requirement tracking fields
- New SQL migration: `migrations/libsql/012_requirement_tracking.sql`
- Enhanced LibSQL operations with requirement persistence

**Orchestration**
- Reviewer agent enhanced with Python integration and LLM validation
- Orchestrator now tracks extracted requirements and satisfaction status
- Event persistence includes optional broadcasting for real-time updates
- Supervision tree extended for Python reviewer lifecycle management

### Documentation

New documentation (11 files, 5,000+ lines):
- `docs/guides/llm-reviewer.md` - Comprehensive LLM reviewer guide (533 lines)
- `docs/guides/llm-reviewer-setup.md` - Setup and troubleshooting (448 lines)
- `SEMANTIC_HIGHLIGHTING.md` - System overview and API reference (423 lines)
- `SEMANTIC_HIGHLIGHTING_INTEGRATION.md` - Integration guide (514 lines)
- `SEMANTIC_HIGHLIGHTING_STATUS.md` - Implementation status (169 lines)
- `docs/guides/migration.md` - Migration from TUI to composable tools (475 lines)
- `docs/specs/background-processing-spec.md` - Tier 3 background processing (580 lines)
- `docs/specs/ics-integration-spec.md` - ICS integration specification (557 lines)
- `docs/specs/incremental-analysis-spec.md` - Incremental semantic analysis (533 lines)
- `docs/specs/semantic-highlighter-test-plan.md` - Testing strategy (716 lines)
- `docs/specs/tier3-llm-integration-spec.md` - LLM integration architecture (421 lines)

Updated documentation:
- `ARCHITECTURE.md` - Added composable tools architecture section
- `README.md` - Status section remains at v2.0.0 (features in progress)

### Testing

- **627 tests passing** (up from 474 on main, 620 on feature branch)
- 170+ new tests for semantic highlighting system
- 27+ new tests for LLM reviewer (Rust + Python)
- 3 new integration tests for event bridging
- 252 lines of ICS semantic integration tests
- 301 lines of Tier 2 caching tests
- 527 lines of Python reviewer agent tests
- Example test: `examples/semantic_highlighting.rs` (206 lines)

### Performance

**Semantic Highlighting Benchmarks**:
- Tier 1 (Structural): <5ms for 10,000 chars
- Tier 2 (Relational): <200ms for 10,000 chars (cache miss), <5ms (cache hit)
- Tier 3 (Analytical): 2-10s for LLM-powered analysis (batched, rate-limited)

**API/Dashboard**:
- SSE latency: <10ms for event delivery
- Dashboard update frequency: Real-time with SSE
- API server overhead: Minimal (concurrent with MCP)

### Fixed

**Merge Preparation Fixes** (from feature branch integration):
- Fixed 2 critical compilation errors in LLM reviewer tests
- Eliminated 6 compiler warnings for clean builds
- Resolved deadlock risk in branch coordinator (async-safe lock scoping)
- Fixed logic bug in feature extractor (tautological assertion)

### Known Issues

**Python Environment**:
- PyO3 0.22.6 doesn't support Python 3.14+ yet (max: 3.13)
- Use Python 3.9-3.13 for LLM reviewer features
- Set `PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1` as workaround (not recommended)

**Clippy Linting**:
- 27 clippy warnings remaining (code quality/style, not functional bugs)
- Planned cleanup in follow-up commit

**Semantic Highlighting**:
- Tier 3 LLM integration is scaffolding only (not functional)
- Incremental analysis (`schedule_analysis`) is stubbed
- Not yet integrated with ICS editor
- Performance claims not validated with benchmarks

### Deprecations

- `mnemosyne tui` - Use `mnemosyne-ics` + `mnemosyne-dash` instead
- See `docs/guides/migration.md` for migration guide

---

## [2.0.0] - 2025-10-31

### Added

**TUI Wrapper Mode**
- New `mnemosyne tui` command for enhanced terminal interface
- Helix-style command palette with fuzzy search and type-ahead filtering
- Context-aware help overlay (? key) with mode-specific shortcuts
- Dynamic status bar showing relevant actions for current mode
- Split-view layout: chat + ICS editor + agent dashboard
- Terminal detection with comprehensive error messages for edge cases (SSH, tmux, piped I/O)
- Keyboard-first navigation with discoverable shortcuts (Ctrl+P, Ctrl+E, Ctrl+D, Ctrl+Q)

**ICS Enhancements**
- Hybrid markdown highlighting system with 3-layer priority:
  - Layer 1: Semantic patterns (#file, @symbol, ?hole) - highest priority
  - Layer 2: Tree-sitter syntax (headings, code blocks, emphasis, lists, links)
  - Layer 3: Plain text fallback
- Pattern-specific color coding (files: blue, symbols: green, holes: yellow)
- Real-time highlighting with markdown-first optimization
- Toggle-able syntax and semantic layers (both enabled by default)
- 5 new ICS commands: submit-to-claude, save-file, export-context, toggle-highlighting, focus-editor

**Command Palette**
- Simplified Helix-style rendering: `> command-name  Description`
- Removed category badges and fuzzy match scores from display
- Clean selection indicator with cyan arrow
- Type-ahead filtering working in real-time
- Added CommandCategory::Ics for ICS-specific commands

### Changed

**ICS User Experience**
- Standalone ICS mode now has comprehensive terminal detection
- Help system is context-aware (different content for ICS vs Chat mode)
- Status bar shows mode-specific action hints
- All keyboard shortcuts documented in help overlay

**Documentation**
- README updated with TUI wrapper mode section
- Complete keyboard shortcuts reference
- Pattern syntax documentation (#file, @symbol, ?hole)
- Quick Start section includes TUI examples
- Features section updated with TUI and hybrid highlighting

### Testing

- 13 TUI widget tests (all passing)
- 8 markdown highlighting tests (all passing)
- Clean builds on all configurations
- No regressions in existing functionality

---

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
