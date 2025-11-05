#!/bin/bash
# Mnemosyne post-commit hook
# Links git commits to architectural decisions and memories

set -e

# Get project directory
PROJECT_DIR="$(pwd)"
PROJECT_NAME="$(basename "$PROJECT_DIR")"
NAMESPACE="project:${PROJECT_NAME}"

# Log hook execution (only in debug mode)
if [ "${CC_HOOK_DEBUG:-0}" = "1" ]; then
  echo "🔗 Mnemosyne: Analyzing commit for memory links" >&2
fi

# Get mnemosyne binary path
# Try installed binary first, fall back to local build
if command -v mnemosyne &> /dev/null; then
    MNEMOSYNE_BIN="mnemosyne"
elif [ -f "${PROJECT_DIR}/target/release/mnemosyne" ]; then
    MNEMOSYNE_BIN="${PROJECT_DIR}/target/release/mnemosyne"
elif [ -f "${PROJECT_DIR}/target/debug/mnemosyne" ]; then
    MNEMOSYNE_BIN="${PROJECT_DIR}/target/debug/mnemosyne"
else
    exit 0
fi

# Get latest commit info
COMMIT_HASH=$(git log -1 --format=%h)
COMMIT_MSG=$(git log -1 --format=%s)
COMMIT_BODY=$(git log -1 --format=%b)
FILES_CHANGED=$(git diff-tree --no-commit-id --name-only -r HEAD | wc -l | tr -d ' ')

# Debug output only
if [ "${CC_HOOK_DEBUG:-0}" = "1" ]; then
  echo "[*] Commit: $COMMIT_HASH - $COMMIT_MSG" >&2
  echo "[~] Files changed: $FILES_CHANGED" >&2
fi

# Check if this commit relates to architectural decisions
# Keywords that suggest architectural significance
if echo "$COMMIT_MSG $COMMIT_BODY" | grep -qiE "(architecture|implement|refactor|migrate|design|pattern|decision|integrate|add|remove|fix|update|create|improve|enhance|complete|wire|establish)"; then
    if [ "${CC_HOOK_DEBUG:-0}" = "1" ]; then
      echo "[#] Architectural commit detected" >&2
    fi

    # Create memory linking commit to decision
    MEMORY_CONTENT="Git commit $COMMIT_HASH: $COMMIT_MSG

**Files changed**: $FILES_CHANGED

**Commit message**:
$COMMIT_MSG

**Details**:
$COMMIT_BODY

**Context**: This commit implements or relates to an architectural decision in the project."

    # Determine importance based on file count and keywords
    IMPORTANCE=6
    if echo "$COMMIT_MSG" | grep -qiE "(critical|breaking|major|architecture|design)"; then
        IMPORTANCE=8
    elif [ "$FILES_CHANGED" -gt 10 ]; then
        IMPORTANCE=7
    fi

    if [ "${CC_HOOK_DEBUG:-0}" = "1" ]; then
      echo "[+] Saving commit memory (importance: $IMPORTANCE)" >&2
    fi

    "$MNEMOSYNE_BIN" remember \
        --content "$MEMORY_CONTENT" \
        --namespace "$NAMESPACE" \
        --importance "$IMPORTANCE" \
        --context "Git commit $COMMIT_HASH" \
        --tags "commit,${COMMIT_HASH}" \
        --format json >/dev/null 2>&1 || {
            echo "[!] Failed to save commit memory" >&2
        }

    # Try to link to related memories (debug output only)
    if [ "${CC_HOOK_DEBUG:-0}" = "1" ]; then
      echo "[?] Searching for related memories..." >&2
    fi

    RELATED=$("$MNEMOSYNE_BIN" recall \
        --query "$COMMIT_MSG" \
        --namespace "$NAMESPACE" \
        --limit 3 \
        --format json 2>/dev/null || echo '{"results": []}')

    RELATED_COUNT=$(echo "$RELATED" | jq -r '.results | length' 2>/dev/null || echo "0")

    if [ "$RELATED_COUNT" -gt 0 ] && [ "${CC_HOOK_DEBUG:-0}" = "1" ]; then
        echo "[✓] Found $RELATED_COUNT related memories" >&2
    fi
fi

# Return empty output
echo ""
