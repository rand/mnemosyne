#!/usr/bin/env bash
#
# Mnemosyne Uninstallation Script
#
# This script safely removes Mnemosyne from your system.
# By default, it preserves user data (database, API keys).
#
# Usage:
#   ./uninstall.sh [OPTIONS]
#
# Options:
#   --help                Show this help message
#   --purge               Remove all data including database and API keys
#   --bin-dir DIR         Remove binary from DIR (default: ~/.local/bin)
#   --global-mcp          Remove global MCP config (~/.claude)
#   --yes                 Answer yes to all prompts (DANGEROUS with --purge)

set -e  # Exit on error
set -o pipefail  # Pipe failures propagate

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DEFAULT_BIN_DIR="${HOME}/.local/bin"
BIN_DIR="${DEFAULT_BIN_DIR}"
PURGE=false
GLOBAL_MCP=false
AUTO_YES=false

# Track what was removed
REMOVED_BINARY=false
REMOVED_DATABASE=false
REMOVED_API_KEY=false
REMOVED_MCP_PROJECT=false
REMOVED_MCP_GLOBAL=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --help)
            cat <<EOF
Mnemosyne Uninstallation Script

Usage: $0 [OPTIONS]

Options:
  --help                Show this help message
  --purge               Remove all data including database and API keys
  --bin-dir DIR         Remove binary from DIR (default: ~/.local/bin)
  --global-mcp          Remove global MCP config (~/.claude)
  --yes                 Answer yes to all prompts (DANGEROUS with --purge)

Safety:
  By default, this script only removes the binary and MCP configuration.
  User data (database, API keys) is preserved unless --purge is specified.

Examples:
  $0                              # Remove binary and MCP config (safe)
  $0 --purge                      # Remove everything (prompts for confirmation)
  $0 --global-mcp                 # Also remove global MCP config
  $0 --bin-dir /usr/local/bin     # Remove from custom location
  $0 --purge --yes                # DANGER: Remove everything without prompts

EOF
            exit 0
            ;;
        --purge)
            PURGE=true
            shift
            ;;
        --bin-dir)
            BIN_DIR="$2"
            shift 2
            ;;
        --global-mcp)
            GLOBAL_MCP=true
            shift
            ;;
        --yes)
            AUTO_YES=true
            shift
            ;;
        *)
            echo -e "${RED}Error: Unknown option $1${NC}" >&2
            echo "Run '$0 --help' for usage information." >&2
            exit 1
            ;;
    esac
done

# Utility functions
print_header() {
    echo -e "\n${BOLD}${BLUE}==> $1${NC}"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_error() {
    echo -e "${RED}✗ Error:${NC} $1" >&2
}

print_warning() {
    echo -e "${YELLOW}⚠ Warning:${NC} $1" >&2
}

print_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

prompt_yes_no() {
    if [ "$AUTO_YES" = true ]; then
        return 0
    fi

    local prompt="$1"
    local default="${2:-n}"

    if [ "$default" = "y" ]; then
        prompt="$prompt [Y/n]: "
    else
        prompt="$prompt [y/N]: "
    fi

    while true; do
        read -p "$(echo -e "${prompt}")" response
        response="${response:-$default}"
        case "$response" in
            [Yy]|[Yy][Ee][Ss]) return 0 ;;
            [Nn]|[Nn][Oo]) return 1 ;;
            *) echo "Please answer yes or no." ;;
        esac
    done
}

# Check if binary exists
check_installation() {
    print_header "Checking installation"

    if [ -x "${BIN_DIR}/mnemosyne" ]; then
        local version=$("${BIN_DIR}/mnemosyne" --version 2>&1 || echo "unknown")
        print_success "Found Mnemosyne binary: ${BIN_DIR}/mnemosyne ($version)"
        return 0
    else
        print_warning "Mnemosyne binary not found in ${BIN_DIR}"
        return 1
    fi
}

# Show what will be removed
show_removal_plan() {
    print_header "Removal plan"

    echo ""
    echo "The following will be removed:"
    echo ""

    # Binary
    if [ -f "${BIN_DIR}/mnemosyne" ]; then
        echo "  ${BOLD}✓${NC} Binary: ${BIN_DIR}/mnemosyne"
    else
        echo "  ${YELLOW}-${NC} Binary: ${BIN_DIR}/mnemosyne (not found)"
    fi

    # Project MCP config
    if [ -f "${SCRIPT_DIR}/.claude/mcp_config.json" ]; then
        echo "  ${BOLD}✓${NC} Project MCP config: ${SCRIPT_DIR}/.claude/mcp_config.json"
    else
        echo "  ${YELLOW}-${NC} Project MCP config: (not found)"
    fi

    # Global MCP config
    if [ "$GLOBAL_MCP" = true ]; then
        if [ -f "${HOME}/.claude/mcp_config.json" ]; then
            echo "  ${BOLD}✓${NC} Global MCP config: ${HOME}/.claude/mcp_config.json"
        else
            echo "  ${YELLOW}-${NC} Global MCP config: (not found)"
        fi
    fi

    echo ""
    echo "The following will be ${BOLD}preserved${NC} (unless --purge is used):"
    echo ""

    # Database
    local db_files=()
    if [ -f "${SCRIPT_DIR}/mnemosyne.db" ]; then
        db_files+=("${SCRIPT_DIR}/mnemosyne.db")
    fi
    if [ -f "${HOME}/.mnemosyne/mnemosyne.db" ]; then
        db_files+=("${HOME}/.mnemosyne/mnemosyne.db")
    fi

    if [ ${#db_files[@]} -gt 0 ]; then
        for db in "${db_files[@]}"; do
            local size=$(du -h "$db" 2>/dev/null | cut -f1)
            if [ "$PURGE" = true ]; then
                echo "  ${RED}✗${NC} Database: $db ($size) [WILL BE DELETED]"
            else
                echo "  ${GREEN}✓${NC} Database: $db ($size)"
            fi
        done
    else
        echo "  ${YELLOW}-${NC} Database: (not found)"
    fi

    # API key
    if [ -x "${BIN_DIR}/mnemosyne" ] && "${BIN_DIR}/mnemosyne" config show-key &> /dev/null; then
        if [ "$PURGE" = true ]; then
            echo "  ${RED}✗${NC} API key: (configured in keychain) [WILL BE DELETED]"
        else
            echo "  ${GREEN}✓${NC} API key: (configured in keychain)"
        fi
    else
        echo "  ${YELLOW}-${NC} API key: (not found)"
    fi

    echo ""
}

# Remove binary
remove_binary() {
    print_header "Removing binary"

    if [ -f "${BIN_DIR}/mnemosyne" ]; then
        if rm -f "${BIN_DIR}/mnemosyne"; then
            print_success "Removed ${BIN_DIR}/mnemosyne"
            REMOVED_BINARY=true
        else
            print_error "Failed to remove binary (permission denied?)"
            return 1
        fi
    else
        print_info "Binary not found, skipping"
    fi
}

# Remove database
remove_database() {
    if [ "$PURGE" != true ]; then
        return 0
    fi

    print_header "Removing database"

    local removed_count=0
    local db_files=()

    # Find all database files
    if [ -f "${SCRIPT_DIR}/mnemosyne.db" ]; then
        db_files+=("${SCRIPT_DIR}/mnemosyne.db")
    fi
    if [ -f "${HOME}/.mnemosyne/mnemosyne.db" ]; then
        db_files+=("${HOME}/.mnemosyne/mnemosyne.db")
    fi

    if [ ${#db_files[@]} -eq 0 ]; then
        print_info "No database files found"
        return 0
    fi

    # Show files and confirm
    echo ""
    echo "Found database files:"
    for db in "${db_files[@]}"; do
        local size=$(du -h "$db" 2>/dev/null | cut -f1)
        echo "  - $db ($size)"
    done
    echo ""

    if prompt_yes_no "${RED}${BOLD}Permanently delete database files? This cannot be undone!${NC}" "n"; then
        for db in "${db_files[@]}"; do
            if rm -f "$db"; then
                print_success "Deleted $db"
                removed_count=$((removed_count + 1))
            else
                print_error "Failed to delete $db"
            fi

            # Remove journal and wal files
            rm -f "${db}-journal" "${db}-wal" "${db}-shm" 2>/dev/null || true
        done

        if [ $removed_count -gt 0 ]; then
            REMOVED_DATABASE=true
        fi
    else
        print_info "Skipped database removal (data preserved)"
    fi
}

# Remove API key
remove_api_key() {
    if [ "$PURGE" != true ]; then
        return 0
    fi

    print_header "Removing API key"

    if [ ! -x "${BIN_DIR}/mnemosyne" ]; then
        print_info "Binary not available, cannot check API key"
        return 0
    fi

    if "${BIN_DIR}/mnemosyne" config show-key &> /dev/null; then
        echo ""
        if prompt_yes_no "${YELLOW}Remove API key from system keychain?${NC}" "n"; then
            if "${BIN_DIR}/mnemosyne" config delete-key; then
                print_success "Removed API key from keychain"
                REMOVED_API_KEY=true
            else
                print_error "Failed to remove API key"
            fi
        else
            print_info "API key preserved in keychain"
        fi
    else
        print_info "No API key configured"
    fi
}

# Remove MCP configuration
remove_mcp_config() {
    print_header "Removing MCP configuration"

    # Project-level config
    local project_mcp="${SCRIPT_DIR}/.claude/mcp_config.json"
    if [ -f "$project_mcp" ]; then
        # Create backup
        local backup_file="${project_mcp}.backup.$(date +%Y%m%d_%H%M%S)"
        cp "$project_mcp" "$backup_file"
        print_info "Created backup: $backup_file"

        # Check if this is the only server configured
        if command -v jq &> /dev/null; then
            local server_count=$(jq '.mcpServers | length' "$project_mcp")
            if [ "$server_count" -eq 1 ]; then
                # Only mnemosyne, remove entire file
                if rm -f "$project_mcp"; then
                    print_success "Removed project MCP config"
                    REMOVED_MCP_PROJECT=true
                fi
            else
                # Multiple servers, remove only mnemosyne
                local temp_file=$(mktemp)
                jq 'del(.mcpServers.mnemosyne)' "$project_mcp" > "$temp_file"
                mv "$temp_file" "$project_mcp"
                print_success "Removed Mnemosyne from project MCP config"
                REMOVED_MCP_PROJECT=true
            fi
        else
            # No jq, just remove file
            if rm -f "$project_mcp"; then
                print_success "Removed project MCP config"
                print_warning "jq not found - removed entire file (backup created)"
                REMOVED_MCP_PROJECT=true
            fi
        fi
    else
        print_info "No project MCP config found"
    fi

    # Global config
    if [ "$GLOBAL_MCP" = true ]; then
        local global_mcp="${HOME}/.claude/mcp_config.json"
        if [ -f "$global_mcp" ]; then
            # Create backup
            local backup_file="${global_mcp}.backup.$(date +%Y%m%d_%H%M%S)"
            cp "$global_mcp" "$backup_file"
            print_info "Created backup: $backup_file"

            if command -v jq &> /dev/null; then
                local server_count=$(jq '.mcpServers | length' "$global_mcp")
                if [ "$server_count" -eq 1 ]; then
                    if prompt_yes_no "Remove entire global MCP config?" "y"; then
                        rm -f "$global_mcp"
                        print_success "Removed global MCP config"
                        REMOVED_MCP_GLOBAL=true
                    fi
                else
                    local temp_file=$(mktemp)
                    jq 'del(.mcpServers.mnemosyne)' "$global_mcp" > "$temp_file"
                    mv "$temp_file" "$global_mcp"
                    print_success "Removed Mnemosyne from global MCP config"
                    REMOVED_MCP_GLOBAL=true
                fi
            else
                print_warning "jq not found - manual removal required"
                echo "Edit $global_mcp and remove the 'mnemosyne' section"
            fi
        else
            print_info "No global MCP config found"
        fi
    fi
}

# Print summary
print_summary() {
    echo ""
    echo -e "${BOLD}${GREEN}Uninstallation complete!${NC}"
    echo ""
    echo "Summary:"
    echo ""

    if [ "$REMOVED_BINARY" = true ]; then
        echo "  ${GREEN}✓${NC} Removed binary from ${BIN_DIR}"
    fi

    if [ "$REMOVED_MCP_PROJECT" = true ]; then
        echo "  ${GREEN}✓${NC} Removed project MCP configuration"
    fi

    if [ "$REMOVED_MCP_GLOBAL" = true ]; then
        echo "  ${GREEN}✓${NC} Removed global MCP configuration"
    fi

    if [ "$REMOVED_DATABASE" = true ]; then
        echo "  ${GREEN}✓${NC} Removed database files"
    fi

    if [ "$REMOVED_API_KEY" = true ]; then
        echo "  ${GREEN}✓${NC} Removed API key from keychain"
    fi

    echo ""

    if [ "$PURGE" != true ]; then
        echo "Your data was preserved:"
        echo "  - Database files remain in place"
        echo "  - API key remains in system keychain"
        echo ""
        echo "To remove all data, run: $0 --purge"
        echo ""
    fi

    # Check for any remaining files
    if [ -f "${SCRIPT_DIR}/mnemosyne.db" ] || [ -f "${HOME}/.mnemosyne/mnemosyne.db" ]; then
        echo "Remaining files:"
        [ -f "${SCRIPT_DIR}/mnemosyne.db" ] && echo "  - ${SCRIPT_DIR}/mnemosyne.db"
        [ -f "${HOME}/.mnemosyne/mnemosyne.db" ] && echo "  - ${HOME}/.mnemosyne/mnemosyne.db"
        echo ""
    fi
}

# Confirm uninstallation
confirm_uninstall() {
    echo ""
    if [ "$PURGE" = true ]; then
        if [ "$AUTO_YES" != true ]; then
            echo -e "${RED}${BOLD}WARNING: --purge will delete ALL data including databases and API keys!${NC}"
            echo ""
            if ! prompt_yes_no "Are you absolutely sure you want to continue?" "n"; then
                echo "Uninstallation cancelled."
                exit 0
            fi
        fi
    else
        if ! prompt_yes_no "Proceed with uninstallation?" "y"; then
            echo "Uninstallation cancelled."
            exit 0
        fi
    fi
}

# Main uninstallation flow
main() {
    echo -e "${BOLD}${YELLOW}"
    echo "╔═══════════════════════════════════════╗"
    echo "║  Mnemosyne Uninstallation Script    ║"
    echo "╚═══════════════════════════════════════╝"
    echo -e "${NC}"

    check_installation || true
    show_removal_plan
    confirm_uninstall
    remove_api_key          # Remove API key first (needs binary)
    remove_binary
    remove_database
    remove_mcp_config
    print_summary
}

# Run main function
main "$@"
