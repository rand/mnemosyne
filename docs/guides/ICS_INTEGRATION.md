# ICS Integration Guide

**Integrated Context Studio (ICS) within Mnemosyne**

The ICS integration allows you to access powerful context editing features directly from within Claude Code sessions, without breaking your workflow.

---

## Overview

ICS is now integrated directly into `mnemosyne` via the `edit` command (alias `ics`). This provides:

- **Seamless Integration**: Edit context files from within Claude Code sessions
- **Template System**: 5 built-in templates for common scenarios
- **Memory Panel**: Browse and reference existing memories while editing
- **Typed Holes**: Structured prompts for guided context creation
- **File-Based Handoff**: Automatic coordination between Claude Code and ICS

### Architecture

```
Claude Code Session
        ↓
    /ics command (.claude/commands/ics.md)
        ↓
    mnemosyne edit [file] [options]
        ↓
    Full ICS Terminal (CRDT editor, vim mode, panels)
        ↓
    Save & Exit
        ↓
    Context returns to Claude Code
```

---

## Quick Start

### Basic Usage

From within a Claude Code session:

```bash
# Edit a context file
/ics context.md

# Create with template
/ics --template feature new-feature.md

# Open memory panel
/ics --panel memory context.md

# Read-only viewing
/ics --readonly important.md
```

### Command-Line Usage

Outside of Claude Code sessions:

```bash
# Direct invocation
mnemosyne edit context.md
mnemosyne ics context.md  # Alias

# With options
mnemosyne edit --template api api-spec.md
mnemosyne edit --panel memory --template feature feature.md
```

---

## Templates

### Available Templates

ICS provides 5 built-in templates for common scenarios:

#### 1. API Design (`api`)

Template for designing API endpoints with typed holes for:
- Endpoint definition
- Request/response schemas
- Implementation files
- Test cases

**Example**:
```bash
/ics --template api auth-endpoint.md
```

**Template Structure**:
```markdown
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
```

#### 2. Architecture Decision (`architecture`)

Template for documenting architectural decisions:
- Context and problem statement
- Decision rationale
- Consequences and trade-offs
- Alternative approaches

**Example**:
```bash
/ics --template architecture database-choice.md
```

#### 3. Bug Fix (`bugfix`)

Template for documenting bug fixes:
- Issue description and reproduction
- Root cause analysis
- Fix implementation
- Test coverage

**Example**:
```bash
/ics --template bugfix race-condition.md
```

#### 4. Feature Implementation (`feature`)

Template for planning new features:
- Requirements and goals
- Architecture and design
- Implementation components
- Testing plan

**Example**:
```bash
/ics --template feature user-auth.md
```

#### 5. Refactoring (`refactor`)

Template for planning refactoring work:
- Current state analysis
- Target design
- Migration strategy
- Risk mitigation

**Example**:
```bash
/ics --template refactor database-layer.md
```

---

## Panels

### Available Panels

ICS provides 4 panels that can be opened during editing:

#### 1. Memory Panel (`memory`)

Browse and reference existing memories from your project:

```bash
/ics --panel memory context.md
```

**Features**:
- View memories by namespace
- Filter by importance
- See semantic relationships
- Reference memories in your context

**Use Cases**:
- Recalling architecture decisions
- Finding related implementations
- Checking existing patterns

#### 2. Diagnostics Panel (`diagnostics`)

View real-time analysis of your context:

```bash
/ics --panel diagnostics draft.md
```

**Features**:
- Ambiguity detection
- Typed hole validation
- Semantic triple extraction
- Dependency analysis

**Use Cases**:
- Validating context completeness
- Detecting unclear requirements
- Ensuring all holes are filled

#### 3. Proposals Panel (`proposals`)

AI-powered suggestions for context improvement:

```bash
/ics --panel proposals spec.md
```

**Features**:
- Auto-generated clarifications
- Missing information detection
- Consistency checking

**Use Cases**:
- Improving draft specifications
- Finding gaps in documentation
- Enhancing clarity

#### 4. Holes Panel (`holes`)

Track and manage typed holes in your context:

```bash
/ics --panel holes feature.md
```

**Features**:
- List all `?hole` markers
- Track completion status
- Navigate between holes
- Validation

**Use Cases**:
- Template-based workflows
- Systematic context creation
- Progress tracking

---

## ICS Pattern Language

### Typed Holes (`?name`)

Placeholders for information that needs to be filled in:

```markdown
## Authentication
?auth_method - Which auth strategy should we use?
?session_timeout - How long should sessions last?
```

**Behavior**:
- Highlighted in the editor
- Tracked in holes panel
- Validated on save

### File References (`#path`)

References to specific files in your codebase:

```markdown
## Implementation
#src/auth/jwt.rs - JWT token handling
#src/middleware/auth.rs - Auth middleware
```

**Behavior**:
- Syntax highlighted
- Enables quick navigation (future)
- Creates semantic links

### Symbol References (`@symbol`)

References to specific functions, types, or symbols:

```markdown
## Key Functions
@authenticate_user - Main auth function
@generate_token - Token generation
@validate_permissions - Permission checking
```

**Behavior**:
- Syntax highlighted
- Creates semantic relationships
- Enables symbol search (future)

---

## Options

### `--template <TEMPLATE>`

Start with a pre-filled template. Available templates:
- `api` - API endpoint design
- `architecture` - Architecture decisions
- `bugfix` - Bug fix documentation
- `feature` - Feature implementation
- `refactor` - Refactoring plans

**Example**:
```bash
mnemosyne edit --template feature new-auth.md
```

### `--panel <PANEL>`

Open ICS with a specific panel visible. Available panels:
- `memory` - Browse project memories
- `diagnostics` - Real-time analysis
- `proposals` - AI suggestions
- `holes` - Typed hole tracker

**Example**:
```bash
mnemosyne edit --panel memory context.md
```

### `--readonly`

Open file in read-only mode (view without editing):

**Example**:
```bash
mnemosyne edit --readonly important-doc.md
```

**Use Cases**:
- Reviewing without risk of accidental changes
- Sharing context with collaborators
- Viewing archived decisions

---

## Workflows

### 1. Feature Planning with Claude Code

**Goal**: Plan a new feature with AI assistance

```bash
# From Claude Code session
/ics --template feature --panel holes user-notifications.md

# In ICS:
# 1. Fill in ?requirements hole
# 2. Fill in ?architecture hole
# 3. Use diagnostics panel to validate
# 4. Save (Ctrl+S) and quit (Ctrl+Q)

# Back in Claude Code - context is now available
```

### 2. API Design Review

**Goal**: Design and validate an API endpoint

```bash
/ics --template api --panel diagnostics /api/users.md

# In ICS:
# 1. Define endpoint and schemas
# 2. Check diagnostics for issues
# 3. Reference existing APIs via #file markers
# 4. Save and return to Claude
```

### 3. Architecture Decision Documentation

**Goal**: Document a design decision for future reference

```bash
/ics --template architecture --panel memory database-choice.md

# In ICS:
# 1. Browse memory panel for related decisions
# 2. Fill in decision rationale
# 3. Document alternatives and trade-offs
# 4. Save - will be stored as memory
```

### 4. Context Refinement Loop

**Goal**: Iteratively improve context quality

```bash
/ics --panel proposals context-draft.md

# In ICS:
# 1. Review AI proposals
# 2. Address ambiguities
# 3. Fill missing information
# 4. Re-validate with diagnostics
# 5. Repeat until complete
```

---

## Keyboard Shortcuts

### Vim Mode

ICS includes full vim keybindings when editing:

**Movement**:
- `h/j/k/l` - Left/Down/Up/Right
- `w/b/e` - Word forward/backward/end
- `f/F` - Find character forward/backward
- `t/T` - Till character forward/backward
- `gg/G` - Start/End of file
- `PageUp/PageDown` - Page navigation

**Editing**:
- `i/a` - Insert before/after cursor
- `I/A` - Insert at line start/end
- `o/O` - Open line below/above
- `dd` - Delete line
- `yy` - Yank (copy) line
- `p` - Paste
- `u` - Undo
- `Ctrl+r` - Redo

**Modes**:
- `Esc` - Return to normal mode
- `v` - Visual mode

### Global Shortcuts

- `Ctrl+S` - Save file
- `Ctrl+Q` - Quit (prompts if unsaved)
- `Ctrl+M` - Toggle memory panel
- `Ctrl+D` - Toggle diagnostics panel
- `?` - Help overlay

---

## Integration with Claude Code

### Slash Command (`/ics`)

The `/ics` slash command is defined in `.claude/commands/ics.md`:

**Workflow**:
1. User types `/ics [options] <file>`
2. Claude Code creates `.claude/sessions/edit-intent.json`
3. Launches `mnemosyne edit` with session context
4. ICS runs with full terminal control
5. On exit, writes `.claude/sessions/edit-result.json`
6. Claude Code reads result and continues conversation

**Intent Structure**:
```json
{
  "session_id": "unique-id",
  "timestamp": "2025-11-04T20:00:00Z",
  "action": "edit",
  "file_path": "context.md",
  "template": "feature",
  "readonly": false,
  "panel": "memory",
  "context": {
    "conversation_summary": "User wants to implement auth",
    "relevant_memories": ["mem_abc"],
    "related_files": ["src/auth.rs"]
  }
}
```

**Result Structure**:
```json
{
  "session_id": "unique-id",
  "timestamp": "2025-11-04T20:01:00Z",
  "status": "completed",
  "file_path": "context.md",
  "changes_made": true,
  "exit_reason": "user_saved",
  "analysis": {
    "holes_filled": 3,
    "memories_referenced": 2,
    "diagnostics_resolved": 1,
    "entities": ["User", "Auth"],
    "relationships": ["implements"]
  }
}
```

### Cleanup

Coordination files are automatically cleaned up after use:
- `.claude/sessions/edit-intent.json` - Removed
- `.claude/sessions/edit-result.json` - Removed

Files are only present during active handoff.

---

## Advanced Usage

### Combining Options

Multiple options can be combined for powerful workflows:

```bash
# Feature planning with memory reference
/ics --template feature --panel memory --session-context .claude/sessions/intent.json feature.md

# Read-only review with diagnostics
/ics --readonly --panel diagnostics old-spec.md
```

### Custom Templates

While built-in templates cover common cases, you can create custom workflows:

1. Create initial file with your structure
2. Use `?hole` markers for guided filling
3. Reference with `#file` and `@symbol` markers
4. Open with appropriate panel

**Example Custom Template**:
```markdown
# Custom Workflow: Database Migration

## Current Schema
#db/schema.sql - Current schema

## Target Schema
?new_tables - What tables do we need?
?migrations - What migration steps?

## Data Migration
?data_transform - How to migrate existing data?
?rollback_plan - What's the rollback strategy?

## Testing
?test_data - What test scenarios?
@run_migration - Migration executor function
```

### Programmatic Integration

The coordination protocol can be used programmatically:

```rust
use mnemosyne::coordination::{HandoffCoordinator, EditIntent, EditContext};

// Create coordinator
let coordinator = HandoffCoordinator::new(session_dir)?;

// Write intent
let intent = EditIntent {
    session_id: "my-session".to_string(),
    timestamp: chrono::Utc::now(),
    action: "edit".to_string(),
    file_path: PathBuf::from("context.md"),
    template: Some("feature".to_string()),
    readonly: false,
    panel: Some("memory".to_string()),
    context: EditContext {
        conversation_summary: "Planning auth".to_string(),
        relevant_memories: vec![],
        related_files: vec![],
    },
};
coordinator.write_intent(&intent)?;

// Launch ICS (user interaction)
// ...

// Read result (with timeout)
let result = coordinator.read_result(Duration::from_secs(300)).await?;
println!("Changes made: {}", result.changes_made);

// Cleanup
coordinator.cleanup()?;
```

---

## Troubleshooting

### ICS Doesn't Launch

**Symptoms**: Command fails or hangs

**Solutions**:
1. Verify binary installed: `which mnemosyne`
2. Check permissions: `chmod +x $(which mnemosyne)`
3. Try full path: `/path/to/mnemosyne edit file.md`
4. Check logs: `RUST_LOG=debug mnemosyne edit file.md`

### Coordination Files Not Cleaned Up

**Symptoms**: `.claude/sessions/edit-*.json` files remain

**Solutions**:
1. Manual cleanup: `rm .claude/sessions/edit-*.json`
2. Check file permissions on `.claude/sessions/`
3. Verify ICS exits normally (not killed)

### Template Not Applied

**Symptoms**: File opens empty or with wrong content

**Solutions**:
1. If file exists, template is NOT applied (by design)
2. Delete existing file first, or use different filename
3. Verify template name: `--template api|architecture|bugfix|feature|refactor`

### Panel Not Visible

**Symptoms**: Panel doesn't appear after opening

**Solutions**:
1. Use keyboard shortcut to toggle (Ctrl+M, Ctrl+D)
2. Verify panel name: `--panel memory|diagnostics|proposals|holes`
3. Check terminal size (minimum 80x24)

### Changes Not Saved

**Symptoms**: Edits lost after exiting

**Solutions**:
1. Always save before quitting (Ctrl+S, then Ctrl+Q)
2. Check for save errors in status bar
3. Verify write permissions on file
4. Don't force-quit (Ctrl+C) - use Ctrl+Q

---

## Best Practices

### 1. Use Templates for New Files

Templates provide structure and guidance:
- Reduces cognitive load
- Ensures completeness
- Maintains consistency

### 2. Fill All Typed Holes

Before exiting, fill all `?hole` markers:
- Check holes panel for remaining items
- Use diagnostics to validate completeness

### 3. Reference with `#file` and `@symbol`

Create semantic links for better context:
- Improves AI understanding
- Enables future navigation features
- Documents dependencies

### 4. Combine Panels Strategically

Different workflows benefit from different panels:
- **Planning**: holes + memory
- **Review**: diagnostics + proposals
- **Documentation**: memory + diagnostics

### 5. Save Frequently

ICS uses CRDT for undo/redo, but save to persist:
- Ctrl+S writes to disk
- Automerge tracks in-memory history
- Regular saves prevent data loss

---

## Performance

### Benchmarks

From `TEST_RESULTS.md`:

| Operation | Throughput | Latency |
|-----------|------------|---------|
| File creation | 6394 files/sec | - |
| JSON serialization (12KB) | - | 289µs |
| JSON deserialization (12KB) | - | 72µs |
| Session file cycle | - | 0.42ms |
| Template lookup | 12.6M/sec | 79ns |

### Optimization Tips

1. **Large Files**: ICS handles 10MB+ files efficiently (2.84ms read)
2. **Concurrent Sessions**: 5+ parallel sessions work without conflicts
3. **Template Access**: In-memory lookups are nearly instant
4. **JSON Processing**: Coordination overhead minimal (<1ms)

---

## API Reference

See `src/coordination/handoff.rs` for full API documentation:

### Key Types

- `EditIntent` - What Claude Code wants ICS to do
- `EditResult` - What ICS produced
- `EditContext` - Conversation context from Claude Code
- `ExitReason` - Why ICS exited (user_saved, user_cancelled, error, timeout)
- `SemanticAnalysisSummary` - Analysis results from ICS session
- `HandoffCoordinator` - Manages file-based coordination

### Key Functions

- `HandoffCoordinator::new(session_dir)` - Create coordinator
- `write_intent(intent)` - Write edit intent
- `read_result(timeout)` - Read edit result (async)
- `cleanup()` - Remove coordination files

---

## Migration from `mnemosyne-ics`

If you were using the standalone `mnemosyne-ics` binary:

### Before (v2.0)
```bash
mnemosyne-ics context.md
```

### After (v2.1)
```bash
mnemosyne edit context.md
# or
mnemosyne ics context.md
```

### Changes

1. **Single Binary**: ICS features integrated into main `mnemosyne` binary
2. **Slash Command**: Use `/ics` from within Claude Code sessions
3. **Same Features**: All ICS functionality preserved
4. **Better Integration**: Seamless coordination with Claude Code

### Deprecated

- `mnemosyne-ics` binary (removed in v2.1.0)
- Use `mnemosyne edit` or `mnemosyne ics` instead

---

## Further Reading

- [TEST_RESULTS.md](/TEST_RESULTS.md) - Comprehensive test results
- [.claude/commands/ics.md](/.claude/commands/ics.md) - Slash command implementation
- [src/coordination/handoff.rs](/src/coordination/handoff.rs) - Coordination API
- [tests/manual/README.md](/tests/manual/README.md) - Manual test scenarios
