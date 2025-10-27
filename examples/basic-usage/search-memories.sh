#!/usr/bin/env bash
#
# Example: Search Memories
#
# This example demonstrates hybrid search (FTS5 keyword + graph traversal).
# Mnemosyne finds relevant memories using keyword matching and relationship graphs.
#
# Usage:
#   ./search-memories.sh [query]

set -e

QUERY="${1:-architecture decision}"

echo "🔍 Searching for: '$QUERY'"
echo ""

# Search with default settings
echo "=== Basic Search ==="
mnemosyne recall \
  --query "$QUERY" \
  --limit 5 \
  --format json | \
  jq -r '.results[] | "[\(.importance)/10] \(.summary)\n  Tags: \(.tags | join(", "))\n"'

echo ""
echo "=== Filtered by Importance (7+) ==="
# Search with importance filter
mnemosyne recall \
  --query "$QUERY" \
  --min-importance 7 \
  --limit 3 \
  --format json | \
  jq -r '.results[] | "[\(.importance)/10] \(.summary)"'

echo ""
echo "=== Search Tips ==="
echo "  - Use specific technical terms for better results"
echo "  - Combine with --min-importance to filter low-priority items"
echo "  - Use --namespace to search within specific projects"
echo "  - Graph traversal automatically finds related memories"
echo ""
echo "Examples:"
echo "  mnemosyne recall --query \"bug race condition\""
echo "  mnemosyne recall --query \"authentication\" --min-importance 8"
echo "  mnemosyne recall --query \"database\" --namespace \"project:myapp\""
