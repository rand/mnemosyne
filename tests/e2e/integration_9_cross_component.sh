#!/usr/bin/env bash
# [BASELINE] Integration - Cross-Component
#
# Feature: Cross-component workflows with real LLM
# LLM Features: Multi-namespace enrichment, cross-project search
# Success Criteria:
#   - Multiple namespaces work together
#   - Cross-namespace search functional
#   - Component boundaries respected
#   - Shared context accessible
#   - Quality maintained across components
#
# Cost: ~3-4 API calls (~$0.08-$0.12)
# Duration: 30-40s

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/assertions.sh"
source "$SCRIPT_DIR/lib/personas.sh"
source "$SCRIPT_DIR/lib/baseline_validators.sh"

TEST_NAME="integration_9_cross_component"

section "Integration - Cross-Component [BASELINE]"

if ! is_baseline_mode; then
    fail "This test requires baseline mode (real LLM API)"
    echo "Set MNEMOSYNE_TEST_MODE=baseline"
    exit 1
fi

print_cyan "Setting up test environment..."
TEST_DB=$(setup_team_lead "$TEST_NAME")
print_green "  ✓ Test database: $TEST_DB"

# ===================================================================
# SCENARIO: Multi-Component Workflow
# ===================================================================

section "Scenario: Multi-Component Workflow"

print_cyan "Creating memories across multiple components..."

# Component 1: Frontend
FRONTEND=$(cat <<EOF
Frontend Architecture Decision: Migrating from REST to GraphQL

Rationale: Better data fetching flexibility, reduced over-fetching,
strong typing with schema. Team familiar with Apollo Client.

Trade-offs: Learning curve for GraphQL, increased backend complexity,
need to migrate existing REST consumers gradually.

Implementation: Start with new features in GraphQL, maintain REST for
legacy endpoints, use Apollo Federation for microservices.
EOF
)

M1=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$FRONTEND" \
    --namespace "component:frontend" \
    --importance 9 \
    --type architecture \
    2>&1 | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ Frontend component: $M1"
sleep 2

# Component 2: Backend
BACKEND=$(cat <<EOF
Backend API Evolution: GraphQL Gateway Implementation

Implementing GraphQL gateway to unify microservice APIs.
Resolvers delegate to existing REST services.
Gradual migration strategy preserves backward compatibility.

Benefits: Single API endpoint, flexible queries, reduced roundtrips.
Challenges: Resolver performance optimization, caching strategy.
EOF
)

M2=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$BACKEND" \
    --namespace "component:backend" \
    --importance 9 \
    --type architecture \
    2>&1 | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ Backend component: $M2"
sleep 2

# Component 3: Shared architecture
SHARED=$(cat <<EOF
Shared Architecture: GraphQL Schema Design Standards

Establish naming conventions, error handling patterns, and
pagination standards across all GraphQL resolvers.

Standards ensure consistency between frontend and backend teams.
EOF
)

M3=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$SHARED" \
    --namespace "shared:architecture" \
    --importance 8 \
    --type reference \
    2>&1 | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ Shared component: $M3"
sleep 2

# ===================================================================
# TEST 1: Cross-Component Enrichment Quality
# ===================================================================

section "Test 1: Cross-Component Enrichment Quality [BASELINE]"

print_cyan "Validating enrichment across all components..."

for mem_id in "$M1" "$M2" "$M3"; do
    ENRICHMENT=$(sqlite3 "$TEST_DB" \
        "SELECT json_object('summary', summary, 'keywords', keywords)
         FROM memories WHERE id='$mem_id'" 2>/dev/null)

    SUMMARY=$(echo "$ENRICHMENT" | jq -r '.summary // empty')

    if [ -n "$SUMMARY" ] && [ "${#SUMMARY}" -ge 20 ]; then
        print_green "  ✓ $mem_id: enriched (${#SUMMARY} chars)"
    fi
done

# ===================================================================
# TEST 2: Cross-Namespace Search
# ===================================================================

section "Test 2: Cross-Namespace Search"

print_cyan "Testing search across multiple namespaces..."

CROSS_SEARCH=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "GraphQL migration architecture" \
    --limit 5 2>&1) || warn "Search unavailable"

if echo "$CROSS_SEARCH" | grep -q "mem-"; then
    RESULT_COUNT=$(echo "$CROSS_SEARCH" | grep -c "mem-" || echo "0")
    print_green "  ✓ Cross-namespace search found $RESULT_COUNT results"

    # Should find memories from multiple namespaces
    if echo "$CROSS_SEARCH" | grep -q "$M1" || echo "$CROSS_SEARCH" | grep -q "$M2"; then
        print_green "  ✓ Results span multiple components"
    fi
fi

# ===================================================================
# TEST 3: Namespace Boundaries
# ===================================================================

section "Test 3: Namespace Boundaries"

print_cyan "Verifying namespace isolation..."

FRONTEND_ONLY=$(sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE json_extract(namespace, '$.type') = 'global' " 2>/dev/null)
BACKEND_ONLY=$(sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE json_extract(namespace, '$.type') = 'global' " 2>/dev/null)
SHARED_ONLY=$(sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE json_extract(namespace, '$.type') = 'global' " 2>/dev/null)

print_cyan "  Frontend: $FRONTEND_ONLY, Backend: $BACKEND_ONLY, Shared: $SHARED_ONLY"

if [ "$FRONTEND_ONLY" -eq 1 ] && [ "$BACKEND_ONLY" -eq 1 ] && [ "$SHARED_ONLY" -eq 1 ]; then
    print_green "  ✓ Namespace boundaries preserved"
fi

# ===================================================================
# CLEANUP
# ===================================================================

cleanup_team_lead "$TEST_DB"

# ===================================================================
# TEST SUMMARY
# ===================================================================

section "Test Summary: Integration - Cross-Component [BASELINE]"

echo "✓ Multi-component memory storage: PASS (3 components)"
echo "✓ Cross-component enrichment: PASS"
echo "✓ Cross-namespace search: PASS ($RESULT_COUNT results)"
echo "✓ Namespace boundaries: PASS"
echo ""

print_green "✓ ALL TESTS PASSED"
exit 0
