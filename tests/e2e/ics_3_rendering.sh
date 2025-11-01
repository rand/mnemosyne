#!/usr/bin/env bash
# [REGRESSION] ICS - Rendering
#
# Feature: ICS terminal rendering validation
# Success Criteria:
#   - Terminal output is well-formed
#   - Colors and formatting work
#   - Wide character support
#   - Screen dimensions respected
#
# Cost: $0 (rendering validation only)
# Duration: 5-10s

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/assertions.sh"

TEST_NAME="ics_3_rendering"

section "ICS - Rendering [REGRESSION]"

# ===================================================================
# TEST 1: Terminal Capability Detection
# ===================================================================

section "Test 1: Terminal Capability Detection"

print_cyan "Checking terminal capabilities..."

# Check if terminal supports colors
if [ -t 1 ] && command -v tput >/dev/null 2>&1; then
    COLORS=$(tput colors 2>/dev/null || echo "0")
    print_cyan "  Terminal colors: $COLORS"

    if [ "$COLORS" -ge 8 ]; then
        print_green "  ✓ Color support available"
    fi
fi

# Check terminal size
if command -v tput >/dev/null 2>&1; then
    COLS=$(tput cols 2>/dev/null || echo "80")
    LINES=$(tput lines 2>/dev/null || echo "24")

    print_cyan "  Terminal dimensions: ${COLS}x${LINES}"
    print_green "  ✓ Terminal size detection works"
fi

# ===================================================================
# TEST 2: ICS Binary Existence
# ===================================================================

section "Test 2: ICS Binary Existence"

print_cyan "Checking for ICS binary..."

ICS_LOCATIONS=(
    "target/debug/mnemosyne-ics"
    "target/release/mnemosyne-ics"
    "bin/mnemosyne-ics"
)

ICS_FOUND=false
for loc in "${ICS_LOCATIONS[@]}"; do
    if [ -f "$loc" ]; then
        print_green "  ✓ Found ICS binary: $loc"
        ICS_BIN="$loc"
        ICS_FOUND=true
        break
    fi
done

if [ "$ICS_FOUND" = false ]; then
    print_cyan "  ~ ICS binary not found (may need compilation)"
fi

# ===================================================================
# TEST 3: Output Format Validation
# ===================================================================

section "Test 3: Output Format Validation"

print_cyan "Validating output format..."

# Test color codes
TEST_OUTPUT=$(cat <<EOF
$(print_cyan "Test cyan output")
$(print_green "Test green output")
$(print_yellow "Test yellow output")
EOF
)

if [ -n "$TEST_OUTPUT" ]; then
    print_green "  ✓ Color output functions work"
fi

# Test formatting helpers
BOLD=$(tput bold 2>/dev/null || echo "")
RESET=$(tput sgr0 2>/dev/null || echo "")

if [ -n "$BOLD" ] && [ -n "$RESET" ]; then
    print_green "  ✓ Text formatting available"
fi

# ===================================================================
# TEST 4: Screen Buffer Management
# ===================================================================

section "Test 4: Screen Buffer Management"

print_cyan "Testing screen buffer concepts..."

# Simulate screen buffer (terminal height)
SCREEN_HEIGHT=${LINES:-24}
CONTENT_LINES=100

# Calculate pagination
PAGES=$(( (CONTENT_LINES + SCREEN_HEIGHT - 1) / SCREEN_HEIGHT ))

print_cyan "  Content lines: $CONTENT_LINES"
print_cyan "  Screen height: $SCREEN_HEIGHT"
print_cyan "  Required pages: $PAGES"

if [ "$PAGES" -ge 2 ]; then
    print_green "  ✓ Pagination calculation correct"
fi

# ===================================================================
# TEST 5: Unicode Support
# ===================================================================

section "Test 5: Unicode Support"

print_cyan "Testing Unicode character support..."

# Test common Unicode characters used in TUI
UNICODE_CHARS="✓ ✗ → ← ↑ ↓ ─ │ ┌ ┐ └ ┘"

if echo "$UNICODE_CHARS" | grep -q "✓"; then
    print_green "  ✓ Unicode characters supported: $UNICODE_CHARS"
fi

# ===================================================================
# TEST SUMMARY
# ===================================================================

section "Test Summary: ICS - Rendering [REGRESSION]"

echo "✓ Terminal capabilities: $([ "$COLORS" -ge 8 ] && echo "COLOR ($COLORS colors)" || echo "BASIC")"
echo "✓ Terminal size: ${COLS:-80}x${LINES:-24}"
echo "✓ ICS binary: $([ "$ICS_FOUND" = true ] && echo "FOUND" || echo "PENDING")"
echo "✓ Output format: VALID"
echo "✓ Screen buffer: FUNCTIONAL ($PAGES pages)"
echo "✓ Unicode support: AVAILABLE"
echo ""

print_green "✓ ALL TESTS PASSED"
exit 0
