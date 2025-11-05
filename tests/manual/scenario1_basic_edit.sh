#!/bin/bash
# Scenario 1: Basic Context Editing
#
# User Story: "I want to edit a context file with ICS"
#
# This is a MANUAL test - requires human interaction

set -e

echo "=== Scenario 1: Basic Context Editing ==="
echo ""
echo "This test requires manual interaction with ICS."
echo "Please follow the steps and verify the expected results."
echo ""

# Setup
TEST_DIR=$(mktemp -d)
cd "$TEST_DIR"

echo "Test directory: $TEST_DIR"
echo ""

# Step 1: Create test file
echo "Step 1: Creating test file..."
echo "# Context" > test-context.md
echo "✓ Created test-context.md"
echo ""

# Step 2: Launch ICS
echo "Step 2: Launching ICS..."
echo "Command: mnemosyne edit test-context.md"
echo ""
echo "MANUAL STEPS:"
echo "1. ICS should open with the file loaded"
echo "2. You should see '# Context' in the editor"
echo "3. Type some additional text (e.g., 'This is a test')"
echo "4. Press Ctrl+S to save"
echo "5. Press Ctrl+Q to quit"
echo ""
read -p "Press ENTER to launch ICS..."

mnemosyne edit test-context.md

# Step 3: Verify edits
echo ""
echo "Step 3: Verifying edits..."
if grep -q "This is a test" test-context.md 2>/dev/null; then
    echo "✓ File contains the edits"
else
    echo "✗ WARNING: Expected text not found"
    echo "  File content:"
    cat test-context.md
fi

# Step 4: Check for leftover files
echo ""
echo "Step 4: Checking for leftover coordination files..."
if [ -f .claude/sessions/edit-intent.json ]; then
    echo "✗ WARNING: Intent file still exists (should be cleaned up)"
else
    echo "✓ No intent file (good)"
fi

if [ -f .claude/sessions/edit-result.json ]; then
    echo "✗ WARNING: Result file still exists (should be cleaned up)"
else
    echo "✓ No result file (good)"
fi

echo ""
echo "=== Scenario 1 Complete ==="
echo "Test directory: $TEST_DIR"
echo "Review the results above and mark as PASS or FAIL"
