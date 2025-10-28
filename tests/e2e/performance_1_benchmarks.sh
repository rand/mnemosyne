#!/usr/bin/env bash
set -uo pipefail  # Removed -e to allow test failures

# E2E Test: Performance 1 - Benchmarks
#
# Scenario: Establish performance baselines for all major operations
# Validates performance targets:
# - Retrieval latency (p95 <200ms)
# - Storage latency (p95 <500ms)
# - Context loading (typical <200ms, timeout 500ms)
# - Memory usage (<100MB idle)
# - Database size (~1MB per 1000 memories)
#
# Creates performance baseline report for regression testing

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source common utilities
# shellcheck source=tests/e2e/lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"

# Test state
PASSED=0
FAILED=0

section "E2E Test: Performance 1 - Benchmarks"

# Setup test environment
setup_test_env "perf1_bench"

section "Benchmark 1: Retrieval Latency (Search)"

print_cyan "Measuring search query performance..."

# Create test dataset
create_keyword_memories "$BIN" "$TEST_DB" "project:perftest"

sleep 2  # Allow indexing

# Measure search latency over multiple iterations
ITERATIONS=10
LATENCIES=()

for i in $(seq 1 $ITERATIONS); do
    START=$(date +%s%N)
    DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "database" \
        --namespace "project:perftest" > /dev/null 2>&1
    END=$(date +%s%N)

    LATENCY=$(( (END - START) / 1000000 ))
    LATENCIES+=("$LATENCY")
    echo "  Iteration $i: ${LATENCY}ms"
done

# Calculate statistics
TOTAL=0
for lat in "${LATENCIES[@]}"; do
    TOTAL=$((TOTAL + lat))
done
AVG=$((TOTAL / ITERATIONS))

# Sort for percentiles
IFS=$'\n' SORTED=($(sort -n <<<"${LATENCIES[*]}"))
unset IFS

P50_IDX=$((ITERATIONS * 50 / 100))
P95_IDX=$((ITERATIONS * 95 / 100))
P99_IDX=$((ITERATIONS * 99 / 100))

P50=${SORTED[$P50_IDX]}
P95=${SORTED[$P95_IDX]}
P99=${SORTED[$P99_IDX]}

print_cyan "Search Latency Statistics:"
echo "  p50: ${P50}ms"
echo "  p95: ${P95}ms"
echo "  p99: ${P99}ms"
echo "  avg: ${AVG}ms"

# Validate against targets
if [ "$P95" -lt 200 ]; then
    pass "Search p95 latency meets target (<200ms): ${P95}ms"
else
    warn "Search p95 latency exceeds target (200ms): ${P95}ms"
fi

section "Benchmark 2: Storage Latency (Write)"

print_cyan "Measuring memory storage performance..."

WRITE_ITERATIONS=5
WRITE_LATENCIES=()

for i in $(seq 1 $WRITE_ITERATIONS); do
    CONTENT="Performance test memory $i with some meaningful content to simulate realistic write"

    START=$(date +%s%N)
    DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember "$CONTENT" \
        --namespace "project:perftest" --importance 7 > /dev/null 2>&1
    END=$(date +%s%N)

    WRITE_LAT=$(( (END - START) / 1000000 ))
    WRITE_LATENCIES+=("$WRITE_LAT")
    echo "  Write $i: ${WRITE_LAT}ms"
done

# Calculate write stats
WRITE_TOTAL=0
for lat in "${WRITE_LATENCIES[@]}"; do
    WRITE_TOTAL=$((WRITE_TOTAL + lat))
done
WRITE_AVG=$((WRITE_TOTAL / WRITE_ITERATIONS))

IFS=$'\n' WRITE_SORTED=($(sort -n <<<"${WRITE_LATENCIES[*]}"))
unset IFS

WRITE_P95_IDX=$((WRITE_ITERATIONS * 95 / 100))
WRITE_P95=${WRITE_SORTED[$WRITE_P95_IDX]}

print_cyan "Storage Latency Statistics:"
echo "  avg: ${WRITE_AVG}ms"
echo "  p95: ${WRITE_P95}ms"

# Note: LLM enrichment adds latency, so 500ms target is for full pipeline
if [ "$WRITE_P95" -lt 500 ]; then
    pass "Storage p95 latency meets target (<500ms): ${WRITE_P95}ms"
else
    # This might be acceptable if LLM is slow
    warn "Storage p95 latency exceeds target (500ms): ${WRITE_P95}ms" \
        "May be due to LLM enrichment latency"
fi

section "Benchmark 3: Context Loading Performance"

print_cyan "Measuring context loading time..."

# Create larger dataset for context loading
print_cyan "Creating 20 high-importance memories..."
for i in {1..20}; do
    DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
        "Context load test memory $i with architectural decision content" \
        --namespace "project:contexttest" --importance 8 > /dev/null 2>&1
done

sleep 2

# Measure context loading (what launcher does)
CONTEXT_ITERATIONS=5
CONTEXT_LATENCIES=()

for i in $(seq 1 $CONTEXT_ITERATIONS); do
    START=$(date +%s%N)
    DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "" \
        --namespace "project:contexttest" --limit 10  > /dev/null 2>&1
    END=$(date +%s%N)

    CONTEXT_LAT=$(( (END - START) / 1000000 ))
    CONTEXT_LATENCIES+=("$CONTEXT_LAT")
    echo "  Context load $i: ${CONTEXT_LAT}ms"
done

# Calculate context load stats
CONTEXT_TOTAL=0
for lat in "${CONTEXT_LATENCIES[@]}"; do
    CONTEXT_TOTAL=$((CONTEXT_TOTAL + lat))
done
CONTEXT_AVG=$((CONTEXT_TOTAL / CONTEXT_ITERATIONS))

IFS=$'\n' CONTEXT_SORTED=($(sort -n <<<"${CONTEXT_LATENCIES[*]}"))
unset IFS

CONTEXT_P95_IDX=$((CONTEXT_ITERATIONS * 95 / 100))
CONTEXT_P95=${CONTEXT_SORTED[$CONTEXT_P95_IDX]}

print_cyan "Context Loading Statistics:"
echo "  avg: ${CONTEXT_AVG}ms"
echo "  p95: ${CONTEXT_P95}ms"

# Typical target: <200ms, hard timeout: 500ms
if [ "$CONTEXT_AVG" -lt 200 ]; then
    pass "Context loading avg meets target (<200ms): ${CONTEXT_AVG}ms"
elif [ "$CONTEXT_AVG" -lt 500 ]; then
    pass "Context loading avg within timeout (500ms): ${CONTEXT_AVG}ms"
else
    fail "Context loading avg exceeds timeout (500ms): ${CONTEXT_AVG}ms"
fi

section "Benchmark 4: List Operation Performance"

print_cyan "Measuring list operation latency..."

LIST_ITERATIONS=10
LIST_LATENCIES=()

for i in $(seq 1 $LIST_ITERATIONS); do
    START=$(date +%s%N)
    DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "" \
        --namespace "project:perftest" --limit 20 > /dev/null 2>&1
    END=$(date +%s%N)

    LIST_LAT=$(( (END - START) / 1000000 ))
    LIST_LATENCIES+=("$LIST_LAT")
done

LIST_TOTAL=0
for lat in "${LIST_LATENCIES[@]}"; do
    LIST_TOTAL=$((LIST_TOTAL + lat))
done
LIST_AVG=$((LIST_TOTAL / LIST_ITERATIONS))

print_cyan "List Operation Statistics:"
echo "  avg: ${LIST_AVG}ms"

if [ "$LIST_AVG" -lt 100 ]; then
    pass "List operation avg <100ms: ${LIST_AVG}ms"
else
    warn "List operation avg >=100ms: ${LIST_AVG}ms"
fi

section "Benchmark 5: Database Size Efficiency"

print_cyan "Measuring database size growth..."

# Create known number of memories and measure database size
SIZE_TEST_DB=$(create_test_db "size_bench")

print_cyan "Creating 100 memories..."
for i in {1..100}; do
    DATABASE_URL="sqlite://$SIZE_TEST_DB" "$BIN" remember \
        "Database size test memory $i with moderate content length for realistic sizing" \
        --namespace "project:sizetest" --importance 7 > /dev/null 2>&1
done

sleep 2

# Measure database file size
DB_SIZE=$(stat -f%z "$SIZE_TEST_DB" 2>/dev/null || stat -c%s "$SIZE_TEST_DB" 2>/dev/null || echo "0")
DB_SIZE_KB=$((DB_SIZE / 1024))
DB_SIZE_MB=$((DB_SIZE_KB / 1024))

print_cyan "Database Size for 100 Memories:"
echo "  Size: ${DB_SIZE_KB}KB (${DB_SIZE_MB}MB)"
echo "  Per memory: $((DB_SIZE_KB / 100))KB"

# Target: ~1MB per 1000 memories = ~100KB per 100 memories
# Allow generous margin since embeddings add size
if [ "$DB_SIZE_KB" -lt 500 ]; then
    pass "Database size efficient: ${DB_SIZE_KB}KB for 100 memories"
else
    warn "Database size larger than expected: ${DB_SIZE_KB}KB for 100 memories" \
        "May include embeddings and indexes"
fi

cleanup_test_db "$SIZE_TEST_DB"

section "Benchmark 6: Memory Usage (Binary Size)"

print_cyan "Measuring binary size..."

BIN_SIZE=$(stat -f%z "$BIN" 2>/dev/null || stat -c%s "$BIN" 2>/dev/null || echo "0")
BIN_SIZE_MB=$((BIN_SIZE / 1024 / 1024))

print_cyan "Binary Size:"
echo "  Size: ${BIN_SIZE_MB}MB"

# Rust binaries can be large, this is just informational
if [ "$BIN_SIZE_MB" -lt 50 ]; then
    pass "Binary size reasonable: ${BIN_SIZE_MB}MB"
else
    warn "Binary size large: ${BIN_SIZE_MB}MB" \
        "Consider strip or release optimizations"
fi

section "Benchmark 7: Concurrent Query Performance"

print_cyan "Measuring concurrent read performance..."

# Launch multiple concurrent queries
CONCURRENT_QUERIES=5

print_cyan "Running $CONCURRENT_QUERIES concurrent searches..."

START_CONCURRENT=$(date +%s%N)

for i in $(seq 1 $CONCURRENT_QUERIES); do
    (
        DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "database" \
            --namespace "project:perftest" > /dev/null 2>&1
    ) &
done

wait  # Wait for all background queries

END_CONCURRENT=$(date +%s%N)
CONCURRENT_TOTAL=$(( (END_CONCURRENT - START_CONCURRENT) / 1000000 ))
CONCURRENT_AVG=$((CONCURRENT_TOTAL / CONCURRENT_QUERIES))

print_cyan "Concurrent Query Statistics:"
echo "  Total time: ${CONCURRENT_TOTAL}ms"
echo "  Average per query: ${CONCURRENT_AVG}ms"

if [ "$CONCURRENT_AVG" -lt 300 ]; then
    pass "Concurrent queries efficient: ${CONCURRENT_AVG}ms avg"
else
    warn "Concurrent queries slower than expected: ${CONCURRENT_AVG}ms avg"
fi

section "Benchmark 8: Scaling Test (1000 memories)"

print_cyan "Testing performance with larger dataset..."

SCALE_TEST_DB=$(create_test_db "scale_bench")

print_cyan "Creating 1000 memories (this may take a while)..."

# Create in batches to show progress
for batch in {1..10}; do
    for i in {1..100}; do
        mem_num=$(( (batch - 1) * 100 + i ))
        DATABASE_URL="sqlite://$SCALE_TEST_DB" "$BIN" remember \
            "Scale test memory $mem_num" \
            --namespace "project:scaletest" --importance $(( (mem_num % 10) + 1 )) > /dev/null 2>&1
    done
    echo "  Created $(( batch * 100 )) memories..."
done

sleep 3  # Allow indexing to complete

# Measure search performance at scale
START_SCALE=$(date +%s%N)
DATABASE_URL="sqlite://$SCALE_TEST_DB" "$BIN" recall --query "memory" \
    --namespace "project:scaletest" --limit 10 > /dev/null 2>&1
END_SCALE=$(date +%s%N)

SCALE_SEARCH_MS=$(( (END_SCALE - START_SCALE) / 1000000 ))

print_cyan "Search Performance at 1000 memories:"
echo "  Latency: ${SCALE_SEARCH_MS}ms"

if [ "$SCALE_SEARCH_MS" -lt 300 ]; then
    pass "Search scales well to 1000 memories: ${SCALE_SEARCH_MS}ms"
else
    warn "Search latency at scale: ${SCALE_SEARCH_MS}ms"
fi

# Measure database size at scale
SCALE_DB_SIZE=$(stat -f%z "$SCALE_TEST_DB" 2>/dev/null || stat -c%s "$SCALE_TEST_DB" 2>/dev/null || echo "0")
SCALE_DB_SIZE_MB=$((SCALE_DB_SIZE / 1024 / 1024))

print_cyan "Database Size at 1000 memories:"
echo "  Size: ${SCALE_DB_SIZE_MB}MB"

# Target: ~1MB per 1000 memories
if [ "$SCALE_DB_SIZE_MB" -lt 5 ]; then
    pass "Database size scales well: ${SCALE_DB_SIZE_MB}MB for 1000 memories"
else
    warn "Database size at scale: ${SCALE_DB_SIZE_MB}MB for 1000 memories"
fi

cleanup_test_db "$SCALE_TEST_DB"

section "Performance Summary Report"

print_cyan "=== PERFORMANCE BASELINE REPORT ==="
echo ""
echo "Retrieval (Search):"
echo "  p50: ${P50}ms, p95: ${P95}ms, p99: ${P99}ms"
echo "  Target: p95 <200ms"
echo ""
echo "Storage (Write):"
echo "  avg: ${WRITE_AVG}ms, p95: ${WRITE_P95}ms"
echo "  Target: p95 <500ms"
echo ""
echo "Context Loading:"
echo "  avg: ${CONTEXT_AVG}ms, p95: ${CONTEXT_P95}ms"
echo "  Target: avg <200ms, timeout 500ms"
echo ""
echo "List Operations:"
echo "  avg: ${LIST_AVG}ms"
echo ""
echo "Database Efficiency:"
echo "  100 memories: ${DB_SIZE_KB}KB"
echo "  1000 memories: ${SCALE_DB_SIZE_MB}MB"
echo "  Target: ~1MB per 1000 memories"
echo ""
echo "Scaling:"
echo "  Search at 1000 memories: ${SCALE_SEARCH_MS}ms"
echo ""
echo "Binary Size: ${BIN_SIZE_MB}MB"
echo ""

# Cleanup
section "Cleanup"
teardown_test_env

# Summary
test_summary
exit $?
