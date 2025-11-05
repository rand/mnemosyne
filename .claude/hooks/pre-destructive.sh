#!/bin/bash
# Pre-destructive hook: Block destructive actions if memory debt exists
# Triggers BEFORE git push, PR creation, and other destructive actions
# Returns non-zero exit code to BLOCK the action

set -e

STATE_FILE=".claude/memory-state.json"

# If state file doesn't exist, allow (no debt yet)
if [ ! -f "$STATE_FILE" ]; then
    exit 0
fi

DEBT=$(jq '.memory_debt' "$STATE_FILE" 2>/dev/null || echo "0")

if [ "$DEBT" -gt 0 ]; then
  # Only show blocking message if debug mode enabled
  if [ "${CC_HOOK_DEBUG:-0}" = "1" ]; then
    cat >&2 <<EOF

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🚫 BLOCKED: Cannot proceed with destructive action
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

Memory Debt: $DEBT events without corresponding memories.

WHY BLOCKED:
You cannot push code or create PRs without clearing memory debt.
This ensures all significant work is captured for future sessions.

TO UNBLOCK:
1. Store memories about your recent work:

   mnemosyne remember -c "What you learned/decided/discovered" \\
     -n "project:mnemosyne" -i 7-10 -t "lesson,insight"

2. Repeat for each major change or decision
3. Retry this action

WHAT TO REMEMBER:
• What did you learn?
• What mistakes did you avoid (or make)?
• What decisions did you make and why?
• What gotchas or insights emerged?
• What patterns or anti-patterns did you discover?

Recent unrecorded events:
EOF
    jq -r '.significant_events[] | select(.has_memory == false) | "  • \(.type) at \(.timestamp)"' "$STATE_FILE" | tail -5 >&2

    cat >&2 <<EOF

This is MANDATORY, not optional.
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
EOF
  fi
  exit 1  # Non-zero exit = BLOCKS the action (silently unless debug mode)
fi

# Debt is zero, allow the action
exit 0
