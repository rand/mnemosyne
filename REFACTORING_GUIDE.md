# Phase 1.1: LibSQL Storage Module Extraction Guide

## Status
- **Directory Created**: `src/storage/libsql/` ✅
- **Working Tree**: Clean ✅
- **Current Branch**: `main` (up to date with origin) ✅
- **Next Step**: Extract modules one at a time

---

## File Structure (Target)

```
src/storage/
  ├── libsql.rs           (~200 lines - struct, re-exports, parse_sql_statements)
  └── libsql/
      ├── mod.rs          (Module organization and re-exports)
      ├── connection.rs   (~400 lines)
      ├── schema.rs       (~300 lines)
      ├── memory_crud.rs  (~1,350 lines)
      ├── search.rs       (~260 lines)
      ├── evolution.rs    (~400 lines)
      └── work_items.rs   (~850 lines)
```

---

## Extraction Plan (Exact Line Ranges)

### Keep in `libsql.rs` (Lines 1-102)
- Module documentation
- All imports
- `parse_sql_statements()` helper function (lines 18-60)
- `SchemaType` enum (lines 62-69)
- `LibsqlStorage` struct definition (lines 71-78)
- `ConnectionMode` enum (lines 80-101)

### Module 1: `connection.rs` (Lines 103-505)
**Content**:
- `impl LibsqlStorage` construction methods
- `new_with_validation()` - Database initialization
- `is_file_writable()` - File permission checking
- `validate_database_file()` - Pre-flight validation
- Connection mode logic (Local, LocalReadOnly, InMemory, Remote, EmbeddedReplica)
- Database builder configuration

**Dependencies**:
```rust
use libsql::{Builder, Database};
use std::path::PathBuf;
use crate::error::{MnemosyneError, Result};
use super::{ConnectionMode, SchemaType, LibsqlStorage};
```

**Validation**: Run `cargo check --lib` after extraction

---

### Module 2: `schema.rs` (Lines 506-798)
**Content**:
- Schema type detection logic
- `detect_schema_type()` - Determines StandardSQLite vs LibSQL
- Schema verification methods
- Migration logic
- Table existence checks
- FTS5 setup

**Dependencies**:
```rust
use libsql::Connection;
use crate::error::{MnemosyneError, Result};
use super::{SchemaType, LibsqlStorage, parse_sql_statements};
```

**Validation**: Run `cargo check --lib` after extraction

---

### Module 3: `memory_crud.rs` (Lines 799-2147)
**Content**:
- `row_to_memory()` - Row → MemoryNote conversion (critical helper)
- Part of `impl StorageBackend for LibsqlStorage`:
  - `store_memory()` (lines 1711-1924)
  - `get_memory()` (lines 1925-2000)
  - `update_memory()` (lines 2001-2210)
  - `archive_memory()` (lines 2211-2235)
- Audit logging helper
- Embedding serialization

**Dependencies**:
```rust
use async_trait::async_trait;
use libsql::{params, Connection};
use crate::error::{MnemosyneError, Result};
use crate::types::{MemoryId, MemoryNote, Namespace};
use super::{LibsqlStorage, SchemaType};
```

**Note**: This is a partial `impl StorageBackend for LibsqlStorage` block

**Validation**: Run `cargo check --lib` and `cargo test --lib storage::libsql::memory_crud`

---

### Module 4: `search.rs` (Lines 2148-2403)
**Content**:
- Part of `impl StorageBackend for LibsqlStorage`:
  - `vector_search()` - F32_BLOB vector similarity
  - `keyword_search()` - FTS5 full-text search
  - `graph_traverse()` - Memory link traversal
  - `hybrid_search()` - Combined vector + keyword
- `escape_fts5_query()` - FTS5 query sanitization
- `calculate_vector_distance()` - Cosine similarity helper

**Dependencies**:
```rust
use async_trait::async_trait;
use libsql::{params, Connection};
use crate::error::{MnemosyneError, Result};
use crate::types::{MemoryId, Namespace, SearchResult};
use super::{LibsqlStorage, SchemaType};
```

**Note**: Partial `impl StorageBackend for LibsqlStorage` block

**Validation**: Run `cargo test --lib storage::libsql::search`

---

### Module 5: `evolution.rs` (Lines 2404-2810)
**Content**:
- Part of `impl StorageBackend for LibsqlStorage`:
  - `find_consolidation_candidates()` - Memory consolidation logic
  - `increment_access()` - Access counter updates
  - `count_memories()` - Namespace-aware counts
  - `list_memories()` - Pagination support
  - `store_modification_log()` - Audit trail storage
  - `get_audit_trail()` - Modification history
  - `get_modification_stats()` - Aggregate statistics

**Dependencies**:
```rust
use async_trait::async_trait;
use libsql::{params, Connection};
use chrono::Utc;
use crate::error::{MnemosyneError, Result};
use crate::types::{MemoryId, MemoryNote, Namespace};
use super::LibsqlStorage;
```

**Note**: Partial `impl StorageBackend for LibsqlStorage` block

**Validation**: Run `cargo test --lib storage::libsql::evolution`

---

### Module 6: `work_items.rs` (Lines 2811-3665)
**Content**:
- Part of `impl StorageBackend for LibsqlStorage` (Beads integration):
  - `store_work_item()` - Save task to database
  - `load_work_item()` - Load task by ID
  - `update_work_item()` - Update task state/metadata
  - `load_work_items_by_state()` - Query by workflow state
  - `delete_work_item()` - Remove completed/cancelled tasks
- Work item serialization (JSON metadata, dependencies)
- Beads-specific state management

**Dependencies**:
```rust
use async_trait::async_trait;
use libsql::{params, Connection};
use serde_json;
use crate::error::{MnemosyneError, Result};
use crate::orchestration::state::{WorkItem, WorkItemId, WorkItemState};
use super::LibsqlStorage;
```

**Note**: Final part of `impl StorageBackend for LibsqlStorage` block

**Validation**: Run `cargo test --lib storage::libsql::work_items`

---

## Execution Steps (One Module at a Time)

### Step 1: Extract `connection.rs`
```bash
# 1. Create file with proper imports
# 2. Copy lines 103-505 from libsql.rs
# 3. Wrap in: impl LibsqlStorage { ... }
# 4. Remove extracted lines from libsql.rs
# 5. Add to libsql/mod.rs: pub mod connection;
# 6. Test: cargo check --lib
# 7. Commit: git add -A && git commit -m "refactor: Extract connection module from libsql.rs"
```

### Step 2: Extract `schema.rs`
```bash
# Same pattern as Step 1
# Test: cargo check --lib
# Commit separately
```

### Step 3: Extract `memory_crud.rs`
```bash
# Include row_to_memory() helper (lines 799-1000)
# Partial impl block: impl StorageBackend for LibsqlStorage { ... }
# Test: cargo check --lib && cargo test --lib memory_crud
# Commit separately
```

### Step 4: Extract `search.rs`
```bash
# Partial impl block
# Test: cargo test --lib search
# Commit separately
```

### Step 5: Extract `evolution.rs`
```bash
# Partial impl block
# Test: cargo test --lib evolution
# Commit separately
```

### Step 6: Extract `work_items.rs`
```bash
# Final partial impl block
# Test: cargo test --lib work_items
# Commit separately
```

### Step 7: Refactor main `libsql.rs`
```bash
# Update to re-export modules
# Keep only: struct definition, enums, helper functions
# Add: pub use self::{connection::*, schema::*, ...};
# Test: cargo test --lib storage
# Commit: "refactor: Complete libsql module extraction"
```

---

## Testing Strategy

### After Each Module Extraction:
```bash
cargo check --lib
cargo test --lib storage::libsql::<module_name> -- --nocapture
```

### Final Validation (After All Modules):
```bash
# Full storage test suite
cargo test --lib storage -- --nocapture

# Integration tests
cargo test --test '*' -- --nocapture

# Release build
cargo build --release
```

---

## Rollback Strategy

Each module is committed separately, so rollback is straightforward:
```bash
# If extraction breaks tests:
git log --oneline -10  # Find last good commit
git reset --hard <commit-hash>

# Or revert specific commit:
git revert <commit-hash>
```

---

## Risk Mitigation

1. **No Logic Changes**: Only moving code, not modifying behavior
2. **Granular Commits**: Each module extracted in separate commit
3. **Incremental Testing**: Test after each extraction
4. **Type Safety**: Rust compiler will catch import/visibility issues
5. **Full Test Suite**: All 696 tests must pass before completion

---

## Success Criteria

- [ ] All 6 modules extracted
- [ ] `libsql.rs` reduced to ~200 lines
- [ ] All storage tests pass (no new failures)
- [ ] No clippy warnings introduced
- [ ] Code coverage maintained
- [ ] Documentation updated
- [ ] 7 commits pushed (1 per module + final refactor)

---

## Estimated Time: 2-3 hours

- Module extraction: ~20 minutes per module × 6 = 2 hours
- Testing between modules: ~5 minutes × 6 = 30 minutes
- Final validation and cleanup: 30 minutes

---

## Notes for Next Session

- Start with `work_items.rs` (cleanest extraction - end of file, no dependencies on other parts)
- Work backwards toward `connection.rs` (has most dependencies)
- Keep `parse_sql_statements()` in main file (used by schema module)
- All partial `impl` blocks must be marked with `#[async_trait]` if they contain async methods
- Watch for: embedding_service field access, schema_type checks, db connection usage

---

**Status**: Ready to execute. Directory created, plan documented, working tree clean.
