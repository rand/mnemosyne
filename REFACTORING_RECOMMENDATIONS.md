# Refactoring Recommendations

**Date**: 2025-10-31
**Status**: Recommendations for v2.2+
**Priority**: Medium (code works, but organization can be improved)

---

## Executive Summary

Mnemosyne v2.1.0 is **production-ready** and **functionally complete**. However, two files have grown large enough to warrant refactoring for long-term maintainability:

- **src/main.rs**: 2,051 lines
- **src/storage/libsql.rs**: 3,388 lines

This document provides **detailed, actionable recommendations** for refactoring these files in a future release (v2.2+), with estimated effort and risk assessment.

**Recommendation**: Defer major refactoring to v2.2+ to avoid introducing regressions in the stable v2.1.0 release.

---

## Part 1: main.rs Refactoring (Estimated: 8-12 hours)

### Current Structure

```
src/main.rs (2,051 lines)
├── Utility functions (get_db_path, process_structured_plan, etc.)
├── CLI types (Cli, Commands, ModelsAction, EvolveJob, etc.)
├── main() function
└── 15 command handlers (inline in match statement)
```

### Proposed Structure

```
src/
├── main.rs (200 lines)                    # Entry point, CLI parsing, dispatch
├── cli/
│   ├── mod.rs                             # CLI module exports
│   ├── types.rs                           # Cli, Commands, and subcommand enums
│   ├── util.rs                            # Utility functions (get_db_path, etc.)
│   └── commands/
│       ├── mod.rs                         # Command exports
│       ├── serve.rs                       # MCP server commands
│       ├── memory.rs                      # Remember, recall, forget
│       ├── search.rs                      # Search command
│       ├── list.rs                        # List command
│       ├── consolidate.rs                 # Consolidate command
│       ├── export.rs                      # Export/import commands
│       ├── graph.rs                       # Graph visualization
│       ├── models.rs                      # Model management
│       ├── evolution.rs                   # Evolution jobs
│       ├── config.rs                      # Configuration management
│       ├── secrets.rs                     # Secrets management
│       └── orchestrate.rs                 # Orchestration commands
```

### Refactoring Steps

1. **Phase 1: Create module structure** (1-2 hours)
   ```bash
   mkdir -p src/cli/commands
   touch src/cli/{mod.rs,types.rs,util.rs}
   touch src/cli/commands/{mod.rs,serve.rs,memory.rs,search.rs,...}
   ```

2. **Phase 2: Move CLI types** (1 hour)
   - Move `Cli`, `Commands`, and all subcommand enums to `src/cli/types.rs`
   - Update imports in `main.rs`
   - Run `cargo check` to verify

3. **Phase 3: Move utility functions** (30 minutes)
   - Move `get_db_path`, `get_default_db_path`, `process_structured_plan`, etc. to `src/cli/util.rs`
   - Update imports
   - Run `cargo check`

4. **Phase 4: Extract command handlers** (4-6 hours)
   - For each command, create handler function in appropriate module:
     ```rust
     // src/cli/commands/memory.rs
     pub async fn handle_remember(
         storage: Arc<LibsqlStorage>,
         embeddings: Arc<EmbeddingService>,
         content: String,
         importance: u8,
         // ... other args
     ) -> Result<()> {
         // Extracted logic from main.rs match arm
     }
     ```
   - Update `main.rs` to call handlers:
     ```rust
     Some(Commands::Remember { content, importance, ... }) => {
         commands::memory::handle_remember(storage, embeddings, content, importance, ...).await
     }
     ```
   - Extract one command at a time
   - Run `cargo test` after each extraction
   - Commit after each successful extraction

5. **Phase 5: Final cleanup** (1-2 hours)
   - Simplify `main.rs` to pure dispatch logic
   - Add module documentation
   - Run full test suite
   - Update ARCHITECTURE.md if needed

### Risk Assessment

**Risks**:
- High potential for introducing bugs if not done carefully
- Breaking changes to imports (low user impact since main.rs is not a library)
- Test failures if command logic is incorrectly extracted

**Mitigation**:
- Extract one command at a time
- Run `cargo test` after each extraction
- Commit after each successful step
- Keep a rollback plan (git branch)

**Recommended Approach**:
- Do NOT refactor in v2.1.0 (stable release)
- Plan for v2.2.0 with dedicated testing time
- Use feature branch: `feature/refactor-cli-structure`

---

## Part 2: libsql.rs Refactoring (Estimated: 10-16 hours)

### Current Structure

```
src/storage/libsql.rs (3,388 lines)
├── LibsqlStorage implementation
├── Memory CRUD operations
├── Search and query methods
├── Link management
├── Agent state management
├── Work item management
├── Import/export logic
├── Migration logic
└── Tests
```

### Proposed Structure

```
src/storage/libsql/
├── mod.rs                                 # Public interface, LibsqlStorage struct
├── init.rs                                # Initialization, migration
├── memory.rs                              # Memory CRUD operations
├── search.rs                              # Search and vector queries
├── links.rs                               # Link management
├── agents.rs                              # Agent state persistence
├── workitems.rs                           # Work item management
├── import_export.rs                       # Import/export logic
├── queries.rs                             # Common SQL queries
└── tests.rs                               # Test module (or tests/ subdirectory)
```

### Refactoring Steps

1. **Phase 1: Create module structure** (1 hour)
   ```bash
   mkdir src/storage/libsql
   touch src/storage/libsql/{mod.rs,init.rs,memory.rs,search.rs,...}
   ```

2. **Phase 2: Move struct definition and trait impl** (1-2 hours)
   - Move `LibsqlStorage` struct to `mod.rs`
   - Keep trait implementation stubs
   - Ensure it compiles with `cargo check`

3. **Phase 3: Extract modules one at a time** (6-10 hours)
   - **memory.rs**: Extract `create_memory`, `get_memory`, `update_memory`, `delete_memory`, `list_memories`
   - **search.rs**: Extract `search_memories`, `vector_search`, `semantic_search`
   - **links.rs**: Extract `create_link`, `get_links`, `update_link_strength`
   - **agents.rs**: Extract agent state methods
   - **workitems.rs**: Extract work item methods
   - **import_export.rs**: Extract import/export logic
   - **init.rs**: Extract initialization and migration logic

   For each module:
   ```rust
   // src/storage/libsql/memory.rs
   use super::*;

   impl LibsqlStorage {
       pub async fn create_memory(&self, memory: Memory) -> Result<MemoryId> {
           // Extracted logic
       }
   }
   ```

4. **Phase 4: Move tests** (1-2 hours)
   - Move tests to `tests.rs` or `tests/` directory
   - Ensure all tests still pass

5. **Phase 5: Cleanup** (1-2 hours)
   - Remove old `libsql.rs` file
   - Update imports throughout codebase
   - Update documentation

### Risk Assessment

**Risks**:
- HIGH: This is a critical storage layer - bugs here affect data integrity
- Breaking tests due to visibility changes (pub vs private methods)
- Performance regressions if SQL queries are modified
- Import complexity (circular dependencies between modules)

**Mitigation**:
- Use `#[cfg(test)]` helpers to maintain test access
- Run FULL test suite after each module extraction
- Use `pub(crate)` for internal APIs
- Keep SQL queries identical during extraction
- Test with production database before release

**Recommended Approach**:
- **CRITICAL**: Do NOT refactor in v2.1.0
- Plan for v2.2.0 or v2.3.0 with extensive testing
- Use feature branch: `feature/refactor-libsql-modules`
- Consider adding integration tests before refactoring
- Review with team before merging

---

## Part 3: Deprecated Code Analysis

### TUI Code Status

The TUI (Terminal UI) code in `src/tui/` is **not deprecated** - it's actively used and functional:

```
src/tui/
├── app.rs                                 # Active - TUI application
├── mod.rs                                 # Active - Module exports
└── widgets.rs                             # Active - Custom widgets
```

**Analysis**:
- No deprecated code found in TUI
- All TUI components are integrated and working
- Tests exist and pass (620/620)

**Recommendation**: No action needed. TUI is a first-class interface alongside CLI and MCP.

### Other Deprecated/Unused Code

Searched for obvious deprecation markers:
```bash
grep -r "#\[deprecated\]" src/
grep -r "DEPRECATED" src/
grep -r "@deprecated" src/
```

**Result**: No deprecated code markers found.

**Potential Candidates for Future Deprecation**:
1. **src/services/dspy_llm.rs** - Incomplete DSPy integration (3 TODOs)
   - Status: Experimental, not used in production
   - Recommendation: Complete or mark as experimental in v2.2+

2. **Old embedding service** (if exists)
   - Need to verify if v1.0 embeddings service is still needed
   - Check if newer service has replaced it

### Unused Dependencies

Run dependency analysis:
```bash
cargo machete                              # Check for unused dependencies
cargo tree --duplicates                    # Check for duplicate versions
```

**Recommendation**: Add to v2.2.0 cleanup checklist.

---

## Part 4: Implementation Priority

### High Priority (v2.2.0)
1. ✅ None - v2.1.0 is production-ready

### Medium Priority (v2.2.0 or v2.3.0)
1. **main.rs refactoring** (8-12 hours)
   - Reason: Improves maintainability, low risk
   - Impact: Easier to add new commands
   - Prerequisites: None

2. **Dependency cleanup** (2-4 hours)
   - Reason: Reduce binary size, improve compile times
   - Impact: Faster builds
   - Prerequisites: None

### Low Priority (v2.3.0+)
1. **libsql.rs refactoring** (10-16 hours)
   - Reason: Critical code, high risk, working well
   - Impact: Easier to maintain storage layer
   - Prerequisites: Comprehensive integration tests

---

## Part 5: Refactoring Checklist

Before starting any refactoring:

```
[ ] All tests passing (cargo test)
[ ] No clippy warnings (cargo clippy)
[ ] No TODO markers in affected code
[ ] Feature branch created
[ ] Backup/tag of stable version
[ ] Test plan documented
[ ] Rollback plan documented
[ ] Code review scheduled
```

During refactoring:

```
[ ] Extract one module/function at a time
[ ] Commit after each successful step
[ ] Run cargo test after each commit
[ ] Run cargo clippy after each commit
[ ] Update documentation as you go
[ ] Preserve git blame history where possible
```

After refactoring:

```
[ ] Full test suite passes (cargo test)
[ ] Integration tests pass
[ ] No new clippy warnings
[ ] Documentation updated
[ ] ARCHITECTURE.md updated
[ ] CHANGELOG.md entry added
[ ] Code review completed
[ ] Merged to main
[ ] Tag new version
```

---

## Conclusion

**v2.1.0 Status**: ✅ Production-ready, no blocking issues

**Refactoring Recommendations**:
1. **Defer all major refactoring to v2.2+** to maintain stability
2. **Prioritize main.rs refactoring** (lower risk, high value)
3. **Defer libsql.rs refactoring** (higher risk, working well)
4. **No deprecated code found** - TUI and all components are active

**Estimated Total Effort**:
- main.rs refactoring: 8-12 hours
- libsql.rs refactoring: 10-16 hours
- Testing and validation: +50% time
- **Total**: 27-42 hours of focused work

**Risk Assessment**: Medium risk for main.rs, High risk for libsql.rs

**Recommendation**: Schedule refactoring for v2.2.0 or later, with dedicated testing time and no pressure to rush.

---

## Next Steps

1. ✅ Document recommendations (this file)
2. 📋 Create GitHub issues for tracking:
   - Issue: "Refactor main.rs into cli/ modules (v2.2+)"
   - Issue: "Refactor libsql.rs into storage/libsql/ modules (v2.3+)"
3. 📅 Schedule refactoring in roadmap
4. 🎯 Focus on v2.1.0 release and stability
5. 🔄 Revisit refactoring plan in v2.2.0 planning phase
