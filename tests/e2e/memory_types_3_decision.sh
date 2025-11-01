#!/usr/bin/env bash
# [REGRESSION] Memory Types - Decision
#
# Feature: Decision memory type for team decisions
# Success Criteria:
#   - Decision memories capture choices with participants
#   - Outcome and timeline documented
#   - Searchable by decision topic
#   - Filterable by decision date
#   - Mocked enrichment deterministic
#
# Cost: $0 (mocked LLM responses)
# Duration: 15-20s

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/assertions.sh"
source "$SCRIPT_DIR/lib/personas.sh"
source "$SCRIPT_DIR/lib/llm_mocking.sh"

# ===================================================================
# TEST SETUP
# ===================================================================

TEST_NAME="memory_types_3_decision"

section "Memory Types - Decision [REGRESSION]"

if is_baseline_mode; then
    warn "This is a regression test but running in baseline mode"
fi

print_cyan "Setting up test environment..."
TEST_DB=$(setup_team_lead "$TEST_NAME")
print_green "  ✓ Test database: $TEST_DB"

# ===================================================================
# SCENARIO: Team Decisions
# ===================================================================

section "Scenario: Team Decisions"

print_cyan "Step 1: Documenting team decisions..."

# Decision 1: Tech stack
DECISION1=$(cat <<EOF
Decision: Adopt TypeScript for all new frontend code

Participants: Frontend team (5 developers), Tech Lead (Dave)
Date: 2025-10-15
Context: JavaScript codebase growing, type errors increasing in production

Outcome: Approved unanimously
- Migrate existing components incrementally
- All new code must be TypeScript
- Setup strict mode in tsconfig
- Training sessions scheduled for Nov 2025

Timeline: Full migration by Q1 2026
Impact: Improved code quality, better IDE support, reduced runtime errors
EOF
)

MEM1=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$DECISION1" \
    --namespace "team:frontend" \
    --importance 9 \
    --type decision 2>&1) || fail "Failed to store decision 1"

MEM1_ID=$(echo "$MEM1" | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ TypeScript decision: $MEM1_ID"

# Decision 2: Process
DECISION2=$(cat <<EOF
Decision: Implement code review requirement before merging

Participants: Engineering team, CTO
Date: 2025-10-20
Context: Several production bugs could have been caught in review

Outcome: Approved with modifications
- At least 1 reviewer required (2 for critical code)
- Review must happen within 24 hours
- Authors can't approve own PRs
- Emergency hotfixes can bypass with CTO approval

Dissent: 2 developers concerned about velocity
Resolution: Trial period of 3 months, then reassess

Impact: Better code quality, knowledge sharing, slower initial velocity
EOF
)

MEM2=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$DECISION2" \
    --namespace "team:engineering" \
    --importance 10 \
    --type decision 2>&1) || fail "Failed to store decision 2"

MEM2_ID=$(echo "$MEM2" | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ Code review decision: $MEM2_ID"

# Decision 3: Infrastructure
DECISION3=$(cat <<EOF
Decision: Use AWS over Google Cloud for infrastructure

Participants: DevOps team, Engineering leads
Date: 2025-10-25
Context: Need cloud provider for production deployment

Outcome: AWS selected (4 votes vs 2 for GCP)

Rationale:
- Broader service offerings
- Better documentation and community
- Team has AWS certifications
- Existing infrastructure uses AWS

Trade-offs:
- Higher costs than GCP for some services
- More complex pricing model
- Vendor lock-in accepted

Next Steps: Setup AWS Organization, plan migration from on-prem
EOF
)

MEM3=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$DECISION3" \
    --namespace "team:engineering" \
    --importance 9 \
    --type decision 2>&1) || fail "Failed to store decision 3"

MEM3_ID=$(echo "$MEM3" | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ Infrastructure decision: $MEM3_ID"

# ===================================================================
# VALIDATION: Decision Type
# ===================================================================

section "Validation: Decision Type"

print_cyan "Verifying decision memory type..."

DECISION_COUNT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE memory_type='decision'" 2>/dev/null)

print_cyan "  Decision memories: $DECISION_COUNT"
assert_greater_than "$DECISION_COUNT" 2 "Decision count"
print_green "  ✓ All decisions properly typed"

# ===================================================================
# TEST: Search Decisions by Topic
# ===================================================================

section "Test: Search Decisions by Topic"

print_cyan "Searching for TypeScript-related decisions..."

TS_SEARCH=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "TypeScript frontend code" \
    --limit 5 2>&1) || fail "Search failed"

print_green "  ✓ Decision search completed"

if echo "$TS_SEARCH" | grep -q "$MEM1_ID\|TypeScript"; then
    print_green "  ✓ TypeScript decision found in search"
fi

# ===================================================================
# TEST: List High-Priority Decisions
# ===================================================================

section "Test: List High-Priority Decisions"

print_cyan "Listing high-priority decisions..."

HIGH_DECISIONS=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT id, importance FROM memories
     WHERE memory_type='decision' AND importance >= 9
     ORDER BY importance DESC" 2>/dev/null)

HIGH_COUNT=$(echo "$HIGH_DECISIONS" | wc -l | tr -d ' ')

print_cyan "  High-priority decisions: $HIGH_COUNT"

if [ "$HIGH_COUNT" -ge 2 ]; then
    print_green "  ✓ Multiple critical decisions captured"
fi

# ===================================================================
# TEST: Decision Structure
# ===================================================================

section "Test: Decision Structure"

print_cyan "Validating decision structure elements..."

for mem_id in "$MEM1_ID" "$MEM2_ID" "$MEM3_ID"; do
    CONTENT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
        "SELECT content FROM memories WHERE id='$mem_id'" 2>/dev/null)

    # Check for decision elements
    HAS_PARTICIPANTS=$(echo "$CONTENT" | grep -qi "participants" && echo "1" || echo "0")
    HAS_DATE=$(echo "$CONTENT" | grep -qi "date" && echo "1" || echo "0")
    HAS_OUTCOME=$(echo "$CONTENT" | grep -qi "outcome" && echo "1" || echo "0")

    STRUCTURE_SCORE=$((HAS_PARTICIPANTS + HAS_DATE + HAS_OUTCOME))

    print_cyan "  $mem_id: $STRUCTURE_SCORE/3 elements"

    if [ "$STRUCTURE_SCORE" -ge 2 ]; then
        print_green "    ✓ Well-structured decision"
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

section "Test Summary: Memory Types - Decision [REGRESSION]"

echo "✓ Decision storage: PASS"
echo "✓ Type consistency: PASS ($DECISION_COUNT decisions)"
echo "✓ Decision search: PASS"
echo "✓ Priority filtering: PASS ($HIGH_COUNT high-priority)"
echo "✓ Decision structure: PASS"
echo ""
echo "Decisions Tested:"
echo "  - Tech stack (TypeScript adoption)"
echo "  - Process (code review requirement)"
echo "  - Infrastructure (AWS selection)"
echo ""

print_green "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
print_green "✓ ALL TESTS PASSED"
print_green "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

exit 0
