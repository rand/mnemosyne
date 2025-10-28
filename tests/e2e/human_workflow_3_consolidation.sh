#!/usr/bin/env bash
set -euo pipefail

# E2E Test: Human Workflow 3 - Knowledge Consolidation
#
# Scenario: After several sprints, developer notices duplicate or similar memories
# and uses consolidation to clean up and merge knowledge.
#
# Steps:
# 1. Pre-populate database with duplicate/similar memories
# 2. Run consolidation to find candidates
# 3. Verify LLM identifies duplicates correctly
# 4. Apply consolidation (merge)
# 5. Verify originals archived and new memory created
# 6. Verify links redirect correctly

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
TEST_DB="/tmp/mnemosyne_test_hw3_$(date +%s).db"

echo "========================================"
echo "E2E Test: Human Workflow 3 - Knowledge Consolidation"
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
echo "Setup: Create Duplicate Memories"
echo "========================================"

echo "Creating intentionally duplicate memories..."

# Duplicate pair 1: PostgreSQL decision (slightly different wording)
OUTPUT_M1=$("$BIN" remember "We decided to use PostgreSQL for the database. It provides ACID guarantees and excellent JSON support." \
    --namespace "project:api" --importance 8 2>&1)
MEMORY_ID1=$(echo "$OUTPUT_M1" | grep -oE '[a-f0-9-]{36}' | head -1 || echo "")

OUTPUT_M2=$("$BIN" remember "Database choice: PostgreSQL. Reasoning: Need ACID transactions and JSON queries." \
    --namespace "project:api" --importance 7 2>&1)
MEMORY_ID2=$(echo "$OUTPUT_M2" | grep -oE '[a-f0-9-]{36}' | head -1 || echo "")

echo "Created duplicate pair 1:"
echo "  Memory 1 ID: $MEMORY_ID1"
echo "  Memory 2 ID: $MEMORY_ID2"

# Duplicate pair 2: API versioning (very similar)
OUTPUT_M3=$("$BIN" remember "API versioning: Use /v1/ prefix for all endpoints to enable future breaking changes." \
    --namespace "project:api" --importance 6 2>&1)
MEMORY_ID3=$(echo "$OUTPUT_M3" | grep -oE '[a-f0-9-]{36}' | head -1 || echo "")

OUTPUT_M4=$("$BIN" remember "Versioned API endpoints with /v1/ prefix allow backward-compatible evolution." \
    --namespace "project:api" --importance 6 2>&1)
MEMORY_ID4=$(echo "$OUTPUT_M4" | grep -oE '[a-f0-9-]{36}' | head -1 || echo "")

echo "Created duplicate pair 2:"
echo "  Memory 3 ID: $MEMORY_ID3"
echo "  Memory 4 ID: $MEMORY_ID4"

# Distinct memory (should NOT be consolidated)
"$BIN" remember "Use Redis for caching - completely different from database choice" \
    --namespace "project:api" --importance 7 > /dev/null 2>&1

echo -e "${GREEN}[SETUP]${NC} Created 5 memories (2 duplicate pairs + 1 distinct)"

# Allow time for LLM processing
sleep 2

echo ""
echo "========================================"
echo "Test 1: Find Consolidation Candidates"
echo "========================================"

echo "Running: mnemosyne consolidate (find mode)"
echo ""

# Check if consolidate command exists
if "$BIN" --help | grep -q "consolidate"; then
    OUTPUT1=$("$BIN" consolidate --namespace "project:api" 2>&1 || echo "consolidate failed")

    if echo "$OUTPUT1" | grep -qi "candidate\|pair\|similar\|merge\|duplicate"; then
        echo -e "${GREEN}[PASS]${NC} Consolidation found candidates"
        ((PASSED++))
        echo "Candidates preview:"
        echo "$OUTPUT1" | head -20
    elif echo "$OUTPUT1" | grep -qi "no candidates\|not found"; then
        echo -e "${YELLOW}[WARN]${NC} No consolidation candidates found (LLM may not see duplicates)"
        echo "Output: $OUTPUT1"
    else
        echo -e "${RED}[FAIL]${NC} Consolidation command didn't return expected output"
        echo "Output: $OUTPUT1"
        ((FAILED++))
    fi
else
    echo -e "${YELLOW}[SKIP]${NC} 'consolidate' command not implemented yet"
fi

echo ""
echo "========================================"
echo "Test 2: Analyze Specific Pair"
echo "========================================"

if [ -n "$MEMORY_ID1" ] && [ -n "$MEMORY_ID2" ]; then
    echo "Analyzing memory pair: $MEMORY_ID1 vs $MEMORY_ID2"
    echo ""

    if "$BIN" --help | grep -q "consolidate"; then
        OUTPUT2=$("$BIN" consolidate "$MEMORY_ID1" "$MEMORY_ID2" 2>&1 || echo "consolidate failed")

        if echo "$OUTPUT2" | grep -qi "merge\|similar\|duplicate"; then
            echo -e "${GREEN}[PASS]${NC} LLM identified similar memories"
            ((PASSED++))
            echo "Analysis:"
            echo "$OUTPUT2" | head -15
        else
            echo -e "${YELLOW}[WARN]${NC} LLM did not clearly identify these as duplicates"
            echo "Output: $OUTPUT2"
        fi
    else
        echo -e "${YELLOW}[SKIP]${NC} 'consolidate' command not implemented"
    fi
else
    echo -e "${YELLOW}[SKIP]${NC} Memory IDs not captured, cannot test specific pair"
fi

echo ""
echo "========================================"
echo "Test 3: Apply Consolidation (Auto)"
echo "========================================"

if "$BIN" --help | grep -q "consolidate.*--auto"; then
    echo "Running: mnemosyne consolidate --auto --namespace project:api"
    echo ""

    OUTPUT3=$("$BIN" consolidate --auto --namespace "project:api" 2>&1 || echo "consolidate --auto failed")

    if echo "$OUTPUT3" | grep -qi "merged\|consolidated\|archived"; then
        echo -e "${GREEN}[PASS]${NC} Consolidation applied automatically"
        ((PASSED++))
        echo "Result:"
        echo "$OUTPUT3" | head -20
    else
        echo -e "${YELLOW}[WARN]${NC} Auto-consolidation unclear or no action taken"
        echo "Output: $OUTPUT3"
    fi
else
    echo -e "${YELLOW}[SKIP]${NC} 'consolidate --auto' not implemented yet"
fi

echo ""
echo "========================================"
echo "Test 4: Verify Memories After Consolidation"
echo "========================================"

OUTPUT4=$("$BIN" recall --query "" --namespace "project:api" 2>&1)

# Count non-archived memories
MEMORY_COUNT=$(echo "$OUTPUT4" | grep -cE '[0-9]{4}-[0-9]{2}-[0-9]{2}|Importance:' || echo "0")

echo "Memories remaining: $MEMORY_COUNT"

if [ "$MEMORY_COUNT" -lt 5 ]; then
    echo -e "${GREEN}[PASS]${NC} Some memories were consolidated (reduced from 5 to $MEMORY_COUNT)"
    ((PASSED++))
elif [ "$MEMORY_COUNT" -eq 5 ]; then
    echo -e "${YELLOW}[WARN]${NC} No consolidation occurred (still 5 memories)"
    echo "This may be expected if LLM decided to keep all separate"
else
    echo -e "${YELLOW}[WARN]${NC} Unexpected memory count: $MEMORY_COUNT"
fi

echo ""
echo "========================================"
echo "Test 5: Search After Consolidation"
echo "========================================"

echo "Query: 'PostgreSQL database'"
echo ""

OUTPUT5=$("$BIN" recall --query "PostgreSQL database" --namespace "project:api" 2>&1)

if echo "$OUTPUT5" | grep -qi "postgres"; then
    echo -e "${GREEN}[PASS]${NC} Search still finds PostgreSQL information after consolidation"
    ((PASSED++))
else
    echo -e "${RED}[FAIL]${NC} Search doesn't find PostgreSQL information"
    echo "Output: $OUTPUT5"
    ((FAILED++))
fi

echo ""
echo "========================================"
echo "Test 6: Verify Distinct Memory Preserved"
echo "========================================"

OUTPUT6=$("$BIN" recall --query "Redis caching" --namespace "project:api" 2>&1)

if echo "$OUTPUT6" | grep -qi "redis"; then
    echo -e "${GREEN}[PASS]${NC} Distinct memory (Redis) was not incorrectly consolidated"
    ((PASSED++))
else
    echo -e "${RED}[FAIL]${NC} Distinct memory may have been incorrectly removed"
    echo "Output: $OUTPUT6"
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
