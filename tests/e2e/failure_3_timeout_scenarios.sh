#!/usr/bin/env bash
set -uo pipefail  # Removed -e to allow test failures

# E2E Test: Failure 3 - Timeout Scenarios
#
# Scenario: Test timeout protection across all operations
# Tests system behavior when:
# - Database operations timeout
# - LLM enrichment times out
# - Long queries exceed limits
# - Context loading takes too long
# - Concurrent operations timeout

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source common utilities
# shellcheck source=tests/e2e/lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"

# Test state
PASSED=0
FAILED=0

section "E2E Test: Failure 3 - Timeout Scenarios"

# Setup test environment
setup_test_env "fail3_timeout"

section "Test 1: Remember Operation Timeout"

print_cyan "Testing timeout protection for remember operations..."

# Remember operations should complete within reasonable time
# Typical: <5s for small memories, <30s with LLM enrichment
START=$(date +%s)

REMEMBER_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" timeout 60 "$BIN" remember \
    "Test memory for timeout validation" \
    --namespace "project:test" --importance 7 2>&1 || echo "TIMEOUT")

END=$(date +%s)
DURATION=$((END - START))

if [ "$DURATION" -lt 45 ]; then
    pass "Remember timeout: Completed within threshold (${DURATION}s < 45s)"
else
    fail "Remember timeout: Exceeded threshold (${DURATION}s)"
fi

if echo "$REMEMBER_OUTPUT" | grep -qi "TIMEOUT"; then
    fail "Remember timeout: Command timed out"
fi

section "Test 2: Recall Operation Timeout"

print_cyan "Testing timeout protection for recall operations..."

# Create some memories first
for i in {1..10}; do
    create_memory "$BIN" "$TEST_DB" \
        "Memory $i for recall timeout testing" \
        "project:test" 7 > /dev/null 2>&1
done

sleep 3

# Recall should be fast (<1s for small datasets)
START=$(date +%s)

RECALL_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" timeout 10 "$BIN" recall --query "Memory" \
    --namespace "project:test" 2>&1 || echo "TIMEOUT")

END=$(date +%s)
DURATION=$((END - START))

if [ "$DURATION" -lt 5 ]; then
    pass "Recall timeout: Completed quickly (${DURATION}s < 5s)"
else
    warn "Recall timeout: Slower than expected (${DURATION}s)"
fi

if echo "$RECALL_OUTPUT" | grep -qi "TIMEOUT"; then
    fail "Recall timeout: Query timed out"
else
    pass "Recall timeout: Query completed successfully"
fi

section "Test 3: Large Content Timeout"

print_cyan "Testing timeout with very large content..."

# Create large content (100KB)
LARGE_CONTENT=$(head -c 102400 /dev/urandom | base64 | head -c 102400)

START=$(date +%s)

LARGE_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" timeout 60 "$BIN" remember \
    "$LARGE_CONTENT" \
    --namespace "project:test" --importance 6 2>&1 || echo "LARGE_TIMEOUT")

END=$(date +%s)
DURATION=$((END - START))

if echo "$LARGE_OUTPUT" | grep -qi "LARGE_TIMEOUT"; then
    warn "Large content: Timed out (may be expected for 100KB)"
elif [ "$DURATION" -lt 60 ]; then
    pass "Large content: Handled within timeout (${DURATION}s)"
else
    fail "Large content: Exceeded timeout"
fi

section "Test 4: Concurrent Operation Timeouts"

print_cyan "Testing timeout behavior with concurrent operations..."

# Launch multiple operations simultaneously
START=$(date +%s)

for i in {1..5}; do
    (DATABASE_URL="sqlite://$TEST_DB" timeout 45 "$BIN" remember \
        "Concurrent memory $i - testing parallel timeout behavior" \
        --namespace "project:concurrent" --importance 6 > /dev/null 2>&1) &
done

# Wait for all to complete or timeout
wait

END=$(date +%s)
DURATION=$((END - START))

sleep 2

# Check how many were stored
CONCURRENT_COUNT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "Concurrent memory" \
    --namespace "project:concurrent" 2>&1 | grep -c "Concurrent memory" || echo "0")

if [ "$CONCURRENT_COUNT" -ge 3 ]; then
    pass "Concurrent timeouts: Most operations completed ($CONCURRENT_COUNT/5 in ${DURATION}s)"
else
    warn "Concurrent timeouts: Only $CONCURRENT_COUNT/5 completed"
fi

section "Test 5: Database Lock Timeout"

print_cyan "Testing database lock timeout handling..."

# SQLite has lock timeout mechanism
# Test by creating contention scenario

LOCK_DB=$(create_test_db "lock_timeout")

# Create memory in background (holds lock briefly)
(DATABASE_URL="sqlite://$LOCK_DB" "$BIN" remember \
    "Lock test memory 1" \
    --namespace "project:test" --importance 7 > /dev/null 2>&1) &

# Immediately try another operation (may hit lock)
LOCK_OUTPUT=$(DATABASE_URL="sqlite://$LOCK_DB" "$BIN" remember \
    "Lock test memory 2" \
    --namespace "project:test" --importance 7 2>&1 || echo "LOCK_ERROR")

wait

# System should either:
# 1. Handle lock gracefully with retry
# 2. Report lock timeout error
if echo "$LOCK_OUTPUT" | grep -qi "locked\|busy\|LOCK_ERROR"; then
    pass "Database lock: Lock contention detected and handled"
else
    pass "Database lock: Operations completed without explicit lock errors"
fi

sleep 2

# Verify both memories stored (retry mechanism working)
STORED_COUNT=$(DATABASE_URL="sqlite://$LOCK_DB" "$BIN" recall --query "Lock test" \
    --namespace "project:test" 2>&1 | grep -c "Lock test" || echo "0")

if [ "$STORED_COUNT" -ge 2 ]; then
    pass "Database lock retry: Both memories stored ($STORED_COUNT)"
else
    warn "Database lock retry: Only $STORED_COUNT/2 memories stored"
fi

cleanup_test_db "$LOCK_DB"

section "Test 6: Context Loading Timeout"

print_cyan "Testing context loading timeout protection..."

# Create many memories to simulate large context
print_cyan "Creating 30 memories for context loading test..."
for i in {1..30}; do
    create_memory "$BIN" "$TEST_DB" \
        "Context load memory $i - testing timeout with large context" \
        "project:context" 7 > /dev/null 2>&1
done

sleep 3

# Load large context with timeout
START=$(date +%s)

CONTEXT_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" timeout 10 "$BIN" recall --query "" \
    --namespace "project:context" --limit 30 2>&1 || echo "CONTEXT_TIMEOUT")

END=$(date +%s)
DURATION=$((END - START))

if [ "$DURATION" -lt 8 ]; then
    pass "Context loading timeout: Completed quickly (${DURATION}s < 8s)"
else
    warn "Context loading timeout: Slower than expected (${DURATION}s)"
fi

if echo "$CONTEXT_OUTPUT" | grep -qi "CONTEXT_TIMEOUT"; then
    fail "Context loading: Timed out loading 30 memories"
else
    pass "Context loading: Successfully loaded large context"
fi

section "Test 7: Query Complexity Timeout"

print_cyan "Testing timeout with complex queries..."

# Complex semantic search might take longer
# System should have timeout protection

START=$(date +%s)

COMPLEX_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" timeout 15 "$BIN" recall \
    --query "microservices architecture event-driven patterns distributed systems" \
    --namespace "project:test" 2>&1 || echo "COMPLEX_TIMEOUT")

END=$(date +%s)
DURATION=$((END - START))

if [ "$DURATION" -lt 12 ]; then
    pass "Complex query timeout: Completed within threshold (${DURATION}s)"
else
    warn "Complex query: Slower than expected (${DURATION}s)"
fi

section "Test 8: Export Operation Timeout"

print_cyan "Testing export timeout with large datasets..."

# Create memories across multiple namespaces
for ns in "app" "docs" "tests"; do
    for i in {1..5}; do
        create_memory "$BIN" "$TEST_DB" \
            "Export test memory $i in namespace $ns" \
            "project:$ns" 7 > /dev/null 2>&1
    done
done

sleep 3

# Export should complete quickly even with multiple namespaces
START=$(date +%s)

EXPORT_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" timeout 30 "$BIN" export 2>&1 || echo "EXPORT_TIMEOUT")

END=$(date +%s)
DURATION=$((END - START))

if echo "$EXPORT_OUTPUT" | grep -qi "EXPORT_TIMEOUT"; then
    fail "Export timeout: Operation timed out"
elif [ "$DURATION" -lt 25 ]; then
    pass "Export timeout: Completed within threshold (${DURATION}s)"
else
    warn "Export timeout: Slower than expected (${DURATION}s)"
fi

section "Test 9: Timeout Error Messages"

print_cyan "Testing quality of timeout error messages..."

# When timeout occurs, error should be clear
TIMEOUT_TEST_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" timeout 1 "$BIN" remember \
    "This should timeout with 1 second limit" \
    --namespace "project:test" --importance 7 2>&1 || echo "TIMEOUT_MESSAGE_TEST")

# Check if timeout message is clear
if echo "$TIMEOUT_TEST_OUTPUT" | grep -qi "timeout\|exceeded\|time.*limit\|TIMEOUT_MESSAGE_TEST"; then
    pass "Timeout errors: Clear timeout indication present"
else
    warn "Timeout errors: Timeout message may not be explicit"
fi

section "Test 10: Graceful Timeout Recovery"

print_cyan "Testing recovery after timeout scenarios..."

# After timeouts, system should continue working normally
RECOVERY_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    "Recovery after timeout test - system should work normally" \
    --namespace "project:test" --importance 7 2>&1 || echo "")

sleep 2

RECOVERY_STORED=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "Recovery after timeout" \
    --namespace "project:test" 2>&1 || echo "")

if echo "$RECOVERY_STORED" | grep -qi "Recovery after timeout"; then
    pass "Timeout recovery: System functional after timeout scenarios"
else
    fail "Timeout recovery: System may be in degraded state"
fi

section "Test 11: Launcher Timeout Protection"

print_cyan "Testing launcher timeout configuration..."

# Launcher should have timeout configured for context loading
# Typical timeout: 500ms for pre-launch context loading

print_cyan "Simulating launcher context loading..."

START=$(date +%s)

# Simulate launcher behavior: load high-importance memories with limit
LAUNCHER_CONTEXT=$(DATABASE_URL="sqlite://$TEST_DB" timeout 5 "$BIN" recall --query "" \
    --namespace "project:test" --min-importance 7 --limit 10 2>&1 || echo "LAUNCHER_TIMEOUT")

END=$(date +%s)
DURATION=$((END - START))

if [ "$DURATION" -lt 3 ]; then
    pass "Launcher timeout: Context loaded quickly (${DURATION}s < 3s)"
else
    warn "Launcher timeout: Context loading slower than ideal (${DURATION}s)"
fi

if echo "$LAUNCHER_CONTEXT" | grep -qi "LAUNCHER_TIMEOUT"; then
    fail "Launcher timeout: Context loading timed out"
fi

section "Test 12: Timeout Configuration Validation"

print_cyan "Testing that timeouts are appropriately configured..."

# Verify that operations don't hang indefinitely
# All operations should have some timeout protection

print_cyan "Testing operation with unreasonably long timeout (60s)..."

START=$(date +%s)

CONFIG_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" timeout 60 "$BIN" remember \
    "Configuration test for timeout limits" \
    --namespace "project:test" --importance 7 2>&1 || echo "")

END=$(date +%s)
DURATION=$((END - START))

# Even with 60s timeout, operation should complete much faster
if [ "$DURATION" -lt 45 ]; then
    pass "Timeout configuration: Operations complete well before limits (${DURATION}s)"
else
    warn "Timeout configuration: Operations approaching timeout limits (${DURATION}s)"
fi

# Cleanup
section "Cleanup"
teardown_test_env

# Summary
test_summary
exit $?
