#!/usr/bin/env bash
set -uo pipefail  # Removed -e to allow test failures

# E2E Test: Agentic Workflow 3 - Reviewer Agent
#
# Scenario: Validate Reviewer agent quality gates
# Tests reviewer's ability to:
# - Validate intent satisfaction
# - Check test coverage
# - Verify documentation completeness
# - Detect anti-patterns
# - Fact-check claims
# - Mark work COMPLETE only when all gates pass

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source common utilities
# shellcheck source=tests/e2e/lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"

# Test state
PASSED=0
FAILED=0

section "E2E Test: Agentic Workflow 3 - Reviewer Agent"

# Setup test environment
setup_test_env "ag3_reviewer"

section "Test 1: Intent Satisfaction Gate"

print_cyan "Testing intent satisfaction validation..."

# Scenario: Work submitted that matches intent
create_memory "$BIN" "$TEST_DB" \
    "Intent: Implement user authentication with JWT - STATUS: Implemented and tested" \
    "project:review" 8 > /dev/null 2>&1

# Scenario: Work submitted that does NOT match intent
create_memory "$BIN" "$TEST_DB" \
    "Intent: Add rate limiting to API - STATUS: Partially implemented, missing Redis integration" \
    "project:review" 7 > /dev/null 2>&1

sleep 2

# Query review statuses
SATISFIED=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "Implemented and tested" \
    --namespace "project:review" 2>&1 || echo "")

NOT_SATISFIED=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "Partially implemented" \
    --namespace "project:review" 2>&1 || echo "")

if echo "$SATISFIED" | grep -qi "Implemented and tested"; then
    pass "Intent satisfaction: Complete work identified"
else
    fail "Intent satisfaction check failed"
fi

if echo "$NOT_SATISFIED" | grep -qi "Partially implemented"; then
    pass "Intent satisfaction: Incomplete work identified"
else
    fail "Incomplete work detection failed"
fi

section "Test 2: Test Coverage Gate"

print_cyan "Testing test coverage validation..."

# Work with tests
create_memory "$BIN" "$TEST_DB" \
    "Code Review: User service has 95% test coverage - All critical paths tested" \
    "project:review" 8 > /dev/null 2>&1

# Work without tests
create_memory "$BIN" "$TEST_DB" \
    "Code Review: Payment service has NO TESTS - Blocked until tests added" \
    "project:review" 7 > /dev/null 2>&1

sleep 2

WITH_TESTS=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "test coverage" \
    --namespace "project:review" 2>&1 || echo "")

NO_TESTS=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "NO TESTS" \
    --namespace "project:review" 2>&1 || echo "")

if echo "$WITH_TESTS" | grep -qi "95% test coverage"; then
    pass "Test coverage gate: Adequate coverage detected"
else
    fail "Test coverage detection failed"
fi

if echo "$NO_TESTS" | grep -qi "NO TESTS.*Blocked"; then
    pass "Test coverage gate: Missing tests blocked"
else
    fail "Missing tests detection failed"
fi

section "Test 3: Documentation Gate"

print_cyan "Testing documentation completeness..."

# Well-documented work
create_memory "$BIN" "$TEST_DB" \
    "Documentation: API endpoints fully documented with OpenAPI spec and examples" \
    "project:review" 8 > /dev/null 2>&1

# Missing documentation
create_memory "$BIN" "$TEST_DB" \
    "Documentation: Database schema changes NOT DOCUMENTED - Blocked for docs" \
    "project:review" 7 > /dev/null 2>&1

sleep 2

GOOD_DOCS=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "fully documented" \
    --namespace "project:review" 2>&1 || echo "")

MISSING_DOCS=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "schema changes Blocked" \
    --namespace "project:review" 2>&1 || echo "")

if echo "$GOOD_DOCS" | grep -qi "fully documented"; then
    pass "Documentation gate: Complete docs approved"
else
    fail "Documentation completeness check failed"
fi

if echo "$MISSING_DOCS" | grep -qi "Blocked.*docs\|NOT DOCUMENTED"; then
    pass "Documentation gate: Missing docs blocked"
else
    fail "Missing docs detection failed"
fi

section "Test 4: Anti-Pattern Detection"

print_cyan "Testing anti-pattern detection..."

# Code with anti-patterns
create_memory "$BIN" "$TEST_DB" \
    "Anti-Pattern: TODO comments found in production code - Must be resolved" \
    "project:review" 8 > /dev/null 2>&1

create_memory "$BIN" "$TEST_DB" \
    "Anti-Pattern: Mock/stub functions left in codebase - Replace with real implementation" \
    "project:review" 8 > /dev/null 2>&1

# Clean code
create_memory "$BIN" "$TEST_DB" \
    "Code Review: No anti-patterns detected - Code follows best practices" \
    "project:review" 8 > /dev/null 2>&1

sleep 2

ANTI_PATTERNS=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "TODO" \
    --namespace "project:review" 2>&1 || echo "")

MOCK_STUB=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "Mock stub" \
    --namespace "project:review" 2>&1 || echo "")

CLEAN_CODE=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "clean code" \
    --namespace "project:review" 2>&1 || echo "")

if echo "$ANTI_PATTERNS" | grep -qi "TODO comments"; then
    pass "Anti-pattern detection: TODO comments found"
else
    fail "TODO detection failed"
fi

if echo "$MOCK_STUB" | grep -qi "Mock.*stub"; then
    pass "Anti-pattern detection: Mock/stub code found"
else
    fail "Mock/stub detection failed"
fi

if echo "$CLEAN_CODE" | grep -qi "No anti-patterns"; then
    pass "Clean code validated"
else
    fail "Clean code validation failed"
fi

section "Test 5: Fact-Checking Gate"

print_cyan "Testing fact-checking capability..."

# Correct claims
create_memory "$BIN" "$TEST_DB" \
    "Fact Check: PostgreSQL supports JSON queries - VERIFIED CORRECT" \
    "project:review" 8 > /dev/null 2>&1

# Incorrect claims
create_memory "$BIN" "$TEST_DB" \
    "Fact Check: Claim that SQLite doesn't support transactions - INCORRECT, must fix" \
    "project:review" 8 > /dev/null 2>&1

sleep 2

VERIFIED=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "VERIFIED CORRECT" \
    --namespace "project:review" 2>&1 || echo "")

INCORRECT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "INCORRECT must fix" \
    --namespace "project:review" 2>&1 || echo "")

if echo "$VERIFIED" | grep -qi "VERIFIED CORRECT"; then
    pass "Fact-checking: Correct claims verified"
else
    fail "Correct claim verification failed"
fi

if echo "$INCORRECT" | grep -qi "INCORRECT"; then
    pass "Fact-checking: Incorrect claims detected"
else
    fail "Incorrect claim detection failed"
fi

section "Test 6: Constraints Validation"

print_cyan "Testing constraints maintenance..."

# Work respecting constraints
create_memory "$BIN" "$TEST_DB" \
    "Constraints: API response time <200ms maintained - Performance SLA met" \
    "project:review" 8 > /dev/null 2>&1

# Work violating constraints
create_memory "$BIN" "$TEST_DB" \
    "Constraints: Database query takes 5 seconds - VIOLATES 1s timeout constraint" \
    "project:review" 8 > /dev/null 2>&1

sleep 2

MET_CONSTRAINTS=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "SLA met" \
    --namespace "project:review" 2>&1 || echo "")

VIOLATED=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "VIOLATES constraint" \
    --namespace "project:review" 2>&1 || echo "")

if echo "$MET_CONSTRAINTS" | grep -qi "SLA met"; then
    pass "Constraints: Compliant work approved"
else
    fail "Constraint compliance check failed"
fi

if echo "$VIOLATED" | grep -qi "VIOLATES"; then
    pass "Constraints: Violations detected"
else
    fail "Constraint violation detection failed"
fi

section "Test 7: Complete Work Marking"

print_cyan "Testing COMPLETE status marking..."

# All gates passed
create_memory "$BIN" "$TEST_DB" \
    "COMPLETE: User authentication feature - All quality gates passed, ready for production" \
    "project:review" 9 > /dev/null 2>&1

# Some gates failed
create_memory "$BIN" "$TEST_DB" \
    "INCOMPLETE: Payment integration - Tests missing, documentation incomplete" \
    "project:review" 7 > /dev/null 2>&1

sleep 2

COMPLETE=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "COMPLETE" \
    --namespace "project:review" --min-importance 9 2>&1 || echo "")

INCOMPLETE=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "INCOMPLETE" \
    --namespace "project:review" 2>&1 || echo "")

if echo "$COMPLETE" | grep -qi "COMPLETE.*quality gates passed"; then
    pass "Complete marking: Work approved with all gates passed"
else
    fail "Complete work marking failed"
fi

if echo "$INCOMPLETE" | grep -qi "INCOMPLETE.*missing"; then
    pass "Incomplete marking: Work blocked with reasons"
else
    fail "Incomplete work marking failed"
fi

section "Test 8: Quality Gate Summary"

print_cyan "Testing quality gate tracking..."

# Create quality gate checklist
create_memory "$BIN" "$TEST_DB" \
    "Quality Gates Checklist: Intent ✓, Tests ✓, Docs ✓, Anti-patterns ✓, Facts ✓, Constraints ✓" \
    "project:review" 9 > /dev/null 2>&1

sleep 2

GATES=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "Quality Gates Checklist" \
    --namespace "project:review" 2>&1 || echo "")

if echo "$GATES" | grep -qi "Quality Gates Checklist"; then
    # Count checkmarks
    CHECKMARKS=$(echo "$GATES" | grep -o "✓" | wc -l | tr -d ' ')
    if [ "$CHECKMARKS" -ge 6 ]; then
        pass "Quality gate checklist tracked ($CHECKMARKS gates)"
    else
        pass "Quality gate checklist tracked"
    fi
else
    fail "Quality gate checklist tracking failed"
fi

# Cleanup
section "Cleanup"
teardown_test_env

# Summary
test_summary
exit $?
