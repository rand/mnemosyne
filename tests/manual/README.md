# Manual Workflow Tests for ICS Integration

These tests require human interaction and cannot be fully automated.
Each scenario validates a specific user workflow.

## Prerequisites

1. Build mnemosyne: `cargo build --release`
2. Install binary: `cargo install --path .` or ensure `target/release/mnemosyne` is in PATH
3. Have a terminal with at least 80x24 character space

## Test Scenarios

### Scenario 1: Basic Context Editing ✓
**File**: `scenario1_basic_edit.sh`
**User Story**: "I want to edit a context file with ICS"

**Steps**:
1. Run `./scenario1_basic_edit.sh`
2. Follow interactive prompts
3. Verify file was edited correctly
4. Check no leftover coordination files

**Expected Result**: File edited, clean exit, no coordination files left

---

### Scenario 2: Template-Based Creation
**User Story**: "I want to create a new API spec from template"

**Manual Steps**:
```bash
# Create temp directory
cd $(mktemp -d)

# Launch with template
mnemosyne edit --template api new-api-spec.md

# In ICS:
# 1. Verify API template content is loaded
# 2. Verify you see: "# API Design Context"
# 3. Fill in ?endpoint hole (e.g., "POST /api/users")
# 4. Fill in ?request_schema hole
# 5. Save (Ctrl+S) and quit (Ctrl+Q)

# Verify
cat new-api-spec.md
# Should contain your edits

# Check template was applied
grep -q "API Design Context" new-api-spec.md && echo "✓ Template applied"
grep -q "POST /api/users" new-api-spec.md && echo "✓ Holes filled"
```

**Expected Result**: New file with API template, holes filled

---

### Scenario 3: Memory Panel Integration
**User Story**: "I want to reference existing memories while editing"

**Manual Steps**:
```bash
# Setup: Store test memories
mnemosyne remember -c "API design pattern: REST" -n "project:test" -i 8
mnemosyne remember -c "Use PostgreSQL for data" -n "project:test" -i 7

# Launch ICS with memory panel
cd $(mktemp -d)
mnemosyne edit --panel memory context.md

# In ICS:
# 1. Memory panel should be visible (Ctrl+M if not)
# 2. You should see the two memories listed
# 3. Browse memories with arrow keys
# 4. Type some text referencing the memories
# 5. Save and quit

# Verify
cat context.md
# Should contain your text
```

**Expected Result**: Memory panel accessible, memories displayed

---

### Scenario 4: Readonly Mode
**User Story**: "I want to view a file without risk of editing it"

**Manual Steps**:
```bash
# Create file with content
cd $(mktemp -d)
echo "# Important Document" > important.md
echo "Do not modify this!" >> important.md

# Launch in readonly mode
mnemosyne edit --readonly important.md

# In ICS:
# 1. You should see a readonly indicator
# 2. Try to type - changes should be prevented or warned
# 3. Quit (Ctrl+Q)

# Verify file unchanged
cat important.md
# Should still contain exact original content
diff <(echo "# Important Document
Do not modify this!") important.md && echo "✓ File unchanged"
```

**Expected Result**: No changes possible, file unchanged

---

### Scenario 5: Handoff Coordination
**User Story**: "The /ics command works seamlessly from Claude Code"

**Manual Steps**:
```bash
# Simulate what /ics does
cd $(mktemp -d)
mkdir -p .claude/sessions

# Create intent (what Claude Code would write)
cat > .claude/sessions/edit-intent.json << 'EOF'
{
  "session_id": "test-123",
  "timestamp": "2025-11-04T20:00:00Z",
  "action": "edit",
  "file_path": "context-draft.md",
  "template": "feature",
  "readonly": false,
  "panel": "holes",
  "context": {
    "conversation_summary": "User wants to implement authentication",
    "relevant_memories": ["mem_abc"],
    "related_files": ["src/auth.rs"]
  }
}
EOF

# Launch ICS with session context
mnemosyne edit context-draft.md \
  --template feature \
  --panel holes \
  --session-context .claude/sessions/edit-intent.json

# In ICS:
# 1. Verify feature template loaded
# 2. Verify holes panel is open
# 3. Fill some holes
# 4. Save and quit

# Check result file
if [ -f .claude/sessions/edit-result.json ]; then
  echo "✓ Result file created"
  cat .claude/sessions/edit-result.json | jq .

  # Verify structure
  cat .claude/sessions/edit-result.json | jq -e '.session_id' > /dev/null && echo "✓ Has session_id"
  cat .claude/sessions/edit-result.json | jq -e '.status' > /dev/null && echo "✓ Has status"
  cat .claude/sessions/edit-result.json | jq -e '.changes_made' > /dev/null && echo "✓ Has changes_made"
else
  echo "✗ FAIL: No result file created"
fi
```

**Expected Result**: Full handoff works, result file has correct structure

---

## Test Execution Checklist

- [ ] Scenario 1: Basic Context Editing
- [ ] Scenario 2: Template-Based Creation
- [ ] Scenario 3: Memory Panel Integration
- [ ] Scenario 4: Readonly Mode
- [ ] Scenario 5: Handoff Coordination

## Common Issues

### ICS doesn't launch
- Check binary is in PATH: `which mnemosyne`
- Check permissions: `chmod +x $(which mnemosyne)`
- Try with full path: `/path/to/target/release/mnemosyne edit file.md`

### Terminal rendering issues
- Ensure terminal size is at least 80x24
- Try different terminal emulator
- Check TERM environment variable: `echo $TERM`

### Coordination files not created
- Ensure directory exists: `mkdir -p .claude/sessions`
- Check file permissions on directory
- Run with debug logging: `RUST_LOG=debug mnemosyne edit ...`

## Reporting Results

After running tests, document results in `TEST_RESULTS.md`:

```markdown
## Manual Workflow Tests

### Scenario 1: Basic Context Editing
- **Status**: PASS/FAIL
- **Notes**: [Any observations]

### Scenario 2: Template-Based Creation
- **Status**: PASS/FAIL
- **Notes**: [Any observations]

[etc.]
```
