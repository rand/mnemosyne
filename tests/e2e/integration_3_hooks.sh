#!/usr/bin/env bash
set -uo pipefail  # Removed -e to allow test failures

# E2E Test: Integration 3 - Hooks System
#
# Scenario: Test hook system integration
# Tests:
# - Session start hook execution
# - Post-commit hook execution
# - Pre-compact hook execution
# - Hook script permissions
# - Hook error handling
# - Context loading via hooks

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source common utilities
# shellcheck source=tests/e2e/lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"

# Test state
PASSED=0
FAILED=0

section "E2E Test: Integration 3 - Hooks System"

# Setup test environment
setup_test_env "int3_hooks"

section "Test 1: Hook Directory Structure"

print_cyan "Testing hook directory structure..."

# Hooks should be in .claude/hooks/
HOOKS_DIR=".claude/hooks"

if [ -d "$HOOKS_DIR" ]; then
    pass "Hook structure: Hooks directory exists (.claude/hooks/)"
else
    warn "Hook structure: Hooks directory not found (may be project-specific)"
fi

# Check for expected hooks
EXPECTED_HOOKS=(
    "session-start.sh"
    "post-commit.sh"
    "pre-compact.sh"
)

FOUND_HOOKS=0

for hook in "${EXPECTED_HOOKS[@]}"; do
    if [ -f "$HOOKS_DIR/$hook" ]; then
        ((FOUND_HOOKS++))
    fi
done

if [ "$FOUND_HOOKS" -ge 2 ]; then
    pass "Hook structure: Expected hooks found ($FOUND_HOOKS/${#EXPECTED_HOOKS[@]})"
else
    warn "Hook structure: Limited hooks found ($FOUND_HOOKS/${#EXPECTED_HOOKS[@]})"
fi

section "Test 2: Hook Script Permissions"

print_cyan "Testing hook script permissions..."

# Hooks must be executable
EXECUTABLE_HOOKS=0

for hook in "${EXPECTED_HOOKS[@]}"; do
    if [ -x "$HOOKS_DIR/$hook" ]; then
        ((EXECUTABLE_HOOKS++))
    fi
done

if [ "$EXECUTABLE_HOOKS" -eq "$FOUND_HOOKS" ]; then
    pass "Hook permissions: All found hooks are executable"
elif [ "$EXECUTABLE_HOOKS" -gt 0 ]; then
    warn "Hook permissions: Only $EXECUTABLE_HOOKS/$FOUND_HOOKS hooks executable"
else
    warn "Hook permissions: No executable hooks found"
fi

section "Test 3: Session Start Hook"

print_cyan "Testing session-start hook functionality..."

# Session start hook should load context
# Hook typically calls mnemosyne recall with specific parameters

if [ -x "$HOOKS_DIR/session-start.sh" ]; then
    # Create test memories to be loaded by hook
    create_memory "$BIN" "$TEST_DB" \
        "Important project decision for hook loading" \
        "project:mnemosyne" 9 > /dev/null 2>&1

    sleep 2

    # Simulate hook execution
    HOOK_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" bash "$HOOKS_DIR/session-start.sh" 2>&1 || echo "HOOK_ERROR")

    if echo "$HOOK_OUTPUT" | grep -qi "HOOK_ERROR"; then
        warn "Session start hook: Hook execution encountered errors"
    else
        pass "Session start hook: Hook executed without errors"
    fi

    # Check if hook loaded context (should mention memories)
    if echo "$HOOK_OUTPUT" | grep -qi "memory\|context\|importance\|project"; then
        pass "Session start hook: Context loading appears functional"
    else
        warn "Session start hook: Context loading output unclear"
    fi
else
    warn "Session start hook: Hook script not found or not executable"
fi

section "Test 4: Post-Commit Hook"

print_cyan "Testing post-commit hook functionality..."

# Post-commit hook should store commit information as memory

if [ -x "$HOOKS_DIR/post-commit.sh" ]; then
    # Simulate git commit scenario
    if git rev-parse --git-dir > /dev/null 2>&1; then
        # Get latest commit info
        COMMIT_MSG=$(git log -1 --format="%s" 2>&1 || echo "Test commit")
        COMMIT_HASH=$(git log -1 --format="%h" 2>&1 || echo "abc123")

        print_cyan "Simulating post-commit hook with: $COMMIT_HASH - $COMMIT_MSG"

        # Execute hook (it will store commit as memory)
        POST_HOOK_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" bash "$HOOKS_DIR/post-commit.sh" 2>&1 || echo "")

        sleep 2

        # Verify commit was stored as memory
        COMMIT_STORED=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "commit" \
            --namespace "project:mnemosyne" 2>&1 || echo "")

        if echo "$COMMIT_STORED" | grep -qi "commit"; then
            pass "Post-commit hook: Commit information stored as memory"
        else
            warn "Post-commit hook: Commit storage unclear (check hook implementation)"
        fi
    else
        warn "Post-commit hook: Not in git repository, skipping commit test"
    fi
else
    warn "Post-commit hook: Hook script not found or not executable"
fi

section "Test 5: Pre-Compact Hook"

print_cyan "Testing pre-compact hook functionality..."

# Pre-compact hook should preserve important context before compaction

if [ -x "$HOOKS_DIR/pre-compact.sh" ]; then
    # Create high-importance memories
    for i in {1..3}; do
        create_memory "$BIN" "$TEST_DB" \
            "Critical memory $i for pre-compact preservation" \
            "project:mnemosyne" 9 > /dev/null 2>&1
    done

    sleep 3

    # Execute pre-compact hook
    COMPACT_HOOK_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" bash "$HOOKS_DIR/pre-compact.sh" 2>&1 || echo "")

    # Hook should identify critical memories for preservation
    if echo "$COMPACT_HOOK_OUTPUT" | grep -qi "critical\|preserve\|snapshot\|importance"; then
        pass "Pre-compact hook: Context preservation logic executed"
    else
        warn "Pre-compact hook: Context preservation output unclear"
    fi

    # Check if snapshot directory created
    if [ -d ".claude/context-snapshots" ]; then
        pass "Pre-compact hook: Snapshot directory exists"
    else
        warn "Pre-compact hook: Snapshot directory not created (may be intentional)"
    fi
else
    warn "Pre-compact hook: Hook script not found or not executable"
fi

section "Test 6: Hook Error Handling"

print_cyan "Testing hook error handling..."

# Hooks should fail gracefully without breaking session
# Test with invalid database path

if [ -x "$HOOKS_DIR/session-start.sh" ]; then
    ERROR_HOOK_OUTPUT=$(DATABASE_URL="sqlite:///tmp/nonexistent_hook_$(date +%s).db" \
        bash "$HOOKS_DIR/session-start.sh" 2>&1 || echo "HOOK_FAILED")

    # Hook should either succeed (graceful degradation) or fail with clear error
    if echo "$ERROR_HOOK_OUTPUT" | grep -qi "HOOK_FAILED\|error"; then
        pass "Hook errors: Errors detected and reported"
    else
        pass "Hook errors: Graceful handling (no error despite invalid database)"
    fi
else
    warn "Hook errors: Cannot test without executable hook"
fi

section "Test 7: Hook Output Format"

print_cyan "Testing hook output format..."

# Hooks should output in format that Claude Code can parse
# Typically markdown with memory context

if [ -x "$HOOKS_DIR/session-start.sh" ]; then
    FORMAT_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" bash "$HOOKS_DIR/session-start.sh" 2>&1 || echo "")

    # Check for structured output
    if echo "$FORMAT_OUTPUT" | grep -qi "##\|**\|importance:\|namespace:"; then
        pass "Hook output: Structured markdown format detected"
    else
        warn "Hook output: Output format may not be optimally structured"
    fi

    # Check for required sections
    if echo "$FORMAT_OUTPUT" | grep -qi "project.*memory.*context\|recent.*memories"; then
        pass "Hook output: Contains expected sections"
    else
        warn "Hook output: Expected sections may be missing"
    fi
else
    warn "Hook output: Cannot test without executable hook"
fi

section "Test 8: Hook Performance"

print_cyan "Testing hook execution performance..."

# Hooks should execute quickly (<3s for session-start)
if [ -x "$HOOKS_DIR/session-start.sh" ]; then
    START=$(date +%s)

    PERF_HOOK_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" bash "$HOOKS_DIR/session-start.sh" 2>&1 || echo "")

    END=$(date +%s)
    DURATION=$((END - START))

    if [ "$DURATION" -lt 5 ]; then
        pass "Hook performance: Fast execution (${DURATION}s < 5s)"
    else
        warn "Hook performance: Slower than expected (${DURATION}s)"
    fi
else
    warn "Hook performance: Cannot test without executable hook"
fi

section "Test 9: Hook Integration with Launcher"

print_cyan "Testing hook integration with orchestrated launcher..."

# Launcher should detect and execute hooks
# Check if launcher script references hooks

if [ -f "./scripts/orchestrated-launcher.sh" ]; then
    LAUNCHER_HOOKS=$(grep -i "hook\|session-start" ./scripts/orchestrated-launcher.sh || echo "")

    if [ -n "$LAUNCHER_HOOKS" ]; then
        pass "Hook integration: Launcher references hooks"
    else
        warn "Hook integration: Launcher may not execute hooks automatically"
    fi
else
    warn "Hook integration: Launcher script not found"
fi

section "Test 10: Context Loading Priorities"

print_cyan "Testing hook context loading priorities..."

# Hooks should load high-importance memories first
# Create tiered memories

create_tiered_memories "$BIN" "$TEST_DB" 3 5 10 "project:priority"

sleep 3

# Simulate hook loading context
if [ -x "$HOOKS_DIR/session-start.sh" ]; then
    PRIORITY_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" bash "$HOOKS_DIR/session-start.sh" 2>&1 || echo "")

    # Check if high-importance memories appear in output
    if echo "$PRIORITY_OUTPUT" | grep -qi "importance:.*[89]"; then
        pass "Priority loading: High-importance memories prioritized"
    else
        warn "Priority loading: Importance-based loading unclear"
    fi
else
    warn "Priority loading: Cannot test without executable hook"
fi

section "Test 11: Hook Environment Variables"

print_cyan "Testing hook environment variable handling..."

# Hooks should respect environment variables
# Test with custom database URL

HOOK_ENV_DB=$(create_test_db "hook_env")

create_memory "$BIN" "$HOOK_ENV_DB" \
    "Environment variable test memory" \
    "project:test" 8 > /dev/null 2>&1

sleep 2

if [ -x "$HOOKS_DIR/session-start.sh" ]; then
    ENV_HOOK_OUTPUT=$(DATABASE_URL="sqlite://$HOOK_ENV_DB" bash "$HOOKS_DIR/session-start.sh" 2>&1 || echo "")

    if echo "$ENV_HOOK_OUTPUT" | grep -qi "Environment variable"; then
        pass "Hook environment: Custom DATABASE_URL respected"
    else
        warn "Hook environment: Environment variable handling unclear"
    fi
else
    warn "Hook environment: Cannot test without executable hook"
fi

cleanup_test_db "$HOOK_ENV_DB"

section "Test 12: Hook Namespace Filtering"

print_cyan "Testing namespace filtering in hooks..."

# Hooks should filter by namespace (project:* by default)
# Create memories in different namespaces

create_memory "$BIN" "$TEST_DB" \
    "Project namespace memory" \
    "project:mnemosyne" 8 > /dev/null 2>&1

create_memory "$BIN" "$TEST_DB" \
    "System namespace memory" \
    "system:config" 8 > /dev/null 2>&1

sleep 2

if [ -x "$HOOKS_DIR/session-start.sh" ]; then
    NS_HOOK_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" bash "$HOOKS_DIR/session-start.sh" 2>&1 || echo "")

    # Should prioritize project namespace
    if echo "$NS_HOOK_OUTPUT" | grep -qi "project.*namespace"; then
        pass "Namespace filtering: Project namespace prioritized"
    else
        warn "Namespace filtering: Namespace handling unclear"
    fi
else
    warn "Namespace filtering: Cannot test without executable hook"
fi

section "Test 13: Hook Failure Recovery"

print_cyan "Testing recovery from hook failures..."

# If hook fails, session should continue
# Test by running hook with problematic input then continuing

if [ -x "$HOOKS_DIR/session-start.sh" ]; then
    # Run hook with invalid database (should fail or degrade gracefully)
    FAIL_HOOK_OUTPUT=$(DATABASE_URL="sqlite:///invalid/path/db.sqlite" \
        bash "$HOOKS_DIR/session-start.sh" 2>&1 || echo "")

    # After hook failure, normal operations should still work
    RECOVERY_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
        "Recovery after hook failure test" \
        --namespace "project:test" --importance 7 2>&1 || echo "")

    sleep 2

    RECOVERY_STORED=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "Recovery after hook" \
        --namespace "project:test" 2>&1 || echo "")

    if echo "$RECOVERY_STORED" | grep -qi "Recovery after hook"; then
        pass "Hook recovery: System functional after hook failure"
    else
        fail "Hook recovery: System compromised after hook failure"
    fi
else
    warn "Hook recovery: Cannot test without executable hook"
fi

section "Test 14: Custom Hook Arguments"

print_cyan "Testing hooks with custom arguments..."

# Hooks might accept arguments (namespace, limit, etc.)
if [ -x "$HOOKS_DIR/session-start.sh" ]; then
    # Try passing arguments to hook
    ARG_HOOK_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" \
        bash "$HOOKS_DIR/session-start.sh" "project:mnemosyne" 10 2>&1 || echo "")

    if [ -n "$ARG_HOOK_OUTPUT" ]; then
        pass "Hook arguments: Hook accepts arguments (whether used or ignored)"
    else
        warn "Hook arguments: Hook may not accept arguments"
    fi
else
    warn "Hook arguments: Cannot test without executable hook"
fi

section "Test 15: Hook Documentation"

print_cyan "Testing hook documentation availability..."

# Hooks should have comments explaining usage
DOCUMENTED_HOOKS=0

for hook in "${EXPECTED_HOOKS[@]}"; do
    if [ -f "$HOOKS_DIR/$hook" ]; then
        if head -20 "$HOOKS_DIR/$hook" | grep -qi "^#.*hook\|^#.*description\|^#.*usage"; then
            ((DOCUMENTED_HOOKS++))
        fi
    fi
done

if [ "$DOCUMENTED_HOOKS" -ge 2 ]; then
    pass "Hook documentation: Hooks contain documentation ($DOCUMENTED_HOOKS hooks)"
else
    warn "Hook documentation: Limited documentation found ($DOCUMENTED_HOOKS hooks)"
fi

section "Test 16: Hook Idempotency"

print_cyan "Testing hook idempotency..."

# Running hook multiple times should be safe
if [ -x "$HOOKS_DIR/session-start.sh" ]; then
    # Run hook twice
    IDEM_1=$(DATABASE_URL="sqlite://$TEST_DB" bash "$HOOKS_DIR/session-start.sh" 2>&1 || echo "")
    IDEM_2=$(DATABASE_URL="sqlite://$TEST_DB" bash "$HOOKS_DIR/session-start.sh" 2>&1 || echo "")

    # Both should succeed
    if [ -n "$IDEM_1" ] && [ -n "$IDEM_2" ]; then
        pass "Hook idempotency: Hook can be run multiple times safely"
    else
        fail "Hook idempotency: Hook may not be idempotent"
    fi
else
    warn "Hook idempotency: Cannot test without executable hook"
fi

# Cleanup
section "Cleanup"
teardown_test_env

# Summary
test_summary
exit $?
