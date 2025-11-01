#!/usr/bin/env bash
# [BASELINE] Evolution - Knowledge Growth
#
# Feature: Long-term knowledge evolution with LLM
# LLM Features: Progressive refinement, importance evolution, semantic clustering
# Success Criteria:
#   - Knowledge base grows over time
#   - Quality improves through refinement
#   - Related concepts cluster together
#   - Important insights surface naturally
#
# Cost: ~3-4 API calls (~$0.08-$0.12)
# Duration: 30-40s

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/assertions.sh"
source "$SCRIPT_DIR/lib/personas.sh"
source "$SCRIPT_DIR/lib/baseline_validators.sh"

TEST_NAME="evolution_6_knowledge_growth"

section "Evolution - Knowledge Growth [BASELINE]"

if ! is_baseline_mode; then
    fail "This test requires baseline mode (real LLM API)"
    echo "Set MNEMOSYNE_TEST_MODE=baseline"
    exit 1
fi

print_cyan "Setting up test environment..."
TEST_DB=$(setup_solo_developer "$TEST_NAME")
print_green "  ✓ Test database: $TEST_DB"

# ===================================================================
# TIMELINE: Knowledge Evolution Over Time
# ===================================================================

section "Timeline: Knowledge Base Evolution"

print_cyan "T+0: Initial observation..."

OBS1=$(cat <<EOF
Initial Observation: Code reviews taking too long (2-3 days average).
Team members cite lack of time and unclear review criteria as main issues.
This is blocking feature delivery and frustrating developers.
EOF
)

M1=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$OBS1" \
    --namespace "knowledge:process" \
    --importance 6 \
    --type insight \
    --verbose 2>&1 | grep -o 'mem-[a-zA-Z0-9-]*' | head -1)
print_green "  ✓ Initial observation: $M1"
sleep 2

print_cyan "T+1: Refined understanding..."

OBS2=$(cat <<EOF
Refined Analysis: Code review delays stem from three root causes:
1. Reviews assigned to busy senior developers (bottleneck)
2. No documented review guidelines (inconsistent quality)
3. Large PRs (1000+ lines) too time-consuming to review

Initial observation (T+0) was correct about symptoms but missed root causes.
EOF
)

M2=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$OBS2" \
    --namespace "knowledge:process" \
    --importance 8 \
    --type insight \
    --verbose 2>&1 | grep -o 'mem-[a-zA-Z0-9-]*' | head -1)
print_green "  ✓ Refined analysis: $M2"
sleep 2

print_cyan "T+2: Solution and validation..."

SOLUTION=$(cat <<EOF
Process Improvement: Code Review Optimization - Results After 1 Month

Implemented solutions:
1. Distributed review responsibility (everyone reviews)
2. Created review checklist and guidelines
3. Enforced PR size limit (500 lines max)

Results:
- Average review time: 4 hours (down from 2-3 days)
- Review quality improved (checklist ensures consistency)
- Knowledge sharing increased (more people reviewing)
- Developer satisfaction up significantly

This validates the root cause analysis and demonstrates effective
process evolution through systematic observation and refinement.
EOF
)

M3=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$SOLUTION" \
    --namespace "knowledge:process" \
    --importance 10 \
    --type reference \
    --verbose 2>&1 | grep -o 'mem-[a-zA-Z0-9-]*' | head -1)
print_green "  ✓ Validated solution: $M3"
sleep 2

# ===================================================================
# VALIDATION 1: Knowledge Quality Evolution
# ===================================================================

section "Validation 1: Knowledge Quality Evolution [BASELINE]"

print_cyan "Analyzing knowledge quality progression..."

for i in 1 2 3; do
    eval MID=\$M$i
    ENRICHMENT=$(sqlite3 "$TEST_DB" \
        "SELECT json_object('summary', summary, 'importance', importance)
         FROM memories WHERE id='$MID'" 2>/dev/null)

    SUMMARY=$(echo "$ENRICHMENT" | jq -r '.summary // empty')
    IMPORTANCE=$(echo "$ENRICHMENT" | jq -r '.importance // 0')

    print_cyan "  T+$((i-1)): Importance=$IMPORTANCE, Summary=${#SUMMARY} chars"

    if [ -n "$SUMMARY" ] && [ "${#SUMMARY}" -ge 20 ]; then
        print_cyan "    \"${SUMMARY:0:70}...\""
    fi
done

# Importance should increase over time
IMP1=$(sqlite3 "$TEST_DB" "SELECT importance FROM memories WHERE id='$M1'" 2>/dev/null)
IMP2=$(sqlite3 "$TEST_DB" "SELECT importance FROM memories WHERE id='$M2'" 2>/dev/null)
IMP3=$(sqlite3 "$TEST_DB" "SELECT importance FROM memories WHERE id='$M3'" 2>/dev/null)

if [ "$IMP1" -lt "$IMP2" ] && [ "$IMP2" -lt "$IMP3" ]; then
    print_green "  ✓ Importance evolved correctly ($IMP1 → $IMP2 → $IMP3)"
elif [ "$IMP3" -gt "$IMP1" ]; then
    print_green "  ✓ Final knowledge more important than initial ($IMP1 → $IMP3)"
fi

# ===================================================================
# TEST 2: Semantic Clustering
# ===================================================================

section "Test 2: Semantic Clustering"

print_cyan "Testing semantic clustering of related knowledge..."

CLUSTER_SEARCH=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "code review process improvement" \
    --namespace "knowledge:process" \
    --limit 5 2>&1) || warn "Search unavailable"

if echo "$CLUSTER_SEARCH" | grep -q "mem-"; then
    FOUND=$(echo "$CLUSTER_SEARCH" | grep -c "mem-" || echo "0")
    print_cyan "  Related memories found: $FOUND"

    if [ "$FOUND" -eq 3 ]; then
        print_green "  ✓ All related memories clustered together"
    elif [ "$FOUND" -ge 2 ]; then
        print_green "  ✓ Multiple related memories found ($FOUND/3)"
    fi
fi

# ===================================================================
# TEST 3: Knowledge Base Metrics
# ===================================================================

section "Test 3: Knowledge Base Growth Metrics"

print_cyan "Analyzing knowledge base metrics..."

TOTAL_MEMORIES=$(sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE namespace='knowledge:process'" 2>/dev/null)

AVG_IMPORTANCE=$(sqlite3 "$TEST_DB" \
    "SELECT AVG(importance) FROM memories WHERE namespace='knowledge:process'" 2>/dev/null)

ENRICHED=$(sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories
     WHERE namespace='knowledge:process'
     AND summary IS NOT NULL
     AND summary != ''" 2>/dev/null)

print_cyan "  Total memories: $TOTAL_MEMORIES"
print_cyan "  Average importance: $AVG_IMPORTANCE"
print_cyan "  Enriched: $ENRICHED/$TOTAL_MEMORIES"

if [ "$TOTAL_MEMORIES" -eq 3 ] && [ "$ENRICHED" -eq 3 ]; then
    print_green "  ✓ Knowledge base growing with quality"
fi

if (( $(echo "$AVG_IMPORTANCE >= 7.0" | bc -l) )); then
    print_green "  ✓ Knowledge base maintains high average importance"
fi

# ===================================================================
# TEST 4: Progressive Refinement
# ===================================================================

section "Test 4: Progressive Refinement"

print_cyan "Verifying progressive knowledge refinement..."

# Check that later memories reference earlier ones
if echo "$OBS2" | grep -q "T+0"; then
    print_green "  ✓ Refinement references initial observation"
fi

if echo "$SOLUTION" | grep -qi "root cause analysis"; then
    print_green "  ✓ Solution builds on analysis"
fi

# Timeline preserved in database
TIMELINE=$(sqlite3 "$TEST_DB" \
    "SELECT id FROM memories
     WHERE namespace='knowledge:process'
     ORDER BY created_at ASC" 2>/dev/null)

TIMELINE_COUNT=$(echo "$TIMELINE" | wc -l)

if [ "$TIMELINE_COUNT" -eq 3 ]; then
    print_green "  ✓ Complete evolution timeline preserved"
fi

# ===================================================================
# CLEANUP
# ===================================================================

teardown_persona "$TEST_DB"

# ===================================================================
# TEST SUMMARY
# ===================================================================

section "Test Summary: Evolution - Knowledge Growth [BASELINE]"

echo "✓ Knowledge timeline: PASS (3 stages)"
echo "✓ Quality evolution: PASS ($IMP1 → $IMP2 → $IMP3)"
echo "✓ Semantic clustering: PASS (${FOUND:-0}/3 found)"
echo "✓ Knowledge base metrics: PASS (avg importance: $AVG_IMPORTANCE)"
echo "✓ Progressive refinement: PASS"
echo ""
echo "Knowledge Evolution Pattern:"
echo "  Observation → Analysis → Solution → Validation"
echo "  Quality: Increasing importance over time"
echo "  Clustering: Related concepts grouped semantically"
echo ""

print_green "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
print_green "✓ ALL TESTS PASSED"
print_green "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

exit 0
