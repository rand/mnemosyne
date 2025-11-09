#!/bin/bash
# Mnemosyne session-start hook v2.0
# Loads project memory context and initializes memory state tracking
#
# Output behavior:
# - All context goes to Claude via JSON stdout
# - Status messages only shown if CC_HOOK_DEBUG=1
# - User sees clean startup (no terminal noise)

set -e

# Hook version for debugging
HOOK_VERSION="3.0"

# Get project directory
PROJECT_DIR="$(pwd)"
PROJECT_NAME="$(basename "$PROJECT_DIR")"

# Initialize memory state file
STATE_FILE=".claude/memory-state.json"
SESSION_ID=$(uuidgen < /dev/null)

cat > "$STATE_FILE" <<EOF
{
  "session_id": "$SESSION_ID",
  "memories_stored_count": 0,
  "last_memory_timestamp": null,
  "significant_events": [],
  "memory_debt": 0,
  "session_start": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "last_recall": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
EOF

# Will output single status line after memory loading

# Get mnemosyne binary path
# Try installed binary first, fall back to local build
if command -v mnemosyne &> /dev/null; then
    MNEMOSYNE_BIN="mnemosyne"
elif [ -f "${PROJECT_DIR}/target/release/mnemosyne" ]; then
    MNEMOSYNE_BIN="${PROJECT_DIR}/target/release/mnemosyne"
elif [ -f "${PROJECT_DIR}/target/debug/mnemosyne" ]; then
    MNEMOSYNE_BIN="${PROJECT_DIR}/target/debug/mnemosyne"
else
    echo "ℹ️  Mnemosyne not installed. Install with './install.sh' or build with 'cargo build --release'" >&2
    exit 0
fi

# Load recent memories for this project
NAMESPACE="project:${PROJECT_NAME}"

# Get recent important memories with proper namespace isolation
# Query syntax: Space-separated keywords (NOT "OR" operators)
# Namespace format: "project:name" gets parsed to {"type":"project","name":"name"}
MEMORIES=$("$MNEMOSYNE_BIN" recall \
    --query "$PROJECT_NAME project architecture implementation decisions hooks" \
    --namespace "$NAMESPACE" \
    --limit 10 \
    --min-importance 7 \
    --format json < /dev/null 2>/dev/null || echo '{"results": []}')

# Count memories
MEMORY_COUNT=$(echo "$MEMORIES" | jq -r '.results | length' 2>/dev/null || echo "0")
HIGH_IMPORTANCE_COUNT=$(echo "$MEMORIES" | jq -r '[.results[] | select(.importance >= 8)] | length' 2>/dev/null || echo "0")

if [ "$MEMORY_COUNT" -gt 0 ]; then
    # User-visible status line (stderr)
    echo "🧠 Loaded $MEMORY_COUNT memories ($HIGH_IMPORTANCE_COUNT critical)" >&2

    # Build context string for Claude
    CRITICAL_MEMORIES=$(echo "$MEMORIES" | jq -r '.results[] | select(.importance >= 8) | "**\(.summary)** — \(.memory_type) — \(.tags | join(", "))\n\(.content)\n\n---\n"' 2>/dev/null)
    IMPORTANT_MEMORIES=$(echo "$MEMORIES" | jq -r '.results[] | select(.importance == 7) | "- **\(.summary)** (\(.memory_type))"' 2>/dev/null)
    LINK_COUNT=$(echo "$MEMORIES" | jq -r '[.results[].related_memories // [] | length] | add // 0' 2>/dev/null || echo "0")

    # Build full context
    CONTEXT="# Project Context: $PROJECT_NAME

## Critical Memories (Importance ≥ 8)

$CRITICAL_MEMORIES

## Important Memories (Importance 7)

$IMPORTANT_MEMORIES

## Knowledge Graph
$LINK_COUNT semantic connections across $MEMORY_COUNT memories

---
*Context from Mnemosyne • $MEMORY_COUNT memories loaded*"

    # Output JSON with suppressOutput to hide from terminal
    jq -n \
        --arg context "$CONTEXT" \
        '{
            "hookSpecificOutput": {
                "hookEventName": "SessionStart",
                "additionalContext": $context
            },
            "suppressOutput": true
        }' < /dev/null
else
    # User-visible status line (stderr)
    echo "🧠 No memories found (building context)" >&2

    # Build context for starting project
    CONTEXT="# Project Context: $PROJECT_NAME

No important memories found for this project yet.

**Start building project memory**:
\`\`\`bash
mnemosyne remember -c \"Your insight or decision\" \\
  -n \"$NAMESPACE\" -i 7-10 -t \"architecture,decision\"
\`\`\`

Memory enforcement is active. Store memories to avoid blocking later."

    # Output JSON with suppressOutput
    jq -n \
        --arg context "$CONTEXT" \
        '{
            "hookSpecificOutput": {
                "hookEventName": "SessionStart",
                "additionalContext": $context
            },
            "suppressOutput": true
        }' < /dev/null
fi

# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Auto-Start API Server for Event Broadcasting
# ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

# Check if auto-start is disabled
if [ -n "$MNEMOSYNE_DISABLE_AUTO_START_API" ]; then
    [ -n "$CC_HOOK_DEBUG" ] && echo "[DEBUG] API auto-start disabled via MNEMOSYNE_DISABLE_AUTO_START_API" >&2
    exit 0
fi

# Check if API server is already running
if curl -s --max-time 1 http://localhost:3000/health > /dev/null 2>&1; then
    API_STATUS=$(curl -s http://localhost:3000/health 2>/dev/null | jq -r '.status // "unknown"')
    [ -n "$CC_HOOK_DEBUG" ] && echo "[DEBUG] API server already running (status: $API_STATUS)" >&2

    # Emit SessionStarted event via existing API
    if command -v "$MNEMOSYNE_BIN" &> /dev/null; then
        "$MNEMOSYNE_BIN" internal session-started --instance-id "$SESSION_ID" < /dev/null 2>/dev/null || true
    fi
    exit 0
fi

# API server not running - start it
[ -n "$CC_HOOK_DEBUG" ] && echo "[DEBUG] Starting API server..." >&2

# Start API server in background with nohup
PID_FILE=".claude/server.pid"
LOG_FILE=".claude/server.log"

# Clean up old PID file if process no longer exists
if [ -f "$PID_FILE" ]; then
    OLD_PID=$(cat "$PID_FILE")
    if ! ps -p "$OLD_PID" > /dev/null 2>&1; then
        rm -f "$PID_FILE"
        [ -n "$CC_HOOK_DEBUG" ] && echo "[DEBUG] Cleaned up stale PID file (old PID: $OLD_PID)" >&2
    fi
fi

# Start server
nohup "$MNEMOSYNE_BIN" api-server > "$LOG_FILE" 2>&1 &
SERVER_PID=$!
echo "$SERVER_PID" > "$PID_FILE"

[ -n "$CC_HOOK_DEBUG" ] && echo "[DEBUG] Started API server (PID: $SERVER_PID)" >&2

# Wait for server to be ready (up to 10 seconds)
MAX_WAIT=10
WAITED=0
while [ $WAITED -lt $MAX_WAIT ]; do
    if curl -s --max-time 1 http://localhost:3000/health > /dev/null 2>&1; then
        API_VERSION=$(curl -s http://localhost:3000/health 2>/dev/null | jq -r '.version // "unknown"')
        echo "✓ API server ready (v$API_VERSION, PID: $SERVER_PID)" >&2

        # Emit SessionStarted event
        "$MNEMOSYNE_BIN" internal session-started --instance-id "$SESSION_ID" < /dev/null 2>/dev/null || true

        exit 0
    fi

    # Check if process died
    if ! ps -p "$SERVER_PID" > /dev/null 2>&1; then
        echo "✗ API server failed to start (check $LOG_FILE)" >&2
        rm -f "$PID_FILE"
        exit 1
    fi

    sleep 1
    WAITED=$((WAITED + 1))
done

# Timeout waiting for server
echo "⚠ API server started but not responding (PID: $SERVER_PID, check $LOG_FILE)" >&2
exit 0
