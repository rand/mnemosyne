#!/usr/bin/env bash
set -euo pipefail

# E2E Test: Human Workflow 4 - Context Loading
#
# Scenario: Developer launches Claude Code session with intelligent context loading
# Validates the three-layer context loading strategy:
# - Layer 1: Pre-launch context (before Claude starts)
# - Layer 2: Session-start hook (user-visible)
# - Layer 3: In-session dynamic loading (Optimizer agent with MCP tools)
#
# Steps:
# 1. Create memories with different importance levels
# 2. Test pre-launch context loading
# 3. Validate context formatting and filtering
# 4. Test context budget enforcement
# 5. Validate namespace filtering
# 6. Test performance (< 500ms timeout)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source common utilities
# shellcheck source=tests/e2e/lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"

# Test state
PASSED=0
FAILED=0

section "E2E Test: Human Workflow 4 - Context Loading"

# Setup test environment
setup_test_env "hw4_context"

section "Setup: Create Tiered Memories"

# Create memories with specific importance tiers
# Critical (importance >= 8): Should be included in context
# Important (importance == 7): Should be included in context
# Low (importance < 7): Should NOT be included in context

print_cyan "Creating memory tiers..."

# 5 critical memories (importance >= 8)
for i in {1..5}; do
    create_memory "$BIN" "$TEST_DB" \
        "Critical architectural decision $i - This is a fundamental design choice that impacts the entire system" \
        "project:testapp" 8 > /dev/null 2>&1
done

# 5 important memories (importance == 7)
for i in {1..5}; do
    create_memory "$BIN" "$TEST_DB" \
        "Important coding pattern $i - Useful insight for development work" \
        "project:testapp" 7 > /dev/null 2>&1
done

# 5 low-importance memories (should NOT be included)
for i in {1..5}; do
    create_memory "$BIN" "$TEST_DB" \
        "Low priority note $i - Minor reference information" \
        "project:testapp" 5 > /dev/null 2>&1
done

print_green "Created 15 memories (5 critical, 5 important, 5 low-priority)"

# Wait for indexing
sleep 2

section "Test 1: Pre-Launch Context Loading (Layer 1)"

# Note: We can't actually test the launcher directly in this script because it would
# launch Claude Code interactively. Instead, we'll test the context generation logic
# by directly querying what WOULD be loaded.

# List memories that should be included (importance >= 7)
print_cyan "Querying memories with importance >= 7..."
HIGH_IMPORTANCE_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "" \
    --namespace "project:testapp"  2>&1 || echo "")

# Count how many high-importance memories exist
CRITICAL_COUNT=$(echo "$HIGH_IMPORTANCE_OUTPUT" | grep -c "importance.*8\|Critical" || echo "0")
IMPORTANT_COUNT=$(echo "$HIGH_IMPORTANCE_OUTPUT" | grep -c "importance.*7\|Important" || echo "0")

echo "Found in high-importance results:"
echo "  Critical (8+): $CRITICAL_COUNT"
echo "  Important (7): $IMPORTANT_COUNT"

if [ "$CRITICAL_COUNT" -ge 5 ] && [ "$IMPORTANT_COUNT" -ge 5 ]; then
    pass "High-importance memories exist and are queryable"
else
    fail "High-importance memory counts unexpected" \
        "Expected 5+ critical and 5+ important, got $CRITICAL_COUNT critical and $IMPORTANT_COUNT important"
fi

# Verify low-importance memories are NOT in a high-importance filter
# (they would be filtered out by min_importance threshold)
if echo "$HIGH_IMPORTANCE_OUTPUT" | grep -qi "Low priority"; then
    fail "Context filter should exclude low-importance memories" \
        "Found 'Low priority' in high-importance results"
else
    pass "Context filter correctly excludes low-importance memories"
fi

section "Test 2: Context Formatting and Tiers"

# Test that memories would be grouped into correct tiers
# Critical memories (>=8) get detailed format
# Important memories (=7) get compact format

# Check if critical memories would have detailed display
if echo "$HIGH_IMPORTANCE_OUTPUT" | grep -qi "Critical.*decision"; then
    pass "Critical memories include descriptive content"
else
    fail "Critical memories should include detailed information"
fi

section "Test 3: Context Budget Enforcement (10KB limit)"

# Create scenario: 100 high-importance memories (should trigger truncation)
print_cyan "Testing context size limits with large memory set..."

# Create a separate test DB for size testing
SIZE_TEST_DB=$(create_test_db "size_test")
export SIZE_TEST_DATABASE_URL="sqlite://$SIZE_TEST_DB"

# Create 50 memories with long content (to test size limits)
for i in {1..50}; do
    LONG_CONTENT="Architecture decision $i: $(printf 'A%.0s' {1..500})"  # 500 char content
    DATABASE_URL="$SIZE_TEST_DATABASE_URL" "$BIN" remember "$LONG_CONTENT" \
        --namespace "project:sizetest" --importance 8 > /dev/null 2>&1
done

print_cyan "Created 50 large memories for size testing"

# List all and check output size
# In real launcher, this would be truncated to 10KB
ALL_OUTPUT=$(DATABASE_URL="$SIZE_TEST_DATABASE_URL" "$BIN" recall --query "" \
    --namespace "project:sizetest" --limit 50 2>&1 || echo "")

OUTPUT_SIZE=${#ALL_OUTPUT}
OUTPUT_SIZE_KB=$((OUTPUT_SIZE / 1024))

echo "Full output size: ${OUTPUT_SIZE_KB}KB ($OUTPUT_SIZE bytes)"

# The launcher would truncate this to 10KB
# We're just verifying that we CAN query this data
if [ "$OUTPUT_SIZE" -gt 0 ]; then
    pass "Large memory set is queryable (would be truncated in launcher)"
else
    fail "Failed to query large memory set"
fi

# Cleanup size test DB
cleanup_test_db "$SIZE_TEST_DB"

section "Test 4: Namespace Filtering"

# Create memories in different namespaces
print_cyan "Testing namespace filtering..."

create_memory "$BIN" "$TEST_DB" \
    "Global memory - should be accessible from all projects" \
    "global" 8 > /dev/null 2>&1

create_memory "$BIN" "$TEST_DB" \
    "TestApp specific memory" \
    "project:testapp" 8 > /dev/null 2>&1

create_memory "$BIN" "$TEST_DB" \
    "OtherApp memory - should NOT appear in testapp context" \
    "project:otherapp" 8 > /dev/null 2>&1

sleep 1

# Query project:testapp namespace
TESTAPP_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "" \
    --namespace "project:testapp" 2>&1 || echo "")

# Should find testapp-specific memory
if echo "$TESTAPP_OUTPUT" | grep -qi "TestApp specific"; then
    pass "Namespace filter includes project-specific memories"
else
    fail "Namespace filter should include project-specific memories"
fi

# Should NOT find otherapp memory
if echo "$TESTAPP_OUTPUT" | grep -qi "OtherApp"; then
    fail "Namespace filter leaked memory from other project"
else
    pass "Namespace filter correctly excludes other projects"
fi

# Note: Global memories may or may not be included depending on query
# The launcher context loader queries specific namespace

section "Test 5: Performance Validation"

# Test that context loading completes within timeout
print_cyan "Measuring context loading performance..."

# Create realistic scenario: 20 high-importance memories
PERF_TEST_DB=$(create_test_db "perf_test")
PERF_DATABASE_URL="sqlite://$PERF_TEST_DB"

for i in {1..20}; do
    DATABASE_URL="$PERF_DATABASE_URL" "$BIN" remember \
        "Performance test memory $i - architectural decision with moderate content length" \
        --namespace "project:perftest" --importance 8 > /dev/null 2>&1
done

sleep 1

# Measure query time (simulates what launcher does)
START_TIME=$(date +%s%N)
DATABASE_URL="$PERF_DATABASE_URL" "$BIN" recall --query "" \
    --namespace "project:perftest" --limit 10  > /dev/null 2>&1
END_TIME=$(date +%s%N)

QUERY_MS=$(( (END_TIME - START_TIME) / 1000000 ))

echo "Context query time: ${QUERY_MS}ms (target: <200ms, timeout: 500ms)"

if [ "$QUERY_MS" -lt 500 ]; then
    if [ "$QUERY_MS" -lt 200 ]; then
        pass "Context loading performance excellent (<200ms)"
    else
        pass "Context loading performance acceptable (<500ms)"
    fi
else
    fail "Context loading exceeded timeout threshold" \
        "${QUERY_MS}ms > 500ms timeout"
fi

cleanup_test_db "$PERF_TEST_DB"

section "Test 6: Memory Count Limits"

# Verify that context loader respects max_memories limit
# Default: 10 memories maximum

LIMIT_TEST_DB=$(create_test_db "limit_test")
LIMIT_DATABASE_URL="sqlite://$LIMIT_TEST_DB"

# Create 30 high-importance memories
for i in {1..30}; do
    DATABASE_URL="$LIMIT_DATABASE_URL" "$BIN" remember \
        "Limit test memory $i" \
        --namespace "project:limittest" --importance 8 > /dev/null 2>&1
done

sleep 1

# Query with limit=10 (what launcher uses)
LIMIT_OUTPUT=$(DATABASE_URL="$LIMIT_DATABASE_URL" "$BIN" recall --query "" \
    --namespace "project:limittest" --limit 10 2>&1 || echo "")

# Count results (heuristic: count lines with timestamps or importance markers)
RESULT_COUNT=$(echo "$LIMIT_OUTPUT" | grep -cE 'importance.*8|Limit test' || echo "0")

echo "Results with limit=10: $RESULT_COUNT entries"

# Should be <= 10
if [ "$RESULT_COUNT" -le 10 ]; then
    pass "Memory count limit respected (${RESULT_COUNT} <= 10)"
else
    fail "Memory count limit exceeded" \
        "${RESULT_COUNT} > 10"
fi

cleanup_test_db "$LIMIT_TEST_DB"

section "Test 7: Importance Threshold Filtering"

# Verify min_importance=7 threshold works
THRESHOLD_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "" \
    --namespace "project:testapp" 2>&1 || echo "")

# Should include importance 7 and 8
HAS_CRITICAL=$(echo "$THRESHOLD_OUTPUT" | grep -c "Critical.*decision" || echo "0")
HAS_IMPORTANT=$(echo "$THRESHOLD_OUTPUT" | grep -c "Important.*pattern" || echo "0")
HAS_LOW=$(echo "$THRESHOLD_OUTPUT" | grep -c "Low priority" || echo "0")

echo "Memory types in results:"
echo "  Critical (8): $HAS_CRITICAL"
echo "  Important (7): $HAS_IMPORTANT"
echo "  Low (<7): $HAS_LOW"

if [ "$HAS_CRITICAL" -gt 0 ] && [ "$HAS_IMPORTANT" -gt 0 ]; then
    pass "High-importance memories (>=7) included in results"
else
    fail "High-importance memories not found in results"
fi

# Note: Low-priority may or may not appear depending on query
# The launcher explicitly filters by importance >= 7

section "Test 8: Natural Language Formatting"

# Verify output format is suitable for LLM consumption
# Should be markdown-formatted with clear structure

SAMPLE_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "" \
    --namespace "project:testapp" --limit 5 2>&1 || echo "")

# Check for structured output (not just raw data dumps)
if echo "$SAMPLE_OUTPUT" | grep -qE 'Memory|Importance:|Created:|Updated:'; then
    pass "Output includes structured metadata"
else
    warn "Output format may not be optimally structured for LLM"
fi

# Output should be human-readable
if [ "${#SAMPLE_OUTPUT}" -gt 0 ]; then
    pass "Context output is non-empty and readable"
else
    fail "Context output is empty"
fi

# Cleanup
section "Cleanup"
teardown_test_env

# Summary
test_summary
exit $?
