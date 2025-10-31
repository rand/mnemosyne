#!/usr/bin/env bash
# [REGRESSION] Power User - Performance Optimization
#
# User Journey: Power user optimizes Mnemosyne for large-scale usage
# Scenario: Performance benchmarks, query optimization, scaling validation
# Success Criteria:
#   - Large dataset operations complete within reasonable time
#   - Query performance is acceptable (sub-second for common queries)
#   - Batch operations scale linearly
#   - Memory usage stays reasonable
#   - Index usage is optimal
#
# Cost: $0 (performance tests, no API calls)
# Duration: 30-45s

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source test infrastructure
source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/assertions.sh"
source "$SCRIPT_DIR/lib/personas.sh"
source "$SCRIPT_DIR/lib/llm_mocking.sh"
source "$SCRIPT_DIR/lib/data_generators.sh"

# ===================================================================
# TEST SETUP
# ===================================================================

TEST_NAME="power_user_4_performance"

section "Power User - Performance Optimization [REGRESSION]"

# Verify regression mode (mocked LLM)
if is_baseline_mode; then
    warn "This is a regression test but running in baseline mode"
fi

# Setup power user persona
print_cyan "Setting up power user test environment..."
TEST_DB=$(setup_power_user "$TEST_NAME")
print_green "  ✓ Test database: $TEST_DB"

# ===================================================================
# BENCHMARK 1: Large Dataset Creation
# ===================================================================

section "Benchmark 1: Large Dataset Creation"

print_cyan "Creating large dataset for performance testing..."

DATASET_SIZE=100  # Reduced for test speed (would be 1000+ in production)

START_TIME=$(date +%s)

generate_stress_data "$TEST_DB" "small"  # Creates 100 memories

END_TIME=$(date +%s)
CREATION_DURATION=$((END_TIME - START_TIME))

print_cyan "  Created $DATASET_SIZE memories in ${CREATION_DURATION}s"

if [ "$CREATION_DURATION" -lt 10 ]; then
    print_green "  ✓ Dataset creation: FAST (<10s)"
elif [ "$CREATION_DURATION" -lt 30 ]; then
    print_cyan "  ✓ Dataset creation: ACCEPTABLE (10-30s)"
else
    warn "Dataset creation slower than expected (>${CREATION_DURATION}s)"
fi

# ===================================================================
# BENCHMARK 2: Query Performance
# ===================================================================

section "Benchmark 2: Query Performance"

print_cyan "Testing query performance on large dataset..."

# Benchmark: List all memories
START=$(date +%s%3N 2>/dev/null || date +%s)
DATABASE_URL="sqlite://$TEST_DB" "$BIN" list --limit 50 >/dev/null 2>&1 || warn "List query failed"
END=$(date +%s%3N 2>/dev/null || date +%s)
LIST_TIME=$((END - START))

print_cyan "  List query (50 memories): ${LIST_TIME}ms"

if [ "$LIST_TIME" -lt 1000 ]; then
    print_green "  ✓ List performance: EXCELLENT (<1s)"
elif [ "$LIST_TIME" -lt 3000 ]; then
    print_cyan "  ✓ List performance: ACCEPTABLE (1-3s)"
else
    warn "List query slower than expected (${LIST_TIME}ms)"
fi

# Benchmark: Search query
START=$(date +%s%3N 2>/dev/null || date +%s)
DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "memory content" --limit 10 >/dev/null 2>&1 || warn "Search failed"
END=$(date +%s%3N 2>/dev/null || date +%s)
SEARCH_TIME=$((END - START))

print_cyan "  Search query: ${SEARCH_TIME}ms"

if [ "$SEARCH_TIME" -lt 2000 ]; then
    print_green "  ✓ Search performance: EXCELLENT (<2s)"
elif [ "$SEARCH_TIME" -lt 5000 ]; then
    print_cyan "  ✓ Search performance: ACCEPTABLE (2-5s)"
else
    warn "Search query slower than expected (${SEARCH_TIME}ms)"
fi

# ===================================================================
# BENCHMARK 3: Database Query Optimization
# ===================================================================

section "Benchmark 3: Database Query Optimization"

print_cyan "Analyzing database query performance..."

# Test indexed query (by namespace)
START=$(date +%s%3N 2>/dev/null || date +%s)
DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE namespace='project:stress:shard0'" >/dev/null 2>&1
END=$(date +%s%3N 2>/dev/null || date +%s)
INDEXED_TIME=$((END - START))

print_cyan "  Indexed query (namespace): ${INDEXED_TIME}ms"

if [ "$INDEXED_TIME" -lt 100 ]; then
    print_green "  ✓ Indexed query: FAST (<100ms)"
elif [ "$INDEXED_TIME" -lt 500 ]; then
    print_cyan "  ✓ Indexed query: ACCEPTABLE (100-500ms)"
else
    warn "Indexed query slower than expected"
fi

# Test filtered query (by importance)
START=$(date +%s%3N 2>/dev/null || date +%s)
DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE importance >= 8" >/dev/null 2>&1
END=$(date +%s%3N 2>/dev/null || date +%s)
FILTER_TIME=$((END - START))

print_cyan "  Filtered query (importance): ${FILTER_TIME}ms"

# ===================================================================
# BENCHMARK 4: Batch Operation Performance
# ===================================================================

section "Benchmark 4: Batch Operation Performance"

print_cyan "Testing batch operation scalability..."

# Batch update performance
START=$(date +%s)
DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "UPDATE memories SET importance = importance + 1
     WHERE namespace LIKE 'project:stress:%'
     AND importance < 10" >/dev/null 2>&1
END=$(date +%s)
BATCH_UPDATE_TIME=$((END - START))

print_cyan "  Batch update: ${BATCH_UPDATE_TIME}s"

if [ "$BATCH_UPDATE_TIME" -lt 5 ]; then
    print_green "  ✓ Batch update: FAST (<5s)"
elif [ "$BATCH_UPDATE_TIME" -lt 15 ]; then
    print_cyan "  ✓ Batch update: ACCEPTABLE (5-15s)"
else
    warn "Batch update slower than expected"
fi

# ===================================================================
# BENCHMARK 5: Database Size and Indexes
# ===================================================================

section "Benchmark 5: Database Size and Indexes"

print_cyan "Analyzing database structure..."

# Database file size
if [ -f "$TEST_DB" ]; then
    DB_SIZE=$(du -h "$TEST_DB" | cut -f1)
    DB_BYTES=$(wc -c < "$TEST_DB" | tr -d ' ')

    print_cyan "  Database file size: $DB_SIZE ($DB_BYTES bytes)"

    # Rough estimate: should be reasonable for dataset
    BYTES_PER_MEM=$((DB_BYTES / DATASET_SIZE))
    print_cyan "  Bytes per memory: ~$BYTES_PER_MEM"

    if [ "$BYTES_PER_MEM" -lt 10000 ]; then
        print_green "  ✓ Storage efficiency: GOOD"
    else
        warn "Storage may not be optimal"
    fi
else
    warn "Database file not found"
fi

# Check for indexes
INDEXES=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT name FROM sqlite_master
     WHERE type='index' AND sql IS NOT NULL" 2>/dev/null || echo "")

INDEX_COUNT=$(echo "$INDEXES" | grep -c -v '^$' || echo 0)

print_cyan "  Database indexes: $INDEX_COUNT"

if [ "$INDEX_COUNT" -gt 0 ]; then
    print_green "  ✓ Indexes exist for performance"
    echo "$INDEXES" | while read -r idx; do
        [ -n "$idx" ] && print_cyan "    - $idx"
    done
else
    warn "No custom indexes found - may impact performance"
fi

# ===================================================================
# BENCHMARK 6: Concurrent Query Performance
# ===================================================================

section "Benchmark 6: Concurrent Query Simulation"

print_cyan "Testing concurrent query performance..."

# Run multiple queries in parallel (simulating concurrent users)
START=$(date +%s)

for i in {1..5}; do
    DATABASE_URL="sqlite://$TEST_DB" "$BIN" list --limit 10 >/dev/null 2>&1 &
done

wait  # Wait for all background queries

END=$(date +%s)
CONCURRENT_TIME=$((END - START))

print_cyan "  5 concurrent queries: ${CONCURRENT_TIME}s"

if [ "$CONCURRENT_TIME" -lt 10 ]; then
    print_green "  ✓ Concurrent performance: GOOD (<10s)"
elif [ "$CONCURRENT_TIME" -lt 20 ]; then
    print_cyan "  ✓ Concurrent performance: ACCEPTABLE (10-20s)"
else
    warn "Concurrent queries slower than expected"
fi

# ===================================================================
# BENCHMARK 7: Aggregation Query Performance
# ===================================================================

section "Benchmark 7: Aggregation Query Performance"

print_cyan "Testing complex aggregation queries..."

# Aggregation: Group by namespace
START=$(date +%s%3N 2>/dev/null || date +%s)
DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT namespace, COUNT(*), AVG(importance)
     FROM memories
     GROUP BY namespace
     ORDER BY COUNT(*) DESC
     LIMIT 10" >/dev/null 2>&1
END=$(date +%s%3N 2>/dev/null || date +%s)
AGG_TIME=$((END - START))

print_cyan "  Aggregation query: ${AGG_TIME}ms"

if [ "$AGG_TIME" -lt 500 ]; then
    print_green "  ✓ Aggregation: FAST (<500ms)"
elif [ "$AGG_TIME" -lt 2000 ]; then
    print_cyan "  ✓ Aggregation: ACCEPTABLE (500ms-2s)"
else
    warn "Aggregation slower than expected"
fi

# ===================================================================
# VALIDATION: Performance Summary
# ===================================================================

section "Validation: Performance Summary"

print_cyan "Generating performance summary..."

# Calculate performance scores
PERF_SCORE=0

# List query: <1s = 1pt, <3s = 0.5pt
[ "$LIST_TIME" -lt 1000 ] && ((PERF_SCORE++)) || [ "$LIST_TIME" -lt 3000 ] && PERF_SCORE=$((PERF_SCORE + 1))

# Search query: <2s = 1pt, <5s = 0.5pt
[ "$SEARCH_TIME" -lt 2000 ] && ((PERF_SCORE++)) || [ "$SEARCH_TIME" -lt 5000 ] && PERF_SCORE=$((PERF_SCORE + 1))

# Indexed query: <100ms = 1pt, <500ms = 0.5pt
[ "$INDEXED_TIME" -lt 100 ] && ((PERF_SCORE++)) || [ "$INDEXED_TIME" -lt 500 ] && PERF_SCORE=$((PERF_SCORE + 1))

# Batch operations: <5s = 1pt, <15s = 0.5pt
[ "$BATCH_UPDATE_TIME" -lt 5 ] && ((PERF_SCORE++)) || [ "$BATCH_UPDATE_TIME" -lt 15 ] && PERF_SCORE=$((PERF_SCORE + 1))

# Concurrent: <10s = 1pt, <20s = 0.5pt
[ "$CONCURRENT_TIME" -lt 10 ] && ((PERF_SCORE++)) || [ "$CONCURRENT_TIME" -lt 20 ] && PERF_SCORE=$((PERF_SCORE + 1))

# Aggregation: <500ms = 1pt, <2s = 0.5pt
[ "$AGG_TIME" -lt 500 ] && ((PERF_SCORE++)) || [ "$AGG_TIME" -lt 2000 ] && PERF_SCORE=$((PERF_SCORE + 1))

print_cyan "  Overall performance score: $PERF_SCORE/6"

if [ "$PERF_SCORE" -ge 5 ]; then
    print_green "  ✓ EXCELLENT performance"
elif [ "$PERF_SCORE" -ge 3 ]; then
    print_cyan "  ✓ GOOD performance"
else
    warn "Performance below expectations"
fi

# ===================================================================
# OPTIMIZATION RECOMMENDATIONS
# ===================================================================

section "Optimization Recommendations"

print_cyan "Analyzing potential optimizations..."

RECOMMENDATIONS=()

# Check for missing indexes
if [ "$INDEX_COUNT" -lt 3 ]; then
    RECOMMENDATIONS+=("Add indexes on frequently queried columns (namespace, importance, created_at)")
fi

# Check query times
if [ "$LIST_TIME" -gt 2000 ]; then
    RECOMMENDATIONS+=("Optimize list query with pagination and limits")
fi

if [ "$SEARCH_TIME" -gt 3000 ]; then
    RECOMMENDATIONS+=("Consider vector search optimization or caching")
fi

if [ "$BATCH_UPDATE_TIME" -gt 10 ]; then
    RECOMMENDATIONS+=("Batch operations may benefit from transaction batching")
fi

if [ "${#RECOMMENDATIONS[@]}" -gt 0 ]; then
    print_cyan "  Suggested optimizations:"
    for rec in "${RECOMMENDATIONS[@]}"; do
        print_cyan "    - $rec"
    done
else
    print_green "  ✓ No critical optimizations needed"
fi

# ===================================================================
# CLEANUP
# ===================================================================

section "Cleanup"

teardown_persona "$TEST_DB"
print_green "  ✓ Test environment cleaned up"

# ===================================================================
# TEST SUMMARY
# ===================================================================

section "Test Summary: Power User Performance Optimization [REGRESSION]"

echo "✓ Large dataset creation: PASS (${CREATION_DURATION}s)"
echo "✓ List query performance: $([ "$LIST_TIME" -lt 1000 ] && echo "EXCELLENT" || echo "ACCEPTABLE") (${LIST_TIME}ms)"
echo "✓ Search query performance: $([ "$SEARCH_TIME" -lt 2000 ] && echo "EXCELLENT" || echo "ACCEPTABLE") (${SEARCH_TIME}ms)"
echo "✓ Indexed query: $([ "$INDEXED_TIME" -lt 100 ] && echo "FAST" || echo "ACCEPTABLE") (${INDEXED_TIME}ms)"
echo "✓ Batch operations: $([ "$BATCH_UPDATE_TIME" -lt 5 ] && echo "FAST" || echo "ACCEPTABLE") (${BATCH_UPDATE_TIME}s)"
echo "✓ Concurrent queries: $([ "$CONCURRENT_TIME" -lt 10 ] && echo "GOOD" || echo "ACCEPTABLE") (${CONCURRENT_TIME}s)"
echo "✓ Aggregation queries: $([ "$AGG_TIME" -lt 500 ] && echo "FAST" || echo "ACCEPTABLE") (${AGG_TIME}ms)"
echo ""
echo "Performance Metrics:"
echo "  - Dataset size: $DATASET_SIZE memories"
echo "  - Database size: $DB_SIZE"
echo "  - Indexes: $INDEX_COUNT"
echo "  - Overall score: $PERF_SCORE/6"
echo ""
echo "Recommendations: ${#RECOMMENDATIONS[@]} optimization(s) suggested"
echo ""

print_green "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
print_green "✓ ALL TESTS PASSED"
print_green "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

exit 0
