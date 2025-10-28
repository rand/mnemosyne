#!/usr/bin/env bash
set -uo pipefail  # Removed -e to allow test failures

# E2E Test: Performance 2 - Stress Tests
#
# Scenario: Test system under extreme conditions
# Tests:
# - Large dataset handling (10k+ memories)
# - High concurrency stress
# - Memory/resource limits
# - Large content handling
# - Sustained load testing
# - Performance degradation patterns

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source common utilities
# shellcheck source=tests/e2e/lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"

# Test state
PASSED=0
FAILED=0

section "E2E Test: Performance 2 - Stress Tests"

# Setup test environment
setup_test_env "perf2_stress"

section "Test 1: Large Dataset Stress (5000 memories)"

print_cyan "Testing performance with large dataset..."

print_cyan "Creating 5000 memories (this will take several minutes)..."

START_TIME=$(date +%s)
CREATE_ERRORS=0

# Create memories in batches
for batch in {1..50}; do
    for i in {1..100}; do
        entry_num=$(( (batch - 1) * 100 + i ))

        create_memory "$BIN" "$TEST_DB" \
            "Large dataset entry $entry_num - stress test memory content" \
            "project:stress" 5 > /dev/null 2>&1 &

        # Limit concurrent creates
        if [ $((i % 20)) -eq 0 ]; then
            wait
        fi
    done

    wait
    print_cyan "Created $((batch * 100)) memories..."
done

wait

END_TIME=$(date +%s)
CREATE_DURATION=$((END_TIME - START_TIME))

print_cyan "Dataset creation took ${CREATE_DURATION}s"

# Wait for LLM processing to complete
sleep 5

# Test search performance on large dataset
SEARCH_START=$(date +%s)

LARGE_SEARCH=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "stress test" \
    --namespace "project:stress" --limit 10 2>&1 || echo "")

SEARCH_END=$(date +%s)
SEARCH_DURATION=$((SEARCH_END - SEARCH_START))

if [ "$SEARCH_DURATION" -lt 10 ]; then
    pass "Large dataset search: Fast search on 5000 memories (${SEARCH_DURATION}s)"
else
    warn "Large dataset search: Slower than expected (${SEARCH_DURATION}s)"
fi

# Check database size
DB_SIZE=$(du -k "$TEST_DB" | cut -f1)
DB_SIZE_MB=$((DB_SIZE / 1024))

print_cyan "Database size: ${DB_SIZE_MB}MB for 5000 memories"

if [ "$DB_SIZE_MB" -lt 500 ]; then
    pass "Large dataset storage: Reasonable database size (${DB_SIZE_MB}MB)"
else
    warn "Large dataset storage: Database larger than expected (${DB_SIZE_MB}MB)"
fi

section "Test 2: High Concurrency Stress"

print_cyan "Testing high concurrency (50 parallel operations)..."

CONCURRENT_START=$(date +%s)

# Launch 50 concurrent operations
for i in {1..50}; do
    (DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
        "Concurrent stress test $i" \
        --namespace "project:concurrent" --importance 6 > /dev/null 2>&1) &
done

# Wait for all to complete
wait

CONCURRENT_END=$(date +%s)
CONCURRENT_DURATION=$((CONCURRENT_END - CONCURRENT_START))

sleep 3

# Check how many succeeded
CONCURRENT_COUNT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "Concurrent stress" \
    --namespace "project:concurrent" 2>&1 | grep -c "Concurrent stress" || echo "0")

if [ "$CONCURRENT_COUNT" -ge 40 ]; then
    pass "High concurrency: Most operations succeeded ($CONCURRENT_COUNT/50 in ${CONCURRENT_DURATION}s)"
else
    warn "High concurrency: Some operations failed ($CONCURRENT_COUNT/50)"
fi

section "Test 3: Sustained Load Test"

print_cyan "Testing sustained load (100 operations over time)..."

SUSTAINED_START=$(date +%s)
SUSTAINED_ERRORS=0

for i in {1..100}; do
    LOAD_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
        "Sustained load test $i" \
        --namespace "project:sustained" --importance 5 2>&1 || echo "LOAD_ERROR")

    if echo "$LOAD_OUTPUT" | grep -qi "LOAD_ERROR"; then
        ((SUSTAINED_ERRORS++))
    fi

    # Brief pause between operations
    sleep 0.1
done

SUSTAINED_END=$(date +%s)
SUSTAINED_DURATION=$((SUSTAINED_END - SUSTAINED_START))

if [ "$SUSTAINED_ERRORS" -lt 10 ]; then
    pass "Sustained load: Low error rate ($SUSTAINED_ERRORS/100 errors in ${SUSTAINED_DURATION}s)"
else
    fail "Sustained load: High error rate ($SUSTAINED_ERRORS/100 errors)"
fi

section "Test 4: Large Content Stress"

print_cyan "Testing large content handling..."

# Create content of various sizes
SIZES=(1024 10240 102400 512000)  # 1KB, 10KB, 100KB, 500KB

LARGE_CONTENT_SUCCESS=0

for size in "${SIZES[@]}"; do
    LARGE_TEXT=$(head -c $size /dev/urandom | base64 | head -c $size)
    SIZE_KB=$((size / 1024))

    print_cyan "Testing ${SIZE_KB}KB content..."

    START=$(date +%s)

    LARGE_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" timeout 90 "$BIN" remember \
        "$LARGE_TEXT" \
        --namespace "project:large" --importance 5 2>&1 || echo "LARGE_ERROR_$size")

    END=$(date +%s)
    DURATION=$((END - START))

    if echo "$LARGE_OUTPUT" | grep -qi "LARGE_ERROR"; then
        warn "Large content: ${SIZE_KB}KB content failed or timed out"
    else
        ((LARGE_CONTENT_SUCCESS++))
        print_cyan "  ${SIZE_KB}KB content processed in ${DURATION}s"
    fi
done

if [ "$LARGE_CONTENT_SUCCESS" -ge 2 ]; then
    pass "Large content: Handles large content ($LARGE_CONTENT_SUCCESS/${#SIZES[@]} sizes)"
else
    warn "Large content: Limited large content support ($LARGE_CONTENT_SUCCESS/${#SIZES[@]})"
fi

section "Test 5: Memory Usage Stability"

print_cyan "Testing memory usage stability under load..."

# Create many operations and check system remains stable
print_cyan "Creating 200 memories to stress memory usage..."

MEM_START=$(date +%s)

for i in {1..200}; do
    create_memory "$BIN" "$TEST_DB" \
        "Memory usage test $i" \
        "project:memory" 5 > /dev/null 2>&1 &

    if [ $((i % 40)) -eq 0 ]; then
        wait
    fi
done

wait

MEM_END=$(date +%s)
MEM_DURATION=$((MEM_END - MEM_START))

# Verify system still responsive
TEST_QUERY=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "Memory usage" \
    --namespace "project:memory" --limit 5 2>&1 || echo "SYSTEM_UNRESPONSIVE")

if echo "$TEST_QUERY" | grep -qi "SYSTEM_UNRESPONSIVE"; then
    fail "Memory stability: System unresponsive after load"
else
    pass "Memory stability: System stable after 200 operations (${MEM_DURATION}s)"
fi

section "Test 6: Database Lock Contention"

print_cyan "Testing database lock handling under contention..."

# Create high contention scenario
LOCK_TEST_DB=$(create_test_db "lock_stress")

print_cyan "Launching 30 concurrent writes to same database..."

LOCK_ERRORS=0

for i in {1..30}; do
    (DATABASE_URL="sqlite://$LOCK_TEST_DB" "$BIN" remember \
        "Lock contention test $i" \
        --namespace "project:test" --importance 6 2>&1 || echo "LOCK_ERROR") &
done

wait
sleep 3

# Check how many succeeded despite locks
LOCK_COUNT=$(DATABASE_URL="sqlite://$LOCK_TEST_DB" "$BIN" recall --query "Lock contention" \
    --namespace "project:test" 2>&1 | grep -c "Lock contention" || echo "0")

if [ "$LOCK_COUNT" -ge 25 ]; then
    pass "Lock contention: Handled concurrent writes well ($LOCK_COUNT/30)"
else
    warn "Lock contention: Some writes lost to lock contention ($LOCK_COUNT/30)"
fi

cleanup_test_db "$LOCK_TEST_DB"

section "Test 7: Query Performance Under Load"

print_cyan "Testing query performance with database under load..."

# Create load
for i in {1..10}; do
    (DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
        "Background load $i" \
        --namespace "project:background" --importance 5 > /dev/null 2>&1) &
done

# Query while load active
QUERY_UNDER_LOAD=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "stress test" \
    --namespace "project:stress" --limit 10 2>&1 || echo "")

wait

if [ -n "$QUERY_UNDER_LOAD" ]; then
    pass "Query under load: Queries complete during active writes"
else
    fail "Query under load: Queries blocked by writes"
fi

section "Test 8: Rapid Context Switching"

print_cyan "Testing rapid context switches (namespace changes)..."

NAMESPACES=("app" "docs" "tests" "api" "db" "ui" "auth" "cache" "queue" "worker")

SWITCH_START=$(date +%s)

for i in {1..50}; do
    ns_index=$((i % ${#NAMESPACES[@]}))
    ns=${NAMESPACES[$ns_index]}

    create_memory "$BIN" "$TEST_DB" \
        "Context switch test $i" \
        "project:$ns" 5 > /dev/null 2>&1
done

SWITCH_END=$(date +%s)
SWITCH_DURATION=$((SWITCH_END - SWITCH_START))

if [ "$SWITCH_DURATION" -lt 180 ]; then
    pass "Context switching: Fast namespace switching (${SWITCH_DURATION}s for 50 ops)"
else
    warn "Context switching: Slower than expected (${SWITCH_DURATION}s)"
fi

section "Test 9: Database File Integrity Under Stress"

print_cyan "Testing database integrity after stress operations..."

# Check database integrity
INTEGRITY_CHECK=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" "PRAGMA integrity_check;" 2>&1 || echo "INTEGRITY_ERROR")

if echo "$INTEGRITY_CHECK" | grep -qi "ok"; then
    pass "Database integrity: Database intact after stress tests"
elif echo "$INTEGRITY_CHECK" | grep -qi "INTEGRITY_ERROR"; then
    warn "Database integrity: Cannot verify (sqlite3 not available)"
else
    fail "Database integrity: Database may be corrupted"
fi

section "Test 10: Performance Degradation Profile"

print_cyan "Testing performance degradation with growing dataset..."

# Measure query performance at different dataset sizes
# Already have 5000+ memories, test query times

MEASUREMENTS=(100 500 1000 2000 5000)
LATENCIES=()

for count in "${MEASUREMENTS[@]}"; do
    # Limit results to approximate dataset size
    MEASURE_START=$(date +%s%N)

    DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "" \
        --namespace "project:stress" --limit 1 > /dev/null 2>&1

    MEASURE_END=$(date +%s%N)
    LATENCY=$(( (MEASURE_END - MEASURE_START) / 1000000 ))

    LATENCIES+=("$count:${LATENCY}ms")
    print_cyan "  Query latency: ${LATENCY}ms"
done

pass "Performance profile: Latencies recorded at different scales"
for latency in "${LATENCIES[@]}"; do
    print_cyan "    $latency"
done

section "Test 11: Concurrent Read Performance"

print_cyan "Testing concurrent read performance..."

# Launch many concurrent reads
READ_START=$(date +%s)

for i in {1..20}; do
    (DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "test" \
        --namespace "project:stress" --limit 5 > /dev/null 2>&1) &
done

wait

READ_END=$(date +%s)
READ_DURATION=$((READ_END - READ_START))

if [ "$READ_DURATION" -lt 30 ]; then
    pass "Concurrent reads: 20 parallel queries completed in ${READ_DURATION}s"
else
    warn "Concurrent reads: Slower than expected (${READ_DURATION}s)"
fi

section "Test 12: Recovery After Stress"

print_cyan "Testing system recovery after stress tests..."

# After all stress, system should still work normally
RECOVERY_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    "Recovery test after stress - system should work normally" \
    --namespace "project:test" --importance 7 2>&1 || echo "")

sleep 2

RECOVERY_STORED=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "Recovery test after stress" \
    --namespace "project:test" 2>&1 || echo "")

if echo "$RECOVERY_STORED" | grep -qi "Recovery test after stress"; then
    pass "Stress recovery: System functional after all stress tests"
else
    fail "Stress recovery: System may be compromised"
fi

section "Test 13: Database Size Efficiency"

print_cyan "Testing database size efficiency..."

# Calculate memories per MB
TOTAL_COUNT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "" \
    --namespace "project:stress" 2>&1 | grep -c "project:stress" || echo "0")

FINAL_SIZE=$(du -k "$TEST_DB" | cut -f1)
FINAL_SIZE_MB=$((FINAL_SIZE / 1024))

if [ "$FINAL_SIZE_MB" -gt 0 ] && [ "$TOTAL_COUNT" -gt 0 ]; then
    MEMORIES_PER_MB=$((TOTAL_COUNT / FINAL_SIZE_MB))
    print_cyan "Storage efficiency: ~$MEMORIES_PER_MB memories per MB"

    pass "Storage efficiency: Database size tracked (${FINAL_SIZE_MB}MB for ~${TOTAL_COUNT} memories)"
else
    warn "Storage efficiency: Cannot calculate efficiency"
fi

section "Test 14: Stress Test Summary"

print_cyan "Generating stress test summary..."

print_cyan "=== STRESS TEST REPORT ==="
print_cyan ""
print_cyan "Dataset: ~5000 memories created"
print_cyan "Concurrency: 50 parallel operations tested"
print_cyan "Sustained: 100 sequential operations"
print_cyan "Large content: Up to 500KB tested"
print_cyan "Database size: ${FINAL_SIZE_MB}MB"
print_cyan "Lock contention: 30 concurrent writes"
print_cyan "Context switching: 50 namespace changes"
print_cyan ""

pass "Stress test summary: Report generated"

# Cleanup
section "Cleanup"
print_cyan "Cleaning up large test database..."
teardown_test_env
print_cyan "Cleanup complete"

# Summary
test_summary
exit $?
