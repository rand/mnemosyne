#!/bin/bash
# Mnemosyne session-start hook
# Loads project memory context and initializes memory state tracking

set -e

# Get project directory
PROJECT_DIR="$(pwd)"
PROJECT_NAME="$(basename "$PROJECT_DIR")"

# Initialize memory state file
STATE_FILE=".claude/memory-state.json"
SESSION_ID=$(uuidgen)

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

# Log hook execution
echo "🧠 Mnemosyne: Loading memory context for $PROJECT_NAME" >&2
echo "📊 Session ID: $SESSION_ID" >&2

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

# Get recent important memories
# Use broad query to find any important memories
MEMORIES=$("$MNEMOSYNE_BIN" recall \
    --query "memory OR project OR architecture OR implementation" \
    --namespace "$NAMESPACE" \
    --limit 5 \
    --min-importance 7 \
    --format json 2>/dev/null || echo '{"results": []}')

# Count memories
MEMORY_COUNT=$(echo "$MEMORIES" | jq -r '.results | length' 2>/dev/null || echo "0")

if [ "$MEMORY_COUNT" -gt 0 ]; then
    echo "📚 Loaded $MEMORY_COUNT important memories from $NAMESPACE" >&2

    # Format memories as prominent context
    cat <<EOF

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Project Context: $PROJECT_NAME
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

**Namespace**: $NAMESPACE
**Important Memories Loaded**: $MEMORY_COUNT (importance ≥ 7)
**Session**: $SESSION_ID

## Critical Memories (Importance ≥ 8)

EOF
    echo "$MEMORIES" | jq -r '.results[] | select(.importance >= 8) | "### \(.summary)\n**Importance**: \(.importance)/10 | **Type**: \(.memory_type) | **Tags**: \(.tags | join(", "))\n\n\(.content)\n\n---\n"' 2>/dev/null

    cat <<EOF

## Important Memories (Importance 7)

EOF
    echo "$MEMORIES" | jq -r '.results[] | select(.importance == 7) | "- **\(.summary)** (\(.memory_type))" ' 2>/dev/null

    cat <<EOF

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

**Memory Enforcement**: Active
- Memory debt tracking: Enabled
- Automatic prompts after 3 events
- Blocking on git push if debt > 0

Use \`mnemosyne recall -q "query"\` to search for specific memories.
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
EOF
else
    echo "ℹ️  No important memories found for $NAMESPACE" >&2
    cat <<EOF

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
# Project Context: $PROJECT_NAME
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

No important memories found for this project yet.

**Start building project memory**:
\`\`\`bash
mnemosyne remember -c "Your insight or decision" \\
  -n "$NAMESPACE" -i 7-10 -t "architecture,decision"
\`\`\`

Memory enforcement is active. Store memories to avoid blocking later.
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
EOF
fi
