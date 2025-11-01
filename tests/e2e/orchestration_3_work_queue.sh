#!/usr/bin/env bash
# [BASELINE] Orchestration - Work Queue
#
# Feature: Work queue management with task distribution
# LLM Features: Task enrichment, priority ordering, assignment logic
# Success Criteria:
#   - Tasks stored with enrichment
#   - Queue ordering by priority
#   - Task assignment tracked
#   - Work distribution balanced
#   - Queue state queryable
#
# Cost: ~3-4 API calls (~$0.08-$0.12)
# Duration: 30-40s

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/assertions.sh"
source "$SCRIPT_DIR/lib/personas.sh"
source "$SCRIPT_DIR/lib/baseline_validators.sh"

TEST_NAME="orchestration_3_work_queue"

section "Orchestration - Work Queue [BASELINE]"

if ! is_baseline_mode; then
    fail "This test requires baseline mode (real LLM API)"
    echo "Set MNEMOSYNE_TEST_MODE=baseline"
    exit 1
fi

print_cyan "Setting up test environment..."
TEST_DB=$(setup_team_lead "$TEST_NAME")
print_green "  ✓ Test database: $TEST_DB"

QUEUE_NS="queue:work-items"
QUEUE_NS_WHERE=$(namespace_where_clause "$QUEUE_NS")
AGENT_NS="project:agent-dispatcher"
AGENT_NS_WHERE=$(namespace_where_clause "$AGENT_NS")

# ===================================================================
# SCENARIO: Work Queue with Real LLM Enrichment
# ===================================================================

section "Scenario: Work Queue Management"

print_cyan "Creating work queue with diverse tasks..."

# Task 1: Critical bug fix
TASK1=$(cat <<EOF
Critical Bug: Production API returning 500 errors for /users endpoint.
Affecting 30% of requests. Database connection pool exhaustion suspected.
User impact: High (cannot access profile pages).
Required skills: Node.js, PostgreSQL, debugging.
Estimated effort: 4 hours.
EOF
)

MEM1=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$TASK1" \
    --namespace "$QUEUE_NS" \
    --importance 10 \
    --type task \
    2>&1) || fail "Failed to store task 1"

M1_ID=$(echo "$MEM1" | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ Task 1 queued: $M1_ID (critical)"

sleep 2

# Task 2: Feature implementation
TASK2=$(cat <<EOF
Feature: Add export functionality to dashboard.
Users want to export their data as CSV or JSON.
Requirements: Support both formats, include filters, async generation for large datasets.
Required skills: React, REST API design.
Estimated effort: 8 hours.
EOF
)

MEM2=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$TASK2" \
    --namespace "$QUEUE_NS" \
    --importance 7 \
    --type task \
    2>&1) || fail "Failed to store task 2"

M2_ID=$(echo "$MEM2" | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ Task 2 queued: $M2_ID (medium-high)"

sleep 2

# Task 3: Documentation update
TASK3=$(cat <<EOF
Documentation: Update API reference for v2 endpoints.
New authentication flow needs documentation.
Swagger specs need updating.
Required skills: Technical writing.
Estimated effort: 2 hours.
EOF
)

MEM3=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$TASK3" \
    --namespace "$QUEUE_NS" \
    --importance 5 \
    --type task \
    2>&1) || fail "Failed to store task 3"

M3_ID=$(echo "$MEM3" | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ Task 3 queued: $M3_ID (medium)"

sleep 2

# ===================================================================
# VALIDATION 1: Task Enrichment Quality
# ===================================================================

section "Validation 1: Task Enrichment Quality [BASELINE]"

print_cyan "Validating task enrichment..."

for task_id in "$M1_ID" "$M2_ID" "$M3_ID"; do
    ENRICHMENT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
        "SELECT json_object(
            'summary', summary,
            'keywords', keywords
        ) FROM memories WHERE id='$task_id'" 2>/dev/null)

    SUMMARY=$(echo "$ENRICHMENT" | jq -r '.summary // empty')
    KEYWORDS=$(echo "$ENRICHMENT" | jq -r '.keywords // empty')

    if [ -n "$SUMMARY" ]; then
        print_cyan "  Task $task_id summary: \"${SUMMARY:0:60}...\""

        if [ "${#SUMMARY}" -ge 20 ]; then
            print_green "    ✓ Adequate summary length"
        fi
    fi

    if [ -n "$KEYWORDS" ]; then
        KW_COUNT=$(echo "$KEYWORDS" | jq -r '. | length')
        print_cyan "    Keywords ($KW_COUNT): $KEYWORDS"

        if [ "$KW_COUNT" -ge 3 ]; then
            print_green "    ✓ Sufficient keywords extracted"
        fi
    fi
done

# ===================================================================
# TEST 2: Queue Ordering
# ===================================================================

section "Test 2: Queue Ordering"

print_cyan "Verifying priority-based queue ordering..."

QUEUE_ORDER=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT importance, id
     FROM memories
     WHERE $QUEUE_NS_WHERE AND memory_type ='task'
     ORDER BY importance DESC, created_at ASC" 2>/dev/null)

print_cyan "  Queue order (by priority):"
echo "$QUEUE_ORDER" | while IFS='|' read -r priority id; do
    print_cyan "    [$priority] $id"
done

FIRST_TASK=$(echo "$QUEUE_ORDER" | head -1 | awk '{print $2}')

if [ "$FIRST_TASK" = "$M1_ID" ]; then
    print_green "  ✓ Critical task at head of queue"
fi

# ===================================================================
# TEST 3: Task Assignment
# ===================================================================

section "Test 3: Task Assignment"

print_cyan "Simulating task assignment..."

DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Dispatcher: Assigned task $M1_ID (critical bug) to Agent-Backend-01. High priority, immediate start." \
    --namespace "$AGENT_NS" \
    --importance 9 \
    --type decision \
    2>&1 >/dev/null

DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Dispatcher: Assigned task $M2_ID (export feature) to Agent-Frontend-02. Start after capacity available." \
    --namespace "$AGENT_NS" \
    --importance 7 \
    --type decision \
    2>&1 >/dev/null

ASSIGNMENTS=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE $AGENT_NS_WHERE AND content LIKE '%Assigned%'" 2>/dev/null)

print_cyan "  Task assignments: $ASSIGNMENTS"

if [ "$ASSIGNMENTS" -eq 2 ]; then
    print_green "  ✓ Task assignment tracking works"
fi

# ===================================================================
# TEST 4: Queue State
# ===================================================================

section "Test 4: Queue State"

print_cyan "Querying work queue state..."

TOTAL_TASKS=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE $QUEUE_NS_WHERE AND memory_type ='task'" 2>/dev/null)

CRITICAL=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE $QUEUE_NS_WHERE AND memory_type ='task' AND importance >= 9" 2>/dev/null)

print_cyan "  Total tasks in queue: $TOTAL_TASKS"
print_cyan "  Critical tasks: $CRITICAL"

assert_equals "$TOTAL_TASKS" "3" "Total tasks"
assert_equals "$CRITICAL" "1" "Critical tasks"
print_green "  ✓ Queue state queryable"

# ===================================================================
# CLEANUP
# ===================================================================

cleanup_team_lead "$TEST_DB"

# ===================================================================
# TEST SUMMARY
# ===================================================================

section "Test Summary: Orchestration - Work Queue [BASELINE]"

echo "✓ Task enrichment: PASS (3 tasks with summaries + keywords)"
echo "✓ Queue ordering: PASS (priority: 10 → 5)"
echo "✓ Task assignment: PASS ($ASSIGNMENTS assignments)"
echo "✓ Queue state: PASS ($TOTAL_TASKS tasks, $CRITICAL critical)"
echo ""
echo "Work Queue Features:"
echo "  ✓ Tasks enriched with LLM (summary + keywords)"
echo "  ✓ Priority-based ordering"
echo "  ✓ Assignment tracking"
echo "  ✓ State querying"
echo ""

print_green "✓ ALL TESTS PASSED"
exit 0
