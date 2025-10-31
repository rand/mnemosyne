# ICS Integration Specification

## Overview

Integrate the semantic highlighting system with the ICS editor to provide real-time semantic analysis as users type, with configurable settings and keybindings to control analysis features.

## Current State

**What Exists:**
- Standalone `SemanticHighlightEngine` with full API
- `IcsEditor` managing multiple `CrdtBuffer` instances
- `EditorWidget` for rendering with attribution colors
- Syntax highlighting via `Highlighter`
- Diagnostic display system (errors, warnings)

**What's Missing:**
- Engine instance in IcsEditor or CrdtBuffer
- Text change event hooks to trigger `schedule_analysis()`
- Integration of semantic spans into rendering
- Settings UI/configuration for semantic features
- Keybindings for requesting analysis
- Performance monitoring and fallback

---

## Requirements

### Functional Requirements

**FR-1**: Each buffer must have its own semantic engine
- One `SemanticHighlightEngine` per `CrdtBuffer`
- Engine lifecycle tied to buffer lifecycle
- Separate cache per buffer

**FR-2**: Text changes must trigger incremental analysis
- Hook buffer insert/delete operations
- Extract changed range
- Call `engine.schedule_analysis(text, range)`
- Debounced to avoid per-keystroke analysis

**FR-3**: Rendering must display semantic highlights
- Merge semantic spans with syntax highlights
- Priority: Semantic > Syntax > Plain
- Respect user settings (enable/disable tiers)

**FR-4**: Settings must control semantic features
- Enable/disable each tier independently
- Adjust confidence thresholds
- Configure debounce delays
- Toggle visual annotations

**FR-5**: Keybindings must trigger full analysis
- Keybinding for "Analyze Document" (e.g., Ctrl+Shift+A)
- Keybinding for "Clear Semantic Cache"
- Visual feedback while analysis in progress

### Non-Functional Requirements

**NFR-1**: Performance
- No blocking on main render thread
- Tier 1 must stay <5ms (always real-time)
- Tier 2/3 results appear progressively
- Graceful fallback if analysis times out

**NFR-2**: UX
- Visual indicator for analysis in progress
- Status bar shows cache hit rate
- Errors logged but don't crash editor
- User can disable features without restart

**NFR-3**: Memory
- Semantic caches bounded in size
- Old results expired after TTL
- No memory leaks from retained engines

---

## Design

### Type Definitions

```rust
/// Settings for semantic highlighting in ICS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IcsSemanticSettings {
    /// Enable semantic highlighting
    pub enabled: bool,

    /// Enable Tier 1 (Structural)
    pub enable_structural: bool,

    /// Enable Tier 2 (Relational)
    pub enable_relational: bool,

    /// Enable Tier 3 (Analytical, requires API key)
    pub enable_analytical: bool,

    /// Relational analysis settings
    pub relational: RelationalSettings,

    /// Analytical settings (API key, rate limits)
    pub analytical: AnalyticalSettings,

    /// Visual settings
    pub visual: VisualSettings,
}

impl Default for IcsSemanticSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            enable_structural: true,
            enable_relational: true,
            enable_analytical: false, // Off by default (requires API key)
            relational: RelationalSettings::default(),
            analytical: AnalyticalSettings::default(),
            visual: VisualSettings::default(),
        }
    }
}
```

### Implementation Plan

#### 1. Add Engine to CrdtBuffer

**File**: `src/ics/editor/crdt_buffer.rs`

```rust
use crate::ics::semantic_highlighter::{SemanticHighlightEngine, HighlightSettings};

pub struct CrdtBuffer {
    // ... existing fields ...

    /// Semantic highlighting engine
    pub semantic_engine: Option<SemanticHighlightEngine>,
}

impl CrdtBuffer {
    pub fn new(id: BufferId, actor: Actor, path: Option<PathBuf>) -> Result<Self> {
        // ... existing initialization ...

        // Initialize semantic engine (disabled if no LLM service)
        let semantic_engine = Some(SemanticHighlightEngine::new(None));

        Ok(Self {
            // ... existing fields ...
            semantic_engine,
        })
    }

    /// Enable semantic highlighting with LLM service
    pub fn enable_semantic_analysis(&mut self, llm_service: Arc<LlmService>, settings: HighlightSettings) {
        self.semantic_engine = Some(SemanticHighlightEngine::with_settings(settings, Some(llm_service)));
    }

    /// Disable semantic highlighting
    pub fn disable_semantic_analysis(&mut self) {
        self.semantic_engine = None;
    }

    /// Update semantic settings
    pub fn update_semantic_settings(&mut self, settings: HighlightSettings) {
        if let Some(ref mut engine) = self.semantic_engine {
            engine.update_settings(settings);
        }
    }
}
```

#### 2. Hook Text Change Events

**File**: `src/ics/editor/crdt_buffer.rs`

Modify insert and delete methods:

```rust
impl CrdtBuffer {
    /// Insert text at position
    pub fn insert(&mut self, content: &str) -> Result<()> {
        // ... existing insert logic ...

        let start_char = self.cursor.to_char_pos(&self)?;
        let end_char = start_char + content.len();

        // Mark as dirty
        self.dirty = true;

        // Trigger semantic analysis for changed region
        if let Some(ref mut engine) = self.semantic_engine {
            let full_text = self.text()?;
            engine.schedule_analysis(&full_text, start_char..end_char);
        }

        Ok(())
    }

    /// Delete text range
    pub fn delete_range(&mut self, start: usize, end: usize) -> Result<()> {
        // ... existing delete logic ...

        // Mark as dirty
        self.dirty = true;

        // Trigger semantic analysis for affected region
        if let Some(ref mut engine) = self.semantic_engine {
            let full_text = self.text()?;
            // Expand range slightly to catch context changes
            let context_start = start.saturating_sub(50);
            let context_end = (end + 50).min(full_text.len());
            engine.schedule_analysis(&full_text, context_start..context_end);
        }

        Ok(())
    }

    /// Request full document analysis
    pub async fn analyze_full(&mut self) -> Result<()> {
        if let Some(ref engine) = self.semantic_engine {
            let text = self.text()?;
            engine.request_analysis(
                text,
                crate::ics::semantic_highlighter::engine::AnalysisRequestType::Full
            ).await?;
        }
        Ok(())
    }

    /// Clear semantic cache
    pub fn clear_semantic_cache(&mut self) {
        if let Some(ref engine) = self.semantic_engine {
            engine.clear_caches();
        }
    }
}
```

#### 3. Integrate Semantic Highlights into Rendering

**File**: `src/ics/editor/widget.rs`

```rust
use crate::ics::semantic_highlighter::SemanticHighlightEngine;

impl<'a> EditorWidget<'a> {
    /// Render line with semantic + syntax highlighting
    fn render_line(
        &self,
        line_num: usize,
        line_text: &str,
        area: Rect,
        buf: &mut RatatuiBuffer,
        state: &EditorState,
    ) {
        let cursor_on_line = self.buffer.cursor.position.line == line_num;

        // Get semantic highlighting if available
        let semantic_line = if let Some(ref engine) = self.buffer.semantic_engine {
            engine.highlight_line(line_text)
        } else {
            // Fallback to plain text
            Line::from(line_text)
        };

        // Merge with syntax highlighting
        // Priority: Semantic spans override syntax spans
        let merged_line = if state.show_syntax_highlighting {
            self.merge_syntax_and_semantic(line_text, semantic_line)
        } else {
            semantic_line
        };

        // Apply attribution colors if enabled
        let final_line = if state.show_attribution {
            self.apply_attribution(merged_line, line_num)
        } else {
            merged_line
        };

        // Render the line
        // ... existing rendering code ...
    }

    /// Merge syntax and semantic highlighting spans
    fn merge_syntax_and_semantic(&self, line_text: &str, semantic_line: Line<'static>) -> Line<'static> {
        // Get syntax highlighting from existing highlighter
        let syntax_spans = self.buffer.highlight_line(line_text);

        // Semantic spans have priority over syntax
        // This is already handled by SemanticHighlightEngine's SpanMerger
        // which respects HighlightSource priority levels

        semantic_line
    }
}
```

#### 4. Add IcsEditor-Level Settings

**File**: `src/ics/editor/mod.rs`

```rust
use crate::ics::semantic_highlighter::HighlightSettings;

pub struct IcsEditor {
    // ... existing fields ...

    /// Semantic highlighting settings
    semantic_settings: IcsSemanticSettings,

    /// Optional LLM service for Tier 3
    llm_service: Option<Arc<LlmService>>,
}

impl IcsEditor {
    /// Update semantic settings for all buffers
    pub fn update_semantic_settings(&mut self, settings: IcsSemanticSettings) {
        self.semantic_settings = settings.clone();

        // Convert to HighlightSettings
        let highlight_settings = HighlightSettings {
            enable_structural: settings.enable_structural,
            enable_relational: settings.enable_relational,
            enable_analytical: settings.enable_analytical,
            relational: settings.relational,
            analytical: settings.analytical,
            visual: settings.visual,
        };

        // Apply to all buffers
        for buffer in self.buffers.values_mut() {
            buffer.update_semantic_settings(highlight_settings.clone());
        }
    }

    /// Set LLM service for Tier 3 analysis
    pub fn set_llm_service(&mut self, llm_service: Arc<LlmService>) {
        self.llm_service = Some(llm_service.clone());

        // Enable Tier 3 for all buffers
        for buffer in self.buffers.values_mut() {
            if self.semantic_settings.enable_analytical {
                let settings = HighlightSettings {
                    enable_analytical: true,
                    ..Default::default()
                };
                buffer.enable_semantic_analysis(llm_service.clone(), settings);
            }
        }
    }
}
```

#### 5. Add Keybindings and Commands

**File**: Create `src/ics/commands/semantic.rs`

```rust
/// Semantic analysis commands
pub enum SemanticCommand {
    /// Analyze full document
    AnalyzeFull,

    /// Clear semantic cache
    ClearCache,

    /// Toggle Tier 1 (Structural)
    ToggleStructural,

    /// Toggle Tier 2 (Relational)
    ToggleRelational,

    /// Toggle Tier 3 (Analytical)
    ToggleAnalytical,

    /// Show cache statistics
    ShowCacheStats,
}

impl SemanticCommand {
    pub async fn execute(&self, editor: &mut IcsEditor) -> Result<String> {
        let buffer = editor.active_buffer_mut();

        match self {
            SemanticCommand::AnalyzeFull => {
                buffer.analyze_full().await?;
                Ok("Full document analysis requested".to_string())
            }
            SemanticCommand::ClearCache => {
                buffer.clear_semantic_cache();
                Ok("Semantic cache cleared".to_string())
            }
            SemanticCommand::ToggleStructural => {
                editor.semantic_settings.enable_structural = !editor.semantic_settings.enable_structural;
                editor.update_semantic_settings(editor.semantic_settings.clone());
                Ok(format!("Structural analysis: {}",
                    if editor.semantic_settings.enable_structural { "enabled" } else { "disabled" }))
            }
            SemanticCommand::ToggleRelational => {
                editor.semantic_settings.enable_relational = !editor.semantic_settings.enable_relational;
                editor.update_semantic_settings(editor.semantic_settings.clone());
                Ok(format!("Relational analysis: {}",
                    if editor.semantic_settings.enable_relational { "enabled" } else { "disabled" }))
            }
            SemanticCommand::ToggleAnalytical => {
                editor.semantic_settings.enable_analytical = !editor.semantic_settings.enable_analytical;
                editor.update_semantic_settings(editor.semantic_settings.clone());
                Ok(format!("Analytical analysis: {}",
                    if editor.semantic_settings.enable_analytical { "enabled" } else { "disabled" }))
            }
            SemanticCommand::ShowCacheStats => {
                if let Some(ref engine) = buffer.semantic_engine {
                    let (relational, analytical) = engine.cache_stats();
                    Ok(format!(
                        "Cache Stats:\n  Relational: {}/{} ({:.1}%)\n  Analytical: {}/{} ({:.1}%)",
                        relational.size, relational.capacity, relational.utilization() * 100.0,
                        analytical.size, analytical.capacity, analytical.utilization() * 100.0
                    ))
                } else {
                    Ok("Semantic analysis not enabled".to_string())
                }
            }
        }
    }
}
```

#### 6. Add Status Bar Indicators

**File**: Modify `src/tui/app.rs` (or wherever status bar is rendered)

```rust
fn render_status_bar(&self, area: Rect, buf: &mut Buffer) {
    // ... existing status bar code ...

    // Add semantic analysis indicator
    if let Some(ref buffer) = self.editor.active_buffer().semantic_engine {
        let (rel_stats, ana_stats) = buffer.cache_stats();

        let semantic_status = format!(
            "Semantic: T1✓ T2({}) T3({})",
            if rel_stats.size > 0 { "●" } else { "○" },
            if ana_stats.size > 0 { "●" } else { "○" }
        );

        // Render at right side of status bar
        // ...
    }
}
```

---

## Testing Strategy

### Unit Tests

**Test 1: Engine Initialization**
- Given: New CrdtBuffer
- When: Creating buffer
- Then: Semantic engine initialized

**Test 2: Text Change Hook**
- Given: Buffer with engine
- When: Inserting text
- Then: `schedule_analysis()` called with correct range

**Test 3: Settings Update**
- Given: Editor with multiple buffers
- When: Updating semantic settings
- Then: All buffers' engines updated

**Test 4: Cache Clearing**
- Given: Buffer with cached results
- When: Clearing cache
- Then: Subsequent calls re-analyze

### Integration Tests

**Test 5: Full Edit-Analyze-Render Flow**
- Given: Editor with buffer
- When: Typing text with semantic patterns
- Then: Highlights appear in rendered output

**Test 6: Keybinding Execution**
- Given: Editor with active buffer
- When: Executing `AnalyzeFull` command
- Then: Full analysis requested, status updated

**Test 7: Settings Persistence**
- Given: Settings changed
- When: Restarting editor
- Then: Settings loaded, engines configured correctly

---

## Acceptance Criteria

- [ ] `CrdtBuffer` has `semantic_engine` field
- [ ] Text changes trigger `schedule_analysis()`
- [ ] Rendering merges semantic + syntax highlights
- [ ] Settings control each tier independently
- [ ] Keybindings trigger analysis commands
- [ ] Status bar shows semantic analysis state
- [ ] Cache statistics accessible
- [ ] All unit tests passing
- [ ] Integration tests with real editing passing
- [ ] No performance regression in rendering
- [ ] Documentation updated

---

## Estimated Effort

- Add engine to CrdtBuffer: 0.5 days
- Hook text change events: 0.5 days
- Integrate rendering: 1 day
- Settings and configuration: 0.5 days
- Commands and keybindings: 0.5 days
- Status bar indicators: 0.5 days
- Testing: 0.5 days

**Total: 4 days**

---

## Dependencies

- Semantic highlighter Tiers 1-3 complete
- Background processing working
- Incremental analysis working
- `HighlightSettings` structure finalized

---

## Risks & Mitigation

**Risk 1**: Rendering performance degradation
- Mitigation: Profile with benchmarks, use caching aggressively, make Tier 3 opt-in

**Risk 2**: Memory growth from per-buffer engines
- Mitigation: Shared cache across buffers, bounded cache sizes, TTL expiration

**Risk 3**: User confusion about features
- Mitigation: Clear status indicators, help text, sensible defaults (Tier 3 off)

**Risk 4**: Settings complexity
- Mitigation: Preset profiles (minimal, balanced, full), good documentation

---

## References

- IcsEditor: `src/ics/editor/mod.rs`
- CrdtBuffer: `src/ics/editor/crdt_buffer.rs`
- EditorWidget: `src/ics/editor/widget.rs`
- SemanticHighlightEngine: `src/ics/semantic_highlighter/engine.rs`
