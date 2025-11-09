#!/usr/bin/env bash
#
# Quick Build and Install Script for Mnemosyne
#
# This script provides a fast way to rebuild and install the mnemosyne binary
# for development iterations, with proper macOS code signing.
#
# Usage:
#   ./scripts/build-and-install.sh

set -e  # Exit on error
set -o pipefail  # Pipe failures propagate

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
BIN_DIR="${HOME}/.cargo/bin"
BINARIES=("mnemosyne" "mnemosyne-dash" "mnemosyne-ics")

echo ""
echo "Building and installing mnemosyne..."
echo ""

# Change to project root
cd "$PROJECT_ROOT"

# Build release binary (suppress warnings for clean output)
echo "Building release binary..."
if ! RUSTFLAGS="-A warnings" cargo build --release; then
    echo -e "${YELLOW}Build failed${NC}"
    exit 1
fi
echo -e "${GREEN}✓${NC} Build complete"

# Install Python components if python feature is enabled
if grep -q 'python' Cargo.toml 2>/dev/null; then
    echo ""
    echo "Installing Python components..."
    if ! PYO3_USE_ABI3_FORWARD_COMPATIBILITY=1 uv run maturin develop --release --features python --quiet >/dev/null 2>&1; then
        echo -e "${YELLOW}⚠ Warning:${NC} Python component installation failed"
        echo "Orchestration agents may not work correctly"
    else
        echo -e "${GREEN}✓${NC} Python components installed"
    fi
fi

# Install using cargo install (handles dependencies and copies all binaries)
echo ""
echo "Installing all binaries to ${BIN_DIR}..."
if ! RUSTFLAGS="-A warnings" cargo install --path . --bins --locked --force; then
    echo -e "${YELLOW}Install failed${NC}"
    exit 1
fi
echo -e "${GREEN}✓${NC} Installed all binaries to ${BIN_DIR}/"

# Re-sign for macOS (prevents 'zsh: killed' errors)
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo ""
    echo "Re-signing binaries for macOS compatibility..."

    for binary in "${BINARIES[@]}"; do
        local bin_path="${BIN_DIR}/${binary}"

        # Strip com.apple.provenance attribute
        xattr -d com.apple.provenance "$bin_path" 2>/dev/null || true

        # Force re-sign with adhoc signature
        if ! codesign --force --sign - "$bin_path" 2>/dev/null; then
            echo -e "${YELLOW}⚠ Warning:${NC} Failed to re-sign $binary"
            echo "You may encounter 'zsh: killed' errors"
            echo ""
            echo "To fix manually:"
            echo "  xattr -d com.apple.provenance $bin_path"
            echo "  codesign --force --sign - $bin_path"
            echo ""
        else
            echo -e "${GREEN}✓${NC} $binary re-signed successfully"
        fi
    done
fi

echo ""
echo -e "${GREEN}Installation complete!${NC}"
echo ""
echo "Test with:"
echo "  mnemosyne --version"
echo "  mnemosyne status"
echo ""
