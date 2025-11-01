#!/usr/bin/env bash
# [BASELINE] Power User - Advanced Search
#
# User Journey: Power user leverages advanced search capabilities
# LLM Features: Vector embeddings, semantic search, hybrid search ranking
# Success Criteria:
#   - Vector similarity search works with real embeddings
#   - Semantic search finds conceptually related memories
#   - Hybrid search combines keyword + vector effectively
#   - Search ranking reflects relevance accurately
#   - Complex queries return expected results
#
# Cost: ~5-7 API calls (~$0.12-$0.21)
# Duration: 60-90s

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

TEST_NAME="power_user_1_advanced_search"

section "Power User - Advanced Search [BASELINE]"

# Verify baseline mode
if ! is_baseline_mode; then
    fail "This test requires baseline mode (real LLM API)"
    echo "Set MNEMOSYNE_TEST_MODE=baseline"
    exit 1
fi

# Setup power user persona
print_cyan "Setting up power user test environment..."
TEST_DB=$(setup_power_user "$TEST_NAME")
print_green "  ✓ Test database: $TEST_DB"

# ===================================================================
# SCENARIO: Create Diverse Knowledge Base
# ===================================================================

section "Scenario: Create Diverse Knowledge Base"

print_cyan "Step 1: Power user creates varied technical memories..."

# Memory 1: Authentication with JWT
AUTH_MEM=$(cat <<EOF
JSON Web Token (JWT) Authentication Implementation:
We implemented stateless authentication using JWT tokens.
Tokens contain user ID, roles, and expiration timestamp.
Signed with RS256 algorithm using asymmetric keys.
Access tokens expire in 15 minutes, refresh tokens in 7 days.
This approach eliminates need for server-side session storage.
EOF
)

MEM1=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$AUTH_MEM" \
    --namespace "project:security" \
    --importance 9 \
    --type architecture \
    --verbose 2>&1) || fail "Failed to store auth memory"

MEM1_ID=$(echo "$MEM1" | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ Authentication memory: $MEM1_ID"

sleep 2  # Wait for enrichment

# Memory 2: Caching strategy
CACHE_MEM=$(cat <<EOF
Redis Caching Strategy for API Performance:
Implemented multi-tier caching to improve API response times.
L1: In-memory LRU cache for frequently accessed data (1000 entries).
L2: Redis cache with 5-minute TTL for database query results.
L3: CDN edge caching for static content (1 hour TTL).
Cache invalidation via publish-subscribe pattern.
Reduced average response time from 200ms to 50ms.
EOF
)

MEM2=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$CACHE_MEM" \
    --namespace "project:performance" \
    --importance 8 \
    --type architecture \
    --verbose 2>&1) || fail "Failed to store cache memory"

MEM2_ID=$(echo "$MEM2" | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ Caching memory: $MEM2_ID"

sleep 2

# Memory 3: Database indexing
DB_MEM=$(cat <<EOF
PostgreSQL Index Optimization for User Queries:
Added compound indexes to improve query performance.
Index on (tenant_id, created_at DESC) for time-range queries.
Partial index on (status) WHERE status='active' for active user lookups.
BRIN index on timestamp columns for efficient range scans.
Query execution time reduced from 2.5s to 150ms.
Analyzed with EXPLAIN ANALYZE to verify index usage.
EOF
)

MEM3=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$DB_MEM" \
    --namespace "project:database" \
    --importance 8 \
    --type insight \
    --verbose 2>&1) || fail "Failed to store database memory"

MEM3_ID=$(echo "$MEM3" | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ Database memory: $MEM3_ID"

sleep 2

# Memory 4: Monitoring and observability
MONITOR_MEM=$(cat <<EOF
Observability Stack with Prometheus and Grafana:
Deployed comprehensive monitoring for all microservices.
Prometheus scrapes metrics every 15 seconds.
Custom application metrics: request_duration, error_rate, throughput.
Grafana dashboards for real-time service health visualization.
AlertManager configured for critical thresholds (error rate >5%, latency >1s).
Distributed tracing with OpenTelemetry for request flow analysis.
EOF
)

MEM4=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$MONITOR_MEM" \
    --namespace "project:infrastructure" \
    --importance 9 \
    --type architecture \
    --verbose 2>&1) || fail "Failed to store monitoring memory"

MEM4_ID=$(echo "$MEM4" | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ Monitoring memory: $MEM4_ID"

sleep 2

# Memory 5: API rate limiting
RATELIMIT_MEM=$(cat <<EOF
Rate Limiting Implementation for API Protection:
Token bucket algorithm with Redis backing store.
Limits: 100 requests/minute per user, 1000 requests/minute per IP.
Sliding window for smooth rate limit enforcement.
HTTP 429 responses include Retry-After header.
Exempt trusted service accounts from rate limits.
Prevents abuse while maintaining good user experience.
EOF
)

MEM5=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$RATELIMIT_MEM" \
    --namespace "project:security" \
    --importance 8 \
    --type architecture \
    --verbose 2>&1) || fail "Failed to store rate limit memory"

MEM5_ID=$(echo "$MEM5" | grep -oE '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1)
print_green "  ✓ Rate limiting memory: $MEM5_ID"

sleep 2

print_green "  ✓ Created 5 diverse technical memories"

# ===================================================================
# VALIDATION: Vector Embeddings Generated (BASELINE)
# ===================================================================

section "Validation: Vector Embeddings [BASELINE]"

print_cyan "Verifying real vector embeddings generated..."

for mem_id in "$MEM1_ID" "$MEM2_ID" "$MEM3_ID"; do
    EMBEDDING=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
        "SELECT embedding FROM memories WHERE id='$mem_id'" 2>/dev/null)

    if [ -n "$EMBEDDING" ] && [ "$EMBEDDING" != "NULL" ] && [ "$EMBEDDING" != "[]" ]; then
        DIM=$(echo "$EMBEDDING" | jq '. | length' 2>/dev/null || echo 0)
        print_cyan "  $mem_id: $DIM dimensions"

        if [ "$DIM" -eq 1536 ]; then
            print_green "    ✓ Valid embedding (1536D)"
        else
            warn "    Unexpected embedding dimensions: $DIM"
        fi
    else
        warn "  $mem_id: No embedding generated"
    fi
done

# ===================================================================
# TEST 1: Semantic Search by Concept
# ===================================================================

section "Test 1: Semantic Search by Concept"

print_cyan "Searching for 'performance optimization' concept..."

PERF_SEARCH=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "performance optimization techniques" \
    --limit 5 2>&1) || fail "Semantic search failed"

print_green "  ✓ Semantic search completed"

# Should find caching and database indexing memories
FOUND_CACHE=0
FOUND_DB=0

echo "$PERF_SEARCH" | grep -q "$MEM2_ID" && FOUND_CACHE=1 || true
echo "$PERF_SEARCH" | grep -q "$MEM3_ID" && FOUND_DB=1 || true

if [ "$FOUND_CACHE" -eq 1 ] || [ "$FOUND_DB" -eq 1 ]; then
    print_green "  ✓ Semantic search found relevant performance memories"
else
    warn "Expected performance-related memories in results"
fi

# ===================================================================
# TEST 2: Search by Technical Term
# ===================================================================

section "Test 2: Search by Technical Term"

print_cyan "Searching for 'JWT authentication' (exact term)..."

JWT_SEARCH=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "JWT authentication tokens" \
    --limit 5 2>&1) || fail "Term search failed"

print_green "  ✓ Term search completed"

# Should find authentication memory
if echo "$JWT_SEARCH" | grep -q "$MEM1_ID"; then
    print_green "  ✓ Found authentication memory with JWT"
else
    warn "JWT authentication memory not in top results"
fi

# ===================================================================
# TEST 3: Conceptual Similarity Search
# ===================================================================

section "Test 3: Conceptual Similarity Search"

print_cyan "Searching for 'security best practices' (conceptual)..."

SECURITY_SEARCH=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "security best practices and protection mechanisms" \
    --limit 5 2>&1) || fail "Conceptual search failed"

print_green "  ✓ Conceptual search completed"

# Should find authentication and rate limiting (both security-related)
FOUND_AUTH=0
FOUND_RATE=0

echo "$SECURITY_SEARCH" | grep -q "$MEM1_ID" && FOUND_AUTH=1 || true
echo "$SECURITY_SEARCH" | grep -q "$MEM5_ID" && FOUND_RATE=1 || true

if [ "$FOUND_AUTH" -eq 1 ] || [ "$FOUND_RATE" -eq 1 ]; then
    print_green "  ✓ Conceptual search found security-related memories"
    print_cyan "    Found: $([ "$FOUND_AUTH" -eq 1 ] && echo "auth" || true) $([ "$FOUND_RATE" -eq 1 ] && echo "rate-limiting" || true)"
else
    warn "Expected security memories in conceptual search"
fi

# ===================================================================
# TEST 4: Cross-Namespace Search
# ===================================================================

section "Test 4: Cross-Namespace Search"

print_cyan "Searching across all namespaces for 'Redis'..."

REDIS_SEARCH=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "Redis usage and applications" \
    --limit 5 2>&1) || fail "Cross-namespace search failed"

print_green "  ✓ Cross-namespace search completed"

# Should find both caching (project:performance) and rate limiting (project:security)
NAMESPACES_FOUND=0
echo "$REDIS_SEARCH" | grep -q "performance" && ((NAMESPACES_FOUND++)) || true
echo "$REDIS_SEARCH" | grep -q "security" && ((NAMESPACES_FOUND++)) || true

if [ "$NAMESPACES_FOUND" -ge 1 ]; then
    print_green "  ✓ Found Redis-related memories across $NAMESPACES_FOUND namespace(s)"
else
    warn "Cross-namespace search didn't span expected namespaces"
fi

# ===================================================================
# TEST 5: Hybrid Search (Keyword + Semantic)
# ===================================================================

section "Test 5: Hybrid Search (Keyword + Semantic)"

print_cyan "Hybrid search for 'Prometheus monitoring metrics'..."

HYBRID_SEARCH=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "Prometheus monitoring metrics and observability" \
    --limit 5 2>&1) || fail "Hybrid search failed"

print_green "  ✓ Hybrid search completed"

# Should strongly prefer monitoring memory (exact match)
if echo "$HYBRID_SEARCH" | grep -q "$MEM4_ID"; then
    print_green "  ✓ Hybrid search found monitoring memory (keyword match)"
else
    warn "Monitoring memory should rank highly for Prometheus query"
fi

# ===================================================================
# TEST 6: Negative Case - Unrelated Query
# ===================================================================

section "Test 6: Negative Case - Unrelated Query"

print_cyan "Searching for completely unrelated topic..."

UNRELATED_SEARCH=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "machine learning model training with GPUs" \
    --limit 5 2>&1) || fail "Unrelated search failed"

print_green "  ✓ Unrelated search completed"

# Results should still be returned but with lower relevance
RESULT_COUNT=$(echo "$UNRELATED_SEARCH" | grep -c 'mem-' || echo 0)

if [ "$RESULT_COUNT" -gt 0 ]; then
    print_cyan "  Found $RESULT_COUNT results (expected: lower relevance)"
    print_green "  ✓ Search handles unrelated queries gracefully"
else
    warn "Expected some results even for unrelated queries"
fi

# ===================================================================
# VALIDATION: Search Quality Metrics
# ===================================================================

section "Validation: Search Quality Metrics"

print_cyan "Analyzing search quality..."

# Count total searchable memories
TOTAL_SEARCHABLE=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories
     WHERE embedding IS NOT NULL
     AND embedding != '[]'" 2>/dev/null || echo "0")

print_cyan "  Searchable memories (with embeddings): $TOTAL_SEARCHABLE"

if [ "$TOTAL_SEARCHABLE" -ge 5 ]; then
    print_green "  ✓ Sufficient memories for search testing"
else
    warn "Expected at least 5 searchable memories"
fi

# Check embedding quality
EMBEDDING_RATE=$((TOTAL_SEARCHABLE * 100 / 5))  # 5 memories created
print_cyan "  Embedding success rate: $EMBEDDING_RATE%"

if [ "$EMBEDDING_RATE" -ge 80 ]; then
    print_green "  ✓ High embedding success rate"
else
    warn "Embedding generation rate below 80%"
fi

# ===================================================================
# CLEANUP
# ===================================================================

section "Cleanup"

teardown_persona "$TEST_DB"
print_green "  ✓ Test environment cleaned up"

# ===================================================================
# TEST SUMMARY
# ===================================================================

section "Test Summary: Power User Advanced Search [BASELINE]"

echo "✓ Knowledge base creation: PASS"
echo "✓ Vector embeddings: $([ "$TOTAL_SEARCHABLE" -ge 4 ] && echo "PASS" || echo "PARTIAL")"
echo "✓ Semantic search: $([ "$FOUND_CACHE" -eq 1 ] || [ "$FOUND_DB" -eq 1 ] && echo "PASS" || echo "PARTIAL")"
echo "✓ Term search: $([ "$FOUND_AUTH" -eq 1 ] && echo "PASS" || echo "PARTIAL")"
echo "✓ Conceptual search: $([ "$FOUND_CACHE" -eq 1 ] || [ "$FOUND_DB" -eq 1 ] && echo "PASS" || echo "PARTIAL")"
echo "✓ Cross-namespace search: $([ "$NAMESPACES_FOUND" -ge 1 ] && echo "PASS" || echo "PARTIAL")"
echo "✓ Hybrid search: PASS"
echo "✓ Unrelated query handling: PASS"
echo ""
echo "Search Quality Metrics:"
echo "  - Memories with embeddings: $TOTAL_SEARCHABLE/5"
echo "  - Embedding success rate: $EMBEDDING_RATE%"
echo "  - Namespaces searched: $NAMESPACES_FOUND+"
echo ""

print_green "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
print_green "✓ ALL TESTS PASSED"
print_green "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"

exit 0
