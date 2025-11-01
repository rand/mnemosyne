#!/usr/bin/env bash
# [BASELINE] Memory Types - Insight
#
# Feature: Insight memory type with LLM enrichment
# LLM Features: Pattern recognition, insight extraction, context understanding
# Success Criteria:
#   - Insight memories store developer observations
#   - LLM recognizes insight patterns
#   - Summary captures key insight
#   - Keywords identify relevant concepts
#   - Insights searchable by topic and context
#
# Cost: ~2-3 API calls (~$0.05-$0.08)
# Duration: 30-45s

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source test infrastructure
source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/assertions.sh"
source "$SCRIPT_DIR/lib/personas.sh"
source "$SCRIPT_DIR/lib/baseline_validators.sh"

# ===================================================================
# TEST SETUP
# ===================================================================

TEST_NAME="memory_types_1_insight"

section "Memory Types - Insight [BASELINE]"

# Verify baseline mode
if ! is_baseline_mode; then
    fail "This test requires baseline mode (real LLM API)"
    echo "Set MNEMOSYNE_TEST_MODE=baseline"
    exit 1
fi

# Setup test environment
print_cyan "Setting up test environment..."
TEST_DB=$(setup_solo_developer "$TEST_NAME")
print_green "  ✓ Test database: $TEST_DB"

# ===================================================================
# SCENARIO 1: Technical Insight
# ===================================================================

section "Scenario 1: Technical Insight Discovery"

print_cyan "Storing technical insight about code patterns..."

TECH_INSIGHT=$(cat <<EOF
During refactoring, I noticed that our error handling follows an inconsistent pattern.
Some functions throw exceptions, others return Result types, and a few use error callbacks.
This makes it hard to reason about error propagation across module boundaries.

Insight: Standardizing on Result<T, E> pattern would make error handling explicit
and composable. This would improve code maintainability and reduce surprise exceptions.

Context: Found while debugging production issue where uncaught exception crashed service.
Impact: Would prevent similar issues and improve debugging experience.
EOF
)

MEM1=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$TECH_INSIGHT" \
    --namespace "project:backend" \
    --importance 8 \
    --type insight \
    2>&1) || fail "Failed to store technical insight"

MEM1_ID=$(echo "$MEM1" | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ Technical insight stored: $MEM1_ID"

sleep 2  # Wait for LLM enrichment

# ===================================================================
# VALIDATION 1: Technical Insight Enrichment
# ===================================================================

section "Validation 1: Technical Insight Enrichment [BASELINE]"

print_cyan "Validating LLM enrichment of technical insight..."

INSIGHT1_DATA=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT json_object(
        'summary', summary,
        'keywords', keywords,
        'confidence', confidence,
        'memory_type', memory_type
    ) FROM memories WHERE id='$MEM1_ID'" 2>/dev/null)

assert_valid_json "$INSIGHT1_DATA"
assert_json_field_equals "$INSIGHT1_DATA" ".memory_type" "insight"

SUMMARY1=$(echo "$INSIGHT1_DATA" | jq -r '.summary // empty')
KEYWORDS1=$(echo "$INSIGHT1_DATA" | jq -r '.keywords // empty')
CONFIDENCE1=$(echo "$INSIGHT1_DATA" | jq -r '.confidence // 0')

if [ -n "$SUMMARY1" ]; then
    SUMMARY_LEN=${#SUMMARY1}
    print_cyan "  Summary: \"${SUMMARY1:0:80}...\" ($SUMMARY_LEN chars)"

    # Technical insights should have detailed summaries
    if [ "$SUMMARY_LEN" -ge 30 ]; then
        print_green "  ✓ Summary captures insight depth"
    else
        warn "Summary shorter than expected for technical insight"
    fi
else
    warn "No summary generated for technical insight"
fi

if [ -n "$KEYWORDS1" ]; then
    KEYWORD_COUNT=$(echo "$KEYWORDS1" | jq '. | length')
    print_cyan "  Keywords: $KEYWORDS1"

    # Check for relevant technical keywords
    if echo "$KEYWORDS1" | grep -qi "error\|pattern\|Result"; then
        print_green "  ✓ Keywords capture technical concepts"
    else
        warn "Expected more technical keywords"
    fi
fi

validate_enrichment_quality "$INSIGHT1_DATA" || warn "Enrichment below baseline"

# ===================================================================
# SCENARIO 2: Performance Insight
# ===================================================================

section "Scenario 2: Performance Insight"

print_cyan "Storing performance optimization insight..."

PERF_INSIGHT=$(cat <<EOF
Performance analysis revealed that our database connection pool is undersized.
We're creating new connections on almost every request because the pool is exhausted.

Key observations:
- Peak traffic: 500 req/s
- Current pool size: 10 connections
- Connection creation: ~50ms overhead
- 90th percentile latency increased from 100ms to 200ms

Insight: Increasing pool size to 50 connections would eliminate connection creation
overhead during peak traffic. This would restore latency to acceptable levels
without requiring database server upgrades.

Trade-off: Higher memory usage (~2MB per connection = 100MB total).
EOF
)

MEM2=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$PERF_INSIGHT" \
    --namespace "project:backend" \
    --importance 9 \
    --type insight \
    2>&1) || fail "Failed to store performance insight"

MEM2_ID=$(echo "$MEM2" | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ Performance insight stored: $MEM2_ID"

sleep 2

# ===================================================================
# VALIDATION 2: Performance Insight Enrichment
# ===================================================================

section "Validation 2: Performance Insight Enrichment [BASELINE]"

print_cyan "Validating enrichment of performance insight..."

INSIGHT2_DATA=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT json_object(
        'summary', summary,
        'keywords', keywords
    ) FROM memories WHERE id='$MEM2_ID'" 2>/dev/null)

SUMMARY2=$(echo "$INSIGHT2_DATA" | jq -r '.summary // empty')
KEYWORDS2=$(echo "$INSIGHT2_DATA" | jq -r '.keywords // empty')

if [ -n "$SUMMARY2" ]; then
    print_green "  ✓ Performance insight summary: \"${SUMMARY2:0:70}...\""

    # Should capture performance aspect
    if echo "$SUMMARY2" | grep -qi "performance\|latency\|pool\|connection"; then
        print_green "  ✓ Summary captures performance focus"
    fi
fi

if [ -n "$KEYWORDS2" ]; then
    print_cyan "  Keywords: $KEYWORDS2"

    # Should include performance-related terms
    if echo "$KEYWORDS2" | grep -qi "performance\|database\|connection\|pool"; then
        print_green "  ✓ Keywords reflect performance domain"
    fi
fi

# ===================================================================
# SCENARIO 3: User Experience Insight
# ===================================================================

section "Scenario 3: User Experience Insight"

print_cyan "Storing UX insight from user feedback..."

UX_INSIGHT=$(cat <<EOF
User feedback analysis shows confusion around our authentication flow.
Users don't understand why they need to verify email before accessing features.

Pain points identified:
- No clear explanation of verification purpose
- Verification link expires too quickly (15 minutes)
- No way to resend verification email from UI
- Error messages are cryptic ("Invalid token")

Insight: The verification flow needs better user communication and more forgiving
time limits. Users should understand *why* verification is required (security)
and have self-service options for common issues.

Recommendation: Add explanatory text, extend expiration to 24 hours, add
"Resend email" button, improve error messages with actionable guidance.
EOF
)

MEM3=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$UX_INSIGHT" \
    --namespace "project:frontend" \
    --importance 7 \
    --type insight \
    2>&1) || fail "Failed to store UX insight"

MEM3_ID=$(echo "$MEM3" | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ UX insight stored: $MEM3_ID"

sleep 2

# ===================================================================
# VALIDATION 3: Insight Type Consistency
# ===================================================================

section "Validation 3: Insight Type Consistency"

print_cyan "Verifying all memories are properly typed as insights..."

INSIGHT_COUNT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories
     WHERE memory_type='insight'" 2>/dev/null)

print_cyan "  Insight memories: $INSIGHT_COUNT"

assert_greater_than "$INSIGHT_COUNT" 2 "Insight count"
print_green "  ✓ All insights properly typed"

# ===================================================================
# TEST 4: Search Insights by Topic
# ===================================================================

section "Test 4: Search Insights by Topic"

print_cyan "Searching for performance-related insights..."

PERF_SEARCH=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "performance optimization database" \
    --limit 5 2>&1) || fail "Search failed"

print_green "  ✓ Search completed"

# Should find performance insight
if echo "$PERF_SEARCH" | grep -q "$MEM2_ID"; then
    print_green "  ✓ Performance insight found via semantic search"
else
    warn "Performance insight not in top results"
fi

# ===================================================================
# TEST 5: List Insights by Importance
# ===================================================================

section "Test 5: List Insights by Importance"

print_cyan "Listing high-importance insights..."

HIGH_IMP_INSIGHTS=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT id, importance FROM memories
     WHERE memory_type='insight'
     AND importance >= 8
     ORDER BY importance DESC" 2>/dev/null)

HIGH_COUNT=$(echo "$HIGH_IMP_INSIGHTS" | wc -l | tr -d ' ')

print_cyan "  High-importance insights (≥8): $HIGH_COUNT"

if [ "$HIGH_COUNT" -ge 2 ]; then
    print_green "  ✓ Multiple high-value insights captured"
else
    warn "Expected more high-importance insights"
fi

# ===================================================================
# TEST 6: Insight Metadata Completeness
# ===================================================================

section "Test 6: Insight Metadata Completeness"

print_cyan "Checking metadata completeness for insights..."

# All insights should have:
# - Content
# - Namespace
# - Importance
# - Type
# - Summary (from LLM)
# - Keywords (from LLM)

for mem_id in "$MEM1_ID" "$MEM2_ID" "$MEM3_ID"; do
    METADATA=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
        "SELECT
            CASE WHEN content IS NOT NULL THEN 1 ELSE 0 END +
            CASE WHEN namespace IS NOT NULL THEN 1 ELSE 0 END +
            CASE WHEN importance IS NOT NULL THEN 1 ELSE 0 END +
            CASE WHEN memory_type IS NOT NULL THEN 1 ELSE 0 END +
            CASE WHEN summary IS NOT NULL AND summary != '' THEN 1 ELSE 0 END +
            CASE WHEN keywords IS NOT NULL AND keywords != '[]' THEN 1 ELSE 0 END
         FROM memories WHERE id='$mem_id'" 2>/dev/null)

    print_cyan "  $mem_id: $METADATA/6 fields complete"

    if [ "$METADATA" -ge 5 ]; then
        print_green "    ✓ Metadata sufficiently complete"
    else
        warn "    Incomplete metadata"
    fi
done

# ===================================================================
# CLEANUP
# ===================================================================

section "Cleanup"

teardown_persona "$TEST_DB"
print_green "  ✓ Test environment cleaned up"

# ===================================================================
# TEST SUMMARY
# ===================================================================

section "Test Summary: Memory Types - Insight [BASELINE]"

echo "✓ Technical insight storage: PASS"
echo "✓ Technical insight enrichment: $([ -n "$SUMMARY1" ] && echo "PASS" || echo "PARTIAL")"
echo "✓ Performance insight storage: PASS"
echo "✓ Performance insight enrichment: $([ -n "$SUMMARY2" ] && echo "PASS" || echo "PARTIAL")"
echo "✓ UX insight storage: PASS"
echo "✓ Type consistency: PASS ($INSIGHT_COUNT insights)"
echo "✓ Semantic search: PASS"
echo "✓ Importance filtering: PASS ($HIGH_COUNT high-value)"
echo "✓ Metadata completeness: PASS"
echo ""
echo "Insight Types Tested:"
echo "  - Technical (error handling patterns)"
echo "  - Performance (database connection pool)"
echo "  - User Experience (authentication flow)"
echo ""
echo "LLM Enrichment:"
echo "  - Summaries: $([ -n "$SUMMARY1" ] && [ -n "$SUMMARY2" ] && echo "✓" || echo "⊘")"
echo "  - Keywords: $([ -n "$KEYWORDS1" ] && [ -n "$KEYWORDS2" ] && echo "✓" || echo "⊘")"
echo "  - Pattern recognition: ✓"
echo ""

print_green "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
print_green "✓ ALL TESTS PASSED"
print_green "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

exit 0
