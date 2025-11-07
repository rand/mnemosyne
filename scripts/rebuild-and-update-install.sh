#!/usr/bin/env bash
#
# Fast Rebuild and Install Script for Mnemosyne
#
# Optimized for development iterations with incremental builds.
# Uses fast-release profile (thin LTO) for 50-70% faster builds.
#
# Speed Comparison:
#   Fast-release:   ~10-20s incremental, ~1-2m clean build
#   Full release:   ~40-50s incremental, ~2-3m clean build
#
# Usage:
#   ./scripts/rebuild-and-update-install.sh [OPTIONS]
#
# Options:
#   --full-release    Use full release profile (slower, production-ready)
#   --bin-dir DIR     Install to DIR (default: ~/.local/bin)
#   --help            Show help message

set -e  # Exit on error
set -o pipefail  # Pipe failures propagate

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
DEFAULT_BIN_DIR="${HOME}/.local/bin"
BIN_DIR="${DEFAULT_BIN_DIR}"
USE_FULL_RELEASE=false

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m' # No Color

# Parse arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --full-release)
            USE_FULL_RELEASE=true
            shift
            ;;
        --bin-dir)
            BIN_DIR="$2"
            shift 2
            ;;
        --help)
            cat <<EOF
${BOLD}Mnemosyne Fast Rebuild and Install${NC}

Optimized for development iterations with incremental compilation.

Usage: $0 [OPTIONS]

Options:
  --full-release    Use full release profile (slower, production-ready)
  --bin-dir DIR     Install to DIR (default: ~/.local/bin)
  --help            Show this help message

Speed Comparison:
  ${GREEN}Fast-release${NC} (default):
    - Incremental build: ~10-20 seconds
    - Clean build: ~1-2 minutes
    - Uses thin LTO, parallel codegen, incremental compilation
    - Ideal for development iterations

  ${BLUE}Full release${NC} (--full-release):
    - Incremental build: ~40-50 seconds
    - Clean build: ~2-3 minutes
    - Uses full LTO, optimal codegen
    - Production-ready binary

Technical Details:
  - Skips 'cargo install' overhead (direct binary copy)
  - Leverages sccache for dependency caching
  - Preserves macOS code signing (xattr + codesign)
  - Verifies binary executes successfully

Examples:
  $0                           # Fast rebuild (recommended)
  $0 --full-release            # Production build
  $0 --bin-dir ~/.cargo/bin    # Custom install location
EOF
            exit 0
            ;;
        *)
            echo -e "${YELLOW}Error: Unknown option $1${NC}" >&2
            echo "Run '$0 --help' for usage information." >&2
            exit 1
            ;;
    esac
done

# Detect build status (clean vs incremental)
detect_build_status() {
    local profile="$1"
    local target_dir="${PROJECT_ROOT}/target/${profile}"

    if [ -d "${target_dir}/incremental" ] && [ "$(ls -A ${target_dir}/incremental 2>/dev/null)" ]; then
        echo "incremental"
    else
        echo "clean"
    fi
}

# Build binary with timing
build_binary() {
    cd "$PROJECT_ROOT"

    local profile="release"
    local profile_name="Full Release"

    if [ "$USE_FULL_RELEASE" = false ]; then
        profile="fast-release"
        profile_name="Fast Release"
    fi

    local build_status=$(detect_build_status "$profile")
    local target_dir="target/${profile}"

    echo "" >&2
    echo -e "${BLUE}Building with ${profile_name} profile...${NC}" >&2
    echo -e "  Build type: ${build_status}" >&2
    echo -e "  Target: ${target_dir}/mnemosyne" >&2
    echo "" >&2

    # Build with timing
    local start_time=$(date +%s)

    if ! RUSTFLAGS="-A warnings" cargo build --profile "$profile"; then
        echo "" >&2
        echo -e "${YELLOW}Build failed${NC}" >&2
        exit 1
    fi

    local end_time=$(date +%s)
    local duration=$((end_time - start_time))

    # Install Python components if python feature is enabled
    if grep -q 'python' Cargo.toml 2>/dev/null; then
        echo "" >&2
        echo -e "${BLUE}Installing Python components...${NC}" >&2
        local py_start=$(date +%s)

        if ! PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 uv run maturin develop --profile "$profile" --features python --quiet; then
            echo "" >&2
            echo -e "${YELLOW}⚠ Warning:${NC} Python component installation failed" >&2
            echo "Orchestration agents may not work correctly" >&2
        else
            local py_end=$(date +%s)
            local py_duration=$((py_end - py_start))
            echo -e "${GREEN}✓${NC} Python components installed in ${py_duration}s" >&2
        fi
    fi

    # Get binary size
    local binary_size=$(ls -lh "${target_dir}/mnemosyne" 2>/dev/null | awk '{print $5}')

    echo "" >&2
    echo -e "${GREEN}✓${NC} Build complete in ${duration}s" >&2
    echo -e "  Binary size: ${binary_size}" >&2
    echo -e "  Location: ${target_dir}/mnemosyne" >&2

    # Return profile used (stdout only, no extra output)
    echo "$profile"
}

# Install binary with direct copy (skips cargo install overhead)
install_binary() {
    local profile="$1"
    local source_binary="${PROJECT_ROOT}/target/${profile}/mnemosyne"
    local dest_binary="${BIN_DIR}/mnemosyne"

    echo ""
    echo "Installing to ${BIN_DIR}..."

    # Create bin directory if needed
    mkdir -p "$BIN_DIR"

    # Remove existing file or symlink
    if [ -e "$dest_binary" ] || [ -L "$dest_binary" ]; then
        rm -f "$dest_binary"
    fi

    # Copy binary directly (much faster than cargo install)
    if ! cp "$source_binary" "$dest_binary"; then
        echo -e "${YELLOW}Failed to copy binary${NC}"
        echo "Source: $source_binary"
        echo "Dest: $dest_binary"
        exit 1
    fi

    # Make executable
    chmod +x "$dest_binary"

    echo -e "${GREEN}✓${NC} Binary installed to: ${dest_binary}"
}

# Re-sign binary for macOS Gatekeeper compatibility
sign_binary() {
    local bin_path="${BIN_DIR}/mnemosyne"

    if [[ "$OSTYPE" != "darwin"* ]]; then
        echo ""
        echo -e "${GREEN}✓${NC} Code signing not needed on ${OSTYPE}"
        return 0
    fi

    echo ""
    echo "Re-signing binary for macOS compatibility..."

    # Remove Gatekeeper attribute (prevents 'zsh: killed' errors)
    xattr -d com.apple.provenance "$bin_path" 2>/dev/null || true

    # Force re-sign with adhoc signature
    if ! codesign --force --sign - "$bin_path" 2>/dev/null; then
        echo -e "${YELLOW}⚠ Warning:${NC} Failed to re-sign binary"
        echo "You may encounter 'zsh: killed' errors on macOS"
        echo ""
        echo "To fix manually:"
        echo "  xattr -d com.apple.provenance $bin_path"
        echo "  codesign --force --sign - $bin_path"
        echo ""
        return 1
    fi

    echo -e "${GREEN}✓${NC} Binary re-signed successfully"

    # Verify code signature
    if codesign -dv "$bin_path" 2>&1 | grep -q "adhoc"; then
        echo -e "${GREEN}✓${NC} Code signature verified (adhoc)"
    else
        echo -e "${YELLOW}⚠ Warning:${NC} Unexpected code signature"
    fi
}

# Verify binary executes successfully
verify_binary() {
    echo ""
    echo "Verifying installation..."

    local bin_path="${BIN_DIR}/mnemosyne"

    # Check executable
    if [ ! -x "$bin_path" ]; then
        echo -e "${YELLOW}Binary not executable${NC}"
        exit 1
    fi
    echo -e "${GREEN}✓${NC} Binary is executable"

    # Test execution (detect SIGKILL)
    # Note: timeout command may not be available on all systems
    if command -v timeout &>/dev/null || command -v gtimeout &>/dev/null; then
        local timeout_cmd="timeout"
        command -v gtimeout &>/dev/null && timeout_cmd="gtimeout"

        if ! $timeout_cmd 5 "$bin_path" --version &>/dev/null; then
            echo -e "${YELLOW}Binary won't execute${NC}"
            echo "Possible causes:"
            echo "  - macOS Gatekeeper SIGKILL (needs re-signing)"
            echo "  - Missing library dependencies"
            echo "  - Corrupted binary"
            echo ""
            echo "Try re-signing manually:"
            echo "  xattr -d com.apple.provenance $bin_path"
            echo "  codesign --force --sign - $bin_path"
            exit 1
        fi
    else
        # No timeout command, just test execution
        if ! "$bin_path" --version &>/dev/null; then
            echo -e "${YELLOW}Binary won't execute${NC}"
            exit 1
        fi
    fi

    # Get and display version
    local version=$("$bin_path" --version 2>&1)
    echo -e "${GREEN}✓${NC} Binary verified: $version"

    # Check if bin directory is in PATH
    if [[ ":$PATH:" != *":${BIN_DIR}:"* ]]; then
        echo -e "${YELLOW}⚠ Warning:${NC} $BIN_DIR is not in your PATH"
        echo ""
        echo "Add this to your shell config (~/.zshrc, ~/.bashrc, etc.):"
        echo -e "  ${BOLD}export PATH=\"${BIN_DIR}:\$PATH\"${NC}"
        echo ""
    fi
}

# Main installation flow
main() {
    echo ""
    echo -e "${BLUE}═══════════════════════════════════════════${NC}"
    echo -e "${BLUE}  Mnemosyne Fast Rebuild & Install         ${NC}"
    echo -e "${BLUE}═══════════════════════════════════════════${NC}"

    # Build binary
    local profile=$(build_binary)

    # Install binary (direct copy, skips cargo install)
    install_binary "$profile"

    # Re-sign for macOS (prevents 'zsh: killed' errors)
    sign_binary

    # Verify binary works
    verify_binary

    # Success message
    echo ""
    echo -e "${GREEN}═══════════════════════════════════════════${NC}"
    echo -e "${GREEN}  Installation complete!                    ${NC}"
    echo -e "${GREEN}═══════════════════════════════════════════${NC}"
    echo ""
    echo "Test with:"
    echo "  ${BOLD}mnemosyne --version${NC}"
    echo "  ${BOLD}mnemosyne status${NC}"
    echo ""

    # Show speed tip based on profile used
    if [ "$USE_FULL_RELEASE" = true ]; then
        echo "Tip: For faster development iterations, omit --full-release"
        echo "     Fast-release builds are 50-70% faster and still well-optimized."
        echo ""
    else
        echo "Using fast-release profile (optimized for development speed)."
        echo "For production builds, use: ${BOLD}--full-release${NC}"
        echo ""
    fi
}

# Run main function
main "$@"
