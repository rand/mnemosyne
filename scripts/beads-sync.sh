#!/bin/bash
# Beads State Synchronization Script
#
# This script ensures Beads task state is properly synced between:
# - In-memory Beads database
# - .beads/issues.jsonl (persistent storage)
# - Git version control
#
# Usage:
#   ./scripts/beads-sync.sh export  # Export current state
#   ./scripts/beads-sync.sh import  # Import from .jsonl
#   ./scripts/beads-sync.sh commit  # Export + Git commit
#   ./scripts/beads-sync.sh status  # Show sync status

set -euo pipefail

BEADS_FILE=".beads/issues.jsonl"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_ROOT"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if Beads CLI is available
if ! command -v bd &> /dev/null; then
    echo -e "${RED}Error: Beads CLI not found${NC}"
    echo "Install: go install github.com/steveyegge/beads/cmd/bd@latest"
    exit 1
fi

# Ensure .beads directory exists
mkdir -p .beads

# Export current Beads state to .jsonl
export_beads() {
    echo -e "${GREEN}Exporting Beads state...${NC}"
    bd export -o "$BEADS_FILE"
    echo -e "${GREEN}✓ Exported to $BEADS_FILE${NC}"

    # Show stats
    local total=$(wc -l < "$BEADS_FILE" | tr -d ' ')
    local open=$(grep -c '"status":"open"' "$BEADS_FILE" || true)
    local in_progress=$(grep -c '"status":"in_progress"' "$BEADS_FILE" || true)
    local closed=$(grep -c '"status":"closed"' "$BEADS_FILE" || true)

    echo ""
    echo "Task Summary:"
    echo "  Total: $total"
    echo "  Open: $open"
    echo "  In Progress: $in_progress"
    echo "  Closed: $closed"
}

# Import Beads state from .jsonl
import_beads() {
    if [ ! -f "$BEADS_FILE" ]; then
        echo -e "${RED}Error: $BEADS_FILE not found${NC}"
        exit 1
    fi

    echo -e "${GREEN}Importing Beads state...${NC}"
    bd import -i "$BEADS_FILE"
    echo -e "${GREEN}✓ Imported from $BEADS_FILE${NC}"

    # Show stats
    local total=$(wc -l < "$BEADS_FILE" | tr -d ' ')
    echo "Imported $total tasks"
}

# Export and commit to git
commit_beads() {
    export_beads

    # Check if there are changes
    if git diff --quiet "$BEADS_FILE"; then
        echo -e "${YELLOW}No changes to commit${NC}"
        return
    fi

    echo ""
    echo -e "${GREEN}Committing Beads state to git...${NC}"

    git add "$BEADS_FILE"
    git commit -m "[Beads] Sync task state

$(bd list --status open --json | jq -r '.[] | "- [\(.status)] \(.title)"' | head -5)
$(if [ $(bd list --status open --json | jq '. | length') -gt 5 ]; then echo "...and $(( $(bd list --status open --json | jq '. | length') - 5 )) more"; fi)

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"

    echo -e "${GREEN}✓ Committed to git${NC}"
    git log -1 --oneline
}

# Show sync status
show_status() {
    echo -e "${GREEN}Beads Sync Status${NC}"
    echo ""

    if [ ! -f "$BEADS_FILE" ]; then
        echo -e "${RED}✗ $BEADS_FILE not found${NC}"
        echo "  Run: ./scripts/beads-sync.sh export"
        return
    fi

    # Check if .jsonl is in git
    if git ls-files --error-unmatch "$BEADS_FILE" &> /dev/null; then
        echo -e "${GREEN}✓ $BEADS_FILE tracked in git${NC}"
    else
        echo -e "${YELLOW}⚠ $BEADS_FILE not tracked in git${NC}"
        echo "  Run: git add $BEADS_FILE"
    fi

    # Check if there are uncommitted changes
    if git diff --quiet "$BEADS_FILE"; then
        echo -e "${GREEN}✓ No uncommitted changes${NC}"
    else
        echo -e "${YELLOW}⚠ Uncommitted changes in $BEADS_FILE${NC}"
        echo "  Run: ./scripts/beads-sync.sh commit"
    fi

    # Show task stats
    echo ""
    local total=$(wc -l < "$BEADS_FILE" | tr -d ' ')
    local open=$(grep -c '"status":"open"' "$BEADS_FILE" || true)
    local in_progress=$(grep -c '"status":"in_progress"' "$BEADS_FILE" || true)
    local closed=$(grep -c '"status":"closed"' "$BEADS_FILE" || true)

    echo "Task Summary:"
    echo "  Total: $total tasks"
    echo "  Open: $open"
    echo "  In Progress: $in_progress"
    echo "  Closed: $closed"

    # Check if Beads memory differs from .jsonl
    echo ""
    echo "Recent Activity:"
    bd list --status in_progress,open --json | jq -r '.[] | "  [\(.status)] \(.title)"' | head -3
}

# Main command dispatcher
case "${1:-}" in
    export)
        export_beads
        ;;
    import)
        import_beads
        ;;
    commit)
        commit_beads
        ;;
    status)
        show_status
        ;;
    *)
        echo "Usage: $0 {export|import|commit|status}"
        echo ""
        echo "Commands:"
        echo "  export  - Export Beads state to $BEADS_FILE"
        echo "  import  - Import Beads state from $BEADS_FILE"
        echo "  commit  - Export state and commit to git"
        echo "  status  - Show sync status and task summary"
        exit 1
        ;;
esac
