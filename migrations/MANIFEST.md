# Migration Manifest

**Last Updated**: 2025-11-04
**Purpose**: Authoritative record of database migrations and their application status

## Migration Status

### Applied Migrations (Committed to Git)

| Number | Filename | Date Added | Status | Description |
|--------|----------|------------|--------|-------------|
| 001 | `sqlite/001_initial_schema.sql` | 2025-10-26 | ✅ Applied to both DBs | Core schema: memories, embeddings, links, audit_log, FTS |
| 002 | `sqlite/002_add_indexes.sql` | 2025-10-26 | ✅ Applied to both DBs | Performance indexes for common query patterns |
| 003 | `sqlite/003_fix_fts_triggers.sql` | 2025-10-27 | ⚠️ NOT applied (superseded by 001 edits) | FTS trigger fixes |
| 007 | `sqlite/007_evolution.sql` | 2025-10-28 | ⚠️ NOT applied (obsolete) | Evolution mechanics (not needed) |
| 013 | `sqlite/013_add_task_and_agent_event_types.sql` | 2025-11-01 | ✅ Applied (doc only) | Documents task and agent_event memory types |
| 014 | `sqlite/014_add_specification_workflow_types.sql` | 2025-11-01 | ⚠️ NOT applied | Documents spec workflow memory types |
| 015 | `sqlite/015_fix_audit_log_schema.sql` | 2025-11-04 | ✅ Applied to project DB | Fixes audit_log schema drift (details → metadata) |

### Ghost Migrations (Applied but Never Committed)

⚠️ **WARNING**: The following migrations were applied to databases but the migration files were never committed to git:

| Number | Phantom Name | Applied When | Applied To | Inferred Purpose |
|--------|--------------|--------------|------------|------------------|
| 003 | `003_audit_trail.sql` | 2025-10-30 | Both DBs | Likely fixed audit_log.details → metadata (replaced by 015) |
| 011 | `011_work_items.sql` | 2025-10-30 | Both DBs | Created work_items and memory_modification_log tables |
| 012 | `012_requirement_tracking.sql` | 2025-11-01 | Project DB only | Added requirements columns to work_items |

**Impact**: These ghost migrations cause schema drift and make database recreation impossible from git alone.

**Resolution**: Migration 015 addresses the audit_log issue. Work items tables exist in production DBs but have no migration files.

### Obsolete Migrations (Superseded or Failed)

| Filename | Reason | Date Obsoleted |
|----------|--------|----------------|
| `013_add_audit_metadata.sql.obsolete` | Quick fix attempt, superseded by 015 | 2025-11-04 |

## Current Database State

### Global DB (`~/Library/Application Support/mnemosyne/mnemosyne.db`)

```
Schema Version: 1
Last Migration: 003_audit_trail.sql (ghost)
Created: 2025-10-28 12:31:16

Tables:
  ✓ memories (with F32_BLOB embedding column - LibSQL native vectors)
  ✓ memory_links
  ✓ audit_log (metadata TEXT NOT NULL - correct schema)
  ✓ memories_fts
  ✓ metadata
  ✓ _migrations_applied
  ✓ memory_modification_log
  ✓ work_items

Missing:
  - memory_embeddings (uses F32_BLOB in memories instead)

Note: Global DB uses LibSQL/Turso-optimized schema with native vector support.
```

### Project DB (`.mnemosyne/project.db`)

```
Schema Version: 1
Last Migration: 015_fix_audit_log_schema.sql (manually applied 2025-11-04)
Created: 2025-10-28 12:35:36

Tables:
  ✓ memories (no embedding column)
  ✓ memory_embeddings (separate table for embeddings)
  ✓ memory_links
  ✓ audit_log (metadata TEXT NOT NULL - fixed by migration 015)
  ✓ memories_fts
  ✓ metadata
  ✓ _migrations_applied
  ✓ _sqlx_migrations (legacy, unused)
  ✓ memory_modification_log
  ✓ work_items (with extra requirements columns)

Note: Project DB uses standard SQLite schema with separate embeddings table.
```

## Schema Divergence

### Known Differences Between Global and Project DBs

1. **Embedding Storage**:
   - **Global**: `memories.embedding F32_BLOB(384)` (native vectors)
   - **Project**: Separate `memory_embeddings` table with `BLOB` column
   - **Reason**: Global uses LibSQL/Turso, Project uses standard SQLite

2. **work_items columns**:
   - **Global**: Original schema
   - **Project**: Extra columns (requirements, requirement_status, implementation_evidence)
   - **Reason**: Ghost migration 012 applied only to project DB

3. **Migration Tracking**:
   - **Global**: 4 migrations tracked (001, 002, 003 ghost, 011 ghost)
   - **Project**: 5 migrations tracked (001, 002, 003 ghost, 011 ghost, 012 ghost)
   - **Reason**: Ghost migrations and manual fixes

## Migration Guidelines

### DO:
- ✅ Create new migration file for every schema change
- ✅ Never edit committed migration files
- ✅ Test migrations on copy databases first
- ✅ Document the purpose and impact in migration header
- ✅ Update this manifest after applying migrations
- ✅ Backup databases before applying migrations

### DON'T:
- ❌ Edit migration files after commit
- ❌ Apply manual schema changes without creating migration
- ❌ Skip migration files (creates gaps in numbering)
- ❌ Reuse migration numbers (causes collisions)
- ❌ Apply migrations locally without committing the file

## Recovery Procedures

### Recovering from Schema Drift

If databases have diverged from git migrations:

1. **Audit current state**:
   ```bash
   sqlite3 database.db .schema > current_schema.sql
   ```

2. **Compare with expected state** (from migrations)

3. **Create fix migration** (like 015):
   - Document the drift
   - Provide forward path
   - Handle both states (if possible)

4. **Test on backup first**:
   ```bash
   cp database.db database.backup.db
   sqlite3 database.backup.db < migrations/sqlite/NNN_fix.sql
   ```

5. **Apply to production** only after validation

### Recreating Database from Scratch

If you need to recreate a database from git alone:

⚠️ **WARNING**: Ghost migrations (003, 011, 012) cannot be recreated from git!

**Workaround**:
1. Export schema from existing production database
2. Create migration files for work_items and memory_modification_log tables
3. Add to git
4. Then new databases can be created from complete migration history

## Future Work

### Phase 2 - Complete Migration System Validation (from comprehensive plan)

1. **Recover Ghost Migrations**:
   - Export work_items table schema from production DB
   - Create proper 011_work_items.sql migration file
   - Create proper 012_requirement_tracking.sql migration file
   - Add to git

2. **Resolve Migration 003 Collision**:
   - 003_fix_fts_triggers.sql exists but isn't applied
   - 003_audit_trail.sql (ghost) is applied but doesn't exist
   - Decide: Rename one or accept gap

3. **Create Migration Validation**:
   - Add `mnemosyne migrations validate` command
   - Check files on disk match _migrations_applied records
   - Warn about ghost migrations
   - Verify schema matches expected state

4. **Add to CI**:
   - Run migration validation in CI
   - Test migrations on clean database
   - Ensure reproducible database creation

## References

- Schema Analysis: `/tmp/schema_diff_analysis.md`
- Root Cause: `/tmp/root_cause_analysis.md`
- Database Backups: `/tmp/*_db_backup_*.db`
- Fix Script: `/tmp/fix_audit_log.sql`
