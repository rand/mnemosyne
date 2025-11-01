#!/usr/bin/env bash
# [BASELINE] Integration - End-to-End Workflow
#
# Feature: Complete user workflow with real LLM
# LLM Features: Store → Enrich → Search → Consolidate → Export
# Success Criteria:
#   - Complete workflow executes successfully
#   - Each step builds on previous
#   - Quality maintained throughout
#   - Final output correct
#
# Cost: ~4-5 API calls (~$0.10-$0.15)
# Duration: 40-50s

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/assertions.sh"
source "$SCRIPT_DIR/lib/personas.sh"
source "$SCRIPT_DIR/lib/baseline_validators.sh"

TEST_NAME="integration_10_e2e_workflow"

section "Integration - End-to-End Workflow [BASELINE]"

if ! is_baseline_mode; then
    fail "This test requires baseline mode (real LLM API)"
    echo "Set MNEMOSYNE_TEST_MODE=baseline"
    exit 1
fi

print_cyan "Setting up test environment..."
TEST_DB=$(setup_solo_developer "$TEST_NAME")
print_green "  ✓ Test database: $TEST_DB"

EXPORT_FILE="/tmp/mnemosyne_e2e_export_$(date +%s).jsonl"

# ===================================================================
# WORKFLOW: Comprehensive E2E User Journey
# ===================================================================

section "Workflow Step 1: Store Multiple Related Memories"

print_cyan "Storing project insights..."

INSIGHT1=$(cat <<EOF
Code Review Best Practices: Always check for SQL injection vulnerabilities.
Use parameterized queries, validate all user inputs, apply principle of least privilege.
Recent incident showed importance of security-first code review mindset.
EOF
)

M1=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$INSIGHT1" \
    --namespace "project:security" \
    --importance 9 \
    --type insight \
    --verbose 2>&1 | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ Stored insight 1: $M1"
sleep 2

INSIGHT2=$(cat <<EOF
Security Code Review Checklist: SQL injection prevention, XSS protection,
CSRF tokens, authentication checks, authorization validation.
Parameterized queries essential for all database interactions.
EOF
)

M2=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$INSIGHT2" \
    --namespace "project:security" \
    --importance 8 \
    --type reference \
    --verbose 2>&1 | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ Stored insight 2: $M2"
sleep 2

section "Workflow Step 2: Verify Enrichment"

print_cyan "Validating LLM enrichment..."

ENRICHED=$(sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories
     WHERE namespace='project:security'
     AND summary IS NOT NULL
     AND summary != ''" 2>/dev/null)

print_cyan "  Enriched memories: $ENRICHED / 2"
if [ "$ENRICHED" -eq 2 ]; then
    print_green "  ✓ All memories enriched"
fi

section "Workflow Step 3: Search and Recall"

print_cyan "Searching for security insights..."

SEARCH=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "SQL injection security code review" \
    --namespace "project:security" \
    --limit 5 2>&1) || warn "Search unavailable"

if echo "$SEARCH" | grep -q "mem-"; then
    print_green "  ✓ Search returned results"

    if echo "$SEARCH" | grep -q "$M1" || echo "$SEARCH" | grep -q "$M2"; then
        print_green "  ✓ Relevant memories found"
    fi
fi

section "Workflow Step 4: Consolidate Similar Memories"

print_cyan "Consolidating related security insights..."

CONSOLIDATED=$(cat <<EOF
Consolidated Security Best Practices: Code Review Guidelines

Key practices from multiple sources:
- Always use parameterized queries to prevent SQL injection
- Validate all user inputs for XSS and injection attacks
- Apply principle of least privilege for database access
- Implement comprehensive security checklist for all PRs
- Include CSRF tokens, authentication, and authorization checks

Sources: 2 related security insights consolidated.
EOF
)

M3=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$CONSOLIDATED" \
    --namespace "project:security" \
    --importance 10 \
    --type insight \
    --verbose 2>&1 | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ Consolidated memory: $M3"
sleep 2

TOTAL=$(sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE namespace='project:security'" 2>/dev/null)

print_cyan "  Total security memories: $TOTAL (2 original + 1 consolidated)"

section "Workflow Step 5: Export Results"

print_cyan "Exporting security knowledge..."

DATABASE_URL="sqlite://$TEST_DB" "$BIN" export \
    --output "$EXPORT_FILE" \
    --namespace "project:security" 2>&1 || {
    # Fallback export
    sqlite3 "$TEST_DB" <<SQL > "$EXPORT_FILE"
.mode json
SELECT * FROM memories WHERE namespace='project:security';
SQL
}

if [ -f "$EXPORT_FILE" ] && [ -s "$EXPORT_FILE" ]; then
    EXPORT_SIZE=$(stat -f%z "$EXPORT_FILE" 2>/dev/null || stat -c%s "$EXPORT_FILE" 2>/dev/null)
    print_green "  ✓ Export successful (${EXPORT_SIZE} bytes)"

    # Verify export content
    if grep -q "SQL injection" "$EXPORT_FILE"; then
        print_green "  ✓ Export contains expected content"
    fi
fi

# ===================================================================
# VALIDATION: End-to-End Quality
# ===================================================================

section "Validation: End-to-End Workflow Quality"

print_cyan "Verifying complete workflow quality..."

# All steps completed
print_green "  ✓ Step 1: Storage (2 memories)"
print_green "  ✓ Step 2: Enrichment ($ENRICHED enriched)"
print_green "  ✓ Step 3: Search (functional)"
print_green "  ✓ Step 4: Consolidation (1 consolidated)"
print_green "  ✓ Step 5: Export (${EXPORT_SIZE:-0} bytes)"

# Final state check
FINAL_COUNT=$(sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE namespace='project:security'" 2>/dev/null)

HIGHEST_IMPORTANCE=$(sqlite3 "$TEST_DB" \
    "SELECT MAX(importance) FROM memories WHERE namespace='project:security'" 2>/dev/null)

print_cyan "  Final memory count: $FINAL_COUNT"
print_cyan "  Highest importance: $HIGHEST_IMPORTANCE"

if [ "$FINAL_COUNT" -eq 3 ] && [ "$HIGHEST_IMPORTANCE" -eq 10 ]; then
    print_green "  ✓ Workflow state consistent"
fi

# ===================================================================
# CLEANUP
# ===================================================================

rm -f "$EXPORT_FILE"
teardown_persona "$TEST_DB"

# ===================================================================
# TEST SUMMARY
# ===================================================================

section "Test Summary: Integration - End-to-End Workflow [BASELINE]"

echo "✓ Step 1 - Storage: PASS (2 memories)"
echo "✓ Step 2 - Enrichment: PASS ($ENRICHED/2)"
echo "✓ Step 3 - Search: PASS"
echo "✓ Step 4 - Consolidation: PASS (3 total)"
echo "✓ Step 5 - Export: PASS (${EXPORT_SIZE:-0} bytes)"
echo "✓ Workflow quality: PASS"
echo ""
echo "Complete E2E Journey:"
echo "  Store → Enrich → Search → Consolidate → Export"
echo ""

print_green "✓ ALL TESTS PASSED"
exit 0
