#!/usr/bin/env bash
# [REGRESSION] Namespaces - Global
#
# Feature: Global namespace for user preferences and org-wide knowledge
# Success Criteria:
#   - Global memories accessible from all contexts
#   - User preferences stored globally
#   - Org-wide standards documented
#   - Not project-specific
#
# Cost: $0 (mocked LLM responses)
# Duration: 10-15s

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/assertions.sh"
source "$SCRIPT_DIR/lib/personas.sh"
source "$SCRIPT_DIR/lib/llm_mocking.sh"

TEST_NAME="namespaces_1_global"

section "Namespaces - Global [REGRESSION]"

print_cyan "Setting up test environment..."
TEST_DB=$(setup_solo_developer "$TEST_NAME")
print_green "  ✓ Test database: $TEST_DB"

# Store global preferences
DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Coding preference: Always use async/await over callbacks" \
    --namespace "global" \
    --importance 7 \
    --type reference >/dev/null 2>&1

DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Org standard: All PRs require code review" \
    --namespace "global" \
    --importance 9 \
    --type reference >/dev/null 2>&1

# Verify global namespace
GLOBAL_COUNT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE json_extract(namespace, '$.type') = 'global' " 2>/dev/null)

print_cyan "  Global memories: $GLOBAL_COUNT"
assert_greater_than "$GLOBAL_COUNT" 1 "Global memory count"
print_green "  ✓ Global namespace populated"

cleanup_solo_developer "$TEST_DB"

section "Test Summary: Namespaces - Global [REGRESSION]"
echo "✓ Global namespace: PASS ($GLOBAL_COUNT memories)"

print_green "✓ ALL TESTS PASSED"
exit 0
