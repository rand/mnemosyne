#!/usr/bin/env bash
set -euo pipefail

# Build D2 diagrams to SVG for GitHub Pages
# Usage:
#   ./scripts/build-diagrams.sh          # Build all diagrams
#   ./scripts/build-diagrams.sh <name>   # Build specific diagram

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
SOURCE_DIR="$PROJECT_ROOT/docs/diagrams-d2"
OUTPUT_DIR="$PROJECT_ROOT/docs/assets/diagrams"

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Check if d2 is installed
if ! command -v d2 &> /dev/null; then
    echo -e "${RED}Error: d2 is not installed${NC}"
    echo "Install with: curl -fsSL https://d2lang.com/install.sh | sh -s --"
    exit 1
fi

# Create output directory if it doesn't exist
mkdir -p "$OUTPUT_DIR"

# Function to build a single diagram
build_diagram() {
    local d2_file="$1"
    local base_name="$(basename "$d2_file" .d2)"
    local svg_file="$OUTPUT_DIR/${base_name}.svg"

    echo -e "${YELLOW}Building:${NC} $base_name"

    # Build with D2
    # Options:
    #   --theme=3: Grape Soda theme (professional purple/blue)
    #   --pad=20: Add padding around diagram
    #   --sketch=false: Clean lines (not hand-drawn)
    if d2 --theme=3 --pad=20 --sketch=false "$d2_file" "$svg_file"; then
        echo -e "${GREEN}✓${NC} Generated: $svg_file"
        return 0
    else
        echo -e "${RED}✗${NC} Failed to build: $d2_file"
        return 1
    fi
}

# Main logic
if [ $# -eq 0 ]; then
    # Build all diagrams
    echo "Building all D2 diagrams..."
    echo "Source: $SOURCE_DIR"
    echo "Output: $OUTPUT_DIR"
    echo ""

    if [ ! -d "$SOURCE_DIR" ]; then
        echo -e "${RED}Error: Source directory not found: $SOURCE_DIR${NC}"
        exit 1
    fi

    # Find all .d2 files
    d2_files=("$SOURCE_DIR"/*.d2)

    if [ ! -e "${d2_files[0]}" ]; then
        echo -e "${YELLOW}No .d2 files found in $SOURCE_DIR${NC}"
        exit 0
    fi

    built=0
    failed=0

    for d2_file in "${d2_files[@]}"; do
        if build_diagram "$d2_file"; then
            ((built++))
        else
            ((failed++))
        fi
    done

    echo ""
    echo "=================================================="
    echo -e "${GREEN}Built:${NC} $built diagrams"
    if [ $failed -gt 0 ]; then
        echo -e "${RED}Failed:${NC} $failed diagrams"
        exit 1
    fi
    echo "=================================================="

else
    # Build specific diagram
    diagram_name="$1"

    # Add .d2 extension if not present
    if [[ ! "$diagram_name" =~ \.d2$ ]]; then
        diagram_name="${diagram_name}.d2"
    fi

    d2_file="$SOURCE_DIR/$diagram_name"

    if [ ! -f "$d2_file" ]; then
        echo -e "${RED}Error: Diagram not found: $d2_file${NC}"
        exit 1
    fi

    build_diagram "$d2_file"
fi

echo ""
echo "Diagrams are ready at: $OUTPUT_DIR"
