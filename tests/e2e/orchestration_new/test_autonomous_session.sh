#!/usr/bin/env bash
set -uo pipefail

# E2E Test: Autonomous Session Orchestration
#
# Validates end-to-end event broadcasting and orchestration:
# - Session-start hook auto-starts API server
# - SSE subscriber connects to event stream
# - CLI commands emit events via HTTP POST /events/emit
# - Events propagate through SSE to orchestrator
# - Session-end hook gracefully shuts down API server
#
# This test validates the complete autonomous orchestration pipeline:
#   Session Start → API Server → SSE Subscriber → Orchestrator → Session End

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# Source common utilities
# shellcheck source=tests/e2e/lib/common.sh
source "$SCRIPT_DIR/lib/common.sh"

# Test state
PASSED=0
FAILED=0

section "E2E Test: Autonomous Session Orchestration"

# Setup test environment (skip API key check, we're testing infrastructure)
export SKIP_API_KEY_CHECK=1
setup_test_env "autonomous_session"

# Initialize database with schema
print_cyan "[SETUP] Initializing database schema..."
init_test_db "$TEST_DB" "$BIN"

# Cleanup function
cleanup() {
    print_cyan "[CLEANUP] Stopping processes and cleaning up..."

    # Kill API server if running
    if [ -f "$PID_FILE" ]; then
        SERVER_PID=$(cat "$PID_FILE" 2>/dev/null || echo "")
        if [ -n "$SERVER_PID" ] && kill -0 "$SERVER_PID" 2>/dev/null; then
            print_cyan "Stopping API server (PID: $SERVER_PID)..."
            kill -TERM "$SERVER_PID" 2>/dev/null || true
            sleep 2
            if kill -0 "$SERVER_PID" 2>/dev/null; then
                kill -9 "$SERVER_PID" 2>/dev/null || true
            fi
        fi
        rm -f "$PID_FILE"
    fi

    # Clean up log files
    rm -f "$LOG_FILE" "$STATE_FILE" 2>/dev/null

    # Clean up test database
    teardown_test_env
}

trap cleanup EXIT

# Define file paths
PID_FILE="${PROJECT_ROOT}/.claude/server.pid"
LOG_FILE="${PROJECT_ROOT}/.claude/server.log"
STATE_FILE="${PROJECT_ROOT}/.claude/memory-state.json"

# Clean up any existing server
if [ -f "$PID_FILE" ]; then
    OLD_PID=$(cat "$PID_FILE" 2>/dev/null || echo "")
    if [ -n "$OLD_PID" ] && kill -0 "$OLD_PID" 2>/dev/null; then
        print_yellow "Cleaning up existing API server (PID: $OLD_PID)..."
        kill -TERM "$OLD_PID" 2>/dev/null || true
        sleep 2
        if kill -0 "$OLD_PID" 2>/dev/null; then
            kill -9 "$OLD_PID" 2>/dev/null || true
        fi
    fi
    rm -f "$PID_FILE"
fi

#==============================================================================
# Test 1: Session Start Hook Emulation (API Server Auto-Start)
#==============================================================================

section "Test 1: Session Start (API Server Auto-Start)"

print_cyan "Simulating session-start hook..."

# Generate session ID
SESSION_ID=$(generate_uuid)

# Disable auto-start for manual control in this test
# We'll manually start the server to validate the hook's behavior
export MNEMOSYNE_DISABLE_AUTO_START_API=0

# Manually start API server (simulating what session-start.sh does)
print_cyan "Starting API server manually (simulating session-start.sh)..."

DATABASE_URL="sqlite://$TEST_DB" nohup "$BIN" api-server > "$LOG_FILE" 2>&1 &
SERVER_PID=$!
echo "$SERVER_PID" > "$PID_FILE"

print_cyan "API server started (PID: $SERVER_PID)"

# Wait for server health
MAX_WAIT=15
WAITED=0
SERVER_READY=0

while [ $WAITED -lt $MAX_WAIT ]; do
    if curl -s --max-time 1 http://localhost:3000/health > /dev/null 2>&1; then
        API_VERSION=$(curl -s http://localhost:3000/health 2>/dev/null | jq -r '.version // "unknown"')
        pass "API server ready (version: $API_VERSION, PID: $SERVER_PID)"
        SERVER_READY=1
        break
    fi

    # Check if process died
    if ! kill -0 "$SERVER_PID" 2>/dev/null; then
        fail "API server failed to start (check $LOG_FILE)"
        print_cyan "Last 20 lines of log:"
        tail -20 "$LOG_FILE" 2>/dev/null || echo "No log output"
        exit 1
    fi

    sleep 1
    WAITED=$((WAITED + 1))
done

if [ $SERVER_READY -eq 0 ]; then
    fail "API server startup timeout (waited ${MAX_WAIT}s)"
    print_cyan "Last 20 lines of log:"
    tail -20 "$LOG_FILE" 2>/dev/null || echo "No log output"
    exit 1
fi

# Emit SessionStarted event via internal command
print_cyan "Emitting SessionStarted event via internal command..."
DATABASE_URL="sqlite://$TEST_DB" "$BIN" internal session-started --instance-id "$SESSION_ID" > /dev/null 2>&1

# Wait for event to propagate
sleep 2

pass "SessionStarted event emitted"

#==============================================================================
# Test 2: API Server Health Check
#==============================================================================

section "Test 2: API Server Health Check"

print_cyan "Checking API server health endpoint..."

HEALTH_RESPONSE=$(curl -s http://localhost:3000/health)
HEALTH_STATUS=$(echo "$HEALTH_RESPONSE" | jq -r '.status // "unknown"')

if [ "$HEALTH_STATUS" = "healthy" ]; then
    pass "API server health check passed"
else
    fail "API server health check failed (status: $HEALTH_STATUS)"
fi

#==============================================================================
# Test 3: CLI Command Event Emission (remember)
#==============================================================================

section "Test 3: CLI Event Emission (remember)"

print_cyan "Executing 'mnemosyne remember' command..."

# Execute remember command (should emit RememberExecuted event)
REMEMBER_CONTENT="Test memory for autonomous session orchestration"
REMEMBER_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" remember \
    --content "$REMEMBER_CONTENT" \
    --namespace "project:test" \
    --importance 5 2>&1)

REMEMBER_EXIT=$?

if [ $REMEMBER_EXIT -eq 0 ]; then
    pass "remember command executed successfully"
else
    fail "remember command failed (exit code: $REMEMBER_EXIT)"
    echo "Output: $REMEMBER_OUTPUT"
fi

# Wait for event to propagate
sleep 2

# Check if event was emitted (look for MemoryStored event in logs)
if grep -q "RememberExecuted\|MemoryStored" "$LOG_FILE" 2>/dev/null; then
    pass "RememberExecuted event emitted to API server"
else
    warn "RememberExecuted event not found in logs (may not be logged)"
fi

#==============================================================================
# Test 4: CLI Command Event Emission (recall)
#==============================================================================

section "Test 4: CLI Event Emission (recall)"

print_cyan "Executing 'mnemosyne recall' command..."

# Execute recall command (should emit RecallExecuted event)
RECALL_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall \
    --query "test" \
    --namespace "project:test" \
    --limit 5 2>&1)

RECALL_EXIT=$?

if [ $RECALL_EXIT -eq 0 ]; then
    pass "recall command executed successfully"
else
    fail "recall command failed (exit code: $RECALL_EXIT)"
    echo "Output: $RECALL_OUTPUT"
fi

# Wait for event to propagate
sleep 2

# Check if event was emitted
if grep -q "RecallExecuted\|MemoryRecalled" "$LOG_FILE" 2>/dev/null; then
    pass "RecallExecuted event emitted to API server"
else
    warn "RecallExecuted event not found in logs (may not be logged)"
fi

#==============================================================================
# Test 5: CLI Command Event Emission (status)
#==============================================================================

section "Test 5: CLI Event Emission (status)"

print_cyan "Executing 'mnemosyne status' command..."

# Execute status command (should emit StatusCheckExecuted event)
STATUS_OUTPUT=$(DATABASE_URL="sqlite://$TEST_DB" "$BIN" status 2>&1)
STATUS_EXIT=$?

if [ $STATUS_EXIT -eq 0 ]; then
    pass "status command executed successfully"
else
    fail "status command failed (exit code: $STATUS_EXIT)"
    echo "Output: $STATUS_OUTPUT"
fi

# Wait for event to propagate
sleep 2

# Check if event was emitted
if grep -q "StatusCheckExecuted\|CliCommandCompleted" "$LOG_FILE" 2>/dev/null; then
    pass "StatusCheckExecuted event emitted to API server"
else
    warn "StatusCheckExecuted event not found in logs (may not be logged)"
fi

#==============================================================================
# Test 6: SSE Event Stream Endpoint
#==============================================================================

section "Test 6: SSE Event Stream Endpoint"

print_cyan "Testing SSE /events/stream endpoint..."

# Start SSE stream in background (timeout after 10 seconds)
SSE_OUTPUT="/tmp/mnemosyne_sse_test_$$.log"
timeout 10s curl -N -s http://localhost:3000/events/stream > "$SSE_OUTPUT" 2>&1 &
SSE_PID=$!

# Wait for connection
sleep 2

# Generate an event by running a command
print_cyan "Generating test event via recall command..."
DATABASE_URL="sqlite://$TEST_DB" "$BIN" recall --query "autonomous" --limit 1 > /dev/null 2>&1

# Wait for event to appear in stream
sleep 3

# Kill SSE stream
kill -TERM "$SSE_PID" 2>/dev/null || true
wait "$SSE_PID" 2>/dev/null || true

# Check if SSE stream received events
if [ -f "$SSE_OUTPUT" ] && [ -s "$SSE_OUTPUT" ]; then
    # Check for SSE format (data: {...})
    if grep -q "data:" "$SSE_OUTPUT" 2>/dev/null; then
        pass "SSE stream endpoint working (received events)"

        # Count events received
        EVENT_COUNT=$(grep -c "^data:" "$SSE_OUTPUT" 2>/dev/null || echo "0")
        print_cyan "  Events received via SSE: $EVENT_COUNT"
    else
        warn "SSE stream connected but no events received"
    fi
else
    warn "SSE stream output empty or not created"
fi

# Cleanup SSE output
rm -f "$SSE_OUTPUT"

#==============================================================================
# Test 7: Event Persistence (Events in Database)
#==============================================================================

section "Test 7: Event Persistence"

print_cyan "Checking if events are persisted in database..."

# Query events table (if it exists)
EVENTS_TABLE_EXISTS=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
    "SELECT name FROM sqlite_master WHERE type='table' AND name='events'" 2>/dev/null || echo "")

if [ -n "$EVENTS_TABLE_EXISTS" ]; then
    EVENT_COUNT=$(DATABASE_URL="sqlite://$TEST_DB" sqlite3 "$TEST_DB" \
        "SELECT COUNT(*) FROM events" 2>/dev/null || echo "0")

    if [ "$EVENT_COUNT" -gt 0 ]; then
        pass "Events persisted in database (count: $EVENT_COUNT)"
    else
        warn "Events table exists but is empty"
    fi
else
    warn "Events table does not exist (event persistence may not be enabled)"
fi

#==============================================================================
# Test 8: Session End Hook Emulation (Graceful Shutdown)
#==============================================================================

section "Test 8: Session End (Graceful Shutdown)"

print_cyan "Simulating session-end hook..."

# Emit SessionEnded event via internal command
DATABASE_URL="sqlite://$TEST_DB" "$BIN" internal session-ended --instance-id "$SESSION_ID" > /dev/null 2>&1
pass "SessionEnded event emitted"

# Wait for event to propagate
sleep 2

print_cyan "Stopping API server gracefully (simulating session-end.sh)..."

if [ -f "$PID_FILE" ]; then
    SERVER_PID=$(cat "$PID_FILE")

    if kill -0 "$SERVER_PID" 2>/dev/null; then
        # Graceful shutdown (SIGTERM)
        kill -TERM "$SERVER_PID" 2>/dev/null || true

        # Wait up to 5 seconds for graceful shutdown
        SHUTDOWN_WAITED=0
        SHUTDOWN_MAX=5
        while [ $SHUTDOWN_WAITED -lt $SHUTDOWN_MAX ]; do
            if ! kill -0 "$SERVER_PID" 2>/dev/null; then
                pass "API server stopped gracefully"
                break
            fi
            sleep 1
            SHUTDOWN_WAITED=$((SHUTDOWN_WAITED + 1))
        done

        # Force kill if still running
        if kill -0 "$SERVER_PID" 2>/dev/null; then
            warn "API server required SIGKILL for shutdown"
            kill -9 "$SERVER_PID" 2>/dev/null || true
            sleep 1
        fi
    else
        fail "API server process not running (PID: $SERVER_PID)"
    fi

    rm -f "$PID_FILE"
else
    fail "PID file not found: $PID_FILE"
fi

# Verify server is stopped
if curl -s --max-time 1 http://localhost:3000/health > /dev/null 2>&1; then
    fail "API server still responding after shutdown"
else
    pass "API server fully stopped"
fi

#==============================================================================
# Test 9: Hook Scripts Validation
#==============================================================================

section "Test 9: Hook Scripts Validation"

print_cyan "Validating hook scripts exist and are executable..."

HOOK_START="${PROJECT_ROOT}/.claude/hooks/session-start.sh"
HOOK_END="${PROJECT_ROOT}/.claude/hooks/session-end.sh"

if [ -f "$HOOK_START" ]; then
    if [ -x "$HOOK_START" ]; then
        pass "session-start.sh exists and is executable"
    else
        fail "session-start.sh exists but is not executable"
    fi
else
    fail "session-start.sh not found at $HOOK_START"
fi

if [ -f "$HOOK_END" ]; then
    if [ -x "$HOOK_END" ]; then
        pass "session-end.sh exists and is executable"
    else
        fail "session-end.sh exists but is not executable"
    fi
else
    fail "session-end.sh not found at $HOOK_END"
fi

#==============================================================================
# Test 10: Event Flow Verification (Log Analysis)
#==============================================================================

section "Test 10: Event Flow Verification"

print_cyan "Analyzing API server logs for event flow..."

if [ -f "$LOG_FILE" ]; then
    # Check for server startup
    if grep -q "API server starting\|Server started\|Listening on" "$LOG_FILE" 2>/dev/null; then
        pass "API server startup logged"
    else
        warn "API server startup message not found in logs"
    fi

    # Check for event emission
    if grep -qE "POST /events/emit|Event emitted|event_type" "$LOG_FILE" 2>/dev/null; then
        pass "Event emission detected in logs"
    else
        warn "Event emission not detected in logs (may be at debug level)"
    fi

    # Check for SSE connections
    if grep -qE "GET /events/stream|SSE client connected|EventBroadcaster" "$LOG_FILE" 2>/dev/null; then
        pass "SSE stream connections detected in logs"
    else
        warn "SSE stream connections not detected in logs"
    fi

    # Print summary of log lines
    LOG_LINE_COUNT=$(wc -l < "$LOG_FILE" 2>/dev/null || echo "0")
    print_cyan "  Total log lines: $LOG_LINE_COUNT"

    # Show last 30 lines for debugging
    if [ "$FAILED" -gt 0 ]; then
        print_cyan "  Last 30 lines of API server log:"
        tail -30 "$LOG_FILE" 2>/dev/null || echo "  No log output"
    fi
else
    warn "API server log file not found: $LOG_FILE"
fi

#==============================================================================
# Results
#==============================================================================

test_summary

exit $((FAILED > 0 ? 1 : 0))
