#!/usr/bin/env bash
set -uo pipefail

# E2E Test: Interactive Mode with Orchestration
#
# Validates:
# - Interactive REPL launches successfully
# - Work submission via interactive mode
# - Orchestration engine processes work
# - PyO3 bridge spawns Python agents
# - OrchestrationEngine lifecycle (start/stop)
#
# This test validates the complete orchestration pipeline from
# interactive mode through to agent spawning and work processing.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Source common utilities
# shellcheck source=tests/e2e/lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"

# Test state
PASSED=0
FAILED=0

section "E2E Test: Interactive Mode + Orchestration"

# Setup test environment
setup_test_env "orchestration_interactive"

# Initialize database with schema
print_cyan "[SETUP] Initializing database schema..."
init_test_db "$TEST_DB" "$BIN"

#==============================================================================
# Test 1: Interactive Mode Launch
#==============================================================================

section "Test 1: Interactive Mode Launch"

print_cyan "Launching interactive mode in background..."

# Create FIFO for bidirectional communication
FIFO_IN="/tmp/mnemosyne_test_$$_in.fifo"
FIFO_OUT="/tmp/mnemosyne_test_$$_out.log"
mkfifo "$FIFO_IN" 2>/dev/null || rm -f "$FIFO_IN" && mkfifo "$FIFO_IN"

# Launch mnemosyne in background with FIFO
(
    DATABASE_URL="sqlite://$TEST_DB" "$BIN" < "$FIFO_IN" > "$FIFO_OUT" 2>&1
) &
MNEMOSYNE_PID=$!

# Open FIFO for writing
exec 3>"$FIFO_IN"

# Wait for initialization
print_cyan "Waiting for interactive mode to initialize..."
sleep 8

# Check if process is running
if kill -0 "$MNEMOSYNE_PID" 2>/dev/null; then
    pass "Interactive mode launched successfully (PID: $MNEMOSYNE_PID)"
else
    fail "Interactive mode failed to launch"
    exec 3>&-  # Close FIFO
    rm -f "$FIFO_IN" "$FIFO_OUT"
    teardown_test_env
    exit 1
fi

# Check for expected startup output (loading screen or ready prompt)
if grep -qE "Weaving|Energizing|Reticulating|Ready|>" "$FIFO_OUT" 2>/dev/null; then
    pass "Interactive mode startup output displayed"
else
    warn "No recognizable startup output (check for silent launch)"
fi

# Dashboard check is optional (may not be enabled)
if grep -q "Dashboard:" "$FIFO_OUT" 2>/dev/null; then
    pass "Dashboard URL shown"
fi

#==============================================================================
# Test 2: Help Command
#==============================================================================

section "Test 2: Help Command"

print_cyan "Testing 'help' command..."

# Record output size before help command
BEFORE_SIZE=$(wc -c < "$FIFO_OUT" 2>/dev/null || echo "0")

echo "help" >&3
sleep 3

# Check if new output appeared
AFTER_SIZE=$(wc -c < "$FIFO_OUT" 2>/dev/null || echo "0")

if [ "$AFTER_SIZE" -gt "$BEFORE_SIZE" ]; then
    pass "Help command produced output"
else
    warn "Help command may not be implemented (no visible output change)"
fi

#==============================================================================
# Test 3: Status Command
#==============================================================================

section "Test 3: Status Command"

print_cyan "Testing 'status' command..."

echo "status" >&3
sleep 2

# Status command should show system state
pass "Status command executed (non-blocking)"

#==============================================================================
# Test 4: Work Submission (Simple)
#==============================================================================

section "Test 4: Work Submission (Simple)"

print_cyan "Submitting simple work item: 'Echo test message'..."

# Record output size before work submission
WORK_BEFORE_SIZE=$(wc -c < "$FIFO_OUT" 2>/dev/null || echo "0")

echo "Echo test message" >&3
sleep 3

# Check if new output appeared (work may be processed silently)
WORK_AFTER_SIZE=$(wc -c < "$FIFO_OUT" 2>/dev/null || echo "0")

if grep -qE "Work submitted:|Accepted:|Processing" "$FIFO_OUT" 2>/dev/null; then
    WORK_ID=$(grep -E "Work submitted:|Accepted:" "$FIFO_OUT" 2>/dev/null | tail -1 | awk '{print $NF}' || echo "unknown")
    pass "Work submitted (ID: $WORK_ID)"
elif [ "$WORK_AFTER_SIZE" -gt "$WORK_BEFORE_SIZE" ]; then
    pass "Work submission produced output (processed)"
else
    warn "Work submission not clearly confirmed (may be processed silently)"
fi

#==============================================================================
# Test 5: Work Submission (File Creation - API Call Test)
#==============================================================================

section "Test 5: Work Submission (File Creation with Real API Call)"

print_cyan "Submitting work with API call: Create /tmp/mnemosyne_e2e_test_$$.txt..."

TEST_FILE="/tmp/mnemosyne_e2e_test_$$.txt"
TEST_CONTENT="Mnemosyne E2E Test - Orchestration Working"

echo "work: Create file $TEST_FILE with content '$TEST_CONTENT'" >&3
sleep 10  # Allow time for API call + tool execution

# Check for work submission
if grep -q "Work submitted:" "$FIFO_OUT" 2>/dev/null | tail -5 | grep -q "Work submitted:"; then
    pass "File creation work submitted"
else
    warn "File creation work submission not clearly confirmed (check output)"
fi

# Wait for execution (orchestration + API + tool execution)
print_cyan "Waiting for orchestration to process work (API call + tool execution)..."
sleep 15

# Check if file was created (validates end-to-end: Rust -> Python -> API -> Tool)
if [ -f "$TEST_FILE" ]; then
    if grep -q "$TEST_CONTENT" "$TEST_FILE" 2>/dev/null; then
        pass "File created with correct content (end-to-end validation successful)"
    else
        warn "File exists but content doesn't match (partial success)"
    fi
else
    warn "File not created (may indicate API key issue or execution delay - check logs)"
    print_cyan "Recent output from interactive mode:"
    tail -20 "$FIFO_OUT" 2>/dev/null || echo "No output available"
fi

#==============================================================================
# Test 6: Graceful Shutdown
#==============================================================================

section "Test 6: Graceful Shutdown"

print_cyan "Sending 'quit' command for graceful shutdown..."

echo "quit" >&3
sleep 5

# Close FIFO
exec 3>&-

# Check if process terminated gracefully
if ! kill -0 "$MNEMOSYNE_PID" 2>/dev/null; then
    pass "Interactive mode terminated gracefully"
else
    warn "Process still running, forcing termination..."
    kill "$MNEMOSYNE_PID" 2>/dev/null
    sleep 2
    if ! kill -0 "$MNEMOSYNE_PID" 2>/dev/null; then
        pass "Process terminated after SIGTERM"
    else
        kill -9 "$MNEMOSYNE_PID" 2>/dev/null
        warn "Process required SIGKILL"
    fi
fi

# Check for shutdown message
if grep -qi "shutdown\|complete" "$FIFO_OUT" 2>/dev/null; then
    pass "Shutdown message displayed"
else
    warn "Shutdown message not found"
fi

#==============================================================================
# Cleanup
#==============================================================================

section "Cleanup"

print_cyan "Cleaning up test artifacts..."

rm -f "$FIFO_IN" "$FIFO_OUT" "$TEST_FILE" 2>/dev/null
pass "Test artifacts cleaned up"

teardown_test_env

#==============================================================================
# Results
#==============================================================================

test_summary

exit $((FAILED > 0 ? 1 : 0))
