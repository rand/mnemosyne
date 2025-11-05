#!/bin/bash
# Post-tool-use hook: Track significant events and enforce memory storage
# Triggers after Edit, Write, and other significant actions

set -e

TOOL_USE="$1"  # e.g., "Edit", "Write", "Bash(commit)"
STATE_FILE=".claude/memory-state.json"

# Initialize state file if it doesn't exist
if [ ! -f "$STATE_FILE" ]; then
    cat > "$STATE_FILE" <<EOF
{
  "session_id": "$(uuidgen)",
  "memories_stored_count": 0,
  "last_memory_timestamp": null,
  "significant_events": [],
  "memory_debt": 0,
  "session_start": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "last_recall": null
}
EOF
fi

# Check if this is a significant event
case "$TOOL_USE" in
  Edit|Write|Bash*commit*)
    # Increment memory debt
    DEBT=$(jq '.memory_debt' "$STATE_FILE" 2>/dev/null || echo "0")
    DEBT=$((DEBT + 1))

    # Update state
    TMP_FILE=$(mktemp)
    jq ".memory_debt = $DEBT | .significant_events += [{\"type\": \"$TOOL_USE\", \"timestamp\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\", \"has_memory\": false}]" "$STATE_FILE" > "$TMP_FILE"
    mv "$TMP_FILE" "$STATE_FILE"

    # If debt >= 3, inject urgent prompt (only visible in debug mode)
    if [ "${CC_HOOK_DEBUG:-0}" = "1" ]; then
      if [ "$DEBT" -ge 3 ]; then
        cat >&2 <<EOF

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
⚠️  MEMORY DEBT ALERT: $DEBT significant actions without memories
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

You have made $DEBT significant changes without storing memories.
This violates the mandatory memory protocol.

REQUIRED ACTION:
Store memories about your recent work using:

  mnemosyne remember -c "What you learned" \\
    -n "project:mnemosyne" -i 7-10 -t "lesson,insight"

Recent unrecorded events:
EOF
        jq -r '.significant_events[] | select(.has_memory == false) | "  • \(.type) at \(.timestamp)"' "$STATE_FILE" | tail -5 >&2

        cat >&2 <<EOF

⚠️  You cannot git push or create PRs until debt is cleared.
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
EOF
      elif [ "$DEBT" -ge 1 ]; then
        echo "💭 Memory debt: $DEBT (store memories to avoid blocking later)" >&2
      fi
    fi
    ;;
esac

exit 0
