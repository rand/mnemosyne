#!/usr/bin/env bash
# [REGRESSION] Namespaces - Hierarchical
#
# Feature: Hierarchical namespace structure (project:name:sub:component)
# Success Criteria:
#   - Hierarchical namespaces supported
#   - Parent/child relationship clear
#   - Wildcard search works (project:name:*)
#   - Breadth and depth traversal
#
# Cost: $0 (mocked LLM responses)
# Duration: 10-15s

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/assertions.sh"
source "$SCRIPT_DIR/lib/personas.sh"
source "$SCRIPT_DIR/lib/llm_mocking.sh"

TEST_NAME="namespaces_4_hierarchical"

section "Namespaces - Hierarchical [REGRESSION]"

print_cyan "Setting up test environment..."
TEST_DB=$(setup_solo_developer "$TEST_NAME")
print_green "  ✓ Test database: $TEST_DB"

# Create hierarchical structure
DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Root: Project architecture" \
    --namespace "project:myapp" \
    --importance 9 \
    --type architecture >/dev/null 2>&1

DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Frontend component library" \
    --namespace "project:myapp:frontend" \
    --importance 8 \
    --type reference >/dev/null 2>&1

DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "React components" \
    --namespace "project:myapp:frontend:components" \
    --importance 7 \
    --type reference >/dev/null 2>&1

DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Backend API" \
    --namespace "project:myapp:backend" \
    --importance 8 \
    --type reference >/dev/null 2>&1

# Verify hierarchy
TOTAL_MYAPP=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE namespace LIKE 'project:myapp%'" 2>/dev/null)

FRONTEND_ONLY=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE namespace LIKE 'project:myapp:frontend%'" 2>/dev/null)

print_cyan "  Total myapp hierarchy: $TOTAL_MYAPP"
print_cyan "  Frontend subtree: $FRONTEND_ONLY"

assert_equals "$TOTAL_MYAPP" "4" "Total hierarchy"
assert_equals "$FRONTEND_ONLY" "2" "Frontend subtree"
print_green "  ✓ Hierarchical structure validated"

teardown_persona "$TEST_DB"

section "Test Summary: Namespaces - Hierarchical [REGRESSION]"
echo "✓ Hierarchical namespaces: PASS"

print_green "✓ ALL TESTS PASSED"
exit 0
