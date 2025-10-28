#!/usr/bin/env bash
set -uo pipefail  # Removed -e to allow test failures

# E2E Test: Human Workflow 1 - New Project Setup
#
# Scenario: Developer starting a new project captures initial architecture decisions
# and uses Mnemosyne to store them for future reference.
#
# Steps:
# 1. Store architecture decision (database choice)
# 2. Store architecture decision (API design)
# 3. Store constraint (performance requirement)
# 4. Search for decisions
# 5. List all memories
# 6. Verify enrichment quality

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
BIN="$PROJECT_ROOT/target/release/mnemosyne"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Test state
PASSED=0
FAILED=0
TEST_DB="/tmp/mnemosyne_test_hw1_$(date +%s).db"

echo "========================================"
echo "E2E Test: Human Workflow 1 - New Project Setup"
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
echo "Test 1: Store Architecture Decision (Database)"
echo "========================================"

CONTENT1="We decided to use SQLite for the memory database. Rationale: Embedded database simplifies deployment, ACID guarantees for consistency, FTS5 for full-text search, good performance for <10M memories. Trade-offs: Limited concurrency compared to PostgreSQL, no network access."

echo "Content: $CONTENT1"
echo ""
echo "Running: mnemosyne remember ..."

OUTPUT1=$("$BIN" remember --content "$CONTENT1" --namespace "project:mnemosyne" --importance 8 2>&1)

if echo "$OUTPUT1" | grep -qi "stored successfully\|Stored memory\|Memory saved"; then
    echo -e "${GREEN}[PASS]${NC} Memory stored successfully"
    ((PASSED++))

    # Extract memory ID if possible
    MEMORY_ID1=$(echo "$OUTPUT1" | grep -oE '[a-f0-9-]{36}' | head -1 || echo "")
    echo "Memory ID: $MEMORY_ID1"

    # Check for enrichment
    if echo "$OUTPUT1" | grep -q "Summary:\|Keywords:"; then
        echo -e "${GREEN}[PASS]${NC} LLM enrichment applied"
        ((PASSED++))
    else
        echo -e "${YELLOW}[WARN]${NC} LLM enrichment not visible in output"
    fi
else
    echo -e "${RED}[FAIL]${NC} Memory storage failed"
    echo "Output: $OUTPUT1"
    ((FAILED++))
fi

echo ""
echo "========================================"
echo "Test 2: Store Architecture Decision (API Design)"
echo "========================================"

CONTENT2="API design: REST with JSON for simplicity, versioned endpoints (/v1/), standard HTTP status codes, HAL hypermedia for discoverability. No GraphQL to keep implementation simple. Authentication via API keys stored in OS keychain."

echo "Content: $CONTENT2"
echo ""

OUTPUT2=$("$BIN" remember --content "$CONTENT2" --namespace "project:mnemosyne" --importance 8 2>&1)

if echo "$OUTPUT2" | grep -qi "stored successfully\|Stored memory\|Memory saved"; then
    echo -e "${GREEN}[PASS]${NC} Memory stored successfully"
    ((PASSED++))

    MEMORY_ID2=$(echo "$OUTPUT2" | grep -oE '[a-f0-9-]{36}' | head -1 || echo "")
    echo "Memory ID: $MEMORY_ID2"
else
    echo -e "${RED}[FAIL]${NC} Memory storage failed"
    ((FAILED++))
fi

echo ""
echo "========================================"
echo "Test 3: Store Constraint (Performance)"
echo "========================================"

CONTENT3="Performance constraint: Search must complete in <200ms for typical queries to maintain good UX. This requires FTS5 indexing on content and keywords, importance-weighted ranking, and query optimization."

echo "Content: $CONTENT3"
echo ""

OUTPUT3=$("$BIN" remember --content "$CONTENT3" --namespace "project:mnemosyne" --importance 7 2>&1)

if echo "$OUTPUT3" | grep -qi "stored successfully\|Stored memory\|Memory saved"; then
    echo -e "${GREEN}[PASS]${NC} Memory stored successfully"
    ((PASSED++))
else
    echo -e "${RED}[FAIL]${NC} Memory storage failed"
    ((FAILED++))
fi

echo ""
echo "========================================"
echo "Test 4: Search for Decisions"
echo "========================================"

echo "Query: 'database architecture'"
echo ""

# Allow time for indexing
sleep 1

OUTPUT4=$("$BIN" recall --query "database architecture" --namespace "project:mnemosyne" 2>&1)

if echo "$OUTPUT4" | grep -qi "sqlite\|database"; then
    echo -e "${GREEN}[PASS]${NC} Search returned relevant results"
    ((PASSED++))
    echo "Results preview:"
    echo "$OUTPUT4" | head -20
else
    echo -e "${RED}[FAIL]${NC} Search didn't return expected results"
    echo "Output: $OUTPUT4"
    ((FAILED++))
fi

echo ""
echo "========================================"
echo "Test 5: List All Memories"
echo "========================================"

OUTPUT5=$("$BIN" recall --query "" --namespace "project:mnemosyne" 2>&1)

# Count lines that look like memory entries (heuristic: contain importance or dates)
MEMORY_COUNT=$(echo "$OUTPUT5" | grep -cE '[0-9]{4}-[0-9]{2}-[0-9]{2}|Importance:' || echo "0")

if [ "$MEMORY_COUNT" -ge 3 ]; then
    echo -e "${GREEN}[PASS]${NC} List shows all 3 memories (found $MEMORY_COUNT entries)"
    ((PASSED++))
    echo "$OUTPUT5"
else
    echo -e "${RED}[FAIL]${NC} List should show 3 memories, found $MEMORY_COUNT"
    echo "Output: $OUTPUT5"
    ((FAILED++))
fi

echo ""
echo "========================================"
echo "Test 6: Verify Enrichment Quality"
echo "========================================"

# Get details of first memory to check enrichment
if [ -n "$MEMORY_ID1" ]; then
    # Try to get memory details
    OUTPUT6=$("$BIN" get "$MEMORY_ID1" 2>&1 || echo "get command not available")

    if echo "$OUTPUT6" | grep -q "get command not available\|Unknown command"; then
        echo -e "${YELLOW}[SKIP]${NC} 'get' command not implemented, checking search output instead"

        # Check if search output showed enrichment
        if echo "$OUTPUT4" | grep -qi "summary\|keywords\|tags"; then
            echo -e "${GREEN}[PASS]${NC} Enrichment visible in search results"
            ((PASSED++))
        else
            echo -e "${YELLOW}[WARN]${NC} Enrichment not visible in output"
        fi
    else
        # Check for enrichment fields
        HAS_SUMMARY=$(echo "$OUTPUT6" | grep -c "Summary:" || echo "0")
        HAS_KEYWORDS=$(echo "$OUTPUT6" | grep -c "Keywords:\|Tags:" || echo "0")

        if [ "$HAS_SUMMARY" -gt 0 ] && [ "$HAS_KEYWORDS" -gt 0 ]; then
            echo -e "${GREEN}[PASS]${NC} Memory has summary and keywords"
            ((PASSED++))
        else
            echo -e "${RED}[FAIL]${NC} Memory missing enrichment (summary: $HAS_SUMMARY, keywords: $HAS_KEYWORDS)"
            ((FAILED++))
        fi
    fi
else
    echo -e "${YELLOW}[SKIP]${NC} No memory ID captured, cannot verify enrichment"
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
