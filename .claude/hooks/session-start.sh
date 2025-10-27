#!/bin/bash
# Mnemosyne session-start hook
# Loads project memory context at the beginning of each session

set -e

# Get project directory
PROJECT_DIR="$(pwd)"
PROJECT_NAME="$(basename "$PROJECT_DIR")"

# Log hook execution
echo "🧠 Mnemosyne: Loading memory context for $PROJECT_NAME" >&2

# Get mnemosyne binary path
MNEMOSYNE_BIN="${PROJECT_DIR}/target/release/mnemosyne"
if [ ! -f "$MNEMOSYNE_BIN" ]; then
    MNEMOSYNE_BIN="${PROJECT_DIR}/target/debug/mnemosyne"
fi

# If mnemosyne not built, provide instructions
if [ ! -f "$MNEMOSYNE_BIN" ]; then
    echo "ℹ️  Mnemosyne not built yet. Run 'cargo build --release' to enable memory features." >&2
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

    # Format memories as context for Claude
    echo "# Project Memory Context"
    echo ""
    echo "**Project**: $PROJECT_NAME"
    echo "**Namespace**: $NAMESPACE"
    echo "**Recent Important Memories**:"
    echo ""

    echo "$MEMORIES" | jq -r '.results[] | "## \(.summary)\n\n**Type**: \(.memory_type)\n**Importance**: \(.importance)/10\n**Tags**: \(.tags | join(", "))\n\n\(.content)\n\n---\n"' 2>/dev/null || echo "Error formatting memories"
else
    echo "ℹ️  No important memories found for $NAMESPACE" >&2
    echo "📝 Use \`mnemosyne remember\` to capture architectural decisions and insights." >&2
fi
