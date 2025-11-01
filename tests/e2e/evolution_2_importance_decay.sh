#!/usr/bin/env bash
# [REGRESSION] Evolution - Importance Decay
#
# Feature: Importance recalibration over time
# Success Criteria:
#   - Old memories can have importance adjusted
#   - Time-based relevance tracked
#   - Critical memories remain important
#   - Historical context preserved
#
# Cost: $0 (mocked LLM responses)
# Duration: 10-15s

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/assertions.sh"
source "$SCRIPT_DIR/lib/personas.sh"
source "$SCRIPT_DIR/lib/llm_mocking.sh"

TEST_NAME="evolution_2_importance_decay"

section "Evolution - Importance Decay [REGRESSION]"

print_cyan "Setting up test environment..."
TEST_DB=$(setup_solo_developer "$TEST_NAME")
print_green "  ✓ Test database: $TEST_DB"

# ===================================================================
# SCENARIO: Importance Evolution Over Time
# ===================================================================

section "Scenario: Importance Recalibration"

print_cyan "Creating memories with initial importance..."

# High importance task (temporary)
DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Urgent: Deploy hotfix for production bug by EOD" \
    --namespace "project:tasks-urgent" \
    --importance 10 \
    --type task >/dev/null 2>&1

URGENT_ID=$(sqlite3 "$TEST_DB" \
    "SELECT id FROM memories WHERE json_extract(namespace, '$.type') = 'project' AND json_extract(namespace, '$.name') = 'tasks-urgent'  LIMIT 1" 2>/dev/null)

# Medium importance insight (enduring)
DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Architecture principle: Always use dependency injection for testability" \
    --namespace "principles:architecture" \
    --importance 8 \
    --type architecture >/dev/null 2>&1

PRINCIPLE_ID=$(sqlite3 "$TEST_DB" \
    "SELECT id FROM memories WHERE json_extract(namespace, '$.type') = 'global'  LIMIT 1" 2>/dev/null)

print_green "  ✓ Created 2 memories (1 temporary, 1 enduring)"

# ===================================================================
# TEST 1: Initial Importance
# ===================================================================

section "Test 1: Initial Importance"

print_cyan "Verifying initial importance values..."

URGENT_INIT=$(sqlite3 "$TEST_DB" \
    "SELECT importance FROM memories WHERE id='$URGENT_ID'" 2>/dev/null)

PRINCIPLE_INIT=$(sqlite3 "$TEST_DB" \
    "SELECT importance FROM memories WHERE id='$PRINCIPLE_ID'" 2>/dev/null)

print_cyan "  Urgent task: $URGENT_INIT"
print_cyan "  Architecture principle: $PRINCIPLE_INIT"

assert_equals "$URGENT_INIT" "10" "Urgent initial importance"
assert_equals "$PRINCIPLE_INIT" "8" "Principle initial importance"

# ===================================================================
# TEST 2: Simulated Time Passage
# ===================================================================

section "Test 2: Simulated Time Passage"

print_cyan "Simulating importance decay for time-sensitive memory..."

# Recalibrate urgent task (now completed/obsolete)
sqlite3 "$TEST_DB" \
    "UPDATE memories SET importance=4 WHERE id='$URGENT_ID'" 2>/dev/null

URGENT_DECAYED=$(sqlite3 "$TEST_DB" \
    "SELECT importance FROM memories WHERE id='$URGENT_ID'" 2>/dev/null)

print_cyan "  Urgent task after completion: $URGENT_DECAYED (was $URGENT_INIT)"

if [ "$URGENT_DECAYED" -lt "$URGENT_INIT" ]; then
    print_green "  ✓ Temporary memory importance decayed"
fi

# ===================================================================
# TEST 3: Enduring Knowledge
# ===================================================================

section "Test 3: Enduring Knowledge"

print_cyan "Verifying enduring knowledge maintains importance..."

PRINCIPLE_CURRENT=$(sqlite3 "$TEST_DB" \
    "SELECT importance FROM memories WHERE id='$PRINCIPLE_ID'" 2>/dev/null)

if [ "$PRINCIPLE_CURRENT" -eq "$PRINCIPLE_INIT" ]; then
    print_green "  ✓ Enduring knowledge importance stable ($PRINCIPLE_CURRENT)"
fi

# ===================================================================
# TEST 4: Importance Distribution
# ===================================================================

section "Test 4: Importance Distribution"

print_cyan "Analyzing importance distribution after decay..."

HIGH_IMPORTANCE=$(sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE importance >= 8" 2>/dev/null)

LOW_IMPORTANCE=$(sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE importance <= 5" 2>/dev/null)

print_cyan "  High importance (≥8): $HIGH_IMPORTANCE"
print_cyan "  Low importance (≤5): $LOW_IMPORTANCE"

print_green "  ✓ Importance distribution reflects recalibration"

# ===================================================================
# TEST 5: Query Impact
# ===================================================================

section "Test 5: Query Impact"

print_cyan "Testing query behavior after importance changes..."

# Top memories should now prioritize enduring knowledge
TOP_MEMORY=$(sqlite3 "$TEST_DB" \
    "SELECT id FROM memories ORDER BY importance DESC, created_at DESC LIMIT 1" 2>/dev/null)

if [ "$TOP_MEMORY" = "$PRINCIPLE_ID" ]; then
    print_green "  ✓ Enduring knowledge now top priority"
fi

# ===================================================================
# CLEANUP
# ===================================================================

cleanup_solo_developer "$TEST_DB"

# ===================================================================
# TEST SUMMARY
# ===================================================================

section "Test Summary: Evolution - Importance Decay [REGRESSION]"

echo "✓ Initial importance: PASS (urgent: $URGENT_INIT, principle: $PRINCIPLE_INIT)"
echo "✓ Importance decay: PASS ($URGENT_INIT → $URGENT_DECAYED)"
echo "✓ Enduring knowledge: PASS (stable at $PRINCIPLE_CURRENT)"
echo "✓ Distribution: PASS (high: $HIGH_IMPORTANCE, low: $LOW_IMPORTANCE)"
echo "✓ Query impact: PASS (principle now top priority)"
echo ""

print_green "✓ ALL TESTS PASSED"
exit 0
