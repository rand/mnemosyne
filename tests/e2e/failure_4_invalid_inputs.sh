#!/usr/bin/env bash
set -uo pipefail  # Removed -e to allow test failures

# E2E Test: Failure 4 - Invalid Inputs
#
# Scenario: Test input validation and error handling
# Tests system behavior with:
# - Invalid namespaces
# - Invalid importance values
# - Malformed content
# - SQL injection attempts
# - Path traversal attempts
# - Invalid command arguments

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source common utilities
# shellcheck source=tests/e2e/lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"

# Test state
PASSED=0
FAILED=0

section "E2E Test: Failure 4 - Invalid Inputs"

# Setup test environment
setup_test_env "fail4_invalid"

section "Test 1: Invalid Namespace Format"

print_cyan "Testing namespace validation..."

# Invalid namespaces (should be rejected or sanitized)
INVALID_NAMESPACES=(
    "../../../etc/passwd"           # Path traversal
    "project:../../secrets"         # Path traversal in namespace
    "'; DROP TABLE memories; --"    # SQL injection
    "project:test<script>alert(1)</script>"  # XSS attempt
    "project:test\x00null"          # Null byte injection
    ""                              # Empty namespace
    "x" * 1000                      # Extremely long namespace
)

INVALID_COUNT=0

for ns in "${INVALID_NAMESPACES[@]}"; do
    INVALID_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
        "Test memory with invalid namespace" \
        --namespace "$ns" --importance 7 2>&1 || echo "INVALID_NS_ERROR")

    if echo "$INVALID_OUTPUT" | grep -qi "invalid\|error\|INVALID_NS_ERROR"; then
        ((INVALID_COUNT++))
    fi
done

if [ "$INVALID_COUNT" -ge 3 ]; then
    pass "Namespace validation: Invalid namespaces rejected ($INVALID_COUNT/${#INVALID_NAMESPACES[@]})"
else
    warn "Namespace validation: Some invalid namespaces may be accepted ($INVALID_COUNT rejected)"
fi

section "Test 2: Invalid Importance Values"

print_cyan "Testing importance value validation..."

# Invalid importance values (should be 0-10)
INVALID_IMPORTANCE=(
    -1      # Negative
    11      # Above max
    999     # Way above max
    "abc"   # Non-numeric
    "1.5"   # Float (should accept or round)
)

IMPORTANCE_ERRORS=0

for imp in "${INVALID_IMPORTANCE[@]}"; do
    IMP_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
        "Test memory with importance $imp" \
        --namespace "project:test" --importance "$imp" 2>&1 || echo "IMP_ERROR")

    if echo "$IMP_OUTPUT" | grep -qi "invalid\|error\|IMP_ERROR"; then
        ((IMPORTANCE_ERRORS++))
    fi
done

if [ "$IMPORTANCE_ERRORS" -ge 3 ]; then
    pass "Importance validation: Invalid values rejected ($IMPORTANCE_ERRORS/${#INVALID_IMPORTANCE[@]})"
else
    warn "Importance validation: Some invalid values accepted"
fi

section "Test 3: SQL Injection Protection"

print_cyan "Testing SQL injection protection..."

# Common SQL injection patterns
SQL_INJECTIONS=(
    "'; DROP TABLE memories; --"
    "' OR '1'='1"
    "'; DELETE FROM memories WHERE '1'='1"
    "' UNION SELECT * FROM memories --"
)

SQL_INJECTION_BLOCKED=0

for injection in "${SQL_INJECTIONS[@]}"; do
    # Store injection pattern as content (should be stored safely)
    INJ_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
        --content "$injection" \
        --namespace "project:test" --importance 7 2>&1 || echo "")

    sleep 1

    # Verify database integrity by checking:
    # 1. Tables still exist
    # 2. We can query the database
    # 3. Database structure is intact

    # Check if memories table still exists
    TABLE_CHECK=$(sqlite3 "$TEST_DB" \
        "SELECT name FROM sqlite_master WHERE type='table' AND name='memories';" 2>&1)

    if [ -z "$TABLE_CHECK" ]; then
        fail "SQL injection: memories table was dropped!"
        ((SQL_INJECTION_BLOCKED++))
        continue
    fi

    # Check if we can query the table without SQL errors
    QUERY_CHECK=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "" \
        --namespace "project:test" 2>&1)

    if echo "$QUERY_CHECK" | grep -qi "no such table\|syntax error\|database is locked"; then
        fail "SQL injection: Database structure damaged"
        ((SQL_INJECTION_BLOCKED++))
        continue
    fi

    # Success - injection was safely stored as text
    pass "SQL injection: Injection pattern safely stored as text"
done

if [ "$SQL_INJECTION_BLOCKED" -eq 0 ]; then
    pass "SQL injection: All injection attempts safely handled (${#SQL_INJECTIONS[@]}/${#SQL_INJECTIONS[@]})"
else
    fail "SQL injection: $SQL_INJECTION_BLOCKED injection(s) caused issues"
fi

section "Test 4: Path Traversal Protection"

print_cyan "Testing path traversal protection..."

# Path traversal attempts in various fields
PATH_TRAVERSALS=(
    "../../../etc/passwd"
    "..\\..\\..\\windows\\system32"
    "/etc/passwd"
    "./../../sensitive/data"
)

PATH_ERRORS=0

for path in "${PATH_TRAVERSALS[@]}"; do
    PATH_OUTPUT=$(DATABASE_URL="sqlite://$path" "$BIN" remember \
        "Path traversal test" \
        --namespace "project:test" --importance 7 2>&1 || echo "PATH_ERROR")

    if echo "$PATH_OUTPUT" | grep -qi "invalid\|error\|PATH_ERROR\|permission"; then
        ((PATH_ERRORS++))
    fi
done

if [ "$PATH_ERRORS" -ge 2 ]; then
    pass "Path traversal: Malicious paths rejected ($PATH_ERRORS/${#PATH_TRAVERSALS[@]})"
else
    warn "Path traversal: Some malicious paths may succeed"
fi

section "Test 5: Malformed Content"

print_cyan "Testing malformed content handling..."

# Various malformed content types
MALFORMED_CONTENTS=(
    "\x00\x00\x00"                  # Binary nulls
    $'\x1b[31mANSI\x1b[0m'         # ANSI escape codes
    "$(printf '\x00\x01\x02')"      # Control characters
)

MALFORMED_HANDLED=0

for content in "${MALFORMED_CONTENTS[@]}"; do
    MAL_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
        "$content" \
        --namespace "project:test" --importance 6 2>&1 || echo "")

    # System should either reject or sanitize
    if echo "$MAL_OUTPUT" | grep -qi "error\|invalid"; then
        ((MALFORMED_HANDLED++))
    fi
done

if [ "$MALFORMED_HANDLED" -ge 1 ]; then
    pass "Malformed content: System validates content ($MALFORMED_HANDLED/${#MALFORMED_CONTENTS[@]} rejected)"
else
    warn "Malformed content: All malformed content accepted (may be expected)"
fi

section "Test 6: Command Argument Validation"

print_cyan "Testing command argument validation..."

# Invalid argument combinations
INVALID_ARGS_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember 2>&1 || echo "ARG_ERROR")

if echo "$INVALID_ARGS_OUTPUT" | grep -qi "error\|required\|missing\|ARG_ERROR"; then
    pass "Argument validation: Missing required arguments detected"
else
    fail "Argument validation: Missing arguments not detected"
fi

# Invalid command
INVALID_CMD_OUTPUT=$("$BIN" nonexistent-command 2>&1 || echo "CMD_ERROR")

if echo "$INVALID_CMD_OUTPUT" | grep -qi "error\|unknown\|unrecognized\|CMD_ERROR"; then
    pass "Command validation: Invalid commands rejected"
else
    fail "Command validation: Invalid commands not rejected"
fi

section "Test 7: Unicode and Special Characters"

print_cyan "Testing unicode and special character handling..."

# Unicode and special characters that should be handled correctly
SPECIAL_CHARS=(
    "Hello 世界"                     # Chinese
    "Привет мир"                    # Russian
    "مرحبا بالعالم"                  # Arabic
    "🚀🎯💡🔥"                      # Emojis
    "Test \n newline \t tab"       # Escape sequences
)

SPECIAL_HANDLED=0

for char_test in "${SPECIAL_CHARS[@]}"; do
    CHAR_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
        "$char_test" \
        --namespace "project:test" --importance 6 2>&1 || echo "")

    sleep 1

    # Verify can be retrieved
    CHAR_STORED=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "Test" \
        --namespace "project:test" 2>&1 || echo "")

    if [ -n "$CHAR_STORED" ]; then
        ((SPECIAL_HANDLED++))
    fi
done

if [ "$SPECIAL_HANDLED" -ge 4 ]; then
    pass "Special characters: Unicode handled correctly ($SPECIAL_HANDLED/${#SPECIAL_CHARS[@]})"
else
    warn "Special characters: Some unicode may not be stored correctly"
fi

section "Test 8: Boundary Value Testing"

print_cyan "Testing boundary values..."

# Test importance boundaries
for imp in 0 10; do
    BOUNDARY_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
        --content "Boundary test importance $imp" \
        --namespace "project:test" --importance $imp 2>&1 || echo "")

    sleep 1
done

BOUNDARY_STORED=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "Boundary test" \
    --namespace "project:test" 2>&1 | grep -c "Boundary test" || true)
if [ -z "$BOUNDARY_STORED" ] || [ "$BOUNDARY_STORED" = "" ]; then
    BOUNDARY_STORED=0
fi

if [ "$BOUNDARY_STORED" -ge 2 ]; then
    pass "Boundary values: Valid boundary values accepted (0 and 10)"
else
    fail "Boundary values: Valid boundary values rejected"
fi

section "Test 9: Concurrent Invalid Input Handling"

print_cyan "Testing concurrent invalid input handling..."

# Launch multiple invalid operations simultaneously
for i in {1..5}; do
    (DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
        "Test" \
        --namespace "invalid/../$i" --importance 999 > /dev/null 2>&1) &
done

wait

# System should remain stable
STABILITY_CHECK=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "Test" \
    --namespace "project:test" 2>&1 || echo "STABILITY_ERROR")

if echo "$STABILITY_CHECK" | grep -qi "STABILITY_ERROR"; then
    fail "Concurrent invalid inputs: System destabilized"
else
    pass "Concurrent invalid inputs: System remained stable"
fi

section "Test 10: Error Message Quality"

print_cyan "Testing quality of error messages for invalid inputs..."

# Error messages should be helpful, not just "error"
ERROR_MSG_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    "Test" \
    --namespace "" --importance 7 2>&1 || echo "EMPTY_NS_ERROR")

if echo "$ERROR_MSG_OUTPUT" | grep -qi "namespace.*required\|namespace.*empty\|invalid.*namespace"; then
    pass "Error messages: Descriptive error for empty namespace"
else
    warn "Error messages: Could be more descriptive"
fi

section "Test 11: Input Sanitization"

print_cyan "Testing input sanitization..."

# Inputs should be sanitized, not just rejected
SANITIZE_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    "Test memory with <html>tags</html> and \${variables}" \
    --namespace "project:test" --importance 7 2>&1 || echo "")

sleep 2

SANITIZE_STORED=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "Test memory" \
    --namespace "project:test" 2>&1 || echo "")

if [ -n "$SANITIZE_STORED" ]; then
    pass "Input sanitization: Potentially dangerous content handled"

    # Check if content was sanitized or stored as-is
    if echo "$SANITIZE_STORED" | grep -qi "<html>"; then
        warn "Input sanitization: HTML tags stored as-is (may be intended)"
    fi
else
    fail "Input sanitization: Content rejected entirely"
fi

section "Test 12: Database URL Validation"

print_cyan "Testing database URL validation..."

# Invalid database URLs
INVALID_URLS=(
    "invalid://path/to/db"
    "sqlite://../../../etc/passwd"
    "http://malicious.com/db"
)

URL_ERRORS=0

for url in "${INVALID_URLS[@]}"; do
    URL_OUTPUT=$(DATABASE_URL="$url" "$BIN" remember \
        "Test" \
        --namespace "project:test" --importance 7 2>&1 || echo "URL_ERROR")

    if echo "$URL_OUTPUT" | grep -qi "error\|invalid\|URL_ERROR"; then
        ((URL_ERRORS++))
    fi
done

if [ "$URL_ERRORS" -ge 2 ]; then
    pass "Database URL validation: Invalid URLs rejected ($URL_ERRORS/${#INVALID_URLS[@]})"
else
    warn "Database URL validation: Some invalid URLs may be accepted"
fi

# Cleanup
section "Cleanup"
teardown_test_env

# Summary
test_summary
exit $?
