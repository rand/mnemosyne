# Integrated Context Studio (ICS)

**AI-assisted context engineering environment for Claude Code**

ICS provides a complete editing and analysis environment for creating and managing context documents, with real-time semantic analysis, typed hole tracking, and AI-powered suggestions.

## Features

### Core Editing
- **Multi-buffer editor** with syntax highlighting
- **CRDT-based collaborative editing** with attribution tracking
- **Undo/redo** with full history
- **Syntax highlighting** for Markdown, TOML, JSON, and more
- **Auto-completion** with context-aware suggestions
- **Validation** with real-time diagnostics

### Semantic Analysis
- **Real-time triples extraction** (subject-predicate-object)
- **Typed hole detection** for ambiguities and contradictions
- **Entity tracking** and relationship mapping
- **Symbol resolution** (#file/path, @symbol_name)
- **Background processing** without blocking the UI

### Memory Integration
- **Context-aware memory panel** with search and filtering
- **Quick memory preview** without leaving the editor
- **Memory reference insertion** for linking knowledge
- **Importance-based sorting** to surface relevant memories
- **Loading states** for async operations

### AI Collaboration
- **Change proposals** from semantic analysis
- **Agent attribution** tracking who made each change
- **Agent status** showing current activities
- **Proposal review workflow** (accept/reject/modify)
- **Rationale tracking** for understanding AI decisions

### Progressive Disclosure
- **Hidden by default** to reduce cognitive load
- **Keyboard shortcuts** for quick access (Ctrl+M, P, D, A, T)
- **Calm color palette** (RGB 140-200) for non-intrusive UX
- **Empty states** with helpful guidance
- **Loading indicators** for all async operations

## Usage

### Standalone Mode

```bash
# Edit a context document with ICS
mnemosyne --ics context.md

# Create new document
mnemosyne --ics
```

### Embedded in PTY Mode

When running Mnemosyne in PTY mode:
- Press `Ctrl+E` to toggle ICS panel
- ICS appears as overlay on terminal
- Full editing capabilities while agents run

### Basic Workflow

1. **Edit**: Write context documents with syntax highlighting
2. **Analyze**: Background semantic analysis extracts knowledge
3. **Review**: Check diagnostics panel for issues
4. **Refine**: Accept AI proposals or make manual edits
5. **Link**: Connect to relevant memories via memory panel
6. **Track**: View attribution for all changes

## Keyboard Shortcuts

See [ICS_KEYBOARD_SHORTCUTS.md](./ICS_KEYBOARD_SHORTCUTS.md) for complete reference.

### Quick Reference

| Shortcut | Action |
|----------|--------|
| Ctrl+M | Toggle memory panel |
| Ctrl+P | Toggle proposals panel |
| Ctrl+D | Toggle diagnostics panel |
| Ctrl+A | Toggle agent status |
| Ctrl+T | Toggle attribution panel |
| Ctrl+S | Save current buffer |
| Ctrl+F | Find in document |
| Ctrl+Z / Ctrl+Y | Undo / Redo |

## Architecture

ICS is built on a multi-layer architecture:

```
┌─────────────────────────────────────┐
│     ICS App (Event Loop)            │
├─────────────────────────────────────┤
│  Panels    │ Editor │ Semantic      │
│  - Memory  │ Widget │ Analyzer      │
│  - Diag    │        │ (Background)  │
│  - Props   │        │               │
│  - Agent   │        │               │
│  - Attrib  │        │               │
├─────────────────────────────────────┤
│     Storage Backend (LibSQL)        │
└─────────────────────────────────────┘
```

### Components

- **App**: Main event loop and state management
- **Editor**: Multi-buffer text editing with CRDT support
- **Panels**: Specialized UI components for different views
- **Semantic Analyzer**: Background analysis of document semantics
- **Storage**: Integration with Mnemosyne memory system

See [ICS_ARCHITECTURE.md](./ICS_ARCHITECTURE.md) for detailed architecture.

## Integration

### Embedding ICS in Your Application

```rust
use mnemosyne_core::ics::{IcsApp, IcsConfig};

// Create ICS configuration
let config = IcsConfig {
    enable_semantic_analysis: true,
    enable_memory_integration: true,
    calm_colors: true,
    ..Default::default()
};

// Create ICS app
let mut app = IcsApp::new(config, storage, llm_service)?;

// Run event loop
app.run()?;
```

### Using Semantic Analyzer Standalone

```rust
use mnemosyne_core::ics::SemanticAnalyzer;

// Create analyzer
let mut analyzer = SemanticAnalyzer::new();

// Trigger analysis
analyzer.analyze("The system is distributed.".to_string())?;

// Poll for results (non-blocking)
if let Some(analysis) = analyzer.try_recv() {
    println!("Triples: {:?}", analysis.triples);
    println!("Holes: {:?}", analysis.holes);
    println!("Entities: {:?}", analysis.entities);
}
```

### Custom Panels

```rust
use mnemosyne_core::ics::{MemoryPanel, MemoryPanelState};
use ratatui::Terminal;

// Create panel state
let mut state = MemoryPanelState::new();
state.show();

// Render panel
let panel = MemoryPanel::new(&memories);
terminal.draw(|f| {
    f.render_stateful_widget(panel, area, &mut state);
})?;
```

## Testing

ICS includes comprehensive test coverage:

- **55 unit tests** covering all components
- **10 integration tests** for end-to-end workflows
- **Test helpers** in `tests/common/mod.rs`

```bash
# Run all ICS tests
cargo test ics::

# Run integration tests
cargo test --test ics_integration_test

# Run with coverage
cargo tarpaulin --out Html --output-dir coverage/
```

## Performance

ICS is designed for minimal latency:

- **Non-blocking analysis**: Semantic analysis runs in background
- **Efficient text handling**: Rope data structure for O(log n) operations
- **Progressive rendering**: Only visible content is rendered
- **Lazy loading**: Memories loaded on-demand
- **Zero-copy**: CRDT operations minimize allocations

## Design Philosophy

### Calm Technology
- Muted colors (RGB 140-200 range) reduce visual noise
- Progressive disclosure hides complexity
- Empty states provide gentle guidance
- Loading indicators show progress without urgency

### Progressive Enhancement
- Core editing works without analysis
- Analysis enhances but doesn't block
- Panels provide additional context
- AI suggestions are optional

### Fail-Safe Design
- Analysis panics don't crash the editor
- Invalid buffer IDs are prevented by invariants
- Channel errors provide clear messages
- Fallback to empty results on errors

## Troubleshooting

### Semantic Analysis Not Working

1. Check if background task is alive: `analyzer.is_analyzing()`
2. Verify no panics in stderr
3. Check for channel errors in analyze() return value

### Memory Panel Empty

1. Verify storage backend connection
2. Check namespace filter
3. Ensure memories exist in database
4. Look for loading state indicator

### Keyboard Shortcuts Not Working

1. Check if panel has focus
2. Verify no conflicting shortcuts
3. See keyboard shortcuts documentation

### High Memory Usage

1. Close unused buffers
2. Limit analysis frequency
3. Clear old memories from panel state
4. Check for memory leaks with heaptrack

## Contributing

When contributing to ICS:

1. **Add tests** for all new features
2. **Document** public APIs with rustdoc
3. **Follow** existing error handling patterns
4. **Maintain** invariants documented in code
5. **Test** keyboard shortcuts and UI flows

## License

Part of Mnemosyne - see LICENSE file in repository root.

## Further Reading

- [ICS Architecture](./ICS_ARCHITECTURE.md) - Technical deep-dive
- [ICS Keyboard Shortcuts](./ICS_KEYBOARD_SHORTCUTS.md) - Complete shortcut reference
- [ICS API Documentation](./ICS_API.md) - Integration guide
- [Mnemosyne README](../README.md) - Main project documentation
