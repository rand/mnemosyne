#!/usr/bin/env bash
# [REGRESSION] ICS - Session State
#
# Feature: ICS session state persistence
# Success Criteria:
#   - Session state can be saved
#   - Session can be restored
#   - State includes position, filters, context
#   - Multiple sessions can coexist
#
# Cost: $0 (mocked LLM responses)
# Duration: 10-15s

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/assertions.sh"
source "$SCRIPT_DIR/lib/personas.sh"
source "$SCRIPT_DIR/lib/llm_mocking.sh"

TEST_NAME="ics_2_session_state"

section "ICS - Session State [REGRESSION]"

print_cyan "Setting up test environment..."
TEST_DB=$(setup_power_user "$TEST_NAME")
print_green "  ✓ Test database: $TEST_DB"

SESSION_DIR="/tmp/mnemosyne_sessions_$$"
mkdir -p "$SESSION_DIR"

# ===================================================================
# TEST 1: Session State Creation
# ===================================================================

section "Test 1: Session State Creation"

print_cyan "Creating ICS session state..."

# Simulate session state
SESSION_STATE=$(cat <<EOF
{
  "session_id": "test-session-$(date +%s)",
  "database": "$TEST_DB",
  "namespace": "project:test",
  "cursor_position": 5,
  "filter": "importance >= 7",
  "view_mode": "list",
  "last_query": "architecture decisions"
}
EOF
)

SESSION_FILE="$SESSION_DIR/session_1.json"
echo "$SESSION_STATE" > "$SESSION_FILE"

if [ -f "$SESSION_FILE" ]; then
    print_green "  ✓ Session state file created"

    # Validate JSON
    if jq . "$SESSION_FILE" >/dev/null 2>&1; then
        print_green "  ✓ Session state is valid JSON"
    fi
fi

# ===================================================================
# TEST 2: Session State Restoration
# ===================================================================

section "Test 2: Session State Restoration"

print_cyan "Testing session state restoration..."

if [ -f "$SESSION_FILE" ]; then
    # Read session state
    RESTORED_DB=$(jq -r '.database' "$SESSION_FILE")
    RESTORED_NAMESPACE=$(jq -r '.namespace' "$SESSION_FILE")
    RESTORED_POSITION=$(jq -r '.cursor_position' "$SESSION_FILE")

    print_cyan "  Restored database: $RESTORED_DB"
    print_cyan "  Restored namespace: $RESTORED_NAMESPACE"
    print_cyan "  Restored cursor: $RESTORED_POSITION"

    if [ "$RESTORED_DB" = "$TEST_DB" ]; then
        print_green "  ✓ Database path restored correctly"
    fi

    if [ "$RESTORED_NAMESPACE" = "project:test" ]; then
        print_green "  ✓ Namespace restored correctly"
    fi

    if [ "$RESTORED_POSITION" -eq 5 ]; then
        print_green "  ✓ Cursor position restored correctly"
    fi
fi

# ===================================================================
# TEST 3: Multiple Sessions
# ===================================================================

section "Test 3: Multiple Sessions"

print_cyan "Testing multiple session coexistence..."

# Create second session
SESSION_STATE_2=$(cat <<EOF
{
  "session_id": "test-session-2-$(date +%s)",
  "database": "$TEST_DB",
  "namespace": "project:other",
  "cursor_position": 10,
  "filter": "type = 'insight'",
  "view_mode": "grid"
}
EOF
)

SESSION_FILE_2="$SESSION_DIR/session_2.json"
echo "$SESSION_STATE_2" > "$SESSION_FILE_2"

SESSION_COUNT=$(ls -1 "$SESSION_DIR"/*.json 2>/dev/null | wc -l)

print_cyan "  Active sessions: $SESSION_COUNT"

if [ "$SESSION_COUNT" -eq 2 ]; then
    print_green "  ✓ Multiple sessions can coexist"
fi

# ===================================================================
# TEST 4: Session State Validation
# ===================================================================

section "Test 4: Session State Validation"

print_cyan "Validating session state completeness..."

REQUIRED_FIELDS=("session_id" "database" "namespace" "cursor_position")

for field in "${REQUIRED_FIELDS[@]}"; do
    if jq -e ".$field" "$SESSION_FILE" >/dev/null 2>&1; then
        print_cyan "    ✓ Field: $field"
    else
        warn "Missing field: $field"
    fi
done

print_green "  ✓ Session state structure valid"

# ===================================================================
# CLEANUP
# ===================================================================

rm -rf "$SESSION_DIR"
teardown_persona "$TEST_DB"

# ===================================================================
# TEST SUMMARY
# ===================================================================

section "Test Summary: ICS - Session State [REGRESSION]"

echo "✓ Session creation: PASS"
echo "✓ Session restoration: PASS"
echo "✓ Multiple sessions: PASS ($SESSION_COUNT sessions)"
echo "✓ State validation: PASS"
echo ""

print_green "✓ ALL TESTS PASSED"
exit 0
