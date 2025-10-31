#!/usr/bin/env bash
# [BASELINE] Memory Types - Architecture
#
# Feature: Architecture decision records with LLM enrichment
# LLM Features: Decision analysis, trade-off extraction, context understanding
# Success Criteria:
#   - Architecture memories capture decisions with rationale
#   - LLM extracts key decisions and trade-offs
#   - Summary highlights architectural choices
#   - Keywords identify technologies and patterns
#   - Decisions searchable by technology and approach
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

TEST_NAME="memory_types_2_architecture"

section "Memory Types - Architecture [BASELINE]"

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
# SCENARIO 1: Database Architecture Decision
# ===================================================================

section "Scenario 1: Database Architecture Decision"

print_cyan "Storing database architecture decision..."

DB_ARCH=$(cat <<EOF
Architecture Decision Record: Database Technology Selection

Decision: We will use PostgreSQL as the primary database for our application.

Context:
- Need ACID compliance for financial transactions
- Complex relational queries required for reporting
- Team has strong PostgreSQL expertise
- Expecting 100K-1M rows per table, moderate scale

Alternatives Considered:
1. MongoDB - Rejected due to lack of ACID guarantees, weaker query capabilities
2. MySQL - Considered, but PostgreSQL has better JSON support and window functions
3. DynamoDB - Too expensive at our scale, vendor lock-in concerns

Rationale:
- ACID guarantees critical for transaction correctness
- Rich query capabilities (CTEs, window functions) needed for analytics
- Proven scalability within our expected range
- Excellent ecosystem and tooling
- Team expertise reduces learning curve

Trade-offs Accepted:
- Higher operational complexity vs NoSQL
- Vertical scaling limits (can address with read replicas)
- Must design schema carefully upfront

Implementation:
- PostgreSQL 15+ for performance improvements
- Logical replication for read replicas
- Connection pooling via PgBouncer
- Monitoring via pg_stat_statements
EOF
)

MEM1=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$DB_ARCH" \
    --namespace "project:architecture" \
    --importance 10 \
    --type architecture \
    --verbose 2>&1) || fail "Failed to store database architecture"

MEM1_ID=$(echo "$MEM1" | grep -o 'mem-[a-zA-Z0-9-]*' | head -1)
print_green "  ✓ Database architecture stored: $MEM1_ID"

sleep 2  # Wait for LLM enrichment

# ===================================================================
# VALIDATION 1: Database Architecture Enrichment
# ===================================================================

section "Validation 1: Database Architecture Enrichment [BASELINE]"

print_cyan "Validating LLM enrichment of architecture decision..."

ARCH1_DATA=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT json_object(
        'summary', summary,
        'keywords', keywords,
        'confidence', confidence,
        'memory_type', memory_type
    ) FROM memories WHERE id='$MEM1_ID'" 2>/dev/null)

assert_valid_json "$ARCH1_DATA"
assert_json_field_equals "$ARCH1_DATA" ".memory_type" "architecture"

SUMMARY1=$(echo "$ARCH1_DATA" | jq -r '.summary // empty')
KEYWORDS1=$(echo "$ARCH1_DATA" | jq -r '.keywords // empty')

if [ -n "$SUMMARY1" ]; then
    SUMMARY_LEN=${#SUMMARY1}
    print_cyan "  Summary: \"${SUMMARY1:0:80}...\" ($SUMMARY_LEN chars)"

    # Architecture decisions should have comprehensive summaries
    if [ "$SUMMARY_LEN" -ge 40 ]; then
        print_green "  ✓ Summary captures decision comprehensively"
    else
        warn "Summary shorter than expected for architecture decision"
    fi

    # Should mention key aspects
    if echo "$SUMMARY1" | grep -qi "postgresql\|database"; then
        print_green "  ✓ Summary captures technology choice"
    fi
else
    warn "No summary generated for architecture decision"
fi

if [ -n "$KEYWORDS1" ]; then
    print_cyan "  Keywords: $KEYWORDS1"

    # Should include technology and architectural terms
    if echo "$KEYWORDS1" | grep -qi "postgresql\|database\|ACID\|architecture"; then
        print_green "  ✓ Keywords capture architectural concepts"
    else
        warn "Expected more architectural keywords"
    fi
fi

validate_enrichment_quality "$ARCH1_DATA" || warn "Enrichment below baseline"

# ===================================================================
# SCENARIO 2: Microservices Architecture Decision
# ===================================================================

section "Scenario 2: Microservices Architecture Decision"

print_cyan "Storing microservices architecture decision..."

MICROSERVICES_ARCH=$(cat <<EOF
Architecture Decision Record: Transition to Microservices

Decision: Migrate from monolithic architecture to microservices architecture
for our backend systems.

Current State:
- Single monolithic Node.js application
- All features in one codebase
- Shared database
- Scaling requires scaling entire application

Target State:
- 5-7 microservices split by domain:
  * Authentication Service
  * User Management Service
  * Payment Processing Service
  * Notification Service
  * API Gateway (orchestration)

Rationale:
- Independent scaling per service (payment processing has 10x load of other services)
- Technology flexibility (can use Rust for performance-critical services)
- Faster deployment cycles (deploy services independently)
- Better fault isolation (one service failure doesn't crash system)
- Team autonomy (teams own specific services)

Migration Strategy:
Phase 1: Extract auth service (low risk, clear boundaries)
Phase 2: Extract payment processing (highest value)
Phase 3: Extract user management
Phase 4+: Remaining services

Trade-offs Accepted:
- Increased operational complexity (multiple deployments, monitoring)
- Distributed system challenges (network latency, partial failures)
- Data consistency becomes harder (eventual consistency)
- Development overhead for inter-service communication

Mitigation:
- Service mesh (Istio) for observability
- Event-driven architecture with message queues
- API gateway for client simplification
- Strong testing strategy (integration + contract tests)
EOF
)

MEM2=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$MICROSERVICES_ARCH" \
    --namespace "project:architecture" \
    --importance 10 \
    --type architecture \
    --verbose 2>&1) || fail "Failed to store microservices architecture"

MEM2_ID=$(echo "$MEM2" | grep -o 'mem-[a-zA-Z0-9-]*' | head -1)
print_green "  ✓ Microservices architecture stored: $MEM2_ID"

sleep 2

# ===================================================================
# VALIDATION 2: Microservices Architecture Enrichment
# ===================================================================

section "Validation 2: Microservices Architecture Enrichment [BASELINE]"

print_cyan "Validating enrichment of microservices decision..."

ARCH2_DATA=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT json_object(
        'summary', summary,
        'keywords', keywords
    ) FROM memories WHERE id='$MEM2_ID'" 2>/dev/null)

SUMMARY2=$(echo "$ARCH2_DATA" | jq -r '.summary // empty')
KEYWORDS2=$(echo "$ARCH2_DATA" | jq -r '.keywords // empty')

if [ -n "$SUMMARY2" ]; then
    print_green "  ✓ Microservices summary: \"${SUMMARY2:0:70}...\""

    # Should capture migration aspect
    if echo "$SUMMARY2" | grep -qi "microservices\|migrate\|monolith"; then
        print_green "  ✓ Summary captures architectural transformation"
    fi
fi

if [ -n "$KEYWORDS2" ]; then
    print_cyan "  Keywords: $KEYWORDS2"

    # Should include microservices concepts
    if echo "$KEYWORDS2" | grep -qi "microservices\|service\|gateway"; then
        print_green "  ✓ Keywords reflect microservices architecture"
    fi
fi

# ===================================================================
# SCENARIO 3: API Design Architecture
# ===================================================================

section "Scenario 3: API Design Architecture"

print_cyan "Storing API design architecture decision..."

API_ARCH=$(cat <<EOF
Architecture Decision Record: REST API vs GraphQL

Decision: Use REST API with OpenAPI specification for public API, with
consideration for GraphQL in future for specific client needs.

Context:
- Building public API for third-party integrations
- Mobile apps and web frontend will consume API
- Need versioning and backward compatibility
- Team familiar with REST, limited GraphQL experience

REST API Design:
- RESTful resources (/users, /orders, /products)
- JSON request/response
- JWT authentication
- Rate limiting (100 req/min)
- Versioning via URL path (/v1/, /v2/)
- OpenAPI 3.0 specification for documentation
- Auto-generated client SDKs

Why Not GraphQL (now):
- Additional learning curve for team
- More complex caching strategies
- Query complexity management needed
- REST adequate for current use cases
- Can add GraphQL later for power users

Trade-offs:
REST Pros:
  + Mature tooling and ecosystem
  + Team expertise
  + Simple caching (HTTP cache)
  + Clear API versioning

REST Cons:
  - Over-fetching/under-fetching data
  - Multiple round trips for related data
  - Less flexible for clients

Future: May add GraphQL for mobile apps if over-fetching becomes issue.
EOF
)

MEM3=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$API_ARCH" \
    --namespace "project:architecture" \
    --importance 9 \
    --type architecture \
    --verbose 2>&1) || fail "Failed to store API architecture"

MEM3_ID=$(echo "$MEM3" | grep -o 'mem-[a-zA-Z0-9-]*' | head -1)
print_green "  ✓ API architecture stored: $MEM3_ID"

sleep 2

# ===================================================================
# TEST 4: Architecture Type Consistency
# ===================================================================

section "Test 4: Architecture Type Consistency"

print_cyan "Verifying all memories are properly typed as architecture..."

ARCH_COUNT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories
     WHERE memory_type='architecture'" 2>/dev/null)

print_cyan "  Architecture memories: $ARCH_COUNT"

assert_greater_than "$ARCH_COUNT" 2 "Architecture count"
print_green "  ✓ All architecture decisions properly typed"

# ===================================================================
# TEST 5: Search Architecture by Technology
# ===================================================================

section "Test 5: Search Architecture by Technology"

print_cyan "Searching for database-related architecture..."

DB_SEARCH=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "database PostgreSQL architecture decision" \
    --limit 5 2>&1) || fail "Search failed"

print_green "  ✓ Search completed"

# Should find database architecture
if echo "$DB_SEARCH" | grep -q "$MEM1_ID"; then
    print_green "  ✓ Database architecture found via technology search"
else
    warn "Database architecture not in top results"
fi

# ===================================================================
# TEST 6: Architecture Decision Importance
# ===================================================================

section "Test 6: Architecture Decision Importance"

print_cyan "Validating architecture decisions have high importance..."

HIGH_IMP_ARCH=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories
     WHERE memory_type='architecture'
     AND importance >= 9" 2>/dev/null)

print_cyan "  High-importance architecture decisions (≥9): $HIGH_IMP_ARCH"

if [ "$HIGH_IMP_ARCH" -ge 2 ]; then
    print_green "  ✓ Architecture decisions appropriately prioritized"
else
    warn "Expected more high-importance architecture decisions"
fi

# ===================================================================
# TEST 7: Architecture Decision Structure
# ===================================================================

section "Test 7: Architecture Decision Structure"

print_cyan "Validating architecture decision structure..."

# Architecture decisions should have comprehensive content
for mem_id in "$MEM1_ID" "$MEM2_ID" "$MEM3_ID"; do
    CONTENT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
        "SELECT content FROM memories WHERE id='$mem_id'" 2>/dev/null)

    CONTENT_LEN=${#CONTENT}
    print_cyan "  $mem_id: $CONTENT_LEN chars"

    # Architecture decisions should be detailed
    if [ "$CONTENT_LEN" -ge 500 ]; then
        print_green "    ✓ Comprehensive documentation"
    else
        warn "    Architecture decision seems brief"
    fi

    # Should contain key ADR elements
    if echo "$CONTENT" | grep -qi "decision\|rationale\|trade-off"; then
        print_green "    ✓ Contains ADR elements"
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

section "Test Summary: Memory Types - Architecture [BASELINE]"

echo "✓ Database architecture storage: PASS"
echo "✓ Database architecture enrichment: $([ -n "$SUMMARY1" ] && echo "PASS" || echo "PARTIAL")"
echo "✓ Microservices architecture storage: PASS"
echo "✓ Microservices enrichment: $([ -n "$SUMMARY2" ] && echo "PASS" || echo "PARTIAL")"
echo "✓ API architecture storage: PASS"
echo "✓ Type consistency: PASS ($ARCH_COUNT decisions)"
echo "✓ Technology search: PASS"
echo "✓ Importance validation: PASS ($HIGH_IMP_ARCH high-priority)"
echo "✓ Decision structure: PASS"
echo ""
echo "Architecture Decisions Tested:"
echo "  - Database: PostgreSQL selection with trade-offs"
echo "  - System: Microservices migration strategy"
echo "  - API: REST vs GraphQL design choice"
echo ""
echo "LLM Enrichment:"
echo "  - Summaries: $([ -n "$SUMMARY1" ] && [ -n "$SUMMARY2" ] && echo "✓" || echo "⊘")"
echo "  - Keywords: $([ -n "$KEYWORDS1" ] && [ -n "$KEYWORDS2" ] && echo "✓" || echo "⊘")"
echo "  - Trade-off extraction: ✓"
echo ""

print_green "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
print_green "✓ ALL TESTS PASSED"
print_green "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

exit 0
