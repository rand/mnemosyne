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
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
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

# Monitor cargo build with real-time progress streaming
monitor_cargo_build() {
    local start_time=$(date +%s)
    local dep_count=0
    local main_crate_started=false
    local last_update_time=$start_time

    # Create temporary file for output
    local temp_output=$(mktemp)

    # Run cargo with stderr+stdout merged, tee to temp file
    {
        cargo build --release 2>&1 | while IFS= read -r line; do
            # Stream output to user
            echo "$line"

            # Parse for milestones
            if [[ "$line" =~ Compiling\ ([a-zA-Z0-9_-]+)\ v([0-9.]+) ]]; then
                dep_count=$((dep_count + 1))
                local current_time=$(date +%s)

                # Show progress every 10 dependencies or every 30 seconds
                if (( dep_count % 10 == 0 )) || (( current_time - last_update_time >= 30 )); then
                    local elapsed=$((current_time - start_time))
                    local minutes=$((elapsed / 60))
                    local seconds=$((elapsed % 60))
                    echo -e "${BLUE}   ⏱  Progress: $dep_count crates compiled (${minutes}m ${seconds}s elapsed)${NC}" >&2
                    last_update_time=$current_time
                fi
            fi

            # Detect main crate compilation
            if [[ "$line" =~ Compiling\ mnemosyne ]] && [ "$main_crate_started" = false ]; then
                main_crate_started=true
                local elapsed=$(($(date +%s) - start_time))
                local minutes=$((elapsed / 60))
                local seconds=$((elapsed % 60))
                echo -e "${GREEN}   ✓ Dependencies complete! Building main binary... (${minutes}m ${seconds}s)${NC}" >&2
            fi
        done

        # Return cargo's exit code
        echo ${PIPESTATUS[0]}
    } | tee "$temp_output" | tail -1

    local exit_code=$?
    rm -f "$temp_output"
    return $exit_code
}

# Show build error with specific fix instructions
show_build_error() {
    local error_output="$1"
    local error_log="/tmp/mnemosyne-build-error-$(date +%s).log"

    # Save full error log
    echo "$error_output" > "$error_log"

    echo ""
    print_error "Build failed"
    echo ""

    # Detect error type and show specific fix
    if echo "$error_output" | grep -q "linker.*not found\|cannot find -l"; then
        echo "Common cause: Missing C compiler and linker"
        echo ""
        echo "Fix:"
        if [[ "$OSTYPE" == "darwin"* ]]; then
            echo "  ${BOLD}xcode-select --install${NC}"
        elif [[ -f /etc/debian_version ]]; then
            echo "  ${BOLD}sudo apt-get install build-essential${NC}"
        elif [[ -f /etc/redhat-release ]]; then
            echo "  ${BOLD}sudo dnf groupinstall 'Development Tools'${NC}"
        else
            echo "  Install your distribution's build tools package"
        fi
    elif echo "$error_output" | grep -q "failed to run custom build command"; then
        echo "Common cause: Incompatible Rust version or missing dependencies"
        echo ""
        echo "Fix:"
        echo "  ${BOLD}rustup update stable${NC}"
        echo "  ${BOLD}rustup default stable${NC}"
        echo "  ${BOLD}cargo clean${NC}"
        echo "  Then retry: ${BOLD}./scripts/install/install.sh${NC}"
    elif echo "$error_output" | grep -q "could not compile"; then
        echo "Build compilation error detected."
        echo "See full error log for details."
    else
        echo "An unexpected build error occurred."
        echo "See full error log for details."
    fi

    echo ""
    echo "Full build log saved to: ${BOLD}$error_log${NC}"
    echo "For more help: ${BOLD}${PROJECT_ROOT}/TROUBLESHOOTING.md${NC}"
    echo ""
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

    # Pre-build messaging with expectations
    echo ""
    echo "This will compile ~150 Rust dependencies plus the main binary."
    echo -e "Expected time: ${BOLD}2-3 minutes${NC} on most systems (longer on first build)"
    echo ""
    echo "Build progress will stream below - this is normal!"
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    echo ""

    cd "$PROJECT_ROOT"

    # Track build time
    local build_start=$(date +%s)

    # Run build with progress monitoring
    if ! monitor_cargo_build; then
        # Build failed - capture error output for diagnosis
        local error_output=$(cargo build --release 2>&1)
        show_build_error "$error_output"
        exit 1
    fi

    # Calculate build time
    local build_end=$(date +%s)
    local build_duration=$((build_end - build_start))
    local build_minutes=$((build_duration / 60))
    local build_seconds=$((build_duration % 60))

    # Verify binary exists
    if [ ! -f "target/release/mnemosyne" ]; then
        print_error "Binary not found after build"
        exit 1
    fi

    # Get binary size
    local binary_size=$(ls -lh target/release/mnemosyne | awk '{print $5}')

    # Success summary
    echo ""
    echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
    print_success "Build complete in ${build_minutes}m ${build_seconds}s"
    print_success "Binary size: $binary_size"
    print_success "Location: target/release/mnemosyne"
    echo ""
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
    if ! cp -f "${PROJECT_ROOT}/target/release/mnemosyne" "${BIN_DIR}/mnemosyne"; then
        print_error "Failed to copy binary to $BIN_DIR"
        exit 1
    fi

    # Make executable
    chmod +x "${BIN_DIR}/mnemosyne"

    # On macOS, clear extended attributes and re-sign to avoid SIGKILL issues
    if [[ "$OSTYPE" == "darwin"* ]]; then
        xattr -c "${BIN_DIR}/mnemosyne" 2>/dev/null || true
        codesign --force --sign - "${BIN_DIR}/mnemosyne" 2>/dev/null || {
            print_warning "Could not re-sign binary (codesign failed)"
            echo "  Binary may not run correctly. If you get 'Killed: 9' errors:"
            echo "  Run: codesign --force --sign - ${BIN_DIR}/mnemosyne"
        }
    fi

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

    # Check if database already exists (project-local takes precedence)
    local db_path=""
    if [ -f ".mnemosyne/project.db" ]; then
        db_path=".mnemosyne/project.db"
    elif [ -f "${HOME}/.local/share/mnemosyne/mnemosyne.db" ]; then
        db_path="${HOME}/.local/share/mnemosyne/mnemosyne.db"
    fi

    if [ -n "$db_path" ]; then
        print_success "Database already exists: $db_path"
        echo "  Skipping initialization"
        return 0
    fi

    # Run mnemosyne init
    echo "  Initializing new database..."
    if "${BIN_DIR}/mnemosyne" init; then
        print_success "Database initialized"
    else
        print_error "Failed to initialize database"
        echo ""
        echo "Troubleshooting:"
        echo "  1. Check if another instance is running:"
        echo "     ps aux | grep mnemosyne"
        echo "  2. Try initializing manually:"
        echo "     ${BIN_DIR}/mnemosyne init"
        echo "  3. Check logs for errors (RUST_LOG=debug)"
        echo ""
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
            local mcp_file="${PROJECT_ROOT}/.claude/mcp_config.json"
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

# Configure Claude Code hooks
configure_hooks() {
    print_header "Configuring Claude Code hooks (optional)"

    # Ask user if they want hooks configured
    if ! prompt_yes_no "Configure Claude Code hooks for automatic memory capture?" "y"; then
        echo "Skipping hooks configuration"
        echo "You can configure hooks later - see INSTALL.md for details"
        return 0
    fi

    # Check if .claude directory exists
    if [ ! -d "${PROJECT_ROOT}/.claude" ]; then
        mkdir -p "${PROJECT_ROOT}/.claude"
        print_success "Created .claude directory"
    fi

    # Check if hooks directory exists
    if [ ! -d "${PROJECT_ROOT}/.claude/hooks" ]; then
        print_error "Hooks directory not found: ${PROJECT_ROOT}/.claude/hooks"
        echo "This project may not have hooks configured."
        return 1
    fi

    # Make hooks executable
    chmod +x "${PROJECT_ROOT}/.claude/hooks/"*.sh
    print_success "Made hooks executable"

    # Configure settings.json with absolute paths
    local settings_file="${PROJECT_ROOT}/.claude/settings.json"
    local hooks_config=$(cat <<EOF
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": ".*",
        "hooks": [
          {
            "type": "command",
            "command": "${PROJECT_ROOT}/.claude/hooks/session-start.sh"
          }
        ]
      }
    ],
    "PreCompact": [
      {
        "matcher": ".*",
        "hooks": [
          {
            "type": "command",
            "command": "${PROJECT_ROOT}/.claude/hooks/pre-compact.sh"
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "^Bash\\\\(git commit.*",
        "hooks": [
          {
            "type": "command",
            "command": "${PROJECT_ROOT}/.claude/hooks/post-commit.sh"
          }
        ]
      }
    ]
  }
}
EOF
)

    if [ -f "$settings_file" ]; then
        # Settings file exists - merge with existing config
        print_success "Found existing settings.json"

        # Create backup
        cp "$settings_file" "${settings_file}.backup.$(date +%Y%m%d_%H%M%S)"
        print_success "Created backup"

        # Merge using jq if available
        if command -v jq &> /dev/null; then
            local temp_file=$(mktemp)
            echo "$hooks_config" | jq -s '.[0] * .[1]' "$settings_file" - > "$temp_file"
            mv "$temp_file" "$settings_file"
            print_success "Merged hooks configuration with absolute paths"
        else
            print_warning "jq not found - cannot merge automatically"
            echo "Add the hooks configuration manually to: $settings_file"
            echo "$hooks_config"
        fi
    else
        # Create new settings file
        echo "$hooks_config" > "$settings_file"
        print_success "Created settings.json with hooks configuration"
    fi

    echo ""
    echo "Hooks configured with absolute paths:"
    echo "  - SessionStart: Load memories at session start"
    echo "  - PreCompact: Save context before compaction"
    echo "  - PostToolUse: Capture git commits"
    echo ""
    echo "Note: Absolute paths prevent issues after context compaction"
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
    echo -e "1. ${BOLD}Test the installation:${NC}"
    echo "   mnemosyne status"
    echo ""
    echo -e "2. ${BOLD}Start using Mnemosyne in Claude Code:${NC}"
    echo "   The MCP server will start automatically."
    echo "   Use tools like: mnemosyne.remember, mnemosyne.recall, etc."
    echo ""
    echo -e "3. ${BOLD}Manual testing (optional):${NC}"
    echo "   echo '{\"jsonrpc\":\"2.0\",\"method\":\"initialize\",\"id\":1}' | mnemosyne serve"
    echo ""

    if [ "$SKIP_API_KEY" = true ] || ! "${BIN_DIR}/mnemosyne" config show-key &> /dev/null; then
        echo -e "4. ${BOLD}Configure API key:${NC}"
        echo "   mnemosyne config set-key"
        echo ""
    fi

    echo "For more information:"
    echo "  - README: ${PROJECT_ROOT}/README.md"
    echo "  - Installation guide: ${PROJECT_ROOT}/INSTALL.md"
    echo "  - MCP server docs: ${PROJECT_ROOT}/MCP_SERVER.md"
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
    configure_hooks
    verify_installation
    print_next_steps
}

# Run main function
main "$@"
