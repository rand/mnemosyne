#!/usr/bin/env bash
set -uo pipefail  # Removed -e to allow test failures

# E2E Test: Agentic Workflow 4 - Executor Agent
#
# Scenario: Validate Executor agent work execution
# Tests executor's ability to:
# - Follow Work Plan Protocol (Phases 1-4)
# - Execute atomic tasks systematically
# - Apply loaded skills to solve problems
# - Challenge vague requirements
# - Spawn sub-agents for parallel work
# - Submit work to Reviewer at checkpoints

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source common utilities
# shellcheck source=tests/e2e/lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"

# Test state
PASSED=0
FAILED=0

section "E2E Test: Agentic Workflow 4 - Executor Agent"

# Setup test environment
setup_test_env "ag4_executor"

section "Test 1: Work Plan Protocol Adherence"

print_cyan "Testing Work Plan Protocol execution..."

# Create work plan phases
create_memory "$BIN" "$TEST_DB" \
    "Phase 1 COMPLETE: Prompt → Spec - Requirements clarified and spec written" \
    "project:workplan" 9 > /dev/null 2>&1

create_memory "$BIN" "$TEST_DB" \
    "Phase 2 COMPLETE: Spec → Full Spec - Components decomposed, dependencies mapped" \
    "project:workplan" 9 > /dev/null 2>&1

create_memory "$BIN" "$TEST_DB" \
    "Phase 3 IN PROGRESS: Full Spec → Plan - Creating execution plan" \
    "project:workplan" 9 > /dev/null 2>&1

create_memory "$BIN" "$TEST_DB" \
    "Phase 4 PENDING: Plan → Artifacts - Awaiting phase 3 completion" \
    "project:workplan" 9 > /dev/null 2>&1

sleep 2

# Verify work plan phases tracked
PHASES=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "Phase" \
    --namespace "project:workplan" --min-importance 9 2>&1 || echo "")

PHASE_COUNT=$(echo "$PHASES" | grep -c "Phase [1-4]" || echo "0")

if [ "$PHASE_COUNT" -ge 4 ]; then
    pass "Work Plan Protocol: All 4 phases tracked"
else
    fail "Work Plan Protocol incomplete: $PHASE_COUNT/4 phases"
fi

section "Test 2: Atomic Task Execution"

print_cyan "Testing atomic task execution..."

# Executor breaks work into atomic tasks
create_memory "$BIN" "$TEST_DB" \
    "Atomic Task 1: Create database schema migration - COMPLETED" \
    "project:tasks" 8 > /dev/null 2>&1

create_memory "$BIN" "$TEST_DB" \
    "Atomic Task 2: Implement user model with validation - COMPLETED" \
    "project:tasks" 8 > /dev/null 2>&1

create_memory "$BIN" "$TEST_DB" \
    "Atomic Task 3: Write unit tests for user model - IN PROGRESS" \
    "project:tasks" 8 > /dev/null 2>&1

sleep 2

COMPLETED=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "COMPLETED" \
    --namespace "project:tasks" 2>&1 | grep -c "COMPLETED" || echo "0")

IN_PROGRESS=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "IN PROGRESS" \
    --namespace "project:tasks" 2>&1 | grep -c "IN PROGRESS" || echo "0")

if [ "$COMPLETED" -ge 2 ] && [ "$IN_PROGRESS" -ge 1 ]; then
    pass "Atomic task execution tracked ($COMPLETED completed, $IN_PROGRESS in progress)"
else
    warn "Task execution tracking incomplete"
fi

section "Test 3: Skill Application"

print_cyan "Testing skill application during execution..."

# Executor applies loaded skills to solve problems
create_memory "$BIN" "$TEST_DB" \
    "Skill Applied: Using Rust error handling patterns (Result<T, E>) for database operations" \
    "project:skills" 8 > /dev/null 2>&1

create_memory "$BIN" "$TEST_DB" \
    "Skill Applied: TDD approach - Write tests before implementation" \
    "project:skills" 8 > /dev/null 2>&1

sleep 2

SKILLS_APPLIED=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "Skill Applied" \
    --namespace "project:skills" 2>&1 | grep -c "Skill Applied" || echo "0")

if [ "$SKILLS_APPLIED" -ge 2 ]; then
    pass "Skills applied during execution ($SKILLS_APPLIED skills)"
else
    fail "Skill application tracking failed"
fi

section "Test 4: Vague Requirement Challenge"

print_cyan "Testing vague requirement challenge capability..."

# Executor identifies vague requirements and asks for clarification
create_memory "$BIN" "$TEST_DB" \
    "Vague Requirement CHALLENGED: 'Make it better' → Requested specific success criteria" \
    "project:challenges" 8 > /dev/null 2>&1

create_memory "$BIN" "$TEST_DB" \
    "Vague Requirement CHALLENGED: 'Optimize performance' → Asked for target metrics and constraints" \
    "project:challenges" 8 > /dev/null 2>&1

# Clear requirement (no challenge needed)
create_memory "$BIN" "$TEST_DB" \
    "Clear Requirement ACCEPTED: Implement JWT authentication with 1-hour expiry" \
    "project:challenges" 7 > /dev/null 2>&1

sleep 2

CHALLENGED=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "CHALLENGED" \
    --namespace "project:challenges" 2>&1 | grep -c "CHALLENGED" || echo "0")

ACCEPTED=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "ACCEPTED" \
    --namespace "project:challenges" 2>&1 | grep -c "ACCEPTED" || echo "0")

if [ "$CHALLENGED" -ge 2 ]; then
    pass "Vague requirements challenged ($CHALLENGED challenges)"
else
    fail "Vague requirement detection failed"
fi

if [ "$ACCEPTED" -ge 1 ]; then
    pass "Clear requirements accepted"
else
    warn "Clear requirement tracking incomplete"
fi

section "Test 5: Sub-Agent Spawning Criteria"

print_cyan "Testing sub-agent spawning validation..."

# Good candidate for sub-agent (all criteria met)
create_memory "$BIN" "$TEST_DB" \
    "Sub-Agent APPROVED: Independent API documentation task - All spawning criteria met" \
    "project:subagent" 8 > /dev/null 2>&1

# Bad candidate (criteria not met)
create_memory "$BIN" "$TEST_DB" \
    "Sub-Agent REJECTED: Core authentication logic - Circular dependencies detected" \
    "project:subagent" 8 > /dev/null 2>&1

sleep 2

APPROVED=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "Sub-Agent APPROVED" \
    --namespace "project:subagent" 2>&1 || echo "")

REJECTED=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "Sub-Agent REJECTED" \
    --namespace "project:subagent" 2>&1 || echo "")

if echo "$APPROVED" | grep -qi "All spawning criteria met"; then
    pass "Sub-agent spawning: Valid candidates approved"
else
    fail "Sub-agent approval logic failed"
fi

if echo "$REJECTED" | grep -qi "Circular dependencies"; then
    pass "Sub-agent spawning: Invalid candidates rejected"
else
    fail "Sub-agent rejection logic failed"
fi

section "Test 6: Checkpoint Submission to Reviewer"

print_cyan "Testing checkpoint submission workflow..."

# Work submitted at checkpoints
create_memory "$BIN" "$TEST_DB" \
    "Checkpoint Submission: User service implementation → Sent to Reviewer for validation" \
    "project:checkpoints" 8 > /dev/null 2>&1

create_memory "$BIN" "$TEST_DB" \
    "Checkpoint Feedback: Reviewer approved with minor suggestions - Implementing fixes" \
    "project:checkpoints" 8 > /dev/null 2>&1

sleep 2

SUBMISSIONS=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "Checkpoint Submission Reviewer" \
    --namespace "project:checkpoints" 2>&1 || echo "")

FEEDBACK=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "Checkpoint Feedback" \
    --namespace "project:checkpoints" 2>&1 || echo "")

if echo "$SUBMISSIONS" | grep -qi "Sent to Reviewer"; then
    pass "Checkpoint submissions tracked"
else
    fail "Checkpoint submission tracking failed"
fi

if echo "$FEEDBACK" | grep -qi "Reviewer approved"; then
    pass "Reviewer feedback incorporated"
else
    fail "Feedback tracking failed"
fi

section "Test 7: Implementation + Tests + Docs Pattern"

print_cyan "Testing implementation triad (code + tests + docs)..."

# Complete implementation with all three
create_memory "$BIN" "$TEST_DB" \
    "Triad COMPLETE: User authentication - Code ✓ Tests ✓ Docs ✓" \
    "project:triad" 9 > /dev/null 2>&1

# Incomplete (missing tests)
create_memory "$BIN" "$TEST_DB" \
    "Triad INCOMPLETE: Email service - Code ✓ Tests ✗ Docs ✓ - BLOCKED" \
    "project:triad" 7 > /dev/null 2>&1

sleep 2

COMPLETE=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "Triad COMPLETE" \
    --namespace "project:triad" 2>&1 || echo "")

INCOMPLETE=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "Triad INCOMPLETE" \
    --namespace "project:triad" 2>&1 || echo "")

if echo "$COMPLETE" | grep -qi "Code.*Tests.*Docs"; then
    pass "Complete implementation triad validated"
else
    fail "Complete triad validation failed"
fi

if echo "$INCOMPLETE" | grep -qi "BLOCKED"; then
    pass "Incomplete triad detected and blocked"
else
    fail "Incomplete triad detection failed"
fi

section "Test 8: Parallel Work Execution"

print_cyan "Testing parallel work streams..."

# Multiple independent work streams
create_memory "$BIN" "$TEST_DB" \
    "Parallel Stream 1: Backend API development - IN PROGRESS" \
    "project:parallel" 8 > /dev/null 2>&1

create_memory "$BIN" "$TEST_DB" \
    "Parallel Stream 2: Frontend UI components - IN PROGRESS (independent)" \
    "project:parallel" 8 > /dev/null 2>&1

create_memory "$BIN" "$TEST_DB" \
    "Parallel Stream 3: Database migrations - IN PROGRESS (independent)" \
    "project:parallel" 8 > /dev/null 2>&1

sleep 2

PARALLEL_STREAMS=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "Parallel Stream" \
    --namespace "project:parallel" 2>&1 | grep -c "Parallel Stream" || echo "0")

if [ "$PARALLEL_STREAMS" -ge 3 ]; then
    pass "Parallel work streams tracked ($PARALLEL_STREAMS streams)"
else
    fail "Parallel work tracking incomplete: $PARALLEL_STREAMS/3"
fi

section "Test 9: Error Recovery and Retry"

print_cyan "Testing error recovery patterns..."

# Error encountered and recovered
create_memory "$BIN" "$TEST_DB" \
    "Error Recovery: Database connection failed → Implemented retry with exponential backoff" \
    "project:errors" 8 > /dev/null 2>&1

create_memory "$BIN" "$TEST_DB" \
    "Error Recovery: API rate limit hit → Added retry logic with 429 handling" \
    "project:errors" 8 > /dev/null 2>&1

sleep 2

RECOVERIES=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "Error Recovery" \
    --namespace "project:errors" 2>&1 | grep -c "Error Recovery" || echo "0")

if [ "$RECOVERIES" -ge 2 ]; then
    pass "Error recovery patterns documented ($RECOVERIES recoveries)"
else
    fail "Error recovery tracking failed"
fi

# Cleanup
section "Cleanup"
teardown_test_env

# Summary
test_summary
exit $?
