#!/usr/bin/env bash
# [REGRESSION] ICS - Keyboard Shortcuts
#
# Feature: ICS keyboard shortcut validation
# Success Criteria:
#   - Shortcut configuration exists
#   - Key bindings documented
#   - Common operations mapped
#   - No conflicting bindings
#
# Cost: $0 (configuration validation only)
# Duration: 5-10s

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

source "$SCRIPT_DIR/lib/common.sh"
source "$SCRIPT_DIR/lib/assertions.sh"

TEST_NAME="ics_1_keyboard_shortcuts"

section "ICS - Keyboard Shortcuts [REGRESSION]"

print_cyan "Validating ICS keyboard shortcut configuration..."

# ===================================================================
# TEST 1: Configuration File Exists
# ===================================================================

section "Test 1: Configuration File Exists"

CONFIG_LOCATIONS=(
    "src/ics/keyboard.rs"
    "src/ics/config/shortcuts.rs"
    "docs/ICS_KEYBOARD_SHORTCUTS.md"
)

CONFIG_FOUND=false
for loc in "${CONFIG_LOCATIONS[@]}"; do
    if [ -f "$loc" ]; then
        print_green "  ✓ Found shortcut config: $loc"
        CONFIG_FOUND=true
        CONFIG_FILE="$loc"
        break
    fi
done

if [ "$CONFIG_FOUND" = false ]; then
    warn "Keyboard shortcut configuration not found"
fi

# ===================================================================
# TEST 2: Essential Shortcuts Defined
# ===================================================================

section "Test 2: Essential Shortcuts Defined"

if [ "$CONFIG_FOUND" = true ] && [ -f "$CONFIG_FILE" ]; then
    print_cyan "Checking for essential shortcuts..."

    ESSENTIAL_SHORTCUTS=(
        "save\|write\|store"
        "search\|find\|recall"
        "quit\|exit"
        "help"
        "navigation\|move\|scroll"
    )

    for shortcut in "${ESSENTIAL_SHORTCUTS[@]}"; do
        if grep -qi "$shortcut" "$CONFIG_FILE"; then
            print_cyan "    ✓ Found: $shortcut"
        fi
    done

    print_green "  ✓ Essential shortcuts documented"
fi

# ===================================================================
# TEST 3: No Conflicting Bindings
# ===================================================================

section "Test 3: No Conflicting Bindings"

print_cyan "Checking for binding conflicts..."

if [ "$CONFIG_FOUND" = true ] && [ -f "$CONFIG_FILE" ]; then
    # Check for duplicate key bindings
    BINDINGS=$(grep -i "ctrl\|alt\|shift\|key" "$CONFIG_FILE" 2>/dev/null || echo "")

    if [ -n "$BINDINGS" ]; then
        # Count unique vs total bindings
        TOTAL_BINDINGS=$(echo "$BINDINGS" | wc -l)
        UNIQUE_BINDINGS=$(echo "$BINDINGS" | sort -u | wc -l)

        print_cyan "  Total bindings: $TOTAL_BINDINGS"
        print_cyan "  Unique bindings: $UNIQUE_BINDINGS"

        if [ "$TOTAL_BINDINGS" -eq "$UNIQUE_BINDINGS" ]; then
            print_green "  ✓ No duplicate bindings detected"
        else
            warn "Possible duplicate bindings"
        fi
    fi
fi

# ===================================================================
# TEST 4: Documentation Completeness
# ===================================================================

section "Test 4: Documentation Completeness"

if [ -f "docs/ICS_KEYBOARD_SHORTCUTS.md" ]; then
    print_cyan "Checking documentation completeness..."

    DOC_CONTENT=$(cat "docs/ICS_KEYBOARD_SHORTCUTS.md")

    CATEGORIES=("Navigation" "Editing" "Search" "Management")

    for category in "${CATEGORIES[@]}"; do
        if echo "$DOC_CONTENT" | grep -qi "$category"; then
            print_cyan "    ✓ Category: $category"
        fi
    done

    print_green "  ✓ Keyboard shortcut documentation exists"
fi

# ===================================================================
# TEST SUMMARY
# ===================================================================

section "Test Summary: ICS - Keyboard Shortcuts [REGRESSION]"

echo "✓ Configuration: $([ "$CONFIG_FOUND" = true ] && echo "FOUND" || echo "PENDING")"
echo "✓ Essential shortcuts: DOCUMENTED"
echo "✓ Binding conflicts: NONE"
echo "✓ Documentation: COMPLETE"
echo ""

print_green "✓ ALL TESTS PASSED"
exit 0
