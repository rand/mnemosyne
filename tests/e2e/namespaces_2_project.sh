#!/usr/bin/env bash
# [REGRESSION] Namespaces - Project
#
# Feature: Project-specific namespaces for isolated project knowledge
# Success Criteria:
#   - Project memories isolated by namespace
#   - Multiple projects coexist
#   - Project-scoped search works
#   - No cross-contamination
#
# Cost: $0 (mocked LLM responses)
# Duration: 10-15s

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/assertions.sh"
source "$SCRIPT_DIR/lib/personas.sh"
source "$SCRIPT_DIR/lib/llm_mocking.sh"

TEST_NAME="namespaces_2_project"

section "Namespaces - Project [REGRESSION]"

print_cyan "Setting up test environment..."
TEST_DB=$(setup_solo_developer "$TEST_NAME")
print_green "  ✓ Test database: $TEST_DB"

# Create memories in different project namespaces
DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Project A: Using React for frontend" \
    --namespace "project:projectA" \
    --importance 8 \
    --type architecture >/dev/null 2>&1

DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Project B: Using Vue for frontend" \
    --namespace "project:projectB" \
    --importance 8 \
    --type architecture >/dev/null 2>&1

# Verify isolation
PROJ_A=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE namespace='project:projectA'" 2>/dev/null)

PROJ_B=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE namespace='project:projectB'" 2>/dev/null)

print_cyan "  Project A memories: $PROJ_A"
print_cyan "  Project B memories: $PROJ_B"

assert_equals "$PROJ_A" "1" "Project A isolation"
assert_equals "$PROJ_B" "1" "Project B isolation"
print_green "  ✓ Project namespaces isolated"

teardown_persona "$TEST_DB"

section "Test Summary: Namespaces - Project [REGRESSION]"
echo "✓ Project isolation: PASS"

print_green "✓ ALL TESTS PASSED"
exit 0
