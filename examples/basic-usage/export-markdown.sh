#!/usr/bin/env bash
#
# Example: Export Memories to Markdown
#
# This example shows how to export memories to a readable Markdown file.
# Useful for documentation, sharing with team, or backup.
#
# Usage:
#   ./export-markdown.sh [output-file] [namespace]

set -e

OUTPUT_FILE="${1:-memories-export-$(date +%Y%m%d).md}"
NAMESPACE="${2:-}"

echo "📤 Exporting memories to: $OUTPUT_FILE"
echo ""

if [ -n "$NAMESPACE" ]; then
  echo "Namespace filter: $NAMESPACE"
  mnemosyne export \
    --output "$OUTPUT_FILE" \
    --namespace "$NAMESPACE"
else
  echo "No namespace filter (exporting all memories)"
  mnemosyne export \
    --output "$OUTPUT_FILE"
fi

echo ""
echo "✅ Export complete!"
echo ""
echo "File: $OUTPUT_FILE"
echo "Size: $(wc -c < "$OUTPUT_FILE") bytes"
echo "Memories: $(grep -c "^## " "$OUTPUT_FILE" || echo "0")"
echo ""
echo "Preview:"
head -20 "$OUTPUT_FILE"
echo ""
echo "..."
echo ""
echo "Use cases:"
echo "  - Share knowledge with team members"
echo "  - Create project documentation"
echo "  - Backup important decisions"
echo "  - Onboarding new developers"
echo ""
echo "Examples:"
echo "  ./export-markdown.sh team-knowledge.md \"project:myapp\""
echo "  ./export-markdown.sh architecture-decisions.md \"global\""
