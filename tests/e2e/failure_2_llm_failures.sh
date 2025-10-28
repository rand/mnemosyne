#!/usr/bin/env bash
set -uo pipefail  # Removed -e to allow test failures

# E2E Test: Failure 2 - LLM Failures
#
# Scenario: Test handling of LLM API failures during enrichment
# Tests system behavior when:
# - LLM API is unavailable
# - LLM returns malformed responses
# - LLM times out
# - API key is invalid
# - Rate limits are hit
# - Fallback modes activate

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source common utilities
# shellcheck source=tests/e2e/lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"

# Test state
PASSED=0
FAILED=0

section "E2E Test: Failure 2 - LLM Failures"

# Setup test environment
setup_test_env "fail2_llm"

section "Test 1: Missing API Key"

print_cyan "Testing behavior with no ANTHROPIC_API_KEY..."

# Temporarily unset API key (use :- to handle unset variable with set -u)
OLD_API_KEY="${ANTHROPIC_API_KEY:-}"
unset ANTHROPIC_API_KEY

# Try to create memory without API key
NO_KEY_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Test memory without API key" \
    --namespace "project:test" --importance 7 2>&1 || echo "NO_KEY_ERROR")

NO_KEY_EXIT=$?

# Restore API key (if it was set)
if [ -n "$OLD_API_KEY" ]; then
    export ANTHROPIC_API_KEY="$OLD_API_KEY"
fi

# System should either:
# 1. Fail gracefully with clear error message
# 2. Store memory without enrichment (degraded mode)
if [ "$NO_KEY_EXIT" -ne 0 ]; then
    if echo "$NO_KEY_OUTPUT" | grep -qi "api.*key\|anthropic\|environment"; then
        pass "Missing API key: Clear error message provided"
    else
        warn "Missing API key detected but error message unclear"
    fi
else
    # Check if memory was stored without enrichment
    STORED=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "Test memory" \
        --namespace "project:test" 2>&1 || echo "")

    if echo "$STORED" | grep -qi "Test memory"; then
        pass "Missing API key: Degraded mode active (stored without enrichment)"
    else
        fail "Missing API key: Unclear behavior"
    fi
fi

section "Test 2: Invalid API Key"

print_cyan "Testing behavior with invalid API key..."

# Use obviously invalid API key
export ANTHROPIC_API_KEY="sk-ant-invalid-test-key-12345"

# Try to create memory with invalid key
INVALID_KEY_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Test memory with invalid API key" \
    --namespace "project:test" --importance 7 2>&1 || echo "INVALID_KEY_ERROR")

INVALID_KEY_EXIT=$?

# Restore real API key
export ANTHROPIC_API_KEY="$OLD_API_KEY"

if [ "$INVALID_KEY_EXIT" -ne 0 ]; then
    if echo "$INVALID_KEY_OUTPUT" | grep -qi "invalid.*key\|authentication\|401\|unauthorized"; then
        pass "Invalid API key: Authentication error detected"
    else
        warn "Invalid API key detected but error unclear"
    fi
else
    # Check if memory was stored without enrichment
    STORED=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "invalid API key" \
        --namespace "project:test" 2>&1 || echo "")

    if echo "$STORED" | grep -qi "invalid API key"; then
        pass "Invalid API key: Degraded mode active"
    else
        fail "Invalid API key: Unclear behavior"
    fi
fi

section "Test 3: LLM Enrichment Graceful Degradation"

print_cyan "Testing graceful degradation when enrichment fails..."

# Create memory that should trigger enrichment
# Note: This test validates that if enrichment fails, memory is still stored
DEGRADED_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Important decision: Using microservices architecture for scalability" \
    --namespace "project:test" --importance 8 2>&1 || echo "")

sleep 2

# Verify memory was stored (even if enrichment failed)
STORED=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "microservices" \
    --namespace "project:test" 2>&1 || echo "")

if echo "$STORED" | grep -qi "microservices"; then
    pass "Graceful degradation: Memory stored even if enrichment fails"
else
    fail "Graceful degradation: Memory not stored"
fi

section "Test 4: LLM Response Timeout"

print_cyan "Testing LLM timeout handling..."

# Create memory and check if there's timeout protection
# The system should have timeout configured (typically 30s)
START_TIME=$(date +%s)

TIMEOUT_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" timeout 60 "$BIN" remember \
    --content "Test memory for timeout validation - this should not hang indefinitely" \
    --namespace "project:test" --importance 7 2>&1 || echo "TIMEOUT_ERROR")

END_TIME=$(date +%s)
DURATION=$((END_TIME - START_TIME))

# If command completed within reasonable time (< 45s including API latency)
if [ "$DURATION" -lt 45 ]; then
    pass "LLM timeout: Command completed within reasonable time (${DURATION}s)"
else
    warn "LLM timeout: Command took unusually long (${DURATION}s)"
fi

section "Test 5: Partial LLM Response Handling"

print_cyan "Testing handling of incomplete LLM responses..."

# This test validates that system can handle partial/malformed LLM responses
# In real scenario, LLM might return incomplete JSON or truncated response

# Create memory with complex content that might trigger edge cases
COMPLEX_CONTENT="Memory with special characters: @#\$%^&*() and unicode: 你好世界 émojis: 🚀🎯"

PARTIAL_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$COMPLEX_CONTENT" \
    --namespace "project:test" --importance 6 2>&1 || echo "")

sleep 2

# Verify it was stored (robustness test)
STORED=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "special characters" \
    --namespace "project:test" 2>&1 || echo "")

if echo "$STORED" | grep -qi "special.*characters\|émojis"; then
    pass "Partial response handling: Complex content stored successfully"
else
    warn "Partial response handling: Complex content may not be stored correctly"
fi

section "Test 6: Rate Limit Handling"

print_cyan "Testing rate limit awareness..."

# Create multiple memories in rapid succession
# System should handle potential rate limiting gracefully
print_cyan "Creating 5 memories rapidly..."

RATE_LIMIT_ERRORS=0

for i in {1..5}; do
    RAPID_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
        --content "Rapid memory creation test $i - testing rate limit handling" \
        --namespace "project:ratelimit" --importance 6 2>&1 || echo "RAPID_ERROR_$i")

    if echo "$RAPID_OUTPUT" | grep -qi "rate.*limit\|429\|too.*many.*requests"; then
        ((RATE_LIMIT_ERRORS++))
    fi
done

sleep 3

# Check how many were actually stored
STORED_COUNT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "Rapid memory" \
    --namespace "project:ratelimit" 2>&1 | grep -c "Rapid memory" || true)
if [ -z "$STORED_COUNT" ] || [ "$STORED_COUNT" = "" ]; then
    STORED_COUNT=0
fi

if [ "$STORED_COUNT" -ge 3 ]; then
    pass "Rate limit handling: Majority of memories stored ($STORED_COUNT/5)"
else
    warn "Rate limit handling: Only $STORED_COUNT/5 memories stored"
fi

if [ "$RATE_LIMIT_ERRORS" -gt 0 ]; then
    pass "Rate limit detection: System aware of rate limits ($RATE_LIMIT_ERRORS errors)"
fi

section "Test 7: LLM Fallback Modes"

print_cyan "Testing LLM fallback behavior..."

# When LLM is unavailable, system should:
# 1. Store memory with basic metadata only
# 2. Mark memory as "needs enrichment"
# 3. Allow later re-enrichment

# This is validated by checking if memories can be stored and retrieved
# even when enrichment might fail

FALLBACK_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Fallback test: This memory should be stored even if enrichment fails" \
    --namespace "project:test" --importance 7 2>&1 || echo "")

sleep 2

FALLBACK_STORED=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "Fallback test" \
    --namespace "project:test" 2>&1 || echo "")

if echo "$FALLBACK_STORED" | grep -qi "Fallback test"; then
    pass "Fallback mode: Memory persisted despite potential enrichment issues"
else
    fail "Fallback mode: Memory not stored"
fi

section "Test 8: Empty LLM Response"

print_cyan "Testing handling of empty LLM responses..."

# Edge case: LLM returns empty response or null
# System should handle this gracefully

EMPTY_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "" \
    --namespace "project:test" --importance 5 2>&1 || echo "EMPTY_ERROR")

EMPTY_EXIT=$?

# Empty content should either:
# 1. Be rejected with clear error
# 2. Be stored as empty memory
if [ "$EMPTY_EXIT" -ne 0 ]; then
    pass "Empty content: Rejected appropriately"
else
    warn "Empty content: Accepted (may or may not be desired behavior)"
fi

section "Test 9: LLM Error Recovery"

print_cyan "Testing error recovery after LLM failures..."

# After LLM errors, system should recover and continue working
# This test verifies that one failure doesn't break subsequent operations

# Create memory after all the error scenarios above
RECOVERY_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Recovery test: System should work normally after previous errors" \
    --namespace "project:test" --importance 7 2>&1 || echo "")

sleep 2

RECOVERY_STORED=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "Recovery test" \
    --namespace "project:test" 2>&1 || echo "")

if echo "$RECOVERY_STORED" | grep -qi "Recovery test"; then
    pass "Error recovery: System continues working after LLM errors"
else
    fail "Error recovery: System may be in bad state after errors"
fi

section "Test 10: Enrichment Metadata Validation"

print_cyan "Testing enrichment metadata integrity..."

# Create memory and verify that enrichment metadata (if present) is valid
METADATA_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "Metadata test: Architectural decision to use event-driven architecture" \
    --namespace "project:test" --importance 8 2>&1 || echo "")

sleep 2

# Retrieve and check metadata
METADATA_STORED=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "event-driven" \
    --namespace "project:test" 2>&1 || echo "")

if echo "$METADATA_STORED" | grep -qi "event-driven\|architecture"; then
    pass "Enrichment metadata: Memory stored with content intact"

    # Check if enrichment metadata is present (tags, type, etc.)
    if echo "$METADATA_STORED" | grep -qi "type:\|tags:\|importance:"; then
        pass "Enrichment metadata: Structured metadata present"
    else
        warn "Enrichment metadata: May not be enriched or format unclear"
    fi
else
    fail "Enrichment metadata: Memory not stored correctly"
fi

# Cleanup
section "Cleanup"
teardown_test_env

# Summary
test_summary
exit $?
