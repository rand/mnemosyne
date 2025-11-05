#!/bin/bash
# On-stop hook: Validate memory usage before finishing response
# Triggers when Claude Code finishes responding

set -e

STATE_FILE=".claude/memory-state.json"

# If state file doesn't exist, skip validation
if [ ! -f "$STATE_FILE" ]; then
    exit 0
fi

DEBT=$(jq '.memory_debt' < "$STATE_FILE" 2>/dev/null || echo "0")
COUNT=$(jq '.memories_stored_count' < "$STATE_FILE" 2>/dev/null || echo "0")

# Only show reminder if there's an issue AND debug mode enabled
if [ "${CC_HOOK_DEBUG:-0}" = "1" ]; then
  if [ "$DEBT" -gt 0 ] || [ "$COUNT" -eq 0 ]; then
    cat >&2 <<EOF

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
📊 Session Memory Status
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Memories stored: $COUNT
Memory debt: $DEBT events

EOF

    if [ "$COUNT" -eq 0 ]; then
      cat >&2 <<EOF
⚠️  No memories stored this session.
   Consider storing key learnings before ending session.
EOF
    fi

    if [ "$DEBT" -gt 0 ]; then
      cat >&2 <<EOF
⚠️  Memory debt exists.
   You must store memories before git push or PR creation.
EOF
    fi

    cat >&2 <<EOF

Store memories with:
  mnemosyne remember -c "..." -n "project:mnemosyne" -i 7-10

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
EOF
  fi
fi

exit 0
