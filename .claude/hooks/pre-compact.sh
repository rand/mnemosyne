#!/bin/bash
# Mnemosyne pre-compact hook
# Preserves important context before Claude Code compacts conversation history
#
# Creates two forms of preservation:
# 1. Memory in mnemosyne database (searchable)
# 2. Snapshot file in .claude/context-snapshots/ (recoverable)

set -e

# Get project directory
PROJECT_DIR="$(pwd)"
PROJECT_NAME="$(basename "$PROJECT_DIR")"
NAMESPACE="project:${PROJECT_NAME}"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# Log hook execution
echo "💾 Mnemosyne: Preserving context before compaction" >&2

# Create snapshots directory if it doesn't exist
SNAPSHOTS_DIR="${PROJECT_DIR}/.claude/context-snapshots"
mkdir -p "$SNAPSHOTS_DIR"

# Get mnemosyne binary path
# Try installed binary first, fall back to local build
if command -v mnemosyne &> /dev/null; then
    MNEMOSYNE_BIN="mnemosyne"
elif [ -f "${PROJECT_DIR}/target/release/mnemosyne" ]; then
    MNEMOSYNE_BIN="${PROJECT_DIR}/target/release/mnemosyne"
elif [ -f "${PROJECT_DIR}/target/debug/mnemosyne" ]; then
    MNEMOSYNE_BIN="${PROJECT_DIR}/target/debug/mnemosyne"
else
    echo "ℹ️  Mnemosyne not available. Skipping memory storage." >&2
    MNEMOSYNE_BIN=""
fi

# Read stdin (context about to be compacted)
CONTEXT=$(cat)

# Always save snapshot file (even if mnemosyne not available)
SNAPSHOT_FILE="${SNAPSHOTS_DIR}/context_${TIMESTAMP}.txt"
echo "$CONTEXT" > "$SNAPSHOT_FILE"
echo "📸 Snapshot saved: ${SNAPSHOT_FILE}" >&2

# Count lines in context
CONTEXT_LINES=$(echo "$CONTEXT" | wc -l | tr -d ' ')
echo "📊 Context size: ${CONTEXT_LINES} lines" >&2

# Extract key information from context using simple heuristics
# Look for architectural decisions, important patterns, constraints

# Check if context contains decision markers
if echo "$CONTEXT" | grep -qiE "(decided|decision|architecture|constraint|important|critical)"; then
    echo "🎯 Detected important content in context" >&2

    # Extract potential decision statements (more comprehensive)
    DECISIONS=$(echo "$CONTEXT" | grep -iE "(decided|decision|architecture|constraint|important|critical|implement|design|pattern|approach|strategy)" | head -10)

    if [ -n "$DECISIONS" ] && [ -n "$MNEMOSYNE_BIN" ]; then
        # Create a consolidated memory from context
        MEMORY_CONTENT="Context preserved from compaction at $TIMESTAMP:

**Context size**: ${CONTEXT_LINES} lines
**Snapshot**: ${SNAPSHOT_FILE}

**Key decisions and patterns detected**:
$DECISIONS

**Full context**: See snapshot file for complete conversation history."

        echo "📝 Saving context snippet to memory..." >&2

        "$MNEMOSYNE_BIN" remember \
            --content "$MEMORY_CONTENT" \
            --namespace "$NAMESPACE" \
            --importance 8 \
            --context "Pre-compaction preservation" \
            --tags "compaction,snapshot,${TIMESTAMP}" \
            --format json >/dev/null 2>&1 && {
                echo "✅ Memory preserved in $NAMESPACE" >&2
            } || {
                echo "⚠️  Failed to save memory (snapshot still available)" >&2
            }
    fi
fi

# Clean up old snapshots (keep last 50)
SNAPSHOT_COUNT=$(ls -1 "$SNAPSHOTS_DIR"/context_*.txt 2>/dev/null | wc -l | tr -d ' ')
if [ "$SNAPSHOT_COUNT" -gt 50 ]; then
    echo "🗑️  Cleaning up old snapshots (keeping 50 most recent)" >&2
    ls -1t "$SNAPSHOTS_DIR"/context_*.txt | tail -n +51 | xargs rm -f
fi

# Return empty output (hook is for side-effects only)
echo ""
