#!/usr/bin/env bash
#
# Mnemosyne Installation Script
#
# This script automates the installation of Mnemosyne, including:
# - Building the release binary
# - Installing to PATH
# - Database initialization
# - API key configuration (optional)
# - MCP server configuration
#
# Usage:
#   ./install.sh [OPTIONS]
#
# Options:
#   --help                Show this help message
#   --skip-api-key        Skip API key configuration
#   --bin-dir DIR         Install binary to DIR (default: ~/.local/bin)
#   --global-mcp          Install MCP config globally (~/.claude)
#   --no-mcp              Skip MCP configuration
#   --yes                 Answer yes to all prompts

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
SKIP_API_KEY=false
GLOBAL_MCP=false
NO_MCP=false
AUTO_YES=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --help)
            cat <<EOF
Mnemosyne Installation Script

Usage: $0 [OPTIONS]

Options:
  --help                Show this help message
  --skip-api-key        Skip API key configuration
  --bin-dir DIR         Install binary to DIR (default: ~/.local/bin)
  --global-mcp          Install MCP config globally (~/.claude)
  --no-mcp              Skip MCP configuration
  --yes                 Answer yes to all prompts

Examples:
  $0                                    # Standard installation
  $0 --skip-api-key                     # Install without configuring API key
  $0 --bin-dir /usr/local/bin           # Install to /usr/local/bin
  $0 --global-mcp                       # Install with global MCP config
  $0 --yes                              # Non-interactive installation

EOF
            exit 0
            ;;
        --skip-api-key)
            SKIP_API_KEY=true
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
        --no-mcp)
            NO_MCP=true
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

prompt_yes_no() {
    if [ "$AUTO_YES" = true ]; then
        return 0
    fi

    local prompt="$1"
    local default="${2:-y}"

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

# Check prerequisites
check_prerequisites() {
    print_header "Checking prerequisites"

    # Check Rust
    if ! command -v rustc &> /dev/null; then
        print_error "Rust is not installed"
        echo "Install Rust from: https://rustup.rs/"
        exit 1
    fi

    local rust_version=$(rustc --version | awk '{print $2}')
    print_success "Rust ${rust_version} found"

    # Check cargo
    if ! command -v cargo &> /dev/null; then
        print_error "Cargo is not installed"
        exit 1
    fi
    print_success "Cargo found"

    # Check git (for version info)
    if command -v git &> /dev/null; then
        print_success "Git found"
    else
        print_warning "Git not found (optional)"
    fi
}

# Build binary
build_binary() {
    print_header "Building Mnemosyne (release mode)"

    cd "$SCRIPT_DIR"

    if ! cargo build --release; then
        print_error "Failed to build Mnemosyne"
        exit 1
    fi

    if [ ! -f "target/release/mnemosyne" ]; then
        print_error "Binary not found after build"
        exit 1
    fi

    print_success "Build complete"
}

# Install binary
install_binary() {
    print_header "Installing binary to ${BIN_DIR}"

    # Create bin directory if it doesn't exist
    if [ ! -d "$BIN_DIR" ]; then
        mkdir -p "$BIN_DIR"
        print_success "Created directory: $BIN_DIR"
    fi

    # Copy binary
    if ! cp -f "${SCRIPT_DIR}/target/release/mnemosyne" "${BIN_DIR}/mnemosyne"; then
        print_error "Failed to copy binary to $BIN_DIR"
        exit 1
    fi

    # Make executable
    chmod +x "${BIN_DIR}/mnemosyne"
    print_success "Installed to ${BIN_DIR}/mnemosyne"

    # Check if bin directory is in PATH
    if [[ ":$PATH:" != *":${BIN_DIR}:"* ]]; then
        print_warning "$BIN_DIR is not in your PATH"
        echo ""
        echo "Add this to your shell config (~/.bashrc, ~/.zshrc, etc.):"
        echo -e "  ${BOLD}export PATH=\"${BIN_DIR}:\$PATH\"${NC}"
        echo ""
    fi
}

# Initialize database
init_database() {
    print_header "Initializing database"

    # Run mnemosyne init
    if "${BIN_DIR}/mnemosyne" init; then
        print_success "Database initialized"
    else
        print_error "Failed to initialize database"
        exit 1
    fi
}

# Configure API key
configure_api_key() {
    if [ "$SKIP_API_KEY" = true ]; then
        print_header "Skipping API key configuration"
        return 0
    fi

    print_header "Configuring Anthropic API key"

    # Check if key already exists
    if "${BIN_DIR}/mnemosyne" config show-key &> /dev/null; then
        if prompt_yes_no "API key already configured. Reconfigure?" "n"; then
            "${BIN_DIR}/mnemosyne" config delete-key || true
        else
            print_success "Using existing API key"
            return 0
        fi
    fi

    # Interactive setup
    echo ""
    echo "You need an Anthropic API key for LLM features."
    echo "Get your key from: https://console.anthropic.com/settings/keys"
    echo ""

    if prompt_yes_no "Configure API key now?" "y"; then
        if "${BIN_DIR}/mnemosyne" config set-key; then
            print_success "API key configured"
        else
            print_warning "Failed to configure API key"
            echo "You can configure it later with: mnemosyne config set-key"
        fi
    else
        echo ""
        echo "You can configure the API key later with:"
        echo "  ${BOLD}mnemosyne config set-key${NC}"
        echo ""
        echo "Or set the environment variable:"
        echo "  ${BOLD}export ANTHROPIC_API_KEY=sk-ant-api03-...${NC}"
        echo ""
    fi
}

# Merge MCP configuration
merge_mcp_config() {
    local mcp_file="$1"
    local mcp_dir="$(dirname "$mcp_file")"

    # Create directory if needed
    mkdir -p "$mcp_dir"

    # MCP configuration for Mnemosyne
    local mnemosyne_config='{
  "mcpServers": {
    "mnemosyne": {
      "command": "mnemosyne",
      "args": ["serve"],
      "env": {
        "RUST_LOG": "info"
      },
      "description": "Mnemosyne - Project-aware agentic memory system"
    }
  }
}'

    if [ -f "$mcp_file" ]; then
        # File exists, merge configurations
        print_success "Found existing MCP config"

        # Check if mnemosyne is already configured
        if grep -q '"mnemosyne"' "$mcp_file"; then
            print_warning "Mnemosyne already configured in $mcp_file"

            if prompt_yes_no "Update Mnemosyne MCP configuration?" "y"; then
                # Create backup
                cp "$mcp_file" "${mcp_file}.backup.$(date +%Y%m%d_%H%M%S)"
                print_success "Created backup"

                # Use jq if available, otherwise manual merge
                if command -v jq &> /dev/null; then
                    local temp_file=$(mktemp)
                    echo "$mnemosyne_config" | jq -s '.[0] * .[1]' "$mcp_file" - > "$temp_file"
                    mv "$temp_file" "$mcp_file"
                    print_success "Updated Mnemosyne configuration"
                else
                    print_warning "jq not found - manual merge required"
                    echo "Add this to $mcp_file manually:"
                    echo "$mnemosyne_config"
                fi
            fi
        else
            # Add mnemosyne to existing config
            print_success "Adding Mnemosyne to existing MCP config"

            # Create backup
            cp "$mcp_file" "${mcp_file}.backup.$(date +%Y%m%d_%H%M%S)"

            if command -v jq &> /dev/null; then
                local temp_file=$(mktemp)
                echo "$mnemosyne_config" | jq -s '.[0] * .[1]' "$mcp_file" - > "$temp_file"
                mv "$temp_file" "$mcp_file"
                print_success "Merged MCP configuration"
            else
                print_warning "jq not found - manual merge required"
                echo "Add this to the 'mcpServers' section in $mcp_file:"
                echo "$mnemosyne_config" | jq .mcpServers.mnemosyne
            fi
        fi
    else
        # Create new file
        echo "$mnemosyne_config" > "$mcp_file"
        print_success "Created MCP configuration: $mcp_file"
    fi
}

# Configure MCP server
configure_mcp() {
    if [ "$NO_MCP" = true ]; then
        print_header "Skipping MCP configuration"
        return 0
    fi

    print_header "Configuring MCP server"

    if [ "$GLOBAL_MCP" = true ]; then
        # Global configuration
        local mcp_file="${HOME}/.claude/mcp_config.json"
        echo "Installing global MCP configuration..."
        merge_mcp_config "$mcp_file"
        echo ""
        print_success "Global MCP configuration installed"
        echo "This will be available to all Claude Code projects."
    else
        # Project-level configuration
        if prompt_yes_no "Configure MCP for this project?" "y"; then
            local mcp_file="${SCRIPT_DIR}/.claude/mcp_config.json"
            merge_mcp_config "$mcp_file"
        fi

        echo ""
        if prompt_yes_no "Also install global MCP configuration?" "n"; then
            local mcp_file="${HOME}/.claude/mcp_config.json"
            merge_mcp_config "$mcp_file"
            print_success "Global MCP configuration installed"
        fi
    fi

    echo ""
    echo "MCP server will be available as: ${BOLD}mnemosyne${NC}"
    echo "Claude Code will automatically start the server when needed."
}

# Verify installation
verify_installation() {
    print_header "Verifying installation"

    # Check binary exists and is executable
    if [ ! -x "${BIN_DIR}/mnemosyne" ]; then
        print_error "Binary not found or not executable"
        return 1
    fi
    print_success "Binary is executable"

    # Check version
    local version=$("${BIN_DIR}/mnemosyne" --version 2>&1 || echo "unknown")
    print_success "Version: $version"

    # Check status
    if "${BIN_DIR}/mnemosyne" status &> /dev/null; then
        print_success "Status check passed"
    else
        print_warning "Status check returned non-zero (may be expected)"
    fi

    # Check API key
    if "${BIN_DIR}/mnemosyne" config show-key &> /dev/null; then
        print_success "API key is configured"
    else
        print_warning "API key not configured"
    fi
}

# Print next steps
print_next_steps() {
    echo ""
    echo -e "${BOLD}${GREEN}Installation complete!${NC}"
    echo ""
    echo "Next steps:"
    echo ""
    echo "1. ${BOLD}Test the installation:${NC}"
    echo "   mnemosyne status"
    echo ""
    echo "2. ${BOLD}Start using Mnemosyne in Claude Code:${NC}"
    echo "   The MCP server will start automatically."
    echo "   Use tools like: mnemosyne.remember, mnemosyne.recall, etc."
    echo ""
    echo "3. ${BOLD}Manual testing (optional):${NC}"
    echo "   echo '{\"jsonrpc\":\"2.0\",\"method\":\"initialize\",\"id\":1}' | mnemosyne serve"
    echo ""

    if [ "$SKIP_API_KEY" = true ] || ! "${BIN_DIR}/mnemosyne" config show-key &> /dev/null; then
        echo "4. ${BOLD}Configure API key:${NC}"
        echo "   mnemosyne config set-key"
        echo ""
    fi

    echo "For more information:"
    echo "  - README: ${SCRIPT_DIR}/README.md"
    echo "  - Installation guide: ${SCRIPT_DIR}/INSTALL.md"
    echo "  - MCP server docs: ${SCRIPT_DIR}/MCP_SERVER.md"
    echo ""
}

# Main installation flow
main() {
    echo -e "${BOLD}${BLUE}"
    echo "╔═══════════════════════════════════════╗"
    echo "║   Mnemosyne Installation Script      ║"
    echo "║   Project-aware agentic memory       ║"
    echo "╚═══════════════════════════════════════╝"
    echo -e "${NC}"

    check_prerequisites
    build_binary
    install_binary
    init_database
    configure_api_key
    configure_mcp
    verify_installation
    print_next_steps
}

# Run main function
main "$@"
