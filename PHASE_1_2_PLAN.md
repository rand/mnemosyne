# Phase 1.2: Split main.rs into CLI Modules

## Status: ✅ Ready to Execute

## Current State
- **File**: `src/main.rs` (2,971 lines)
- **Structure**:
  - Lines 1-512: Helper functions
  - Lines 513-949: CLI/enum definitions
  - Lines 950-2971: Main function with 17 command handlers

## Goal
Reduce main.rs from 2,971 → ~300 lines by extracting command handlers into focused modules.

---

## Module Structure

```
src/
├── main.rs (~300 lines - CLI definitions + main entry point)
└── cli/
    ├── mod.rs (module exports)
    ├── helpers.rs (shared helper functions)
    ├── serve.rs (MCP server startup)
    ├── api_server.rs (HTTP API server)
    ├── init.rs (database initialization)
    ├── export.rs (memory export)
    ├── status.rs (system status)
    ├── edit.rs (ICS integration)
    ├── tui.rs (TUI wrapper)
    ├── config.rs (configuration management)
    ├── secrets.rs (secrets management)
    ├── orchestrate.rs (orchestration launcher)
    ├── remember.rs (store memory)
    ├── recall.rs (query memories)
    ├── embed.rs (embedding management)
    ├── models.rs (model management)
    ├── evolve.rs (memory evolution)
    ├── artifact.rs (artifact management)
    └── doctor.rs (health diagnostics)
```

---

## Extraction Strategy

### Phase A: Create Module Structure (5 min)
```bash
mkdir -p src/cli
touch src/cli/mod.rs
touch src/cli/{helpers,serve,api_server,init,export,status,edit,tui,config,secrets,orchestrate,remember,recall,embed,models,evolve,artifact,doctor}.rs
```

### Phase B: Extract Helpers (10 min)
Move shared functions to `cli/helpers.rs`:
- `get_default_db_path()` (lines 23-28)
- `get_db_path()` (lines 31-62)
- `process_structured_plan()` (lines 68-88)
- `extract_tasks_from_plan()` (lines 91-149)
- `extract_task_description()` (lines 153-169)
- `detect_api_server()` (lines 153-167)
- `start_mcp_server()` (lines 169-298)
- `start_mcp_server_with_api()` (lines 299-385)
- `parse_memory_type()` (lines 388-406)

### Phase C: Extract Command Handlers (60 min)
Pattern for each command:

**Template**:
```rust
// src/cli/remember.rs
use mnemosyne_core::*;
use crate::cli::helpers::*;

pub async fn handle(
    db_path: Option<String>,
    content: String,
    namespace: Option<String>,
    // ... command-specific args
) -> Result<()> {
    // Command implementation
}
```

**Commands to extract** (in order of increasing size):
1. ✅ `tui.rs` (~36 lines) - Simple, good starter
2. ✅ `init.rs` (~25 lines)
3. ✅ `api_server.rs` (~39 lines)
4. ✅ `orchestrate.rs` (~59 lines)
5. ✅ `models.rs` (~87 lines)
6. ✅ `config.rs` (~88 lines)
7. ✅ `status.rs` (~92 lines)
8. ✅ `secrets.rs` (~111 lines)
9. ✅ `embed.rs` (~114 lines)
10. ✅ `edit.rs` (~120 lines)
11. ✅ `export.rs` (~138 lines)
12. ✅ `recall.rs` (~156 lines)
13. ✅ `evolve.rs` (~194 lines)
14. ✅ `remember.rs` (~202 lines)
15. ✅ `artifact.rs` (~420 lines)
16. ✅ `doctor.rs` (~100 lines)
17. ✅ `serve.rs` (handles --serve flag, delegates to helpers)

### Phase D: Update main.rs (15 min)
Simplify main() to delegate:
```rust
match cli.command {
    Some(Commands::Remember { content, namespace, ... }) => {
        cli::remember::handle(cli.db_path, content, namespace, ...).await
    }
    // ... other commands
}
```

### Phase E: Test & Validate (10 min)
```bash
cargo check --bin mnemosyne
cargo build --bin mnemosyne --release
cargo test --bin mnemosyne
```

---

## Testing Protocol

After each extraction:
1. ✅ Create module file
2. ✅ Add to `cli/mod.rs`
3. ✅ Extract command handler
4. ✅ Update main.rs to delegate
5. ✅ Run `cargo check --bin mnemosyne`
6. ✅ Commit if successful

**Commit template**:
```
refactor(cli): Extract {command} handler to cli/{module}.rs

- Reduces main.rs from {before} → {after} lines
- Handler logic moved to dedicated module
- All tests passing
```

---

## Validation Criteria

- [ ] `cargo check --bin mnemosyne` succeeds
- [ ] `cargo build --bin mnemosyne --release` succeeds
- [ ] All CLI commands functional (manual spot checks)
- [ ] main.rs reduced to ~300 lines
- [ ] Each module focused and cohesive
- [ ] No duplicate code
- [ ] Clear module boundaries

---

## Estimated Timeline

- Phase A (Setup): 5 min
- Phase B (Helpers): 10 min
- Phase C (Handlers): 60 min (17 × ~3.5 min avg)
- Phase D (Main refactor): 15 min
- Phase E (Testing): 10 min

**Total**: ~100 minutes (1.7 hours)

---

## Rollback Strategy

Each command extraction is committed separately:
```bash
# If something breaks:
git log --oneline -20  # Find last good commit
git reset --hard <commit-hash>

# Or revert specific commit:
git revert <commit-hash>
```

---

## Next Steps

1. ✅ Create directory structure
2. ✅ Extract helpers first (test compilation)
3. ✅ Extract commands one by one (smallest first)
4. ✅ Update main.rs delegation
5. ✅ Full test suite validation
6. ✅ Push all commits

---

**Ready to execute: 2025-11-05**
**Context: 118K tokens used**
