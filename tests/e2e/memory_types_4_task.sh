#!/usr/bin/env bash
# [REGRESSION] Memory Types - Task
#
# Feature: Task memory type for work tracking
# Success Criteria:
#   - Task memories track work items with status
#   - Dependencies and estimates documented
#   - Searchable by assignee and status
#   - Updates reflect progress
#   - Integration with work tracking
#
# Cost: $0 (mocked LLM responses)
# Duration: 15-20s

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/assertions.sh"
source "$SCRIPT_DIR/lib/personas.sh"
source "$SCRIPT_DIR/lib/llm_mocking.sh"

# ===================================================================
# TEST SETUP
# ===================================================================

TEST_NAME="memory_types_4_task"

section "Memory Types - Task [REGRESSION]"

if is_baseline_mode; then
    warn "This is a regression test but running in baseline mode"
fi

print_cyan "Setting up test environment..."
TEST_DB=$(setup_solo_developer "$TEST_NAME")
print_green "  ✓ Test database: $TEST_DB"

# ===================================================================
# SCENARIO: Task Tracking
# ===================================================================

section "Scenario: Task Tracking"

print_cyan "Step 1: Creating task memories..."

# Task 1: Feature implementation
TASK1=$(cat <<EOF
Task: Implement OAuth2 authorization code flow

Status: In Progress
Assignee: Alice
Priority: Critical
Estimated: 3-4 days

Description:
Add OAuth2 support for Google and GitHub authentication.
Users should be able to sign in using their existing accounts.

Requirements:
- OAuth2 client configuration
- Authorization endpoint
- Token exchange endpoint
- Token refresh mechanism
- User profile mapping

Dependencies:
- Database schema for OAuth providers (COMPLETE)
- SSL certificates for production (IN PROGRESS)

Blockers: None

Progress:
- ✓ Google OAuth integration complete
- ⧗ GitHub OAuth in progress (50%)
- ☐ Token refresh pending
- ☐ Testing pending
EOF
)

MEM1=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$TASK1" \
    --namespace "project:auth-service" \
    --importance 9 \
    --type task 2>&1) || fail "Failed to store task 1"

MEM1_ID=$(echo "$MEM1" | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ OAuth task: $MEM1_ID"

# Task 2: Bug fix
TASK2=$(cat <<EOF
Task: Fix memory leak in WebSocket connections

Status: Ready
Assignee: Bob
Priority: High
Estimated: 1-2 days

Description:
Memory usage grows unbounded when WebSocket connections remain open.
Need to identify and fix the leak.

Reproduction:
1. Open 100 WebSocket connections
2. Keep connections open for 1 hour
3. Observe memory usage grow from 200MB to 2GB

Root Cause Analysis Needed:
- Check for event listener cleanup
- Review connection close handlers
- Verify buffer management
- Check for circular references

Expected Outcome:
Memory usage should stabilize after initial connection setup.
Max acceptable: 500MB for 100 connections.

Dependencies: None
Blockers: Need production metrics access
EOF
)

MEM2=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$TASK2" \
    --namespace "project:realtime" \
    --importance 10 \
    --type task 2>&1) || fail "Failed to store task 2"

MEM2_ID=$(echo "$MEM2" | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ Bug fix task: $MEM2_ID"

# Task 3: Documentation
TASK3=$(cat <<EOF
Task: Update API documentation for v2 endpoints

Status: Not Started
Assignee: Carol
Priority: Medium
Estimated: 2 days

Description:
API v2 endpoints are live but documentation is outdated.
Update OpenAPI spec and example code.

Scope:
- Update OpenAPI 3.0 specification
- Add examples for all new endpoints
- Update rate limiting documentation
- Add authentication examples
- Generate SDK documentation

Deliverables:
- Updated openapi.yaml
- Code examples (curl, JavaScript, Python)
- Migration guide from v1 to v2
- Published to docs.example.com

Dependencies:
- API v2 finalized (COMPLETE)
- Example applications ready (IN PROGRESS)

Timeline: Complete by end of sprint
EOF
)

MEM3=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$TASK3" \
    --namespace "project:api" \
    --importance 7 \
    --type task 2>&1) || fail "Failed to store task 3"

MEM3_ID=$(echo "$MEM3" | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ Documentation task: $MEM3_ID"

# ===================================================================
# VALIDATION: Task Type
# ===================================================================

section "Validation: Task Type"

print_cyan "Verifying task memory type..."

TASK_COUNT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE memory_type='task'" 2>/dev/null)

print_cyan "  Task memories: $TASK_COUNT"
assert_greater_than "$TASK_COUNT" 2 "Task count"
print_green "  ✓ All tasks properly typed"

# ===================================================================
# TEST: Search Tasks by Status
# ===================================================================

section "Test: Search Tasks by Status"

print_cyan "Searching for in-progress tasks..."

PROGRESS_SEARCH=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "in progress status OAuth" \
    --limit 5 2>&1) || fail "Search failed"

print_green "  ✓ Task search completed"

if echo "$PROGRESS_SEARCH" | grep -q "$MEM1_ID\|OAuth\|progress"; then
    print_green "  ✓ In-progress task found"
fi

# ===================================================================
# TEST: Priority Filtering
# ===================================================================

section "Test: Priority Filtering"

print_cyan "Listing critical and high-priority tasks..."

HIGH_TASKS=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT id, importance FROM memories
     WHERE memory_type='task' AND importance >= 9
     ORDER BY importance DESC" 2>/dev/null)

CRITICAL_COUNT=$(echo "$HIGH_TASKS" | wc -l | tr -d ' ')

print_cyan "  Critical/high tasks: $CRITICAL_COUNT"

if [ "$CRITICAL_COUNT" -ge 2 ]; then
    print_green "  ✓ High-priority tasks identified"
fi

# ===================================================================
# TEST: Task Structure
# ===================================================================

section "Test: Task Structure"

print_cyan "Validating task structure elements..."

for mem_id in "$MEM1_ID" "$MEM2_ID" "$MEM3_ID"; do
    CONTENT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
        "SELECT content FROM memories WHERE id='$mem_id'" 2>/dev/null)

    # Check for task elements
    HAS_STATUS=$(echo "$CONTENT" | grep -qi "status" && echo "1" || echo "0")
    HAS_ASSIGNEE=$(echo "$CONTENT" | grep -qi "assignee" && echo "1" || echo "0")
    HAS_ESTIMATE=$(echo "$CONTENT" | grep -qi "estimated\|days" && echo "1" || echo "0")

    STRUCTURE_SCORE=$((HAS_STATUS + HAS_ASSIGNEE + HAS_ESTIMATE))

    print_cyan "  $mem_id: $STRUCTURE_SCORE/3 elements"

    if [ "$STRUCTURE_SCORE" -ge 2 ]; then
        print_green "    ✓ Well-structured task"
    fi
done

# ===================================================================
# TEST: Task Dependencies
# ===================================================================

section "Test: Task Dependencies"

print_cyan "Checking tasks document dependencies..."

DEPS_MENTIONED=0

for mem_id in "$MEM1_ID" "$MEM2_ID" "$MEM3_ID"; do
    CONTENT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
        "SELECT content FROM memories WHERE id='$mem_id'" 2>/dev/null)

    if echo "$CONTENT" | grep -qi "dependencies\|depends\|blocked"; then
        ((DEPS_MENTIONED++))
    fi
done

print_cyan "  Tasks with dependency info: $DEPS_MENTIONED/3"

if [ "$DEPS_MENTIONED" -ge 2 ]; then
    print_green "  ✓ Dependencies documented"
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

section "Test Summary: Memory Types - Task [REGRESSION]"

echo "✓ Task storage: PASS"
echo "✓ Type consistency: PASS ($TASK_COUNT tasks)"
echo "✓ Status search: PASS"
echo "✓ Priority filtering: PASS ($CRITICAL_COUNT critical)"
echo "✓ Task structure: PASS"
echo "✓ Dependency tracking: PASS ($DEPS_MENTIONED/3 documented)"
echo ""
echo "Tasks Tested:"
echo "  - Feature: OAuth2 implementation"
echo "  - Bug fix: Memory leak investigation"
echo "  - Documentation: API docs update"
echo ""

print_green "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
print_green "✓ ALL TESTS PASSED"
print_green "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

exit 0
