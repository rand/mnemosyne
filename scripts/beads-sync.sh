#!/bin/bash
# Beads State Management Script
#
# NOTE: As of Beads v0.20.1+, auto-sync is enabled by default.
# This script is primarily for manual operations and diagnostics.
#
# Auto-sync (enabled by default):
# - Automatically exports to .beads/issues.jsonl after changes (5s debounce)
# - Automatically imports from .jsonl after git pull
# - No manual import/export needed in normal workflow
#
# Usage:
#   ./scripts/beads-sync.sh setup   # One-time initialization (bd init)
#   ./scripts/beads-sync.sh sync    # Force immediate sync (rarely needed)
#   ./scripts/beads-sync.sh status  # Show sync status and diagnostics
#   ./scripts/beads-sync.sh commit  # Git commit with task summary
#   ./scripts/beads-sync.sh migrate # Migrate to hash-based IDs

set -euo pipefail

BEADS_FILE=".beads/issues.jsonl"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

cd "$PROJECT_ROOT"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check if Beads CLI is available
if ! command -v bd &> /dev/null; then
    echo -e "${RED}Error: Beads CLI not found${NC}"
    echo ""
    echo "Install options:"
    echo "  npm install -g @beads/bd"
    echo "  curl -fsSL https://raw.githubusercontent.com/steveyegge/beads/main/scripts/install.sh | bash"
    echo "  brew tap steveyegge/beads && brew install bd"
    exit 1
fi

# One-time setup (bd init)
setup_beads() {
    echo -e "${BLUE}Setting up Beads for this project...${NC}"
    echo ""

    if [ -d ".beads" ] && [ -f "$BEADS_FILE" ]; then
        echo -e "${YELLOW}Beads already initialized${NC}"
        bd info
        return
    fi

    echo "This will:"
    echo "  - Create .beads/ directory"
    echo "  - Import existing issues from $BEADS_FILE (if found)"
    echo "  - Install git hooks"
    echo "  - Enable auto-sync (5-second debounce)"
    echo ""
    echo "Run: bd init"
    echo ""
    echo -e "${GREEN}For non-interactive setup:${NC} bd init --quiet"
}

# Force immediate sync (rarely needed with auto-sync)
sync_beads() {
    echo -e "${BLUE}Forcing immediate sync...${NC}"
    bd sync
    echo -e "${GREEN}✓ Sync complete${NC}"

    # Show stats
    if [ -f "$BEADS_FILE" ]; then
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
    fi
}

# Migrate to hash-based IDs
migrate_beads() {
    echo -e "${BLUE}Migrating to hash-based issue IDs...${NC}"
    echo ""
    echo "This will convert sequential IDs (bd-1, bd-2) to hash IDs (bd-a1b2, bd-c3d4)"
    echo ""
    echo "Preview changes:"
    bd migrate --dry-run
    echo ""
    read -p "Proceed with migration? [y/N] " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        bd migrate
        echo -e "${GREEN}✓ Migration complete${NC}"
        echo ""
        echo "New ID format: 4-6 character hashes"
        echo "Auto-synced to $BEADS_FILE"
    else
        echo "Migration cancelled"
    fi
}

# Commit Beads state to git (auto-sync handles export)
commit_beads() {
    echo -e "${BLUE}Committing Beads state to git...${NC}"
    echo ""
    echo -e "${YELLOW}Note: Auto-sync already exported changes to $BEADS_FILE${NC}"
    echo ""

    # Check if there are changes
    if git diff --quiet "$BEADS_FILE" 2>/dev/null; then
        echo -e "${GREEN}No changes to commit (already in sync)${NC}"
        return
    fi

    if [ ! -f "$BEADS_FILE" ]; then
        echo -e "${RED}Error: $BEADS_FILE not found${NC}"
        echo "Run: bd ready (to trigger auto-sync)"
        exit 1
    fi

    # Generate commit message with task summary
    git add "$BEADS_FILE"

    local open_tasks=$(bd list --status open --json 2>/dev/null | jq -r '.[] | "- [\(.status)] \(.title)"' | head -5 || echo "")
    local total_open=$(bd list --status open --json 2>/dev/null | jq '. | length' || echo "0")

    git commit -m "[Beads] Sync task state

${open_tasks}
$(if [ "$total_open" -gt 5 ]; then echo "...and $(( total_open - 5 )) more"; fi)

🤖 Generated with [Claude Code](https://claude.com/claude-code)

Co-Authored-By: Claude <noreply@anthropic.com>"

    echo -e "${GREEN}✓ Committed to git${NC}"
    git log -1 --oneline
}

# Show sync status and diagnostics
show_status() {
    echo -e "${GREEN}Beads Status & Diagnostics${NC}"
    echo ""

    # Beads info
    echo -e "${BLUE}Installation:${NC}"
    bd info 2>/dev/null || echo -e "${RED}  Error: Cannot connect to Beads daemon${NC}"
    echo ""

    # Database file status
    if [ ! -f "$BEADS_FILE" ]; then
        echo -e "${RED}✗ $BEADS_FILE not found${NC}"
        echo "  Run: ./scripts/beads-sync.sh setup"
        echo ""
        return
    fi

    echo -e "${BLUE}Sync Status:${NC}"

    # Check if .jsonl is in git
    if git ls-files --error-unmatch "$BEADS_FILE" &> /dev/null 2>&1; then
        echo -e "${GREEN}✓ $BEADS_FILE tracked in git${NC}"
    else
        echo -e "${YELLOW}⚠ $BEADS_FILE not tracked in git${NC}"
        echo "  Run: git add $BEADS_FILE"
    fi

    # Check if there are uncommitted changes
    if git diff --quiet "$BEADS_FILE" 2>/dev/null; then
        echo -e "${GREEN}✓ No uncommitted changes (auto-sync working)${NC}"
    else
        echo -e "${YELLOW}⚠ Uncommitted changes in $BEADS_FILE${NC}"
        echo "  Auto-sync exported changes, ready to commit"
        echo "  Run: ./scripts/beads-sync.sh commit"
    fi

    echo ""

    # Show task stats from file
    echo -e "${BLUE}Task Summary (from $BEADS_FILE):${NC}"
    local total=$(wc -l < "$BEADS_FILE" | tr -d ' ')
    local open=$(grep -c '"status":"open"' "$BEADS_FILE" || true)
    local in_progress=$(grep -c '"status":"in_progress"' "$BEADS_FILE" || true)
    local closed=$(grep -c '"status":"closed"' "$BEADS_FILE" || true)

    echo "  Total: $total tasks"
    echo "  Open: $open"
    echo "  In Progress: $in_progress"
    echo "  Closed: $closed"

    # Show recent activity
    echo ""
    echo -e "${BLUE}Recent Activity:${NC}"
    bd list --status in_progress,open --json 2>/dev/null | jq -r '.[] | "  [\(.status)] \(.title)"' | head -5 || echo "  No active tasks"

    # Check for hash-based IDs
    echo ""
    echo -e "${BLUE}ID Format:${NC}"
    local first_id=$(head -1 "$BEADS_FILE" | jq -r '.id' 2>/dev/null || echo "unknown")
    if [[ $first_id =~ ^bd-[0-9a-f]{4,6}$ ]]; then
        echo -e "${GREEN}✓ Using hash-based IDs (modern format)${NC}"
    elif [[ $first_id =~ ^bd-[0-9]+$ ]]; then
        echo -e "${YELLOW}⚠ Using sequential IDs (legacy format)${NC}"
        echo "  Consider migrating: ./scripts/beads-sync.sh migrate"
    else
        echo "  Unknown ID format: $first_id"
    fi
}

# Main command dispatcher
case "${1:-}" in
    setup)
        setup_beads
        ;;
    sync)
        sync_beads
        ;;
    migrate)
        migrate_beads
        ;;
    commit)
        commit_beads
        ;;
    status)
        show_status
        ;;
    *)
        echo "Usage: $0 {setup|sync|migrate|commit|status}"
        echo ""
        echo "NOTE: Auto-sync is enabled by default in Beads v0.20.1+"
        echo "      Most sync operations happen automatically."
        echo ""
        echo "Commands:"
        echo "  setup   - One-time initialization (bd init)"
        echo "  sync    - Force immediate sync (rarely needed)"
        echo "  migrate - Migrate to hash-based IDs (bd migrate)"
        echo "  commit  - Commit auto-synced state to git"
        echo "  status  - Show sync status and diagnostics"
        echo ""
        echo "Examples:"
        echo "  ./scripts/beads-sync.sh setup    # First time setup"
        echo "  ./scripts/beads-sync.sh status   # Check status"
        echo "  ./scripts/beads-sync.sh commit   # Commit changes"
        exit 1
        ;;
esac
