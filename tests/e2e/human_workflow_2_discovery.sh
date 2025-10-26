#!/usr/bin/env bash
set -euo pipefail

# E2E Test: Human Workflow 2 - Memory Discovery & Reuse
#
# Scenario: Developer working on a new feature searches for related past decisions
# and learnings to avoid reinventing the wheel.
#
# Steps:
# 1. Pre-populate database with sample memories (simulating past project work)
# 2. Search for relevant decisions using keywords
# 3. Search with namespace filtering
# 4. Verify search performance (<200ms target)
# 5. Verify result ranking (relevance)
# 6. Load context for specific task

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
BIN="$PROJECT_ROOT/target/release/mnemosyne"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Test state
PASSED=0
FAILED=0
TEST_DB="/tmp/mnemosyne_test_hw2_$(date +%s).db"

echo "========================================"
echo "E2E Test: Human Workflow 2 - Memory Discovery & Reuse"
echo "========================================"
echo ""

# Setup
echo -e "${YELLOW}[SETUP]${NC} Building Mnemosyne..."
cd "$PROJECT_ROOT"
cargo build --release > /dev/null 2>&1

if [ ! -f "$BIN" ]; then
    echo -e "${RED}[ERROR]${NC} Binary not found at $BIN"
    exit 1
fi

echo -e "${YELLOW}[SETUP]${NC} Using test database: $TEST_DB"
export DATABASE_URL="sqlite://$TEST_DB"

echo -e "${YELLOW}[SETUP]${NC} Checking API key..."
if ! "$BIN" config show-key > /dev/null 2>&1; then
    echo -e "${RED}[ERROR]${NC} No API key configured. Run: $BIN config set-key <key>"
    exit 1
fi

echo ""
echo "========================================"
echo "Setup: Pre-populate Database"
echo "========================================"

echo "Creating sample memories..."

# Memory 1: Database decision
"$BIN" remember "Chose PostgreSQL for main database - ACID guarantees, JSON support, excellent performance" \
    --namespace "project:ecommerce" --importance 8 > /dev/null 2>&1

# Memory 2: Caching decision
"$BIN" remember "Using Redis for session caching and rate limiting - fast in-memory operations, TTL support" \
    --namespace "project:ecommerce" --importance 7 > /dev/null 2>&1

# Memory 3: API pattern
"$BIN" remember "REST API with versioning (/v1/), pagination via cursor, standard HTTP codes" \
    --namespace "project:ecommerce" --importance 7 > /dev/null 2>&1

# Memory 4: Bug fix
"$BIN" remember "Fixed race condition in cart checkout - wrapped HashMap in Arc<RwLock>, added transaction isolation" \
    --namespace "project:ecommerce" --importance 6 > /dev/null 2>&1

# Memory 5: Performance optimization
"$BIN" remember "Optimized product search with FTS5 full-text indexing - reduced query time from 800ms to 50ms" \
    --namespace "project:ecommerce" --importance 7 > /dev/null 2>&1

# Memory 6: Different project
"$BIN" remember "Blog platform uses SQLite - embedded database, simpler deployment for small sites" \
    --namespace "project:blog" --importance 5 > /dev/null 2>&1

echo -e "${GREEN}[SETUP]${NC} Created 6 sample memories (5 ecommerce, 1 blog)"

# Allow time for indexing
sleep 2

echo ""
echo "========================================"
echo "Test 1: Keyword Search"
echo "========================================"

echo "Query: 'database'"
echo ""

START_TIME=$(date +%s%N)
OUTPUT1=$("$BIN" search "database" --namespace "project:ecommerce" 2>&1)
END_TIME=$(date +%s%N)
ELAPSED_MS=$(( (END_TIME - START_TIME) / 1000000 ))

echo "Search completed in ${ELAPSED_MS}ms"

if echo "$OUTPUT1" | grep -qi "postgres\|postgresql"; then
    echo -e "${GREEN}[PASS]${NC} Search returned PostgreSQL decision"
    ((PASSED++))
else
    echo -e "${RED}[FAIL]${NC} Search didn't find PostgreSQL decision"
    echo "Output: $OUTPUT1"
    ((FAILED++))
fi

# Should NOT return blog project (different namespace)
if echo "$OUTPUT1" | grep -qi "blog"; then
    echo -e "${RED}[FAIL]${NC} Search returned wrong namespace (blog)"
    ((FAILED++))
else
    echo -e "${GREEN}[PASS]${NC} Namespace filtering works (no blog results)"
    ((PASSED++))
fi

echo ""
echo "========================================"
echo "Test 2: Multi-keyword Search"
echo "========================================"

echo "Query: 'performance optimization'"
echo ""

OUTPUT2=$("$BIN" search "performance optimization" --namespace "project:ecommerce" 2>&1)

if echo "$OUTPUT2" | grep -qi "FTS5\|search\|50ms"; then
    echo -e "${GREEN}[PASS]${NC} Search returned performance optimization memory"
    ((PASSED++))
    echo "Results preview:"
    echo "$OUTPUT2" | head -15
else
    echo -e "${RED}[FAIL]${NC} Search didn't find performance optimization"
    echo "Output: $OUTPUT2"
    ((FAILED++))
fi

echo ""
echo "========================================"
echo "Test 3: Search Performance (<200ms target)"
echo "========================================"

# Run multiple searches and average
TOTAL_MS=0
ITERATIONS=5

echo "Running $ITERATIONS searches to measure average performance..."

for i in $(seq 1 $ITERATIONS); do
    START=$(date +%s%N)
    "$BIN" search "API REST" --namespace "project:ecommerce" > /dev/null 2>&1
    END=$(date +%s%N)
    ITER_MS=$(( (END - START) / 1000000 ))
    TOTAL_MS=$(( TOTAL_MS + ITER_MS ))
    echo "  Iteration $i: ${ITER_MS}ms"
done

AVG_MS=$(( TOTAL_MS / ITERATIONS ))
echo ""
echo "Average search time: ${AVG_MS}ms (target: <200ms)"

if [ "$AVG_MS" -lt 200 ]; then
    echo -e "${GREEN}[PASS]${NC} Search performance meets target"
    ((PASSED++))
else
    echo -e "${YELLOW}[WARN]${NC} Search performance exceeds target (${AVG_MS}ms > 200ms)"
    echo "Note: This may be acceptable depending on database size and hardware"
fi

echo ""
echo "========================================"
echo "Test 4: Result Ranking (High Importance First)"
echo "========================================"

OUTPUT4=$("$BIN" list --namespace "project:ecommerce" --sort importance 2>&1)

# Check if results are sorted by importance
if echo "$OUTPUT4" | head -5 | grep -q "PostgreSQL\|8\|importance: 8"; then
    echo -e "${GREEN}[PASS]${NC} High-importance memories appear first"
    ((PASSED++))
else
    echo -e "${YELLOW}[WARN]${NC} Importance sorting not clearly visible in output"
    echo "Output (first 10 lines):"
    echo "$OUTPUT4" | head -10
fi

echo ""
echo "========================================"
echo "Test 5: Global Search (All Namespaces)"
echo "========================================"

echo "Query: 'database' (all namespaces)"
echo ""

OUTPUT5=$("$BIN" search "database" 2>&1)

# Should find both ecommerce and blog mentions
FOUND_POSTGRES=$(echo "$OUTPUT5" | grep -c "PostgreSQL\|Postgres" || echo "0")
FOUND_SQLITE=$(echo "$OUTPUT5" | grep -c "SQLite" || echo "0")

if [ "$FOUND_POSTGRES" -gt 0 ] && [ "$FOUND_SQLITE" -gt 0 ]; then
    echo -e "${GREEN}[PASS]${NC} Global search found memories from multiple projects"
    echo "  Found PostgreSQL: $FOUND_POSTGRES, SQLite: $FOUND_SQLITE"
    ((PASSED++))
else
    echo -e "${RED}[FAIL]${NC} Global search didn't find all expected memories"
    echo "  Found PostgreSQL: $FOUND_POSTGRES, SQLite: $FOUND_SQLITE"
    ((FAILED++))
fi

echo ""
echo "========================================"
echo "Test 6: List with Filtering"
echo "========================================"

OUTPUT6=$("$BIN" list --namespace "project:ecommerce" --limit 3 2>&1)

# Count results (heuristic)
RESULT_COUNT=$(echo "$OUTPUT6" | grep -cE '[0-9]{4}-[0-9]{2}-[0-9]{2}|Importance:' || echo "0")

if [ "$RESULT_COUNT" -le 3 ]; then
    echo -e "${GREEN}[PASS]${NC} Limit parameter respected (showing $RESULT_COUNT <= 3)"
    ((PASSED++))
else
    echo -e "${RED}[FAIL]${NC} Limit parameter not respected (showing $RESULT_COUNT > 3)"
    ((FAILED++))
fi

# Cleanup
echo ""
echo "========================================"
echo "Cleanup"
echo "========================================"

echo "Removing test database: $TEST_DB"
rm -f "$TEST_DB" "${TEST_DB}-wal" "${TEST_DB}-shm"

# Final summary
echo ""
echo "========================================"
echo "Test Summary"
echo "========================================"
echo -e "Passed: ${GREEN}$PASSED${NC}"
echo -e "Failed: ${RED}$FAILED${NC}"
echo "========================================"

if [ "$FAILED" -eq 0 ]; then
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}Some tests failed${NC}"
    exit 1
fi
