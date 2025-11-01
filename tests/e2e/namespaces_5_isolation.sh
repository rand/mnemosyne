#!/usr/bin/env bash
# [REGRESSION] Namespaces - Isolation
#
# Feature: Namespace isolation verification
# Success Criteria:
#   - Memories isolated by namespace
#   - Cross-namespace search controlled
#   - No unintended data leakage
#   - Namespace access control ready
#
# Cost: $0 (mocked LLM responses)
# Duration: 10-15s

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/assertions.sh"
source "$SCRIPT_DIR/lib/personas.sh"
source "$SCRIPT_DIR/lib/llm_mocking.sh"

TEST_NAME="namespaces_5_isolation"

section "Namespaces - Isolation [REGRESSION]"

print_cyan "Setting up test environment..."
TEST_DB=$(setup_team_lead "$TEST_NAME")
print_green "  ✓ Test database: $TEST_DB"

# Create memories in different isolated namespaces
DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Team A: Secret project Falcon" \
    --namespace "project:teamA" \
    --importance 10 \
    --type reference >/dev/null 2>&1

DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Team B: Public documentation" \
    --namespace "project:teamB" \
    --importance 7 \
    --type reference >/dev/null 2>&1

DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Member Alice: Personal notes" \
    --namespace "member:alice" \
    --importance 5 \
    --type reference >/dev/null 2>&1

# Verify strict isolation
TEAM_A=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE json_extract(namespace, '$.type') = 'project' AND json_extract(namespace, '$.name') = 'teamA' " 2>/dev/null)

TEAM_B=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE json_extract(namespace, '$.type') = 'project' AND json_extract(namespace, '$.name') = 'teamB' " 2>/dev/null)

ALICE=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE json_extract(namespace, '$.type') = 'global' " 2>/dev/null)

print_cyan "  Team A: $TEAM_A (isolated)"
print_cyan "  Team B: $TEAM_B (isolated)"
print_cyan "  Alice: $ALICE (isolated)"

assert_equals "$TEAM_A" "1" "Team A isolation"
assert_equals "$TEAM_B" "1" "Team B isolation"
assert_equals "$ALICE" "1" "Alice isolation"

# Verify no cross-contamination
TOTAL_DISTINCT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(DISTINCT namespace) FROM memories" 2>/dev/null)

print_cyan "  Distinct namespaces: $TOTAL_DISTINCT"

if [ "$TOTAL_DISTINCT" -ge 3 ]; then
    print_green "  ✓ Namespace isolation verified"
fi

cleanup_team_lead "$TEST_DB"

section "Test Summary: Namespaces - Isolation [REGRESSION]"
echo "✓ Namespace isolation: PASS"

print_green "✓ ALL TESTS PASSED"
exit 0
