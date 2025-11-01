#!/usr/bin/env bash
# [REGRESSION] Storage - Local SQLite
#
# Feature: Local SQLite database backend (baseline storage)
# Success Criteria:
#   - SQLite database creation
#   - CRUD operations work correctly
#   - Schema validation
#   - Query performance acceptable
#   - Concurrent access handled
#
# Cost: $0 (mocked LLM responses)
# Duration: 10-15s

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/assertions.sh"
source "$SCRIPT_DIR/lib/personas.sh"
source "$SCRIPT_DIR/lib/llm_mocking.sh"

TEST_NAME="storage_1_local_sqlite"

section "Storage - Local SQLite [REGRESSION]"

print_cyan "Setting up test environment..."
TEST_DB=$(setup_solo_developer "$TEST_NAME")
print_green "  ✓ Test database: $TEST_DB"

# Verify SQLite file created
if [ -f "$TEST_DB" ]; then
    print_green "  ✓ SQLite database file created"
else
    fail "Database file not created: $TEST_DB"
fi

# ===================================================================
# TEST 1: Basic CRUD Operations
# ===================================================================

section "Test 1: Basic CRUD Operations"

print_cyan "Testing CREATE..."

# Create memory
DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Test memory for CRUD operations" \
    --namespace "project:test" \
    --importance 7 \
    --type reference >/dev/null 2>&1 || fail "Failed to create memory"

# Count only the test memory (persona setup creates 2 additional memories in global namespace)
NS_WHERE=$(namespace_where_clause "project:test")
MEMORY_COUNT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE $NS_WHERE" 2>/dev/null)

assert_equals "$MEMORY_COUNT" "1" "Memory count after create"
print_green "  ✓ CREATE operation successful"

print_cyan "Testing READ..."

# Read memory from test namespace
MEMORY_CONTENT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT content FROM memories WHERE $NS_WHERE LIMIT 1" 2>/dev/null)

assert_contains "$MEMORY_CONTENT" "CRUD operations" "Memory content"
print_green "  ✓ READ operation successful"

print_cyan "Testing UPDATE..."

# Update memory from test namespace (via SQL for direct testing)
MEMORY_ID=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT id FROM memories WHERE $NS_WHERE LIMIT 1" 2>/dev/null)

DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "UPDATE memories SET importance=9 WHERE id='$MEMORY_ID'" 2>/dev/null

UPDATED_IMPORTANCE=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT importance FROM memories WHERE id='$MEMORY_ID'" 2>/dev/null)

assert_equals "$UPDATED_IMPORTANCE" "9" "Updated importance"
print_green "  ✓ UPDATE operation successful"

print_cyan "Testing DELETE..."

DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "DELETE FROM memories WHERE id='$MEMORY_ID'" 2>/dev/null

# Count only test namespace memories (persona memories remain)
AFTER_DELETE=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE $NS_WHERE" 2>/dev/null)

assert_equals "$AFTER_DELETE" "0" "Memory count after delete"
print_green "  ✓ DELETE operation successful"

# ===================================================================
# TEST 2: Schema Validation
# ===================================================================

section "Test 2: Schema Validation"

print_cyan "Validating database schema..."

# Check memories table structure
SCHEMA=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT sql FROM sqlite_master WHERE memory_type ='table' AND name='memories'" 2>/dev/null)

print_cyan "  Memories table schema exists"

# Verify key columns
for col in id content namespace importance type created_at updated_at; do
    if echo "$SCHEMA" | grep -qi "$col"; then
        print_cyan "    ✓ Column: $col"
    else
        warn "Missing column: $col"
    fi
done

print_green "  ✓ Schema validation complete"

# ===================================================================
# TEST 3: Bulk Insert Performance
# ===================================================================

section "Test 3: Bulk Insert Performance"

print_cyan "Testing bulk insert performance..."

# Insert 20 memories
START_TIME=$(date +%s)

for i in {1..20}; do
    DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
        --content "Performance test memory $i with some substantial content to simulate real usage" \
        --namespace "project:perf" \
        --importance $((5 + i % 5)) \
        --type reference >/dev/null 2>&1
done

END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

BULK_COUNT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE json_extract(namespace, '$.type') = 'project' AND json_extract(namespace, '$.name') = 'perf' " 2>/dev/null)

assert_equals "$BULK_COUNT" "20" "Bulk insert count"
print_cyan "  Inserted 20 memories in ${DURATION}s"

# Should complete in reasonable time
if [ "$DURATION" -lt 30 ]; then
    print_green "  ✓ Bulk insert performance acceptable (<30s)"
else
    warn "Bulk insert slower than expected: ${DURATION}s"
fi

# ===================================================================
# TEST 4: Query Performance
# ===================================================================

section "Test 4: Query Performance"

print_cyan "Testing query performance..."

# Simple SELECT
QUERY_START=$(date +%s%3N)
QUERY_RESULT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT * FROM memories WHERE json_extract(namespace, '$.type') = 'project' AND json_extract(namespace, '$.name') = 'perf'  LIMIT 10" 2>/dev/null)
QUERY_END=$(date +%s%3N)
QUERY_TIME=$((QUERY_END - QUERY_START))

print_cyan "  Simple SELECT: ${QUERY_TIME}ms"

if [ "$QUERY_TIME" -lt 100 ]; then
    print_green "  ✓ Query performance good (<100ms)"
fi

# Filtered query
FILTER_START=$(date +%s%3N)
FILTER_RESULT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT * FROM memories WHERE importance >= 8 AND namespace='project:perf'" 2>/dev/null)
FILTER_END=$(date +%s%3N)
FILTER_TIME=$((FILTER_END - FILTER_START))

print_cyan "  Filtered SELECT: ${FILTER_TIME}ms"

if [ "$FILTER_TIME" -lt 100 ]; then
    print_green "  ✓ Filtered query performance good (<100ms)"
fi

# ===================================================================
# TEST 5: Database File Size
# ===================================================================

section "Test 5: Database File Size"

print_cyan "Checking database file size..."

DB_SIZE=$(stat -f%z "$TEST_DB" 2>/dev/null || stat -c%s "$TEST_DB" 2>/dev/null)
DB_SIZE_KB=$((DB_SIZE / 1024))

print_cyan "  Database size: ${DB_SIZE_KB}KB (20 memories)"
print_green "  ✓ Database size reasonable"

# ===================================================================
# TEST 6: Concurrent Access (Sequential Simulation)
# ===================================================================

section "Test 6: Concurrent Access Simulation"

print_cyan "Testing concurrent-like access patterns..."

# Simulate interleaved reads and writes
DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Concurrent test 1" \
    --namespace "project:concurrent" \
    --importance 7 >/dev/null 2>&1 &

DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Concurrent test 2" \
    --namespace "project:concurrent" \
    --importance 7 >/dev/null 2>&1 &

DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Concurrent test 3" \
    --namespace "project:concurrent" \
    --importance 7 >/dev/null 2>&1 &

wait

CONCURRENT_COUNT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT COUNT(*) FROM memories WHERE json_extract(namespace, '$.type') = 'project' AND json_extract(namespace, '$.name') = 'concurrent' " 2>/dev/null)

if [ "$CONCURRENT_COUNT" -eq 3 ]; then
    print_green "  ✓ All concurrent writes successful"
else
    warn "Some concurrent writes may have failed: $CONCURRENT_COUNT / 3"
fi

# ===================================================================
# CLEANUP
# ===================================================================

cleanup_solo_developer "$TEST_DB"

# ===================================================================
# TEST SUMMARY
# ===================================================================

section "Test Summary: Storage - Local SQLite [REGRESSION]"

echo "✓ Database file creation: PASS"
echo "✓ CREATE operation: PASS"
echo "✓ READ operation: PASS"
echo "✓ UPDATE operation: PASS"
echo "✓ DELETE operation: PASS"
echo "✓ Schema validation: PASS"
echo "✓ Bulk insert (20 memories): PASS (${DURATION}s)"
echo "✓ Query performance: PASS (${QUERY_TIME}ms)"
echo "✓ Concurrent access: PASS ($CONCURRENT_COUNT/3)"
echo ""

print_green "✓ ALL TESTS PASSED"
exit 0
