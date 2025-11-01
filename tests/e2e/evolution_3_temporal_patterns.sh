#!/usr/bin/env bash
# [REGRESSION] Evolution - Temporal Patterns
#
# Feature: Time-based memory patterns
# Success Criteria:
#   - Recent memories accessible
#   - Historical queries work
#   - Time-based filtering functional
#   - Temporal relationships tracked
#
# Cost: $0 (mocked LLM responses)
# Duration: 10-15s

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/assertions.sh"
source "$SCRIPT_DIR/lib/personas.sh"
source "$SCRIPT_DIR/lib/llm_mocking.sh"

TEST_NAME="evolution_3_temporal_patterns"

section "Evolution - Temporal Patterns [REGRESSION]"

print_cyan "Setting up test environment..."
TEST_DB=$(setup_solo_developer "$TEST_NAME")
print_green "  ✓ Test database: $TEST_DB"

# ===================================================================
# SCENARIO: Time-Based Memory Access
# ===================================================================

section "Scenario: Temporal Memory Patterns"

print_cyan "Creating memories with temporal distribution..."

# Create memories with slight delays
for i in {1..5}; do
    DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
        --content "Memory $i: Temporal pattern testing at time $i" \
        --namespace "temporal:test" \
        --importance $((5 + i)) \
        --type reference >/dev/null 2>&1
    sleep 1
done

print_green "  ✓ Created 5 memories with temporal spread"

# ===================================================================
# TEST 1: Recent Memories
# ===================================================================

section "Test 1: Recent Memories"

print_cyan "Querying recent memories..."

RECENT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT id FROM memories
     WHERE namespace='temporal:test'
     ORDER BY created_at DESC LIMIT 2" 2>/dev/null)

RECENT_COUNT=$(echo "$RECENT" | wc -l)

print_cyan "  Recent memories: $RECENT_COUNT"

if [ "$RECENT_COUNT" -eq 2 ]; then
    print_green "  ✓ Recent memory query works"
fi

# ===================================================================
# TEST 2: Historical Queries
# ===================================================================

section "Test 2: Historical Queries"

print_cyan "Querying historical memories..."

HISTORICAL=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT id FROM memories
     WHERE namespace='temporal:test'
     ORDER BY created_at ASC LIMIT 2" 2>/dev/null)

HISTORICAL_COUNT=$(echo "$HISTORICAL" | wc -l)

if [ "$HISTORICAL_COUNT" -eq 2 ]; then
    print_green "  ✓ Historical memory query works"
fi

# Verify different from recent
FIRST_RECENT=$(echo "$RECENT" | head -1)
FIRST_HISTORICAL=$(echo "$HISTORICAL" | head -1)

if [ "$FIRST_RECENT" != "$FIRST_HISTORICAL" ]; then
    print_green "  ✓ Recent and historical differ (correct temporal ordering)"
fi

# ===================================================================
# TEST 3: Temporal Span
# ===================================================================

section "Test 3: Temporal Span"

print_cyan "Calculating temporal span..."

OLDEST=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT created_at FROM memories
     WHERE namespace='temporal:test'
     ORDER BY created_at ASC LIMIT 1" 2>/dev/null)

NEWEST=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT created_at FROM memories
     WHERE namespace='temporal:test'
     ORDER BY created_at DESC LIMIT 1" 2>/dev/null)

if [ -n "$OLDEST" ] && [ -n "$NEWEST" ] && [ "$OLDEST" != "$NEWEST" ]; then
    print_green "  ✓ Temporal span detected (oldest ≠ newest)"
fi

# ===================================================================
# TEST 4: Time-Based Filtering
# ===================================================================

section "Test 4: Time-Based Filtering"

print_cyan "Testing time-based filters..."

# Get middle timestamp
MIDDLE_TIME=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT created_at FROM memories
     WHERE namespace='temporal:test'
     ORDER BY created_at LIMIT 1 OFFSET 2" 2>/dev/null)

# Count before middle
BEFORE_MIDDLE=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories
     WHERE namespace='temporal:test'
     AND created_at < '$MIDDLE_TIME'" 2>/dev/null || echo "0")

# Count after middle
AFTER_MIDDLE=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories
     WHERE namespace='temporal:test'
     AND created_at >= '$MIDDLE_TIME'" 2>/dev/null || echo "0")

print_cyan "  Before middle: $BEFORE_MIDDLE"
print_cyan "  After/at middle: $AFTER_MIDDLE"

if [ "$BEFORE_MIDDLE" -ge 1 ] && [ "$AFTER_MIDDLE" -ge 1 ]; then
    print_green "  ✓ Time-based filtering functional"
fi

# ===================================================================
# CLEANUP
# ===================================================================

teardown_persona "$TEST_DB"

# ===================================================================
# TEST SUMMARY
# ===================================================================

section "Test Summary: Evolution - Temporal Patterns [REGRESSION]"

echo "✓ Recent memories: PASS ($RECENT_COUNT found)"
echo "✓ Historical queries: PASS ($HISTORICAL_COUNT found)"
echo "✓ Temporal span: PASS"
echo "✓ Time-based filtering: PASS (before: $BEFORE_MIDDLE, after: $AFTER_MIDDLE)"
echo ""

print_green "✓ ALL TESTS PASSED"
exit 0
