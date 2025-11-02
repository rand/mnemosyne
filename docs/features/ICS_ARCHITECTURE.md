# ICS Architecture

Technical architecture and design decisions for the Integrated Context Studio.

## Overview

ICS is built as a layered architecture with clear separation of concerns:

```
┌────────────────────────────────────────────────────────────┐
│                        ICS App                              │
│  - Event loop                                               │
│  - State management                                         │
│  - Panel coordination                                       │
└───────────────┬────────────────────────────────────────────┘
                │
    ┌───────────┼───────────────────────┐
    │           │                       │
┌───▼────┐  ┌──▼──────┐  ┌────────────▼───────┐
│ Editor │  │ Panels  │  │ Semantic Analyzer  │
│        │  │         │  │  (Background task)  │
│ Buffers│  │ Memory  │  └────────────────────┘
│ CRDT   │  │ Diag    │
│ Syntax │  │ Props   │
│ Valid  │  │ Agent   │
└───┬────┘  │ Attrib  │
    │       └────┬────┘
    │            │
    └────────┬───┘
             │
    ┌────────▼─────────┐
    │     Storage      │
    │    (LibSQL)      │
    └──────────────────┘
```

## Component Architecture

### 1. ICS App (`src/ics/app.rs`)

**Responsibility**: Main application loop and state coordination

**Key Components**:
- Event loop with crossterm for terminal input
- State management for all panels
- Layout calculation with ratatui
- Keyboard shortcut routing

**Event Flow**:
```
User Input → Crossterm → Event Handler → State Update → Render
     ↓                                        ↓
  Keyboard                                Panel States
  Shortcut                                Editor State
                                          Semantic State
```

**State Management**:
- Each panel maintains its own state struct
- App coordinates visibility and focus
- State transitions trigger re-renders
- Async operations tracked separately

### 2. Editor (`src/ics/editor/`)

**Submodules**:
- `buffer.rs`: Rope-based text storage
- `crdt_buffer.rs`: CRDT for collaborative editing
- `cursor.rs`: Cursor positioning and movement
- `highlight.rs`: Syntax highlighting with tree-sitter
- `completion.rs`: Auto-completion engine
- `validation.rs`: Real-time validation
- `widget.rs`: Ratatui rendering
- `sync.rs`: Multi-user coordination
- `syntax.rs`: Language detection

#### Buffer Architecture

```rust
TextBuffer {
    id: BufferId,
    content: Rope,           // O(log n) operations
    path: Option<PathBuf>,
    language: Language,
    dirty: bool,
    cursor: CursorState,
    undo_stack: VecDeque<Edit>,
    redo_stack: VecDeque<Edit>,
}
```

**Rope Data Structure**:
- Chunks of text in balanced tree
- O(log n) insert, delete, slice
- Efficient for large files (>1MB)
- Memory-mapped for very large files

**Invariants**:
- Cursor always within bounds
- Undo/redo stacks mirror each other
- Dirty flag matches file state

#### CRDT Buffer

```rust
CrdtBuffer {
    actor_id: Actor,
    operations: Vec<Operation>,
    attributions: Vec<Attribution>,
    lamport_clock: u64,
}
```

**Operation Types**:
- `Insert(pos, char, actor, timestamp)`
- `Delete(pos, actor, timestamp)`
- `Move(from, to, actor, timestamp)`

**Conflict Resolution**:
- Lamport timestamps for ordering
- Actor ID as tiebreaker
- Last-write-wins for concurrent edits
- Tombstones for deleted content

### 3. Semantic Analyzer (`src/ics/semantic.rs`)

**Architecture**: Producer-consumer with mpsc channels

```
Main Thread                  Background Task
─────────────                ────────────────
   │                              │
   │  analyze(text)               │
   ├──────────────────────────────>
   │   (via request channel)      │
   │                              │
   │                         analyze_text()
   │                         - Extract triples
   │                         - Find holes
   │                         - Track entities
   │                              │
   │  <──────────────────────────┤
   │   try_recv() → analysis      │
   │   (via result channel)       │
```

**Channel Flow**:
1. Request sent: `AnalysisRequest { text }`
2. Background processing (can take 10-100ms)
3. Result sent: `SemanticAnalysis { triples, holes, entities }`
4. Main thread polls with `try_recv()` (non-blocking)

**Error Handling**:
- Panic recovery with `catch_unwind`
- Empty result on panic (doesn't crash editor)
- Error logged to stderr
- Channel errors provide context

**Algorithms**:

*Triple Extraction*:
```
For each line:
  1. Look for patterns: "X is Y", "X has Y", "X requires Y"
  2. Extract subject and object
  3. Assign confidence based on pattern strength
  4. Store with source line for reference
```

*Typed Hole Detection*:
```
For each line:
  1. Check for TODO/TBD/FIXME → Incomplete
  2. Check for "however", "but" → Contradiction
  3. Check for @undefined, #missing → Undefined
  4. Check for ambiguous terms → Ambiguous
```

*Entity Extraction*:
```
For each word:
  1. Capitalized words (len > 2) → Potential entities
  2. @symbols and #files → Explicit references
  3. Count occurrences for frequency
  4. Track relationships via triples
```

### 4. Panels (`src/ics/*_panel.rs`)

All panels follow the same architecture pattern:

```rust
// State (mutable, owned by app)
PanelState {
    visible: bool,
    list_state: ListState,
    ...specific fields...
}

// Widget (immutable, created per render)
Panel<'a> {
    data: &'a [Item],
    ...view config...
}

impl StatefulWidget for Panel<'a> {
    type State = PanelState;

    fn render(self, area, buf, state) {
        // Render using ratatui primitives
    }
}
```

#### Memory Panel

**State**:
- Selected memory index
- Search query filter
- Loading indicator
- Preview selection

**Data Flow**:
```
Storage → MemoryNote[] → Filter → Render
            ↓
      Selected memory → Preview
```

**Features**:
- Fuzzy search on content and tags
- Importance-based sorting
- Quick preview without modal
- Reference insertion at cursor

#### Diagnostics Panel

**State**:
- Selected diagnostic index
- Severity filter
- Error counts

**Data Flow**:
```
Editor validation → Diagnostic[] → Filter by severity → Render
                                      ↓
                            Navigate to error location
```

**Severity Levels**:
- `Error`: Must fix (red)
- `Warning`: Should fix (yellow)
- `Hint`: Consider fixing (cyan)

#### Proposals Panel

**State**:
- Selected proposal index
- Status filter (pending/accepted/rejected)
- Details view toggle

**Data Flow**:
```
Semantic analysis → ChangeProposal[] → User review → Apply/Reject
                        ↓
                   Show diff view
```

**Proposal Lifecycle**:
```
Pending → User Review → Accepted → Applied
                     ↘ Rejected
```

#### Agent Status Panel

**State**:
- Agent list
- Activity indicators
- Last update timestamp

**Data Flow**:
```
Agent events → AgentInfo[] → Activity status → Render
                  ↓
            Real-time updates
```

#### Attribution Panel

**State**:
- Change history
- Actor filter
- Timestamp range

**Data Flow**:
```
CRDT operations → Attribution[] → Filter → Render timeline
                      ↓
            Show who changed what
```

## Threading Model

ICS uses Tokio for async operations:

```
Main Thread (UI)           Background Tasks
────────────────           ─────────────────
Event loop                 Semantic Analyzer
  ↓                           ↓
State updates              Text analysis
  ↓                           ↓
Render                     Result channel
  ↓                           ↓
Poll async ops  ←──────────  Send results
```

**Async Operations**:
- Semantic analysis (non-blocking)
- Memory loading from storage
- LLM calls for suggestions
- File I/O for save/load

**Synchronization**:
- Channels for producer-consumer
- Arc<Mutex<T>> for shared state (minimal use)
- Message passing preferred over locks

## Error Handling Strategy

### Levels of Error Handling

1. **Recoverable Errors** (Result<T, E>)
   - File I/O failures
   - Storage errors
   - LLM API errors
   - Parse errors

2. **Invariant Violations** (expect with message)
   - Active buffer should exist
   - Cursor within bounds
   - Valid UTF-8 in buffers

3. **Panic Recovery** (catch_unwind)
   - Semantic analysis panics
   - Plugin panics (future)
   - User script errors (future)

4. **Unrecoverable** (panic)
   - Terminal init failure
   - Main channel death
   - Storage corruption

### Error Propagation

```rust
// Public API: Result with context
pub fn analyze(&mut self, text: String) -> Result<()> {
    self.tx.send(request)
        .context("Background task died")?;
    Ok(())
}

// Internal: expect with violation message
fn active_buffer(&self) -> &TextBuffer {
    self.buffers.get(&self.active_buffer)
        .expect("INVARIANT VIOLATION: active_buffer exists")
}

// Background: catch_unwind for isolation
tokio::spawn(async move {
    let result = std::panic::catch_unwind(|| {
        analyze_text(&text)
    });
    // Handle panic...
});
```

## Performance Considerations

### Rope Data Structure

- **Insertion**: O(log n)
- **Deletion**: O(log n)
- **Slicing**: O(log n)
- **Line iteration**: O(lines)

Trade-off: Slightly slower than Vec for small files (<10KB), much faster for large files (>100KB).

### Rendering Optimization

```
Only render visible area:
  viewport_start = scroll_offset
  viewport_end = scroll_offset + height

  For line in viewport_start..viewport_end:
    Render line with syntax highlighting
```

**Caching**:
- Syntax highlighting cached per-line
- Unchanged lines skip re-highlight
- Terminal diffs minimize redraws

### Semantic Analysis

**Batching**:
- Debounce analysis (300ms after last edit)
- Skip analysis for trivial edits (<5 chars)
- Cache results for unchanged text

**Incremental Analysis** (future):
- Only re-analyze changed lines
- Merge with previous results
- Track dirty regions

## State Machines

### Panel Visibility

```
Hidden ──[toggle]──> Visible
  ↑                    │
  └────────[toggle]────┘
```

### Proposal State

```
Pending ──[accept]──> Accepted ──[apply]──> Applied
  │
  └────[reject]────> Rejected
```

### Loading State

```
Idle ──[request]──> Loading ──[complete]──> Idle
                       │            │
                       └──[error]───┘
```

## Testing Strategy

### Unit Tests
- Each module has tests in `mod tests`
- Test pure functions in isolation
- Mock external dependencies
- 55 unit tests covering core functionality

### Integration Tests
- `tests/ics_integration_test.rs`
- Test component interactions
- End-to-end workflows
- 10 integration tests for full scenarios

### Property Tests (future)
- CRDT convergence
- Undo/redo symmetry
- Rope invariants
- Fuzzing with arbitrary input

## Future Architecture

### Planned Improvements

1. **Plugin System**
   - LSP integration
   - Custom analyzers
   - Theme plugins

2. **Incremental Analysis**
   - Only re-analyze changed regions
   - Smarter caching
   - Background indexing

3. **Real-time Collaboration**
   - WebSocket sync
   - Operational transforms
   - Presence awareness

4. **Advanced Features**
   - Multi-file editing
   - Project-wide refactoring
   - Visual diff viewer

## References

- [Rope data structure](https://en.wikipedia.org/wiki/Rope_(data_structure))
- [CRDTs](https://crdt.tech/)
- [Ratatui](https://ratatui.rs/)
- [Tree-sitter](https://tree-sitter.github.io/)
- [Ropey crate](https://docs.rs/ropey/)
