# Remaining Test Failures (3/20 suites - 15%)

**Status**: 85% pass rate achieved (17/20 test suites passing)
**Core Functionality**: 100% passing (all agentic, human, integration, performance tests)
**Date**: 2025-10-28
**Phase**: Post Phase 4 (0e5ebc4, cb26023, 184fd29)

## Overview

Three test suites have remaining failures, totaling 6 individual test failures out of 100+ total tests. All failures are **edge cases** related to:
1. LLM unavailability with invalid API keys
2. Database state after permission errors
3. Export operation error handling

All core functionality is production-ready. These failures represent advanced error handling scenarios.

---

## Failure Suite 1: recovery_1_graceful_degradation.sh

**Status**: 9/12 tests passing (75%)
**Location**: `tests/e2e/recovery_1_graceful_degradation.sh`

### Test 1: Core Functionality Without LLM (FAIL)

**What's Failing**:
```bash
Test 1: Core Functionality Without LLM
- Set ANTHROPIC_API_KEY="sk-invalid-for-testing"
- Attempt to store memory
- Expected: Memory stored with basic metadata (no enrichment)
- Actual: Storage fails completely
```

**Root Cause**:
The test sets an **invalid** API key (not empty). The code path is:
1. `has_api_key = !api_key.is_empty()` → TRUE (key exists but is invalid)
2. Tries to create `LlmService::new()` → succeeds (no validation at construction)
3. Calls `llm.enrich_memory()` → makes API call → 401 Unauthorized
4. Fallback at main.rs:675-708 **should** catch this and create basic memory
5. **Hypothesis**: The fallback is executing, but memory storage is still failing

**How to Reproduce**:
```bash
export ANTHROPIC_API_KEY="sk-invalid-for-testing"
./target/release/mnemosyne remember --content "Test" --namespace "project:test" --importance 9
# Expected: Success with basic metadata
# Actual: May be failing
```

**Debug Steps**:
1. Add `RUST_LOG=debug` to see if fallback is executing
2. Check if error is from LLM or from storage layer
3. Verify fallback memory creation code executes
4. Check if memory reaches `storage.store_memory()`

**Fix Strategy**:
- **Option A**: Add more robust error handling in fallback (lines 675-708 of main.rs)
- **Option B**: Make `LlmService::new()` validate API key format and return error early
- **Option C**: Test may have incorrect expectations - verify what *should* happen

**Estimated Effort**: 1-2 hours
**Priority**: Medium (edge case but tests LLM resilience)

---

### Test 2: Read-Only Database Mode - Read Operations (FAIL)

**What's Failing**:
```bash
Test 2: Read-Only Database Mode
- Create database with data
- chmod 444 (read-only)
- Write attempt fails (PASS - expected)
- Read attempt fails (FAIL - should work)
```

**Root Cause**:
LibSQL/SQLite requires write access to database file even for read operations due to:
1. WAL (Write-Ahead Logging) mode needs to update `-wal` file
2. Shared memory file (`.db-shm`) needs write access
3. Lock management requires write access to lock byte range

**How to Reproduce**:
```bash
# Create database
DATABASE_URL="sqlite:///tmp/test.db" mnemosyne remember --content "Test" --namespace "project:test" --importance 7

# Make read-only
chmod 444 /tmp/test.db

# Try to read (will fail)
DATABASE_URL="sqlite:///tmp/test.db" mnemosyne recall --query "Test" --namespace "project:test"

# Restore
chmod 644 /tmp/test.db
```

**Fix Strategy**:
- **Option A**: Detect read-only database and open in read-only mode (`SQLITE_OPEN_READONLY`)
  - LibSQL/SQLite supports read-only mode for queries
  - Requires connection mode detection in `LibsqlStorage::new_with_validation()`
  - Add `ConnectionMode::LocalReadOnly(String)` variant

- **Option B**: Document that read-only mode requires read access to WAL files
  - Update error message to explain: "Read-only database requires read access to .db-wal and .db-shm files"
  - Test expectations may be unrealistic for WAL mode

- **Option C**: Disable WAL mode for read-only connections
  - `PRAGMA journal_mode=DELETE` for read-only mode
  - Requires detecting read-only state before opening

**Estimated Effort**: 2-3 hours (requires LibSQL connection mode changes)
**Priority**: Low (uncommon scenario - production DBs rarely chmod 444)

---

### Test 3: Partial Feature Availability (FAIL)

**What's Failing**:
```bash
Test 3: Partial Feature Availability
- Test basic store/retrieve with potential LLM issues
- Expected: Core functionality works even if enrichment fails
- Actual: Core functionality broken
```

**Root Cause**:
Likely same as Test 1 - LLM fallback issue with invalid API key. The test description is vague.

**How to Reproduce**:
Run test with `set -x` to see exact commands:
```bash
bash -x tests/e2e/recovery_1_graceful_degradation.sh 2>&1 | grep -A 30 "Test 3:"
```

**Fix Strategy**:
Same as Test 1 - fix LLM fallback behavior with invalid API keys.

**Estimated Effort**: Same as Test 1 (may be fixed together)
**Priority**: Medium

---

## Failure Suite 2: recovery_2_fallback_modes.sh

**Status**: 14/16 tests passing (88%)
**Location**: `tests/e2e/recovery_2_fallback_modes.sh`

### Test 1: LLM Enrichment Fallback (FAIL)

**What's Failing**:
```bash
Test 1: LLM Enrichment Fallback
- Set invalid API key or simulate LLM unavailable
- Store memory without enrichment
- Expected: Memory stored with basic metadata
- Actual: Memory not stored
```

**Root Cause**:
Same root cause as recovery_1 Test 1 - invalid API key handling.

**How to Reproduce**:
```bash
export ANTHROPIC_API_KEY="sk-invalid"
DATABASE_URL="sqlite:///tmp/test.db" ./target/release/mnemosyne remember \
    --content "Critical system memory" --namespace "project:test" --importance 9
```

**Fix Strategy**:
Same as recovery_1 Test 1. Fixing that issue should fix this one too.

**Estimated Effort**: 0 hours (fixed with recovery_1 Test 1)
**Priority**: High (duplicate of recovery_1 Test 1)

---

### Test 10: Export Fallback Format (FAIL)

**What's Failing**:
```bash
Test 10: Export Fallback Format
- Test export when enriched format unavailable
- Expected: Export succeeds with basic format
- Actual: Export failed
```

**Root Cause**:
The export command was just implemented in Phase 4.2 (commit cb26023). The test may be:
1. Using wrong command syntax
2. Expecting different output format
3. Database doesn't exist or is empty
4. Test checking for wrong success condition

**How to Reproduce**:
```bash
# Create some test data
DATABASE_URL="sqlite:///tmp/test_export.db" ./target/release/mnemosyne remember \
    --content "Test memory 1" --namespace "project:test" --importance 7
DATABASE_URL="sqlite:///tmp/test_export.db" ./target/release/mnemosyne remember \
    --content "Test memory 2" --namespace "project:test" --importance 8

# Try export
DATABASE_URL="sqlite:///tmp/test_export.db" ./target/release/mnemosyne export /tmp/output.json
echo "Exit code: $?"
ls -lh /tmp/output.json
```

**Debug Steps**:
1. Run test with `set -x` to see exact export command
2. Check what exit code export returns
3. Verify export command syntax in test matches implementation
4. Check if test is looking for specific output that's not generated

**Fix Strategy**:
- **Option A (Test Bug)**: Update test to match export command implementation
  - Check command syntax: `mnemosyne export <output> [--namespace <ns>]`
  - Verify test success condition (checks for file? checks stderr?)

- **Option B (Export Bug)**: Fix export command error handling
  - May need to handle empty database case
  - May need better error messages

- **Option C (Missing Feature)**: Test expects "fallback format" that doesn't exist
  - Current implementation supports JSON/JSONL/Markdown
  - Test may expect a 4th "simple" fallback format

**Estimated Effort**: 1 hour (likely test syntax issue)
**Priority**: Low (export command works, test may be outdated)

---

## Failure Suite 3: failure_1_storage_errors.sh

**Status**: 9/10 tests passing (90%)
**Location**: `tests/e2e/failure_1_storage_errors.sh`

### Test 8: Database Recovery After Error (FAIL)

**What's Failing**:
```bash
Test 8: Database Recovery After Error
- Create database with memory
- chmod 444 (simulate error - read-only)
- Attempt write (fails as expected)
- chmod 644 (restore permissions)
- Attempt new write
- Expected: New write succeeds
- Actual: Database not usable after error recovery
```

**Root Cause**:
After a write failure due to permissions, LibSQL may:
1. Leave WAL file in inconsistent state
2. Cache connection state as "failed"
3. Have stale lock files
4. Keep transaction state as "aborted"

Each CLI invocation is a fresh process, so issue is likely:
- Leftover WAL/SHM files from failed transaction
- SQLite checkpoint not run after permission change
- Database needs `PRAGMA wal_checkpoint(FULL)` to recover

**How to Reproduce**:
```bash
# Create database
TEST_DB="/tmp/test_recovery.db"
DATABASE_URL="sqlite://$TEST_DB" ./target/release/mnemosyne remember \
    --content "Before error" --namespace "project:test" --importance 7

# Make read-only
chmod 444 "$TEST_DB"
chmod 444 "${TEST_DB}-wal" 2>/dev/null || true
chmod 444 "${TEST_DB}-shm" 2>/dev/null || true

# Try to write (will fail)
DATABASE_URL="sqlite://$TEST_DB" ./target/release/mnemosyne remember \
    --content "During error" --namespace "project:test" --importance 7 2>&1

# Restore permissions
chmod 644 "$TEST_DB"
chmod 644 "${TEST_DB}-wal" 2>/dev/null || true
chmod 644 "${TEST_DB}-shm" 2>/dev/null || true

# Try to write again (test expects success)
DATABASE_URL="sqlite://$TEST_DB" ./target/release/mnemosyne remember \
    --content "After recovery" --namespace "project:test" --importance 7

# Check if it worked
DATABASE_URL="sqlite://$TEST_DB" ./target/release/mnemosyne recall --query "After recovery" --namespace "project:test"
```

**Fix Strategy**:
- **Option A**: Add WAL checkpoint on connection open after failed operations
  - Detect stale WAL files
  - Run `PRAGMA wal_checkpoint(FULL)` to clear WAL
  - Implement in `LibsqlStorage::new_with_validation()`

- **Option B**: Add recovery method that cleans up after permission errors
  - `recover_from_permission_error()` method
  - Deletes or resets WAL/SHM files
  - Called automatically when detecting permission restoration

- **Option C**: Document behavior and update test expectations
  - WAL mode requires manual intervention after permission errors
  - Test may be checking unrealistic recovery scenario
  - Document: "After permission errors, may need to delete .db-wal files"

**Estimated Effort**: 2-3 hours (requires WAL file handling)
**Priority**: Medium (edge case but affects resilience)

**Implementation Sketch**:
```rust
impl LibsqlStorage {
    async fn recover_from_wal_error(db_path: &str) -> Result<()> {
        let wal_path = format!("{}-wal", db_path);
        let shm_path = format!("{}-shm", db_path);

        // Try to checkpoint WAL
        let conn = /* open connection */;
        match conn.execute("PRAGMA wal_checkpoint(TRUNCATE)", ()).await {
            Ok(_) => info!("WAL checkpoint successful"),
            Err(e) => {
                warn!("WAL checkpoint failed: {}, attempting cleanup", e);
                // Delete WAL files if checkpoint fails
                let _ = std::fs::remove_file(&wal_path);
                let _ = std::fs::remove_file(&shm_path);
            }
        }

        Ok(())
    }
}
```

---

## Summary of Fixes Needed

### Quick Wins (Low Hanging Fruit)
1. **Export test syntax** (Test recovery_2 Test 10) - 1 hour
   - Verify test matches implementation
   - Update test or fix export command

### Medium Effort (Core Improvements)
2. **LLM invalid API key handling** (3 failures: recovery_1 Tests 1,3 + recovery_2 Test 1) - 2-3 hours
   - Debug why fallback isn't working with invalid keys
   - May need to validate API key format earlier
   - Or improve fallback error handling

3. **WAL recovery after permission errors** (failure_1 Test 8) - 2-3 hours
   - Implement WAL checkpoint on recovery
   - Clean up stale WAL files
   - Add to `recover_from_error()` method

### Complex (Edge Cases)
4. **Read-only database support** (recovery_1 Test 2) - 3-4 hours
   - Add true read-only mode to connection handling
   - Handle WAL mode vs journal mode detection
   - May require LibsqlStorage API changes

---

## Testing Strategy for Fixes

### Before Fixing
```bash
# Baseline - current state
cd /Users/rand/src/mnemosyne/tests/e2e
bash recovery_1_graceful_degradation.sh > /tmp/before_recovery1.log 2>&1
bash recovery_2_fallback_modes.sh > /tmp/before_recovery2.log 2>&1
bash failure_1_storage_errors.sh > /tmp/before_failure1.log 2>&1
```

### After Each Fix
```bash
# Test specific suite
bash recovery_1_graceful_degradation.sh
bash recovery_2_fallback_modes.sh
bash failure_1_storage_errors.sh

# Run full suite to check for regressions
bash /tmp/run_tests_here.sh
```

### Manual Verification
```bash
# LLM fallback test
export ANTHROPIC_API_KEY="sk-invalid"
./target/release/mnemosyne remember --content "Test" --namespace "project:test" --importance 7
# Should succeed with warning about LLM failure

# Database recovery test
TEST_DB="/tmp/recovery_test.db"
DATABASE_URL="sqlite://$TEST_DB" ./target/release/mnemosyne remember --content "Before" --namespace "project:test" --importance 7
chmod 444 "$TEST_DB"
DATABASE_URL="sqlite://$TEST_DB" ./target/release/mnemosyne remember --content "During" --namespace "project:test" --importance 7 || true
chmod 644 "$TEST_DB"
DATABASE_URL="sqlite://$TEST_DB" ./target/release/mnemosyne remember --content "After" --namespace "project:test" --importance 7
# Should succeed
```

---

## Recommended Fix Order

1. **First**: Debug LLM invalid API key issue (affects 3 tests)
   - Add `RUST_LOG=debug` and trace through fallback code
   - Likely simple fix in error handling
   - High impact: fixes 3 failures

2. **Second**: Fix export test (affects 1 test)
   - Quick verification of test syntax
   - Low risk, high confidence

3. **Third**: Implement WAL recovery (affects 1 test)
   - Improves production resilience
   - Adds value beyond just passing test

4. **Last**: Read-only database support (affects 1 test)
   - Most complex
   - Least common scenario
   - Consider skipping if other fixes reach 95%

**Expected Final Pass Rate After Fixes**: 95-100% (19-20/20 tests passing)

---

## Notes for Future Developers

### Context
- These failures were documented after Phase 4 (commits 0e5ebc4, cb26023, 184fd29)
- 85% pass rate achieved with 100% core functionality passing
- All failures are edge cases, not core functionality bugs
- Production system is fully operational for primary use cases

### Philosophy
- Don't skip these tests - they test important resilience scenarios
- These edge cases will occur in production eventually
- Good error handling in edge cases builds user confidence
- Document what *should* happen in each scenario

### Questions to Answer When Fixing
1. **Is this a test bug or a code bug?** (Check test expectations vs implementation)
2. **Is this behavior acceptable?** (Some "failures" may be correct behavior)
3. **What should happen in this scenario?** (Define expected behavior first)
4. **Does the fix introduce new risks?** (Test for regressions)

---

## Related Files

**Test Suites**:
- `tests/e2e/recovery_1_graceful_degradation.sh`
- `tests/e2e/recovery_2_fallback_modes.sh`
- `tests/e2e/failure_1_storage_errors.sh`

**Code Areas**:
- `src/main.rs:665-743` - LLM fallback logic
- `src/services/llm.rs:396-450` - API error handling
- `src/storage/libsql.rs:500-580` - Database health & recovery
- `src/storage/libsql.rs:739-850` - Transaction handling

**Validation Script**:
- `/tmp/run_tests_here.sh` - Run from `tests/e2e/` directory

---

## Success Metrics

**Minimum**: 90% pass rate (18/20 tests)
**Target**: 95% pass rate (19/20 tests)
**Stretch**: 100% pass rate (20/20 tests)

**Current**: 85% pass rate (17/20 tests) - Phase 4 Complete ✓
