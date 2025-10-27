#!/bin/bash
# Mnemosyne pre-compact hook
# Preserves important context before Claude Code compacts conversation history

set -e

# Get project directory
PROJECT_DIR="$(pwd)"
PROJECT_NAME="$(basename "$PROJECT_DIR")"
NAMESPACE="project:${PROJECT_NAME}"

# Log hook execution
echo "💾 Mnemosyne: Preserving context before compaction" >&2

# Get mnemosyne binary path
# Try installed binary first, fall back to local build
if command -v mnemosyne &> /dev/null; then
    MNEMOSYNE_BIN="mnemosyne"
elif [ -f "${PROJECT_DIR}/target/release/mnemosyne" ]; then
    MNEMOSYNE_BIN="${PROJECT_DIR}/target/release/mnemosyne"
elif [ -f "${PROJECT_DIR}/target/debug/mnemosyne" ]; then
    MNEMOSYNE_BIN="${PROJECT_DIR}/target/debug/mnemosyne"
else
    echo "ℹ️  Mnemosyne not available. Skipping context preservation." >&2
    exit 0
fi

# Read stdin (context about to be compacted)
CONTEXT=$(cat)

# Extract key information from context using simple heuristics
# Look for architectural decisions, important patterns, constraints

# Check if context contains decision markers
if echo "$CONTEXT" | grep -qiE "(decided|decision|architecture|constraint|important|critical)"; then
    echo "🎯 Detected important content in context" >&2

    # Extract potential decision statements
    DECISIONS=$(echo "$CONTEXT" | grep -iE "(decided|decision|architecture|constraint)" | head -5)

    if [ -n "$DECISIONS" ]; then
        # Save to mnemosyne with high importance
        TIMESTAMP=$(date +%Y%m%d_%H%M%S)

        # Create a consolidated memory from context
        MEMORY_CONTENT="Context preserved from compaction at $TIMESTAMP:

$DECISIONS"

        echo "📝 Saving context snippet to memory..." >&2

        "$MNEMOSYNE_BIN" remember \
            --content "$MEMORY_CONTENT" \
            --namespace "$NAMESPACE" \
            --importance 8 \
            --context "Pre-compaction preservation" \
            --format json >/dev/null 2>&1 || {
                echo "⚠️  Failed to save context" >&2
            }

        echo "✅ Context preserved in $NAMESPACE" >&2
    fi
fi

# Return empty output (hook is for side-effects only)
echo ""
