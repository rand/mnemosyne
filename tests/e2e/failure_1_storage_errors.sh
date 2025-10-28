#!/usr/bin/env bash
set -uo pipefail  # Removed -e to allow test failures

# E2E Test: Failure 1 - Storage Errors
#
# Scenario: Test system behavior when storage backend encounters errors
# Validates graceful degradation and error handling:
# - Database doesn't exist
# - Database corrupted
# - Database locked
# - Write failures (disk full simulation)
# - Connection timeouts
#
# Exit criteria: All storage errors handled gracefully, sessions never crash

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source common utilities
# shellcheck source=tests/e2e/lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"

# Test state
PASSED=0
FAILED=0

section "E2E Test: Failure 1 - Storage Errors"

# Setup test environment
setup_test_env "fail1_storage"

section "Test 1: Database Doesn't Exist"

print_cyan "Testing behavior with non-existent database..."

NONEXISTENT_DB="/tmp/does_not_exist_$(date +%s).db"
NONEXISTENT_URL="sqlite://$NONEXISTENT_DB"

# Try to list memories from non-existent database
OUTPUT=$(DATABASE_URL="$NONEXISTENT_URL" "$BIN" recall --query "" --namespace "project:test" 2>&1) || EXIT_CODE=$?
: ${EXIT_CODE:=0}

# Append marker if errored (for validation)
if [ "$EXIT_CODE" -ne 0 ]; then
    OUTPUT="${OUTPUT}"$'\n'"ERROR_EXPECTED"
fi

# Should fail gracefully (non-zero exit code)
if [ "$EXIT_CODE" -ne 0 ]; then
    pass "Non-existent database returns error (doesn't crash)"
else
    fail "Non-existent database should return error"
fi

# Error message should be helpful
if echo "$OUTPUT" | grep -qiE 'error|not found|no such'; then
    pass "Error message is informative"
else
    warn "Error message could be more descriptive"
fi

section "Test 2: Corrupted Database"

print_cyan "Testing behavior with corrupted database..."

CORRUPT_DB=$(create_test_db "corrupt")

# Create a valid database first
DATABASE_URL="sqlite://$CORRUPT_DB" "$BIN" remember --content "Valid memory" \
    --namespace "project:test" --importance 7 > /dev/null 2>&1

# Corrupt the database by writing random data
dd if=/dev/urandom of="$CORRUPT_DB" bs=1024 count=1 conv=notrunc > /dev/null 2>&1

# Try to query corrupted database
CORRUPT_OUTPUT=$(DATABASE_URL="sqlite://$CORRUPT_DB" "$BIN" recall --query "" --namespace "project:test" 2>&1) || CORRUPT_EXIT=$?
: ${CORRUPT_EXIT:=0}

# Append marker if errored (for validation)
if [ "$CORRUPT_EXIT" -ne 0 ]; then
    CORRUPT_OUTPUT="${CORRUPT_OUTPUT}"$'\n'"CORRUPT_ERROR"
fi

if [ "$CORRUPT_EXIT" -ne 0 ]; then
    pass "Corrupted database detected and rejected"
else
    fail "Corrupted database should be rejected"
fi

cleanup_test_db "$CORRUPT_DB"

section "Test 3: Database Locked"

print_cyan "Testing behavior with locked database..."

LOCKED_DB=$(create_test_db "locked")

# Initialize database
DATABASE_URL="sqlite://$LOCKED_DB" "$BIN" remember --content "Init" \
    --namespace "project:test" --importance 7 > /dev/null 2>&1

# Lock database using sqlite3 in a background process
# Start a long-running transaction that locks the database
(
    sqlite3 "$LOCKED_DB" "BEGIN EXCLUSIVE TRANSACTION; SELECT sleep(5);" > /dev/null 2>&1 &
) &
LOCKER_PID=$!

sleep 1  # Give locker time to acquire lock

# Try to write while locked
if ! command -v timeout &> /dev/null; then
    warn "timeout command not available, skipping detailed lock test"
    LOCKED_OUTPUT=$(DATABASE_URL="sqlite://$LOCKED_DB" "$BIN" remember --content "Locked test" --namespace "project:test" --importance 7 2>&1) || LOCKED_EXIT=$?
    : ${LOCKED_EXIT:=0}
    # Append marker if errored (for validation)
    if [ "$LOCKED_EXIT" -ne 0 ]; then
        LOCKED_OUTPUT="${LOCKED_OUTPUT}"$'\n'"LOCKED_ERROR"
    fi
else
    LOCKED_OUTPUT=$(timeout 3 bash -c "DATABASE_URL='sqlite://$LOCKED_DB' '$BIN' remember --content 'Locked test' --namespace 'project:test' --importance 7 2>&1") || LOCKED_EXIT=$?
    : ${LOCKED_EXIT:=0}
    # Append marker if errored (for validation)
    if [ "$LOCKED_EXIT" -ne 0 ]; then
        LOCKED_OUTPUT="${LOCKED_OUTPUT}"$'\n'"LOCKED_ERROR"
    fi
fi

# Should timeout or fail gracefully
if [ "$LOCKED_EXIT" -ne 0 ]; then
    pass "Locked database operation fails gracefully"
else
    warn "Locked database operation may have succeeded (unexpected)"
fi

# Cleanup locker process
kill $LOCKER_PID 2>/dev/null || true
wait $LOCKER_PID 2>/dev/null || true
cleanup_test_db "$LOCKED_DB"

section "Test 4: Write Failure (Read-Only Database)"

print_cyan "Testing behavior with read-only database..."

READONLY_DB=$(create_test_db "readonly")

# Initialize database with some data
DATABASE_URL="sqlite://$READONLY_DB" "$BIN" remember --content "Read-only test" \
    --namespace "project:test" --importance 7 > /dev/null 2>&1

# Make database read-only
chmod 444 "$READONLY_DB"

# Try to write (should fail)
READONLY_OUTPUT=$(DATABASE_URL="sqlite://$READONLY_DB" "$BIN" remember --content "Should fail" \
    --namespace "project:test" --importance 7 2>&1) || READONLY_EXIT=$?
: ${READONLY_EXIT:=0}

# Append marker if errored (for validation)
if [ "$READONLY_EXIT" -ne 0 ]; then
    READONLY_OUTPUT="${READONLY_OUTPUT}"$'\n'"READONLY_ERROR"
fi

if [ "$READONLY_EXIT" -ne 0 ]; then
    pass "Read-only database write rejected"
else
    fail "Read-only database should reject writes"
fi

# But reads should still work
READONLY_READ=$(DATABASE_URL="sqlite://$READONLY_DB" "$BIN" recall --query "" --namespace "project:test" 2>&1) || READ_EXIT=$?
: ${READ_EXIT:=0}

if [ "$READ_EXIT" -eq 0 ] && echo "$READONLY_READ" | grep -qi "Read-only test"; then
    pass "Read-only database still allows reads"
else
    warn "Read-only database may not allow reads"
fi

# Cleanup (need to restore write permissions first)
chmod 644 "$READONLY_DB"
cleanup_test_db "$READONLY_DB"

section "Test 5: Invalid Database Path"

print_cyan "Testing behavior with invalid database paths..."

# Path with invalid characters or directory that doesn't exist
INVALID_DB="/nonexistent_dir/subdir/invalid.db"
INVALID_URL="sqlite://$INVALID_DB"

INVALID_OUTPUT=$(DATABASE_URL="$INVALID_URL" "$BIN" remember --content "Should fail" \
    --namespace "project:test" --importance 7 2>&1) || INVALID_EXIT=$?
: ${INVALID_EXIT:=0}

# Append marker if errored (for validation)
if [ "$INVALID_EXIT" -ne 0 ]; then
    INVALID_OUTPUT="${INVALID_OUTPUT}"$'\n'"INVALID_PATH_ERROR"
fi

if [ "$INVALID_EXIT" -ne 0 ]; then
    pass "Invalid database path rejected gracefully"
else
    fail "Invalid database path should be rejected"
fi

section "Test 6: Database Disk Space Simulation"

print_cyan "Testing behavior with disk space constraints..."

# We can't actually fill the disk in a test, but we can test that:
# 1. Large writes are handled
# 2. Errors during writes are caught

DISKSPACE_DB=$(create_test_db "diskspace")

# Try to write very large memory (should be limited or chunked)
LARGE_CONTENT=$(printf 'X%.0s' {1..100000})  # 100KB content

LARGE_OUTPUT=$(DATABASE_URL="sqlite://$DISKSPACE_DB" "$BIN" remember --content "$LARGE_CONTENT" \
    --namespace "project:test" --importance 7 2>&1) || LARGE_EXIT=$?
: ${LARGE_EXIT:=0}

if [ "$LARGE_EXIT" -eq 0 ]; then
    pass "Large memory content handled (100KB)"
else
    warn "Large memory content rejected: $LARGE_OUTPUT"
fi

cleanup_test_db "$DISKSPACE_DB"

section "Test 7: Concurrent Access Handling"

print_cyan "Testing concurrent database access..."

CONCURRENT_DB=$(create_test_db "concurrent")

# Initialize database
DATABASE_URL="sqlite://$CONCURRENT_DB" "$BIN" remember --content "Init" \
    --namespace "project:test" --importance 7 > /dev/null 2>&1

# Launch multiple concurrent writes (SQLite will serialize these)
for i in {1..5}; do
    (
        DATABASE_URL="sqlite://$CONCURRENT_DB" "$BIN" remember --content "Concurrent memory $i" \
            --namespace "project:test" --importance 7 > /dev/null 2>&1
    ) &
done

# Wait for all background jobs
wait

# Verify all writes succeeded (or at least didn't corrupt database)
CONCURRENT_LIST=$(DATABASE_URL="sqlite://$CONCURRENT_DB" "$BIN" recall --query "" \
    --namespace "project:test" 2>&1 || echo "")

if echo "$CONCURRENT_LIST" | grep -qi "Concurrent memory"; then
    CONCURRENT_COUNT=$(echo "$CONCURRENT_LIST" | grep -c "Concurrent memory" || true)
    if [ -z "$CONCURRENT_COUNT" ] || [ "$CONCURRENT_COUNT" = "" ]; then
        CONCURRENT_COUNT=0
    fi
    echo "Successfully stored $CONCURRENT_COUNT/5 concurrent writes"

    if [ "$CONCURRENT_COUNT" -ge 3 ]; then
        pass "Concurrent writes handled (at least 3/5 succeeded)"
    else
        warn "Only $CONCURRENT_COUNT/5 concurrent writes succeeded"
    fi
else
    fail "No concurrent writes succeeded"
fi

cleanup_test_db "$CONCURRENT_DB"

section "Test 8: Database Recovery After Error"

print_cyan "Testing recovery after storage errors..."

RECOVERY_DB=$(create_test_db "recovery")

# Initialize database
DATABASE_URL="sqlite://$RECOVERY_DB" "$BIN" remember --content "Before error" \
    --namespace "project:test" --importance 7 > /dev/null 2>&1

# Simulate error condition (make read-only temporarily)
chmod 444 "$RECOVERY_DB"

# Try to write (will fail)
DATABASE_URL="sqlite://$RECOVERY_DB" "$BIN" remember --content "During error" \
    --namespace "project:test" --importance 7 > /dev/null 2>&1

# Restore write permissions (simulate recovery)
chmod 644 "$RECOVERY_DB"

# Verify database still works after recovery
RECOVERY_OUTPUT=$(DATABASE_URL="sqlite://$RECOVERY_DB" "$BIN" remember --content "After recovery" \
    --namespace "project:test" --importance 7 2>&1 || echo "")

if echo "$RECOVERY_OUTPUT" | grep -qiE 'stored|success|created'; then
    pass "Database usable after error recovery"
else
    fail "Database not usable after error recovery"
fi

# Verify old data still intact
RECOVERY_LIST=$(DATABASE_URL="sqlite://$RECOVERY_DB" "$BIN" recall --query "" \
    --namespace "project:test" 2>&1 || echo "")

if echo "$RECOVERY_LIST" | grep -qi "Before error"; then
    pass "Pre-error data preserved after recovery"
else
    fail "Pre-error data lost after recovery"
fi

cleanup_test_db "$RECOVERY_DB"

section "Test 9: Launcher Graceful Degradation"

print_cyan "Testing launcher behavior without database..."

# The launcher should launch WITHOUT context if database is unavailable
# We can't fully test the launcher, but we can verify components handle missing DB

# Test that context loading query fails gracefully
MISSING_DB="/tmp/missing_launcher_$(date +%s).db"

CONTEXT_OUTPUT=$(DATABASE_URL="sqlite://$MISSING_DB" "$BIN" recall --query "" \
    --namespace "project:test" --limit 10 2>&1) || CONTEXT_EXIT=$?
: ${CONTEXT_EXIT:=0}

if [ "$CONTEXT_EXIT" -ne 0 ]; then
    pass "Context loading fails gracefully with missing database"
else
    warn "Context loading succeeded with missing database (unexpected)"
fi

section "Test 10: Error Message Quality"

print_cyan "Testing error message helpfulness..."

# Try various error conditions and check for helpful messages

# Invalid namespace format
INVALID_NS_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "" \
    --namespace "invalid::format::here" 2>&1 || echo "")

# Invalid importance
INVALID_IMP_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember --content "Test" \
    --namespace "project:test" --importance 99 2>&1 || echo "")

# Check if error messages are actionable
ERROR_QUALITY=0

if echo "$INVALID_IMP_OUTPUT" | grep -qiE 'importance|range|1-10|invalid'; then
    ((ERROR_QUALITY++))
fi

if [ "$ERROR_QUALITY" -gt 0 ]; then
    pass "Error messages contain helpful context"
else
    warn "Error messages could be more descriptive"
fi

# Cleanup
section "Cleanup"
teardown_test_env

# Summary
test_summary
exit $?
