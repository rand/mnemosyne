#!/bin/bash
# Mnemosyne session-end hook
# Gracefully shuts down API server and emits SessionEnded event
#
# This hook is called when Claude Code session ends.
# Uses the safe-shutdown script to prevent PTY corruption.

set -e

# Get project directory
PROJECT_DIR="$(pwd)"

# Get session ID from state file
STATE_FILE=".claude/memory-state.json"
if [ -f "$STATE_FILE" ]; then
    SESSION_ID=$(jq -r '.session_id // "unknown"' < "$STATE_FILE" 2>/dev/null || echo "unknown")
else
    SESSION_ID="unknown"
fi

# Get mnemosyne binary path
if command -v mnemosyne &> /dev/null; then
    MNEMOSYNE_BIN="mnemosyne"
elif [ -f "${PROJECT_DIR}/target/release/mnemosyne" ]; then
    MNEMOSYNE_BIN="${PROJECT_DIR}/target/release/mnemosyne"
elif [ -f "${PROJECT_DIR}/target/debug/mnemosyne" ]; then
    MNEMOSYNE_BIN="${PROJECT_DIR}/target/debug/mnemosyne"
else
    [ -n "$CC_HOOK_DEBUG" ] && echo "[DEBUG] Mnemosyne binary not found, skipping session-end event" >&2
    exit 0
fi

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Emit SessionEnded Event
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

# Emit SessionEnded event before shutting down API server
[ -n "$CC_HOOK_DEBUG" ] && echo "[DEBUG] Emitting SessionEnded event (session: $SESSION_ID)" >&2

"$MNEMOSYNE_BIN" internal session-ended --instance-id "$SESSION_ID" < /dev/null 2>/dev/null || true

# Small delay to ensure event is processed
sleep 1

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Safe Shutdown of API Server
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

# Check if safe-shutdown script exists
SAFE_SHUTDOWN_SCRIPT="${PROJECT_DIR}/scripts/safe-shutdown.sh"

if [ -f "$SAFE_SHUTDOWN_SCRIPT" ]; then
    [ -n "$CC_HOOK_DEBUG" ] && echo "[DEBUG] Running safe-shutdown script..." >&2

    # Run safe-shutdown with 3 second timeout (faster for session end)
    "$SAFE_SHUTDOWN_SCRIPT" --wait 3 2>&1 | while read line; do
        [ -n "$CC_HOOK_DEBUG" ] && echo "[SHUTDOWN] $line" >&2
    done
else
    # Fallback: Manual shutdown if script doesn't exist
    [ -n "$CC_HOOK_DEBUG" ] && echo "[DEBUG] safe-shutdown.sh not found, using manual shutdown" >&2

    PID_FILE=".claude/server.pid"

    if [ -f "$PID_FILE" ]; then
        SERVER_PID=$(cat "$PID_FILE")

        if ps -p "$SERVER_PID" > /dev/null 2>&1; then
            [ -n "$CC_HOOK_DEBUG" ] && echo "[DEBUG] Stopping API server (PID: $SERVER_PID)" >&2

            # Graceful shutdown
            kill -TERM "$SERVER_PID" 2>/dev/null || true
            sleep 2

            # Force kill if still running
            if ps -p "$SERVER_PID" > /dev/null 2>&1; then
                kill -9 "$SERVER_PID" 2>/dev/null || true
            fi

            echo "✓ API server stopped" >&2
        fi

        rm -f "$PID_FILE"
    else
        [ -n "$CC_HOOK_DEBUG" ] && echo "[DEBUG] No PID file found" >&2
    fi
fi

# Clean up state file
if [ -f "$STATE_FILE" ]; then
    rm -f "$STATE_FILE"
    [ -n "$CC_HOOK_DEBUG" ] && echo "[DEBUG] Cleaned up memory state file" >&2
fi

[ -n "$CC_HOOK_DEBUG" ] && echo "[DEBUG] Session end hook complete" >&2

exit 0
