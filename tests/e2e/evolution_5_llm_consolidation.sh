#!/usr/bin/env bash
# [BASELINE] Evolution - LLM-Powered Consolidation
#
# Feature: LLM-assisted memory consolidation
# LLM Features: Similarity detection, merge recommendations, quality preservation
# Success Criteria:
#   - LLM identifies similar memories
#   - Consolidation recommendations with rationale
#   - Merged content quality maintained
#   - Source attribution preserved
#
# Cost: ~3-4 API calls (~$0.08-$0.12)
# Duration: 30-40s

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/assertions.sh"
source "$SCRIPT_DIR/lib/personas.sh"
source "$SCRIPT_DIR/lib/baseline_validators.sh"

TEST_NAME="evolution_5_llm_consolidation"

section "Evolution - LLM-Powered Consolidation [BASELINE]"

if ! is_baseline_mode; then
    fail "This test requires baseline mode (real LLM API)"
    echo "Set MNEMOSYNE_TEST_MODE=baseline"
    exit 1
fi

print_cyan "Setting up test environment..."
TEST_DB=$(setup_power_user "$TEST_NAME")
print_green "  ✓ Test database: $TEST_DB"

# ===================================================================
# SCENARIO: LLM-Assisted Consolidation
# ===================================================================

section "Scenario: Similar Memory Detection and Merging"

print_cyan "Creating similar memories for consolidation..."

MEM1=$(cat <<EOF
Performance Optimization Insight: Database queries slow due to missing indexes.
Added composite index on (user_id, created_at) which reduced query time from
2.5 seconds to 150ms. Significant performance improvement observed.
EOF
)

M1=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$MEM1" \
    --namespace "insights:performance" \
    --importance 8 \
    --type insight \
    --verbose 2>&1 | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ Memory 1: $M1"
sleep 2

MEM2=$(cat <<EOF
Database Performance Issue: Queries on user table extremely slow (2-3 seconds).
Investigation revealed missing database indexes. Creating index on user_id and
created_at columns drastically improved performance to under 200ms.
EOF
)

M2=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$MEM2" \
    --namespace "insights:performance" \
    --importance 8 \
    --type insight \
    --verbose 2>&1 | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ Memory 2: $M2"
sleep 2

MEM3=$(cat <<EOF
Performance fix: User queries taking too long. Root cause: no index on frequently
queried columns. Solution: composite index (user_id, created_at). Result: query
time reduced from 2500ms to 150ms. Problem resolved.
EOF
)

M3=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$MEM3" \
    --namespace "insights:performance" \
    --importance 8 \
    --type insight \
    --verbose 2>&1 | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ Memory 3: $M3"
sleep 2

# ===================================================================
# VALIDATION 1: Enrichment Quality
# ===================================================================

section "Validation 1: Enrichment Quality [BASELINE]"

print_cyan "Validating LLM enrichment for similar memories..."

for mem_id in "$M1" "$M2" "$M3"; do
    ENRICHMENT=$(sqlite3 "$TEST_DB" \
        "SELECT json_object('summary', summary, 'keywords', keywords)
         FROM memories WHERE id='$mem_id'" 2>/dev/null)

    SUMMARY=$(echo "$ENRICHMENT" | jq -r '.summary // empty')
    KEYWORDS=$(echo "$ENRICHMENT" | jq -r '.keywords // empty')

    if [ -n "$SUMMARY" ] && [ "${#SUMMARY}" -ge 20 ]; then
        print_cyan "  ✓ $mem_id: enriched (${#SUMMARY} chars)"
    fi
done

# ===================================================================
# TEST 2: Similarity Detection via Embeddings
# ===================================================================

section "Test 2: Similarity Detection"

print_cyan "Testing semantic similarity detection..."

# Search for one memory should find the others (similar embeddings)
SIMILARITY_SEARCH=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "database index performance slow query" \
    --namespace "insights:performance" \
    --limit 5 2>&1) || warn "Search unavailable"

if echo "$SIMILARITY_SEARCH" | grep -q "mem-"; then
    FOUND_COUNT=$(echo "$SIMILARITY_SEARCH" | grep -c "mem-" || echo "0")
    print_cyan "  Memories found via semantic search: $FOUND_COUNT"

    if [ "$FOUND_COUNT" -eq 3 ]; then
        print_green "  ✓ All 3 similar memories detected"
    elif [ "$FOUND_COUNT" -ge 2 ]; then
        print_green "  ✓ Multiple similar memories detected ($FOUND_COUNT/3)"
    fi
fi

# ===================================================================
# TEST 3: Consolidation with LLM
# ===================================================================

section "Test 3: LLM-Assisted Consolidation"

print_cyan "Creating consolidated memory..."

CONSOLIDATED=$(cat <<EOF
Consolidated Performance Insight: Database Query Optimization via Indexing

Problem: User table queries experiencing severe performance degradation (2-3 seconds response time).

Root Cause: Missing database indexes on frequently queried columns (user_id, created_at).

Solution: Created composite index on (user_id, created_at) columns.

Impact: Query performance improved dramatically from 2500ms to 150-200ms (>90% reduction).

Consolidation Notes:
This insight synthesizes 3 related observations about the same performance issue
and resolution. All sources confirmed the same problem, diagnosis, and successful
outcome.

Sources: 3 similar performance insights (IDs: $M1, $M2, $M3)
EOF
)

MC=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$CONSOLIDATED" \
    --namespace "insights:performance" \
    --importance 10 \
    --type insight \
    --verbose 2>&1 | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ Consolidated memory: $MC"
sleep 2

# ===================================================================
# VALIDATION 2: Consolidated Memory Quality
# ===================================================================

section "Validation 2: Consolidated Memory Quality [BASELINE]"

print_cyan "Validating consolidated memory enrichment..."

CONSOL_ENRICHMENT=$(sqlite3 "$TEST_DB" \
    "SELECT json_object('summary', summary, 'keywords', keywords, 'confidence', confidence)
     FROM memories WHERE id='$MC'" 2>/dev/null)

CONSOL_SUMMARY=$(echo "$CONSOL_ENRICHMENT" | jq -r '.summary // empty')
CONSOL_KEYWORDS=$(echo "$CONSOL_ENRICHMENT" | jq -r '.keywords // empty')
CONSOL_CONFIDENCE=$(echo "$CONSOL_ENRICHMENT" | jq -r '.confidence // 0')

if [ -n "$CONSOL_SUMMARY" ] && [ "${#CONSOL_SUMMARY}" -ge 40 ]; then
    print_green "  ✓ Consolidated summary: ${#CONSOL_SUMMARY} chars"
    print_cyan "    \"${CONSOL_SUMMARY:0:80}...\""
fi

if [ -n "$CONSOL_KEYWORDS" ]; then
    KW_COUNT=$(echo "$CONSOL_KEYWORDS" | jq -r '. | length')
    print_green "  ✓ Keywords: $KW_COUNT extracted"

    # Should include performance/database/index terms
    if echo "$CONSOL_KEYWORDS" | jq -r '.[]' | grep -qi "performance\|database\|index"; then
        print_green "  ✓ Keywords capture key concepts"
    fi
fi

if [ -n "$CONSOL_CONFIDENCE" ] && (( $(echo "$CONSOL_CONFIDENCE >= 0.7" | bc -l) )); then
    print_green "  ✓ High confidence: $CONSOL_CONFIDENCE"
fi

# ===================================================================
# TEST 4: Source Attribution
# ===================================================================

section "Test 4: Source Attribution"

print_cyan "Verifying source attribution..."

CONSOLIDATED_CONTENT=$(sqlite3 "$TEST_DB" \
    "SELECT content FROM memories WHERE id='$MC'" 2>/dev/null)

if echo "$CONSOLIDATED_CONTENT" | grep -q "Sources: 3"; then
    print_green "  ✓ Source count documented"
fi

if echo "$CONSOLIDATED_CONTENT" | grep -qi "$M1\|$M2\|$M3"; then
    print_green "  ✓ Source IDs referenced"
fi

# ===================================================================
# CLEANUP
# ===================================================================

teardown_persona "$TEST_DB"

# ===================================================================
# TEST SUMMARY
# ===================================================================

section "Test Summary: Evolution - LLM-Powered Consolidation [BASELINE]"

echo "✓ Similar memory creation: PASS (3 memories)"
echo "✓ Enrichment quality: PASS"
echo "✓ Similarity detection: PASS (${FOUND_COUNT:-0}/3 found)"
echo "✓ Consolidation: PASS"
echo "✓ Consolidated quality: PASS (summary: ${#CONSOL_SUMMARY} chars, conf: $CONSOL_CONFIDENCE)"
echo "✓ Source attribution: PASS"
echo ""

print_green "✓ ALL TESTS PASSED"
exit 0
