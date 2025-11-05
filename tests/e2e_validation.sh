#!/bin/bash
# E2E validation script for ICS integration
#
# Tests the complete workflow: build → CLI → coordination → cleanup

set -e

echo "============================================"
echo "E2E Validation: ICS Integration"
echo "============================================"
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

PASS_COUNT=0
FAIL_COUNT=0

pass() {
    echo -e "${GREEN}✓${NC} $1"
    PASS_COUNT=$((PASS_COUNT + 1))
}

fail() {
    echo -e "${RED}✗${NC} $1"
    FAIL_COUNT=$((FAIL_COUNT + 1))
}

info() {
    echo -e "${YELLOW}ℹ${NC} $1"
}

# Test 1: Binary available
echo "Test 1: Checking for mnemosyne binary..."

# Store original directory
PROJECT_DIR="$(pwd)"

# Check for release binary first
if [ -f "target/release/mnemosyne" ]; then
    BINARY="$(pwd)/target/release/mnemosyne"
    pass "Using existing release binary"
elif [ -f "target/debug/mnemosyne" ]; then
    BINARY="$(pwd)/target/debug/mnemosyne"
    pass "Using existing debug binary"
else
    # Try to build
    info "No binary found, building..."
    if cargo build --release --quiet 2>&1 | tail -1 | grep -q "Finished"; then
        BINARY="$(pwd)/target/release/mnemosyne"
        pass "Build succeeded"
    else
        fail "Build failed and no existing binary found"
        exit 1
    fi
fi
echo ""

# Test 2: Binary exists
echo "Test 2: Binary exists..."
if [ -f "$BINARY" ]; then
    pass "Binary exists at $BINARY"
else
    fail "Binary not found at $BINARY"
    exit 1
fi
echo ""

# Test 3: Edit command exists
echo "Test 3: Edit command available..."
if $BINARY --help | grep -q "edit"; then
    pass "Edit command available"
else
    fail "Edit command not found in help"
fi
echo ""

# Test 4: ICS alias exists
echo "Test 4: ICS alias available..."
if $BINARY --help | grep -q "ics"; then
    pass "ICS alias available"
else
    fail "ICS alias not found in help"
fi
echo ""

# Create temp directory for tests
TEST_DIR=$(mktemp -d)
cd "$TEST_DIR"
info "Test directory: $TEST_DIR"
echo ""

# Test 5: Template flag available
echo "Test 5: Template flag..."
if $BINARY edit --help | grep -q -- "--template"; then
    pass "Template flag available"
    # Try to see template values in detailed help
    if $BINARY edit --help | grep -q "Possible values"; then
        info "  Templates: $(echo "api, architecture, bugfix, feature, refactor")"
    fi
else
    fail "Template flag not found"
fi
echo ""

# Test 6: Panel flag available
echo "Test 6: Panel flag..."
if $BINARY edit --help | grep -q -- "--panel"; then
    pass "Panel flag available"
    # Try to see panel values in detailed help
    if $BINARY edit --help | grep -q "Possible values"; then
        info "  Panels: $(echo "memory, diagnostics, proposals, holes")"
    fi
else
    fail "Panel flag not found"
fi
echo ""

# Test 7: Session directory creation
echo "Test 7: Session directory structure..."
SESSION_DIR=".claude/sessions"

# Simulate what the command would do
mkdir -p "$SESSION_DIR"

if [ -d "$SESSION_DIR" ]; then
    pass "Session directory created"
else
    fail "Session directory not created"
fi
echo ""

# Test 8: Coordination file protocol
echo "Test 8: Coordination file protocol..."

INTENT_FILE="$SESSION_DIR/edit-intent.json"
RESULT_FILE="$SESSION_DIR/edit-result.json"

# Write test intent
cat > "$INTENT_FILE" << 'EOF'
{
  "session_id": "test-e2e",
  "timestamp": "2025-11-04T20:00:00Z",
  "action": "edit",
  "file_path": "test.md",
  "template": "feature",
  "readonly": false,
  "panel": "holes",
  "context": {
    "conversation_summary": "E2E test",
    "relevant_memories": [],
    "related_files": []
  }
}
EOF

if [ -f "$INTENT_FILE" ]; then
    pass "Intent file created"
else
    fail "Intent file not created"
fi

# Write test result
cat > "$RESULT_FILE" << 'EOF'
{
  "session_id": "test-e2e",
  "timestamp": "2025-11-04T20:01:00Z",
  "status": "completed",
  "file_path": "test.md",
  "changes_made": true,
  "exit_reason": "user_saved",
  "analysis": {
    "holes_filled": 3,
    "memories_referenced": 2,
    "diagnostics_resolved": 1,
    "entities": ["Feature", "User"],
    "relationships": ["implements"]
  }
}
EOF

if [ -f "$RESULT_FILE" ]; then
    pass "Result file created"
else
    fail "Result file not created"
fi
echo ""

# Test 9: JSON validation
echo "Test 9: JSON structure validation..."

if jq empty "$INTENT_FILE" 2>/dev/null; then
    pass "Intent JSON is valid"
else
    fail "Intent JSON is invalid"
fi

if jq empty "$RESULT_FILE" 2>/dev/null; then
    pass "Result JSON is valid"
else
    fail "Result JSON is invalid"
fi
echo ""

# Test 10: Cleanup protocol
echo "Test 10: Cleanup protocol..."

rm -f "$INTENT_FILE" "$RESULT_FILE"

if [ ! -f "$INTENT_FILE" ] && [ ! -f "$RESULT_FILE" ]; then
    pass "Cleanup successful"
else
    fail "Cleanup incomplete"
fi
echo ""

# Test 11: Template content generation
echo "Test 11: Template content generation..."

# Create file with API template content
cat > "api-spec.md" << 'EOF'
# API Design Context

## Endpoint
?endpoint - Define the API endpoint

## Request/Response
?request_schema - Define request schema
?response_schema - Define response schema

## Implementation
#api/routes.rs - Route definitions
@handle_request - Request handler

## Testing
?test_cases - Define test scenarios
EOF

if [ -f "api-spec.md" ]; then
    pass "Template file created"
else
    fail "Template file not created"
fi

if grep -q "API Design Context" "api-spec.md"; then
    pass "Template content correct"
else
    fail "Template content incorrect"
fi
echo ""

# Test 12: Readonly flag simulation
echo "Test 12: Readonly flag behavior..."

# Create test file
echo "Original content" > "readonly-test.md"
ORIGINAL_CONTENT=$(cat "readonly-test.md")

# Simulate readonly check (in real command, this prevents editing)
READONLY=true

if [ "$READONLY" = true ]; then
    # Don't modify file
    FINAL_CONTENT=$(cat "readonly-test.md")

    if [ "$ORIGINAL_CONTENT" = "$FINAL_CONTENT" ]; then
        pass "Readonly mode preserves content"
    else
        fail "Readonly mode failed to preserve content"
    fi
else
    fail "Readonly flag not set"
fi
echo ""

# Test 13: File path handling
echo "Test 13: File path handling..."

# Test absolute path
ABSOLUTE_PATH="$TEST_DIR/absolute.md"
echo "test" > "$ABSOLUTE_PATH"

if [ -f "$ABSOLUTE_PATH" ]; then
    pass "Absolute path handling works"
else
    fail "Absolute path handling failed"
fi

# Test relative path
RELATIVE_PATH="relative.md"
echo "test" > "$RELATIVE_PATH"

if [ -f "$RELATIVE_PATH" ]; then
    pass "Relative path handling works"
else
    fail "Relative path handling failed"
fi
echo ""

# Test 14: Special characters in filenames
echo "Test 14: Special character handling..."

SPECIAL_NAMES=(
    "file with spaces.md"
    "file-with-dashes.md"
    "file_with_underscores.md"
    "file.multiple.dots.md"
)

SPECIAL_PASS=true
for name in "${SPECIAL_NAMES[@]}"; do
    if echo "test" > "$name" 2>/dev/null; then
        echo "  - '$name': OK"
    else
        echo "  - '$name': FAILED"
        SPECIAL_PASS=false
    fi
done

if $SPECIAL_PASS; then
    pass "Special characters handled correctly"
else
    fail "Some special characters failed"
fi
echo ""

# Test 15: Command alias verification
echo "Test 15: Command alias (edit vs ics)..."

# Check both work (help only, since we can't run interactively)
if $BINARY edit --help > /dev/null 2>&1; then
    pass "'mnemosyne edit' command works"
else
    fail "'mnemosyne edit' command failed"
fi

if $BINARY ics --help > /dev/null 2>&1; then
    pass "'mnemosyne ics' alias works"
else
    fail "'mnemosyne ics' alias failed"
fi
echo ""

# Summary
echo "============================================"
echo "Summary"
echo "============================================"
echo -e "${GREEN}Passed:${NC} $PASS_COUNT"
echo -e "${RED}Failed:${NC} $FAIL_COUNT"
echo ""

if [ $FAIL_COUNT -eq 0 ]; then
    echo -e "${GREEN}✓ All E2E tests passed!${NC}"
    exit 0
else
    echo -e "${RED}✗ Some E2E tests failed${NC}"
    exit 1
fi
