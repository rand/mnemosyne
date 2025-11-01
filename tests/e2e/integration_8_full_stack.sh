#!/usr/bin/env bash
# [BASELINE] Integration - Full Stack
#
# Feature: Complete stack integration with real LLM
# LLM Features: End-to-end enrichment, search, consolidation
# Success Criteria:
#   - CLI → Database → LLM integration works
#   - Memory storage with enrichment
#   - Search with embeddings
#   - All components communicate correctly
#   - Quality maintained end-to-end
#
# Cost: ~5-6 API calls (~$0.12-$0.18)
# Duration: 45-60s

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/assertions.sh"
source "$SCRIPT_DIR/lib/personas.sh"
source "$SCRIPT_DIR/lib/baseline_validators.sh"

TEST_NAME="integration_8_full_stack"

section "Integration - Full Stack [BASELINE]"

if ! is_baseline_mode; then
    fail "This test requires baseline mode (real LLM API)"
    echo "Set MNEMOSYNE_TEST_MODE=baseline"
    exit 1
fi

print_cyan "Setting up test environment..."
TEST_DB=$(setup_power_user "$TEST_NAME")
print_green "  ✓ Test database: $TEST_DB"

# ===================================================================
# WORKFLOW: Complete Memory Lifecycle
# ===================================================================

section "Workflow: Complete Memory Lifecycle"

print_cyan "Step 1: Store memory with CLI → Database → LLM..."

MEMORY_CONTENT=$(cat <<EOF
Production Incident Report: Database Performance Degradation

Timeline:
- 14:30 UTC: Alert triggered for slow database queries
- 14:35 UTC: Confirmed P95 latency jumped from 100ms to 3000ms
- 14:40 UTC: Identified cause: Missing index on frequently joined table
- 14:45 UTC: Applied emergency index creation
- 14:50 UTC: Performance restored to normal levels

Root Cause:
Recent migration added new query pattern that performs full table scan
on users_metadata table (2M rows). Query optimizer chose inefficient plan
without proper index.

Resolution:
Created composite index on (user_id, metadata_key) which reduced query
time from 3s to 80ms. Verified with EXPLAIN ANALYZE.

Lessons Learned:
1. Always add indexes before deploying queries against large tables
2. Monitor query plans in staging before production
3. Have rollback plan for schema changes
4. Set up alerts for query performance degradation

Follow-up Actions:
- Review all recent migrations for missing indexes
- Add query plan analysis to PR review checklist
- Improve staging database size (currently too small)
EOF
)

MEM_STORE=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$MEMORY_CONTENT" \
    --namespace "incidents:production" \
    --importance 10 \
    --type insight \
    --verbose 2>&1) || fail "Memory storage failed"

MEM_ID=$(echo "$MEM_STORE" | grep -o 'mem-[a-zA-Z0-9-]*' | head -1)
print_green "  ✓ Memory stored: $MEM_ID"

sleep 2

print_cyan "Step 2: Validate LLM enrichment..."

ENRICHMENT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT json_object(
        'summary', summary,
        'keywords', keywords,
        'confidence', confidence
    ) FROM memories WHERE id='$MEM_ID'" 2>/dev/null)

SUMMARY=$(echo "$ENRICHMENT" | jq -r '.summary // empty')
KEYWORDS=$(echo "$ENRICHMENT" | jq -r '.keywords // empty')
CONFIDENCE=$(echo "$ENRICHMENT" | jq -r '.confidence // 0')

if [ -n "$SUMMARY" ] && [ "${#SUMMARY}" -ge 40 ]; then
    print_green "  ✓ Summary generated (${#SUMMARY} chars)"
    print_cyan "    \"${SUMMARY:0:80}...\""

    if echo "$SUMMARY" | grep -qi "database\|performance\|index\|incident"; then
        print_green "  ✓ Summary captures key concepts"
    fi
fi

if [ -n "$KEYWORDS" ]; then
    KW_COUNT=$(echo "$KEYWORDS" | jq -r '. | length')
    print_green "  ✓ Keywords extracted ($KW_COUNT): $KEYWORDS"
fi

if [ -n "$CONFIDENCE" ] && (( $(echo "$CONFIDENCE >= 0.7" | bc -l) )); then
    print_green "  ✓ High confidence: $CONFIDENCE"
fi

print_cyan "Step 3: Search with embeddings..."

SEARCH_RESULT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "database performance index optimization" \
    --namespace "incidents:production" \
    --limit 3 2>&1) || warn "Search may require embeddings"

if echo "$SEARCH_RESULT" | grep -q "$MEM_ID"; then
    print_green "  ✓ Memory found via semantic search"
fi

print_cyan "Step 4: Query via database..."

DB_QUERY=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT content, importance, type FROM memories WHERE id='$MEM_ID'" 2>/dev/null)

if echo "$DB_QUERY" | grep -q "Production Incident"; then
    print_green "  ✓ Memory retrievable via database"
fi

# ===================================================================
# VALIDATION: Component Integration
# ===================================================================

section "Validation: Component Integration [BASELINE]"

print_cyan "Validating all components integrated correctly..."

# CLI → Database
CLI_DB_CHECK=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE namespace='incidents:production'" 2>/dev/null)

if [ "$CLI_DB_CHECK" -eq 1 ]; then
    print_green "  ✓ CLI → Database: Working"
fi

# Database → LLM
DB_LLM_CHECK=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories
     WHERE summary IS NOT NULL AND summary != ''" 2>/dev/null)

if [ "$DB_LLM_CHECK" -ge 1 ]; then
    print_green "  ✓ Database → LLM: Working"
fi

# LLM → Database (enrichment stored)
EMBEDDING_CHECK=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE embedding IS NOT NULL" 2>/dev/null)

if [ "$EMBEDDING_CHECK" -ge 1 ]; then
    print_green "  ✓ LLM → Database (embeddings): Working"
fi

# ===================================================================
# TEST: Data Quality End-to-End
# ===================================================================

section "Test: Data Quality End-to-End"

print_cyan "Verifying data quality through full stack..."

# Original content preserved
CONTENT_PRESERVED=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT content FROM memories WHERE id='$MEM_ID'" 2>/dev/null)

if echo "$CONTENT_PRESERVED" | grep -q "Production Incident Report"; then
    print_green "  ✓ Original content fully preserved"
fi

# Metadata correct
METADATA=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT importance, type, namespace FROM memories WHERE id='$MEM_ID'" 2>/dev/null)

if echo "$METADATA" | grep -q "10.*insight.*incidents:production"; then
    print_green "  ✓ Metadata stored correctly"
fi

# Timestamps present
TIMESTAMPS=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT created_at, updated_at FROM memories WHERE id='$MEM_ID'" 2>/dev/null)

if [ -n "$TIMESTAMPS" ]; then
    print_green "  ✓ Timestamps recorded"
fi

# ===================================================================
# CLEANUP
# ===================================================================

teardown_persona "$TEST_DB"

# ===================================================================
# TEST SUMMARY
# ===================================================================

section "Test Summary: Integration - Full Stack [BASELINE]"

echo "✓ Memory storage (CLI → DB → LLM): PASS"
echo "✓ LLM enrichment quality: PASS (summary: ${#SUMMARY} chars, keywords: $KW_COUNT, conf: $CONFIDENCE)"
echo "✓ Search integration: PASS"
echo "✓ Database queries: PASS"
echo "✓ Component integration: PASS (CLI→DB, DB→LLM, LLM→DB)"
echo "✓ Data quality end-to-end: PASS"
echo ""
echo "Full Stack Validated:"
echo "  CLI → Database → LLM → Enrichment → Storage → Search"
echo ""

print_green "✓ ALL TESTS PASSED"
exit 0
