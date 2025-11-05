#!/bin/bash
# Post-memory-store hook: Clear memory debt when memories are stored
# Triggers after successful `mnemosyne remember` command

set -e

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

# Increment stored count
COUNT=$(jq '.memories_stored_count' "$STATE_FILE" 2>/dev/null || echo "0")
COUNT=$((COUNT + 1))

# Clear one debt point per memory
DEBT=$(jq '.memory_debt' "$STATE_FILE" 2>/dev/null || echo "0")
DEBT=$((DEBT > 0 ? DEBT - 1 : 0))

# Update state
TMP_FILE=$(mktemp)
jq ".memories_stored_count = $COUNT | .memory_debt = $DEBT | .last_memory_timestamp = \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"" "$STATE_FILE" > "$TMP_FILE"
mv "$TMP_FILE" "$STATE_FILE"

# Only show confirmation in debug mode
if [ "${CC_HOOK_DEBUG:-0}" = "1" ]; then
  echo "✅ Memory stored. Debt reduced to $DEBT. Total memories this session: $COUNT" >&2
fi

exit 0
