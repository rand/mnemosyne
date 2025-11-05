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
BIN_PATH="${HOME}/.cargo/bin/mnemosyne"

echo ""
echo "Building and installing mnemosyne..."
echo ""

# Change to project root
cd "$PROJECT_ROOT"

# Build release binary
echo "Building release binary..."
if ! RUSTFLAGS="-A warnings" cargo build --release 2>&1 | grep -v "^warning:"; then
    echo -e "${YELLOW}Build failed${NC}"
    exit 1
fi
echo -e "${GREEN}✓${NC} Build complete"

# Install using cargo install (handles dependencies and copies binary)
echo ""
echo "Installing to ${BIN_PATH}..."
if ! RUSTFLAGS="-A warnings" cargo install --path . --locked --force 2>&1 | grep -v "^warning:"; then
    echo -e "${YELLOW}Install failed${NC}"
    exit 1
fi
echo -e "${GREEN}✓${NC} Installed to ${BIN_PATH}"

# Re-sign for macOS (prevents 'zsh: killed' errors)
if [[ "$OSTYPE" == "darwin"* ]]; then
    echo ""
    echo "Re-signing binary for macOS compatibility..."

    # Strip com.apple.provenance attribute
    xattr -d com.apple.provenance "$BIN_PATH" 2>/dev/null || true

    # Force re-sign with adhoc signature
    if ! codesign --force --sign - "$BIN_PATH" 2>/dev/null; then
        echo -e "${YELLOW}⚠ Warning:${NC} Failed to re-sign binary"
        echo "You may encounter 'zsh: killed' errors"
        echo ""
        echo "To fix manually:"
        echo "  xattr -d com.apple.provenance $BIN_PATH"
        echo "  codesign --force --sign - $BIN_PATH"
        echo ""
    else
        echo -e "${GREEN}✓${NC} Binary re-signed successfully"
    fi
fi

echo ""
echo -e "${GREEN}Installation complete!${NC}"
echo ""
echo "Test with:"
echo "  mnemosyne --version"
echo "  mnemosyne status"
echo ""
