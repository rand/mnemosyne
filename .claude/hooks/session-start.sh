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
    --format json 2>/dev/null || echo '{"results": []}')

# Count memories
MEMORY_COUNT=$(echo "$MEMORIES" | jq -r '.results | length' 2>/dev/null || echo "0")
HIGH_IMPORTANCE_COUNT=$(echo "$MEMORIES" | jq -r '[.results[] | select(.importance >= 8)] | length' 2>/dev/null || echo "0")

if [ "$MEMORY_COUNT" -gt 0 ]; then
    # Visible status line (stderr)
    echo "🧠 Mnemosyne: Loaded $MEMORY_COUNT memories (≥7 importance, $HIGH_IMPORTANCE_COUNT critical) • Session: ${SESSION_ID:0:8}" >&2

    # Hidden context for Claude (stdout, wrapped in XML tags)
    cat <<EOF
<session-start-hook># Project Context: $PROJECT_NAME

## Critical Memories (Importance ≥ 8)

EOF
    echo "$MEMORIES" | jq -r '.results[] | select(.importance >= 8) | "**\(.summary)** — \(.memory_type) — \(.tags | join(", "))\n\(.content)\n\n---\n"' 2>/dev/null

    cat <<EOF

## Important Memories (Importance 7)

EOF
    echo "$MEMORIES" | jq -r '.results[] | select(.importance == 7) | "- **\(.summary)** (\(.memory_type))"' 2>/dev/null

    cat <<EOF

## Knowledge Graph
EOF
    # Count semantic connections (links between memories)
    LINK_COUNT=$(echo "$MEMORIES" | jq -r '[.results[].related_memories // [] | length] | add // 0' 2>/dev/null || echo "0")
    echo "$LINK_COUNT semantic connections across $MEMORY_COUNT memories"

    cat <<EOF

---
*Context from Mnemosyne • $MEMORY_COUNT memories loaded*
</session-start-hook>
EOF
else
    # Visible status line (stderr)
    echo "🧠 Mnemosyne: No memories found (building context) • Session: ${SESSION_ID:0:8}" >&2

    # Hidden context for Claude (stdout, wrapped in XML tags)
    cat <<EOF
<session-start-hook># Project Context: $PROJECT_NAME

No important memories found for this project yet.

**Start building project memory**:
\`\`\`bash
mnemosyne remember -c "Your insight or decision" \\
  -n "$NAMESPACE" -i 7-10 -t "architecture,decision"
\`\`\`

Memory enforcement is active. Store memories to avoid blocking later.
</session-start-hook>
EOF
fi
