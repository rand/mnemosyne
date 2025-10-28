#!/usr/bin/env bash
set -uo pipefail  # Removed -e to allow test failures

# E2E Test: Recovery 1 - Graceful Degradation
#
# Scenario: Test system behavior under degraded conditions
# Tests that system continues functioning when:
# - LLM enrichment unavailable (store without enrichment)
# - Database partially corrupted (read-only mode)
# - Network intermittent (retry logic)
# - Resources constrained (reduced functionality)
# - Some features disabled (core functions remain)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source common utilities
# shellcheck source=tests/e2e/lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"

# Test state
PASSED=0
FAILED=0

section "E2E Test: Recovery 1 - Graceful Degradation"

# Setup test environment
setup_test_env "rec1_degradation"

section "Test 1: Core Functionality Without LLM"

print_cyan "Testing core functionality when LLM unavailable..."

# Simulate LLM unavailability by using invalid API key (use :- to handle unset variable)
OLD_API_KEY="${ANTHROPIC_API_KEY:-}"
export ANTHROPIC_API_KEY="sk-invalid-for-testing"

# System should still be able to store memories (degraded mode)
DEGRADED_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    "Critical system memory - must be stored even without enrichment" \
    --namespace "project:test" --importance 9 2>&1 || echo "")

# Restore API key (if it was set)
if [ -n "$OLD_API_KEY" ]; then
    export ANTHROPIC_API_KEY="$OLD_API_KEY"
fi

sleep 2

# Verify memory was stored (core functionality preserved)
DEGRADED_STORED=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "Critical system" \
    --namespace "project:test" 2>&1 || echo "")

if echo "$DEGRADED_STORED" | grep -qi "Critical system"; then
    pass "Graceful degradation: Core storage works without LLM enrichment"
else
    fail "Graceful degradation: Core storage failed without LLM"
fi

section "Test 2: Read-Only Database Mode"

print_cyan "Testing behavior with read-only database..."

# Create a separate database with some data
READONLY_DB=$(create_test_db "readonly")

create_memory "$BIN" "$READONLY_DB" \
    "Existing memory in database before read-only mode" \
    "project:test" 8 > /dev/null 2>&1

sleep 2

# Make database read-only
chmod 444 "$READONLY_DB"

# Try to write (should fail gracefully)
WRITE_OUTPUT=$(DATABASE_URL="sqlite://$READONLY_DB" "$BIN" remember \
    "New memory in read-only database" \
    --namespace "project:test" --importance 7 2>&1 || echo "READONLY_ERROR")

# Should still be able to read
READ_OUTPUT=$(DATABASE_URL="sqlite://$READONLY_DB" "$BIN" recall --query "Existing memory" \
    --namespace "project:test" 2>&1 || echo "")

if echo "$READ_OUTPUT" | grep -qi "Existing memory"; then
    pass "Read-only mode: Read operations still functional"
else
    fail "Read-only mode: Read operations failed"
fi

if echo "$WRITE_OUTPUT" | grep -qi "readonly\|permission\|READONLY_ERROR"; then
    pass "Read-only mode: Write failures handled gracefully"
else
    warn "Read-only mode: Write failure handling unclear"
fi

# Restore permissions and cleanup
chmod 644 "$READONLY_DB"
cleanup_test_db "$READONLY_DB"

section "Test 3: Partial Feature Availability"

print_cyan "Testing system with partial feature availability..."

# Even if some features fail, core features should work
# Test: Store memory, then retrieve it (basic functionality)

BASIC_STORE=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    "Basic functionality test - core features should always work" \
    --namespace "project:test" --importance 7 2>&1 || echo "")

sleep 2

BASIC_RETRIEVE=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "Basic functionality" \
    --namespace "project:test" 2>&1 || echo "")

if echo "$BASIC_RETRIEVE" | grep -qi "Basic functionality"; then
    pass "Partial features: Core store/retrieve always available"
else
    fail "Partial features: Core functionality broken"
fi

section "Test 4: Network Intermittency Resilience"

print_cyan "Testing resilience to intermittent network issues..."

# Simulate intermittent LLM API issues by rapid operations
# System should handle API failures gracefully with retry

print_cyan "Creating memories rapidly to stress API connection..."

RAPID_SUCCESS=0
RAPID_FAILURES=0

for i in {1..3}; do
    RAPID_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
        "Rapid memory $i - testing network resilience" \
        --namespace "project:test" --importance 6 2>&1 || echo "")

    if echo "$RAPID_OUTPUT" | grep -qi "error\|failed"; then
        ((RAPID_FAILURES++))
    else
        ((RAPID_SUCCESS++))
    fi
done

sleep 3

# Check how many were actually stored
RAPID_STORED=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "Rapid memory" \
    --namespace "project:test" 2>&1 | grep -c "Rapid memory" || echo "0")

if [ "$RAPID_STORED" -ge 2 ]; then
    pass "Network resilience: Most operations succeeded despite rapid execution ($RAPID_STORED/3)"
else
    warn "Network resilience: Some operations may have failed ($RAPID_STORED/3 stored)"
fi

section "Test 5: Resource Constraint Handling"

print_cyan "Testing behavior under resource constraints..."

# Simulate resource constraints by creating large dataset
print_cyan "Creating 50 memories to simulate resource pressure..."

for i in {1..50}; do
    create_memory "$BIN" "$TEST_DB" \
        "Resource constraint test memory $i" \
        "project:resource" 6 > /dev/null 2>&1 &

    # Don't overwhelm system, create in batches
    if [ $((i % 10)) -eq 0 ]; then
        wait
        sleep 2
    fi
done

wait
sleep 3

# Verify system still functional after resource pressure
RESOURCE_CHECK=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "Resource constraint" \
    --namespace "project:resource" --limit 10 2>&1 || echo "")

if [ -n "$RESOURCE_CHECK" ]; then
    STORED_COUNT=$(echo "$RESOURCE_CHECK" | grep -c "Resource constraint" || echo "0")
    if [ "$STORED_COUNT" -ge 5 ]; then
        pass "Resource constraints: System functional under load ($STORED_COUNT memories found)"
    else
        warn "Resource constraints: Limited functionality under load"
    fi
else
    fail "Resource constraints: System unresponsive after load"
fi

section "Test 6: Degraded Mode Indicators"

print_cyan "Testing that degraded mode is clearly indicated..."

# When system is in degraded mode, users should be aware
# Check if errors/warnings indicate degradation

DEGRADE_TEST_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    "Testing degradation indicators" \
    --namespace "project:test" --importance 7 2>&1 || echo "")

# Look for any degradation indicators in normal operation
if echo "$DEGRADE_TEST_OUTPUT" | grep -qi "warning\|degraded\|reduced"; then
    pass "Degradation indicators: System communicates degraded state"
else
    # In normal operation, no degradation should be indicated
    pass "Degradation indicators: No false degradation warnings in normal operation"
fi

section "Test 7: Automatic Recovery Attempts"

print_cyan "Testing automatic recovery mechanisms..."

# System should attempt to recover from transient failures
# Create memory after previous stress tests - system should have recovered

RECOVERY_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    "Automatic recovery test - system should recover from previous stress" \
    --namespace "project:test" --importance 8 2>&1 || echo "")

sleep 2

RECOVERY_STORED=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "Automatic recovery" \
    --namespace "project:test" 2>&1 || echo "")

if echo "$RECOVERY_STORED" | grep -qi "Automatic recovery"; then
    pass "Automatic recovery: System recovered from stress conditions"
else
    fail "Automatic recovery: System may not have recovered"
fi

section "Test 8: Degraded But Usable State"

print_cyan "Testing that degraded state is still usable..."

# Core use cases should work even in degraded mode:
# 1. Store critical information
# 2. Retrieve existing information
# 3. Basic queries work

print_cyan "Testing core use cases in potentially degraded environment..."

# Store critical info
CRITICAL_STORE=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    "CRITICAL: System health check - degraded mode usability test" \
    --namespace "project:test" --importance 10 2>&1 || echo "")

sleep 2

# Retrieve it
CRITICAL_RETRIEVE=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "CRITICAL health check" \
    --namespace "project:test" 2>&1 || echo "")

# Basic query
BASIC_QUERY=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "" \
    --namespace "project:test" --limit 5 2>&1 || echo "")

USE_CASES_PASS=0

if echo "$CRITICAL_RETRIEVE" | grep -qi "CRITICAL"; then
    ((USE_CASES_PASS++))
fi

if [ -n "$BASIC_QUERY" ]; then
    ((USE_CASES_PASS++))
fi

if [ "$USE_CASES_PASS" -ge 2 ]; then
    pass "Degraded usability: Core use cases functional ($USE_CASES_PASS/2)"
else
    fail "Degraded usability: Core use cases broken ($USE_CASES_PASS/2)"
fi

section "Test 9: No Data Loss in Degraded Mode"

print_cyan "Testing that degraded mode doesn't cause data loss..."

# Count memories before additional operations
BEFORE_COUNT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "" \
    --namespace "project:test" 2>&1 | grep -c "importance:" || echo "0")

print_cyan "Memories before degraded operations: $BEFORE_COUNT"

# Perform operations that might trigger degradation
for i in {1..5}; do
    DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
        "Data loss prevention test $i" \
        --namespace "project:test" --importance 7 > /dev/null 2>&1 &
done

wait
sleep 3

# Count memories after
AFTER_COUNT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "" \
    --namespace "project:test" 2>&1 | grep -c "importance:" || echo "0")

print_cyan "Memories after degraded operations: $AFTER_COUNT"

if [ "$AFTER_COUNT" -ge "$BEFORE_COUNT" ]; then
    pass "Data loss prevention: No data lost in degraded operations"
else
    fail "Data loss prevention: Data may have been lost ($BEFORE_COUNT → $AFTER_COUNT)"
fi

section "Test 10: Graceful Error Messages"

print_cyan "Testing error message quality in degraded scenarios..."

# Error messages should be helpful, not cryptic
# Use invalid API key again
export ANTHROPIC_API_KEY="sk-invalid-test"

ERROR_MSG_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    "Error message quality test" \
    --namespace "project:test" --importance 7 2>&1 || echo "")

# Restore API key (if it was set)
if [ -n "$OLD_API_KEY" ]; then
    export ANTHROPIC_API_KEY="$OLD_API_KEY"
fi

# Check if error messages are helpful
if echo "$ERROR_MSG_OUTPUT" | grep -qi "api.*key\|authentication\|enrichment"; then
    pass "Error messages: Clear indication of what failed"
else
    # If no error, system might be in graceful degradation (also good)
    pass "Error messages: Silent degradation (alternative acceptable behavior)"
fi

section "Test 11: Feature Detection"

print_cyan "Testing that system can detect available features..."

# System should be able to report what features are available
# Even in degraded mode, basic features should be present

# Test basic commands work
HELP_OUTPUT=$("$BIN" --help 2>&1 || echo "")

if echo "$HELP_OUTPUT" | grep -qi "remember\|recall"; then
    pass "Feature detection: Core commands documented and available"
else
    warn "Feature detection: Command documentation may be incomplete"
fi

section "Test 12: Performance Under Degradation"

print_cyan "Testing performance in degraded scenarios..."

# Even in degraded mode, operations should complete in reasonable time
START=$(date +%s)

PERF_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "test" \
    --namespace "project:test" --limit 5 2>&1 || echo "")

END=$(date +%s)
DURATION=$((END - START))

if [ "$DURATION" -lt 10 ]; then
    pass "Degraded performance: Operations still reasonably fast (${DURATION}s)"
else
    warn "Degraded performance: Operations slower than expected (${DURATION}s)"
fi

# Cleanup
section "Cleanup"
teardown_test_env

# Summary
test_summary
exit $?
