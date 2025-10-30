//! Main ICS application
//!
//! Standalone ICS application that can be run with `mnemosyne --ics`

use super::{
    IcsConfig,
    editor::{EditorState, EditorWidget, IcsEditor, Movement, Validator, Diagnostic},
    memory_panel::{MemoryPanel, MemoryPanelState},
    semantic::{SemanticAnalyzer, SemanticAnalysis},
    agent_status::{AgentStatusWidget, AgentStatusState, AgentInfo},
    attribution::{AttributionPanel, AttributionPanelState, AttributionEntry},
    proposals::{ProposalsPanel, ProposalsPanelState, ChangeProposal},
    diagnostics_panel::{DiagnosticsPanel, DiagnosticsPanelState},
};
use crate::{
    orchestration::{AgentRegistry, ProposalQueue},
    storage::{MemorySortOrder, StorageBackend},
    tui::{EventLoop, TerminalConfig, TerminalManager, TuiEvent},
    types::{MemoryId, MemoryNote, MemoryType, Namespace},
};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, StatefulWidget, Widget},
};
use std::path::PathBuf;
use std::sync::Arc;

/// Application state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppState {
    Running,
    Quitting,
}

/// Main ICS application
pub struct IcsApp {
    /// Configuration
    config: IcsConfig,
    /// Editor instance
    editor: IcsEditor,
    /// Editor widget state
    editor_state: EditorState,
    /// Application state
    state: AppState,
    /// Status message
    status: String,

    // Phase 3: Memory Integration
    /// Storage backend for memory retrieval
    storage: Arc<dyn StorageBackend>,
    /// Optional agent registry for orchestration mode
    agent_registry: Option<AgentRegistry>,
    /// Optional proposal queue for orchestration mode
    proposal_queue: Option<ProposalQueue>,
    /// Memory panel state
    memory_panel: MemoryPanelState,
    /// Loaded memories (fetched from storage)
    memories: Vec<MemoryNote>,

    // Phase 4: Semantic Analysis
    /// Semantic analyzer
    semantic_analyzer: SemanticAnalyzer,
    /// Latest semantic analysis result
    semantic_analysis: Option<SemanticAnalysis>,

    // Phase 5: Agent Collaboration
    /// Agent status panel state
    agent_status_panel: AgentStatusState,
    /// Active agents
    agents: Vec<AgentInfo>,
    /// Attribution panel state
    attribution_panel: AttributionPanelState,
    /// Attribution entries
    attributions: Vec<AttributionEntry>,
    /// Proposals panel state
    proposals_panel: ProposalsPanelState,
    /// Change proposals
    proposals: Vec<ChangeProposal>,

    // Phase 6: Diagnostics
    /// Diagnostics panel state
    diagnostics_panel: DiagnosticsPanelState,
    /// Validator for inline diagnostics
    validator: Validator,
    /// Current diagnostics
    diagnostics: Vec<Diagnostic>,
}

impl IcsApp {
    /// Create new ICS application
    ///
    /// # Arguments
    /// * `config` - ICS configuration
    /// * `storage` - Storage backend for memory retrieval
    /// * `agent_registry` - Optional agent registry for orchestration mode
    /// * `proposal_queue` - Optional proposal queue for orchestration mode
    pub fn new(
        config: IcsConfig,
        storage: Arc<dyn StorageBackend>,
        agent_registry: Option<AgentRegistry>,
        proposal_queue: Option<ProposalQueue>,
    ) -> Self {
        Self {
            config,
            editor: IcsEditor::new(),
            editor_state: EditorState::default(),
            state: AppState::Running,
            status: "ICS | Ctrl+Q: quit | Ctrl+S: save | Ctrl+M: memories | Ctrl+Shift+M: store semantic | Ctrl+P: proposals | Ctrl+D: diagnostics | Ctrl+A: agents".to_string(),

            // Phase 3: Memory Integration
            storage,
            agent_registry,
            proposal_queue,
            memory_panel: MemoryPanelState::new(),
            memories: Vec::new(), // Loaded on demand via load_memories()

            // Phase 4: Semantic Analysis
            semantic_analyzer: SemanticAnalyzer::new(),
            semantic_analysis: None,

            // Phase 5: Agent Collaboration
            agent_status_panel: AgentStatusState::new(),
            // Agent tracking available when AgentRegistry is provided (orchestration mode)
            // In standalone mode (no registry), agents list remains empty
            agents: Vec::new(),
            attribution_panel: AttributionPanelState::new(),
            // Attributions extracted from CrdtBuffer on demand (via Ctrl+T)
            attributions: Vec::new(),
            proposals_panel: ProposalsPanelState::new(),
            // Proposals polled from ProposalQueue on demand (via Ctrl+P)
            // In standalone mode (no queue), proposals list remains empty
            proposals: Vec::new(),

            // Phase 6: Diagnostics
            diagnostics_panel: DiagnosticsPanelState::new(),
            validator: Validator::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Load file into editor
    pub fn load_file(&mut self, path: PathBuf) -> Result<()> {
        let buffer = self.editor.active_buffer_mut();
        buffer.load_file(path.clone())?;
        self.status = format!("Loaded: {}", path.display());
        Ok(())
    }

    /// Save current buffer
    pub fn save_file(&mut self) -> Result<()> {
        let buffer = self.editor.active_buffer_mut();
        buffer.save_file()?;
        self.status = format!("Saved: {}", buffer.path.as_ref().map(|p| p.display().to_string()).unwrap_or_else(|| "untitled".to_string()));
        Ok(())
    }

    /// Trigger semantic analysis on current buffer
    fn trigger_semantic_analysis(&mut self) {
        let buffer = self.editor.active_buffer();
        let text = match buffer.text() {
            Ok(text) => text,
            Err(e) => {
                eprintln!("Error getting buffer text: {}", e);
                return;
            }
        };

        // Trigger background analysis
        if let Err(e) = self.semantic_analyzer.analyze(text) {
            eprintln!("Error triggering semantic analysis: {}", e);
        }

        // Try to get result if ready
        if let Some(analysis) = self.semantic_analyzer.try_recv() {
            self.semantic_analysis = Some(analysis);
        }
    }

    /// Convert semantic analysis results to memories and store them
    ///
    /// Creates memories from:
    /// - Triples (relationship/fact memories)
    /// - Typed holes (issues/TODOs)
    /// - Key entities (reference memories)
    async fn store_semantic_memories(&self) -> Result<Vec<MemoryId>> {
        let Some(ref analysis) = self.semantic_analysis else {
            return Ok(Vec::new());
        };

        let mut memory_ids = Vec::new();
        use chrono::Utc;

        // Create memories from high-confidence triples (relationships/facts)
        for triple in &analysis.triples {
            if triple.confidence >= 70 {
                let now = Utc::now();
                let memory = MemoryNote {
                    id: MemoryId::new(),
                    namespace: Namespace::Session {
                        project: "ics".to_string(),
                        session_id: format!("{}", now.timestamp()),
                    },
                    created_at: now,
                    updated_at: now,
                    content: format!("{} {} {}", triple.subject, triple.predicate, triple.object),
                    summary: format!("Relationship: {} {} {}", triple.subject, triple.predicate, triple.object),
                    keywords: vec![triple.subject.clone(), triple.predicate.clone(), triple.object.clone()],
                    tags: vec!["semantic-analysis".to_string(), "relationship".to_string()],
                    context: format!("Extracted from line {}", triple.source_line + 1),
                    memory_type: MemoryType::Insight,
                    importance: (triple.confidence / 10).max(1),
                    confidence: triple.confidence as f32 / 100.0,
                    links: Vec::new(),
                    related_files: Vec::new(),
                    related_entities: vec![triple.subject.clone(), triple.object.clone()],
                    access_count: 0,
                    last_accessed_at: now,
                    expires_at: None,
                    is_archived: false,
                    superseded_by: None,
                    embedding: None,
                    embedding_model: String::new(),
                };

                if let Err(e) = self.storage.store_memory(&memory).await {
                    eprintln!("Error storing triple memory: {}", e);
                } else {
                    memory_ids.push(memory.id);
                }
            }
        }

        // Create memories from typed holes (issues/TODOs)
        for hole in &analysis.holes {
            let now = Utc::now();
            let memory = MemoryNote {
                id: MemoryId::new(),
                namespace: Namespace::Session {
                    project: "ics".to_string(),
                    session_id: format!("{}", now.timestamp()),
                },
                created_at: now,
                updated_at: now,
                content: format!("{}: {}", hole.name, hole.context),
                summary: format!("{} at line {}", hole.kind.icon(), hole.line + 1),
                keywords: vec![hole.name.clone(), format!("{:?}", hole.kind)],
                tags: vec!["semantic-analysis".to_string(), "typed-hole".to_string()],
                context: format!("Line {}, col {}", hole.line + 1, hole.column),
                memory_type: MemoryType::Constraint,
                importance: match hole.kind {
                    super::semantic::HoleKind::Contradiction => 8,
                    super::semantic::HoleKind::Undefined => 7,
                    super::semantic::HoleKind::Incomplete => 5,
                    super::semantic::HoleKind::Ambiguous => 4,
                    super::semantic::HoleKind::Unknown => 3,
                },
                confidence: 0.8,
                links: Vec::new(),
                related_files: Vec::new(),
                related_entities: Vec::new(),
                access_count: 0,
                last_accessed_at: now,
                expires_at: None,
                is_archived: false,
                superseded_by: None,
                embedding: None,
                embedding_model: String::new(),
            };

            if let Err(e) = self.storage.store_memory(&memory).await {
                eprintln!("Error storing hole memory: {}", e);
            } else {
                memory_ids.push(memory.id);
            }
        }

        // Create memories for frequently mentioned entities (mentioned 3+ times)
        for (entity, count) in &analysis.entities {
            if *count >= 3 {
                let now = Utc::now();
                let memory = MemoryNote {
                    id: MemoryId::new(),
                    namespace: Namespace::Session {
                        project: "ics".to_string(),
                        session_id: format!("{}", now.timestamp()),
                    },
                    created_at: now,
                    updated_at: now,
                    content: format!("Entity '{}' mentioned {} times", entity, count),
                    summary: format!("Key entity: {}", entity),
                    keywords: vec![entity.clone(), "entity".to_string()],
                    tags: vec!["semantic-analysis".to_string(), "entity".to_string()],
                    context: "Extracted from semantic analysis".to_string(),
                    memory_type: MemoryType::Reference,
                    importance: (*count).min(10) as u8,
                    confidence: 0.9,
                    links: Vec::new(),
                    related_files: Vec::new(),
                    related_entities: vec![entity.clone()],
                    access_count: 0,
                    last_accessed_at: now,
                    expires_at: None,
                    is_archived: false,
                    superseded_by: None,
                    embedding: None,
                    embedding_model: String::new(),
                };

                if let Err(e) = self.storage.store_memory(&memory).await {
                    eprintln!("Error storing entity memory: {}", e);
                } else {
                    memory_ids.push(memory.id);
                }
            }
        }

        Ok(memory_ids)
    }

    /// Run validation on current buffer
    fn run_validation(&mut self) {
        let buffer = self.editor.active_buffer();
        let text = match buffer.text() {
            Ok(text) => text,
            Err(e) => {
                eprintln!("Error getting buffer text for validation: {}", e);
                return;
            }
        };

        // Run validation
        self.diagnostics = self.validator.validate(&text);
    }

    /// Extract attributions from CRDT buffer
    ///
    /// Converts CRDT Attribution objects to AttributionEntry objects
    /// with line numbers and change descriptions.
    fn extract_attributions(&mut self) {
        use super::attribution::{AttributionEntry, ChangeType};
        use std::time::SystemTime;

        let buffer = self.editor.active_buffer();

        // Get text to calculate line numbers
        let text = match buffer.text() {
            Ok(text) => text,
            Err(e) => {
                eprintln!("Error getting buffer text for attributions: {}", e);
                return;
            }
        };

        // Get all attributions from buffer
        let crdt_attributions = buffer.get_attributions();

        // Convert to AttributionEntry
        let mut entries = Vec::new();
        for attr in crdt_attributions {
            // Calculate line number from character position
            let line = text[..attr.range.0.min(text.len())]
                .chars()
                .filter(|&c| c == '\n')
                .count();

            // Extract changed text snippet (max 50 chars)
            let changed_text: String = text
                .chars()
                .skip(attr.range.0)
                .take(attr.range.1 - attr.range.0)
                .take(50)
                .collect();

            let description = if changed_text.len() > 47 {
                format!("\"{}...\"", &changed_text[..47])
            } else {
                format!("\"{}\"", changed_text)
            };

            // Convert actor to author name
            let author = format!("{:?}", attr.actor);

            // Convert timestamp
            let timestamp = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(attr.timestamp.timestamp() as u64);

            entries.push(AttributionEntry {
                author,
                change_type: ChangeType::Insert, // CRDT currently tracks all changes as inserts
                timestamp,
                line,
                description,
            });
        }

        // Sort by timestamp (most recent first)
        entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        self.attributions = entries;
    }

    /// Load memories from storage into memory panel
    ///
    /// Queries the storage backend and populates the memories list.
    /// Currently loads all memories without namespace filtering.
    /// Sorts by importance by default.
    pub async fn load_memories(&mut self) -> Result<()> {
        // Query storage backend for memories (limit 100, sorted by importance)
        // Note: Namespace filtering not yet implemented in IcsConfig
        let memories = self.storage
            .list_memories(None, 100, MemorySortOrder::Importance)
            .await?;

        // Update state
        let count = memories.len();
        self.memories = memories;
        self.status = format!("Loaded {} memories", count);

        Ok(())
    }

    /// Load agents from registry
    ///
    /// Queries the agent registry (if available) and populates the agents list.
    /// If no registry is available (standalone mode), shows empty list.
    pub async fn load_agents(&mut self) {
        if let Some(ref registry) = self.agent_registry {
            self.agents = registry.list_agents().await;
        } else {
            self.agents = Vec::new();
        }
    }

    /// Poll proposals from queue
    ///
    /// Polls the proposal queue (if available) and populates the proposals list.
    /// If no queue is available (standalone mode), shows empty list.
    pub async fn poll_proposals(&mut self) {
        if let Some(ref queue) = self.proposal_queue {
            self.proposals = queue.try_recv_all().await;
        } else {
            self.proposals = Vec::new();
        }
    }

    // Test accessors for internal state
    #[cfg(test)]
    pub fn memories(&self) -> &[MemoryNote] {
        &self.memories
    }

    #[cfg(test)]
    pub fn agents(&self) -> &[AgentInfo] {
        &self.agents
    }

    #[cfg(test)]
    pub fn proposals(&self) -> &[ChangeProposal] {
        &self.proposals
    }

    #[cfg(test)]
    pub fn attributions(&self) -> &[AttributionEntry] {
        &self.attributions
    }

    #[cfg(test)]
    pub fn status(&self) -> &str {
        &self.status
    }

    #[cfg(test)]
    pub fn editor(&self) -> &IcsEditor {
        &self.editor
    }

    #[cfg(test)]
    pub fn editor_mut(&mut self) -> &mut IcsEditor {
        &mut self.editor
    }

    // Test-only methods for triggering internal operations
    #[cfg(test)]
    pub fn test_trigger_semantic_analysis(&mut self) {
        self.trigger_semantic_analysis()
    }

    #[cfg(test)]
    pub async fn test_store_semantic_memories(&self) -> Result<Vec<MemoryId>> {
        self.store_semantic_memories().await
    }

    #[cfg(test)]
    pub fn test_extract_attributions(&mut self) {
        self.extract_attributions()
    }

    /// Run the ICS application
    pub async fn run(&mut self) -> Result<()> {
        // Initialize terminal
        let mut terminal = TerminalManager::new(TerminalConfig::default())?;
        let event_loop = EventLoop::default();

        // Main event loop
        loop {
            // Render UI
            self.render(&mut terminal)?;

            // Poll for events
            if let Some(event) = event_loop.poll_event()? {
                self.handle_event(event).await?;
            }

            // Check if we should quit
            if self.state == AppState::Quitting {
                break;
            }

            // Small delay to avoid busy looping
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        Ok(())
    }

    /// Handle event
    async fn handle_event(&mut self, event: TuiEvent) -> Result<()> {
        match event {
            TuiEvent::Quit => {
                self.state = AppState::Quitting;
            }
            TuiEvent::Key(key) => {
                let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
                let buffer = self.editor.active_buffer_mut();

                match (key.code, ctrl) {
                    // Quit
                    (KeyCode::Char('q'), true) | (KeyCode::Char('c'), true) => {
                        self.state = AppState::Quitting;
                    }

                    // Save
                    (KeyCode::Char('s'), true) => {
                        if let Err(e) = self.save_file() {
                            self.status = format!("Error saving: {}", e);
                        }
                    }

                    // Toggle memory panel
                    (KeyCode::Char('m'), true) => {
                        self.memory_panel.toggle();
                        if self.memory_panel.is_visible() {
                            // Load memories when panel becomes visible
                            if let Err(e) = self.load_memories().await {
                                self.status = format!("Error loading memories: {}", e);
                            }
                        } else {
                            self.status = "Memory panel: hidden".to_string();
                        }
                    }

                    // Store semantic memories (Ctrl+Shift+M)
                    (KeyCode::Char('M'), true) => {
                        match self.store_semantic_memories().await {
                            Ok(ids) => {
                                self.status = format!("Stored {} semantic memories", ids.len());
                            }
                            Err(e) => {
                                self.status = format!("Error storing memories: {}", e);
                            }
                        }
                    }

                    // Toggle proposals panel
                    (KeyCode::Char('p'), true) => {
                        self.proposals_panel.toggle();
                        if self.proposals_panel.is_visible() {
                            // Poll proposals when panel becomes visible
                            self.poll_proposals().await;
                            let mode = if self.proposal_queue.is_some() {
                                "orchestration"
                            } else {
                                "standalone"
                            };
                            self.status = format!(
                                "Proposals: visible ({} proposals, {} mode)",
                                self.proposals.len(),
                                mode
                            );
                        } else {
                            self.status = "Proposals panel: hidden".to_string();
                        };
                    }

                    // Toggle diagnostics panel
                    (KeyCode::Char('d'), true) => {
                        self.diagnostics_panel.toggle();
                        self.status = if self.diagnostics_panel.is_visible() {
                            "Diagnostics panel: visible".to_string()
                        } else {
                            "Diagnostics panel: hidden".to_string()
                        };
                    }

                    // Toggle agent status panel
                    (KeyCode::Char('a'), true) => {
                        self.agent_status_panel.toggle();
                        if self.agent_status_panel.is_visible() {
                            // Load agents when panel becomes visible
                            self.load_agents().await;
                            let mode = if self.agent_registry.is_some() {
                                "orchestration"
                            } else {
                                "standalone"
                            };
                            self.status = format!(
                                "Agent status: visible ({} agents, {} mode)",
                                self.agents.len(),
                                mode
                            );
                        } else {
                            self.status = "Agent status: hidden".to_string();
                        }
                    }

                    // Toggle attribution panel
                    (KeyCode::Char('t'), true) => {
                        self.attribution_panel.toggle();
                        if self.attribution_panel.is_visible() {
                            // Extract attributions when panel becomes visible
                            self.extract_attributions();
                            self.status = format!("Attribution: visible ({} entries)", self.attributions.len());
                        } else {
                            self.status = "Attribution: hidden".to_string();
                        }
                    }

                    // Undo/Redo
                    (KeyCode::Char('z'), true) => {
                        if let Err(e) = buffer.undo() {
                            self.status = format!("Undo failed: {}", e);
                        } else {
                            self.status = "Undo".to_string();
                        }
                    }
                    (KeyCode::Char('y'), true) => {
                        if let Err(e) = buffer.redo() {
                            self.status = format!("Redo failed: {}", e);
                        } else {
                            self.status = "Redo".to_string();
                        }
                    }

                    // Text input
                    (KeyCode::Char(c), false) => {
                        if let Err(e) = buffer.insert_at_cursor(&c.to_string()) {
                            self.status = format!("Insert failed: {}", e);
                        } else {
                            self.trigger_semantic_analysis();
                            self.run_validation();
                        }
                    }

                    // Newline
                    (KeyCode::Enter, _) => {
                        if let Err(e) = buffer.insert_at_cursor("\n") {
                            self.status = format!("Insert failed: {}", e);
                        } else {
                            self.trigger_semantic_analysis();
                            self.run_validation();
                        }
                    }

                    // Backspace
                    (KeyCode::Backspace, _) => {
                        let pos = buffer.cursor.position.column;
                        if pos > 0 {
                            if let Err(e) = buffer.move_cursor(Movement::Left) {
                                self.status = format!("Move cursor failed: {}", e);
                            } else if let Err(e) = buffer.delete_at_cursor() {
                                self.status = format!("Delete failed: {}", e);
                            } else {
                                self.trigger_semantic_analysis();
                                self.run_validation();
                            }
                        }
                    }

                    // Delete
                    (KeyCode::Delete, _) => {
                        if let Err(e) = buffer.delete_at_cursor() {
                            self.status = format!("Delete failed: {}", e);
                        } else {
                            self.trigger_semantic_analysis();
                            self.run_validation();
                        }
                    }

                    // Cursor movement
                    (KeyCode::Left, _) => {
                        let _ = buffer.move_cursor(Movement::Left);
                    }
                    (KeyCode::Right, _) => {
                        let _ = buffer.move_cursor(Movement::Right);
                    }
                    (KeyCode::Up, _) => {
                        let _ = buffer.move_cursor(Movement::Up);
                    }
                    (KeyCode::Down, _) => {
                        let _ = buffer.move_cursor(Movement::Down);
                    }
                    (KeyCode::Home, _) => {
                        let _ = buffer.move_cursor(Movement::LineStart);
                    }
                    (KeyCode::End, _) => {
                        let _ = buffer.move_cursor(Movement::LineEnd);
                    }

                    _ => {}
                }
            }
            TuiEvent::Resize(_, _) => {
                // Terminal resized
            }
            _ => {}
        }
        Ok(())
    }

    /// Render UI
    fn render(&mut self, terminal: &mut TerminalManager) -> Result<()> {
        let buffer = self.editor.active_buffer();
        let editor_state = &mut self.editor_state;
        let status = &self.status;

        terminal.terminal_mut().draw(|frame| {
            let size = frame.area();

            // Count visible panels
            let bottom_panels_visible = self.diagnostics_panel.is_visible() || self.proposals_panel.is_visible();
            let right_panels_visible = self.memory_panel.is_visible() || self.agent_status_panel.is_visible() || self.attribution_panel.is_visible();

            // Create main vertical layout
            let mut v_constraints = vec![Constraint::Length(1)]; // Status bar

            if bottom_panels_visible {
                v_constraints.push(Constraint::Percentage(60)); // Main area (editor + right panels)
                v_constraints.push(Constraint::Percentage(40)); // Bottom panels
            } else {
                v_constraints.push(Constraint::Min(10)); // Main area
            }

            v_constraints.push(Constraint::Length(1)); // Info bar

            let main_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(v_constraints)
                .split(size);

            // Render status bar at top
            let status_text = format!(" {}", status);
            let status_widget = Paragraph::new(status_text)
                .style(Style::default().fg(Color::White).bg(Color::DarkGray));
            frame.render_widget(status_widget, main_chunks[0]);

            // Split main area horizontally if right panels visible
            let (editor_area, right_panel_area) = if right_panels_visible {
                let h_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(60), // Editor
                        Constraint::Percentage(40), // Right panels
                    ])
                    .split(main_chunks[1]);
                (h_chunks[0], Some(h_chunks[1]))
            } else {
                (main_chunks[1], None)
            };

            // Render editor
            let editor_title = if let Some(path) = &buffer.path {
                let dirty_mark = if buffer.dirty { "*" } else { "" };
                format!(" {}{} ", path.display(), dirty_mark)
            } else {
                let dirty_mark = if buffer.dirty { "*" } else { "" };
                format!(" [untitled]{} ", dirty_mark)
            };

            let editor_block = Block::default()
                .borders(Borders::NONE)
                .title(editor_title)
                .style(Style::default());

            let editor_widget = EditorWidget::new(buffer)
                .block(editor_block)
                .diagnostics(&self.diagnostics)
                .focused(true);

            frame.render_stateful_widget(editor_widget, editor_area, editor_state);

            // Render right panels if visible
            if let Some(right_area) = right_panel_area {
                // Count visible right panels
                let visible_right_count = [
                    self.memory_panel.is_visible(),
                    self.agent_status_panel.is_visible(),
                    self.attribution_panel.is_visible(),
                ]
                .iter()
                .filter(|&&v| v)
                .count();

                if visible_right_count > 0 {
                    let panel_height = 100 / visible_right_count as u16;
                    let mut constraints = Vec::new();
                    for _ in 0..visible_right_count {
                        constraints.push(Constraint::Percentage(panel_height));
                    }

                    let right_chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints(constraints)
                        .split(right_area);

                    let mut chunk_idx = 0;

                    if self.memory_panel.is_visible() {
                        let panel_widget = MemoryPanel::new(&self.memories);
                        frame.render_stateful_widget(panel_widget, right_chunks[chunk_idx], &mut self.memory_panel);
                        chunk_idx += 1;
                    }

                    if self.agent_status_panel.is_visible() {
                        let panel_widget = AgentStatusWidget::new(&self.agents);
                        frame.render_stateful_widget(panel_widget, right_chunks[chunk_idx], &mut self.agent_status_panel);
                        chunk_idx += 1;
                    }

                    if self.attribution_panel.is_visible() {
                        let panel_widget = AttributionPanel::new(&self.attributions);
                        frame.render_stateful_widget(panel_widget, right_chunks[chunk_idx], &mut self.attribution_panel);
                    }
                }
            }

            // Render bottom panels if visible
            if bottom_panels_visible {
                let bottom_area = main_chunks[2];

                // Count visible bottom panels
                let visible_bottom_count = [
                    self.diagnostics_panel.is_visible(),
                    self.proposals_panel.is_visible(),
                ]
                .iter()
                .filter(|&&v| v)
                .count();

                if visible_bottom_count > 0 {
                    let panel_width = 100 / visible_bottom_count as u16;
                    let mut constraints = Vec::new();
                    for _ in 0..visible_bottom_count {
                        constraints.push(Constraint::Percentage(panel_width));
                    }

                    let bottom_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints(constraints)
                        .split(bottom_area);

                    let mut chunk_idx = 0;

                    if self.diagnostics_panel.is_visible() {
                        let panel_widget = DiagnosticsPanel::new(&self.diagnostics);
                        frame.render_stateful_widget(panel_widget, bottom_chunks[chunk_idx], &mut self.diagnostics_panel);
                        chunk_idx += 1;
                    }

                    if self.proposals_panel.is_visible() {
                        let selected_proposal = self.proposals_panel.selected()
                            .and_then(|idx| self.proposals.get(idx));
                        let panel_widget = ProposalsPanel::new(&self.proposals, selected_proposal);
                        frame.render_stateful_widget(panel_widget, bottom_chunks[chunk_idx], &mut self.proposals_panel);
                    }
                }
            }

            // Render info bar at bottom (cursor position, language, semantic stats)
            let cursor_pos = format!(
                "Ln {}, Col {} ",
                buffer.cursor.position.line + 1,
                buffer.cursor.position.column + 1
            );
            let lang = format!("{:?} ", buffer.language);
            let semantic_info = if self.semantic_analyzer.is_analyzing() {
                "| Analyzing... ".to_string()
            } else if let Some(analysis) = &self.semantic_analysis {
                format!(
                    "| Triples: {} | Holes: {} | Entities: {} ",
                    analysis.triples.len(),
                    analysis.holes.len(),
                    analysis.entities.len()
                )
            } else {
                String::new()
            };
            let info_text = format!("{} | {}{}", cursor_pos, lang, semantic_info);

            let info_widget = Paragraph::new(info_text)
                .style(Style::default().fg(Color::DarkGray));
            let info_bar_index = if bottom_panels_visible { 3 } else { 2 };
            frame.render_widget(info_widget, main_chunks[info_bar_index]);
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        launcher::agents::AgentRole,
        orchestration::{AgentRegistry, ProposalQueue},
        ics::proposals::ProposalStatus,
        ConnectionMode, LibsqlStorage,
    };
    use std::sync::Arc;
    use std::time::SystemTime;

    /// Helper to create test memory
    fn create_test_memory(id: &str, content: &str, importance: u8) -> MemoryNote {
        use chrono::Utc;
        let now = Utc::now();
        MemoryNote {
            id: MemoryId::new(),
            namespace: Namespace::Session {
                project: "test".to_string(),
                session_id: id.to_string(),
            },
            created_at: now,
            updated_at: now,
            content: content.to_string(),
            summary: format!("Test: {}", content),
            keywords: vec!["test".to_string()],
            tags: vec!["integration".to_string()],
            context: "test context".to_string(),
            memory_type: MemoryType::Insight,
            importance,
            confidence: 0.9,
            links: Vec::new(),
            related_files: Vec::new(),
            related_entities: Vec::new(),
            access_count: 0,
            last_accessed_at: now,
            expires_at: None,
            is_archived: false,
            superseded_by: None,
            embedding: None,
            embedding_model: String::new(),
        }
    }

    #[tokio::test]
    async fn test_app_initialization_standalone() {
        let storage = Arc::new(
            LibsqlStorage::new(ConnectionMode::InMemory)
                .await
                .expect("Failed to create storage"),
        );

        let config = IcsConfig::default();
        let app = IcsApp::new(config, storage, None, None);

        assert_eq!(app.memories().len(), 0);
        assert_eq!(app.agents().len(), 0);
        assert_eq!(app.proposals().len(), 0);
        assert_eq!(app.attributions().len(), 0);
    }

    #[tokio::test]
    async fn test_memory_loading() {
        let storage = Arc::new(
            LibsqlStorage::new_with_validation(ConnectionMode::InMemory, true)
                .await
                .expect("Failed to create storage"),
        );

        let mem1 = create_test_memory("1", "First memory", 8);
        let mem2 = create_test_memory("2", "Second memory", 5);
        let mem3 = create_test_memory("3", "Third memory", 9);

        storage.store_memory(&mem1).await.expect("Failed to store");
        storage.store_memory(&mem2).await.expect("Failed to store");
        storage.store_memory(&mem3).await.expect("Failed to store");

        let config = IcsConfig::default();
        let mut app = IcsApp::new(config, storage, None, None);

        app.load_memories().await.expect("Failed to load memories");

        let loaded = app.memories();
        assert_eq!(loaded.len(), 3);
        assert!(loaded.iter().any(|m| m.content == "First memory"));
        assert!(app.status().contains("Loaded 3 memories"));
    }

    #[tokio::test]
    async fn test_agent_tracking_standalone() {
        let storage = Arc::new(
            LibsqlStorage::new(ConnectionMode::InMemory)
                .await
                .expect("Failed to create storage"),
        );

        let config = IcsConfig::default();
        let mut app = IcsApp::new(config, storage, None, None);

        app.load_agents().await;

        assert_eq!(app.agents().len(), 0);
    }

    #[tokio::test]
    async fn test_agent_tracking_orchestration() {
        let storage = Arc::new(
            LibsqlStorage::new(ConnectionMode::InMemory)
                .await
                .expect("Failed to create storage"),
        );

        let registry = AgentRegistry::new();
        registry.register("test-optimizer".to_string(), "Optimizer".to_string(), AgentRole::Optimizer).await;
        registry.register("test-reviewer".to_string(), "Reviewer".to_string(), AgentRole::Reviewer).await;

        let config = IcsConfig::default();
        let mut app = IcsApp::new(config, storage, Some(registry), None);

        app.load_agents().await;

        let agents = app.agents();
        assert_eq!(agents.len(), 2);
        assert!(agents.iter().any(|a| a.name == "Optimizer"));
        assert!(agents.iter().any(|a| a.name == "Reviewer"));
    }

    #[tokio::test]
    async fn test_proposal_polling_standalone() {
        let storage = Arc::new(
            LibsqlStorage::new(ConnectionMode::InMemory)
                .await
                .expect("Failed to create storage"),
        );

        let config = IcsConfig::default();
        let mut app = IcsApp::new(config, storage, None, None);

        app.poll_proposals().await;

        assert_eq!(app.proposals().len(), 0);
    }

    #[tokio::test]
    async fn test_proposal_polling_orchestration() {
        let storage = Arc::new(
            LibsqlStorage::new(ConnectionMode::InMemory)
                .await
                .expect("Failed to create storage"),
        );

        let queue = ProposalQueue::new();

        let proposal = ChangeProposal {
            id: "prop-1".to_string(),
            agent: "TestAgent".to_string(),
            description: "Test proposal".to_string(),
            original: "old".to_string(),
            proposed: "new".to_string(),
            line_range: (10, 20),
            created_at: SystemTime::now(),
            status: ProposalStatus::Pending,
            rationale: "Test".to_string(),
        };

        queue.send(proposal).expect("Failed to send");

        let config = IcsConfig::default();
        let mut app = IcsApp::new(config, storage, None, Some(queue));

        app.poll_proposals().await;

        let proposals = app.proposals();
        assert_eq!(proposals.len(), 1);
        assert_eq!(proposals[0].id, "prop-1");
    }

    #[tokio::test]
    async fn test_file_operations() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("ics_test_file.txt");
        std::fs::write(&test_file, "Initial content").expect("Failed to write");

        let storage = Arc::new(
            LibsqlStorage::new(ConnectionMode::InMemory)
                .await
                .expect("Failed to create storage"),
        );

        let config = IcsConfig::default();
        let mut app = IcsApp::new(config, storage, None, None);

        app.load_file(test_file.clone()).expect("Failed to load");

        let buffer = app.editor().active_buffer();
        let text = buffer.text().expect("Failed to get text");
        assert_eq!(text, "Initial content");

        let buffer = app.editor_mut().active_buffer_mut();
        buffer.insert_at_cursor("\nAdded line").expect("Failed to insert");

        app.save_file().expect("Failed to save");

        let saved_content = std::fs::read_to_string(&test_file).expect("Failed to read");
        assert!(saved_content.contains("Initial content"));
        assert!(saved_content.contains("Added line"));

        std::fs::remove_file(&test_file).ok();
    }

    #[tokio::test]
    async fn test_attribution_extraction() {
        let storage = Arc::new(
            LibsqlStorage::new(ConnectionMode::InMemory)
                .await
                .expect("Failed to create storage"),
        );

        let config = IcsConfig::default();
        let mut app = IcsApp::new(config, storage, None, None);

        let buffer = app.editor_mut().active_buffer_mut();
        buffer.insert_at_cursor("Line 1\n").expect("Failed to insert");
        buffer.insert_at_cursor("Line 2\n").expect("Failed to insert");

        app.test_extract_attributions();

        let attributions = app.attributions();
        assert!(attributions.len() > 0);

        for attr in attributions {
            assert!(attr.timestamp > SystemTime::UNIX_EPOCH);
            assert!(!attr.description.is_empty());
        }
    }

    #[tokio::test]
    async fn test_dual_mode_support() {
        let storage1 = Arc::new(
            LibsqlStorage::new(ConnectionMode::InMemory)
                .await
                .expect("Failed to create storage"),
        );
        let storage2 = Arc::new(
            LibsqlStorage::new(ConnectionMode::InMemory)
                .await
                .expect("Failed to create storage"),
        );

        // Standalone mode
        let config1 = IcsConfig::default();
        let mut app1 = IcsApp::new(config1, storage1, None, None);

        app1.load_agents().await;
        app1.poll_proposals().await;

        assert_eq!(app1.agents().len(), 0);
        assert_eq!(app1.proposals().len(), 0);

        // Orchestration mode
        let registry = AgentRegistry::new();
        registry.register("test".to_string(), "Test".to_string(), AgentRole::Optimizer).await;

        let queue = ProposalQueue::new();
        queue.send(ChangeProposal {
            id: "test".to_string(),
            agent: "Test".to_string(),
            description: "Test".to_string(),
            original: "old".to_string(),
            proposed: "new".to_string(),
            line_range: (1, 2),
            created_at: SystemTime::now(),
            status: ProposalStatus::Pending,
            rationale: "Test".to_string(),
        }).expect("Failed to send");

        let config2 = IcsConfig::default();
        let mut app2 = IcsApp::new(config2, storage2, Some(registry), Some(queue));

        app2.load_agents().await;
        app2.poll_proposals().await;

        assert_eq!(app2.agents().len(), 1);
        assert_eq!(app2.proposals().len(), 1);
    }

    #[tokio::test]
    async fn test_memory_sorting_by_importance() {
        let storage = Arc::new(
            LibsqlStorage::new_with_validation(ConnectionMode::InMemory, true)
                .await
                .expect("Failed to create storage"),
        );

        let mem1 = create_test_memory("1", "Low importance", 3);
        let mem2 = create_test_memory("2", "High importance", 9);
        let mem3 = create_test_memory("3", "Medium importance", 6);

        storage.store_memory(&mem1).await.expect("Failed to store");
        storage.store_memory(&mem2).await.expect("Failed to store");
        storage.store_memory(&mem3).await.expect("Failed to store");

        let config = IcsConfig::default();
        let mut app = IcsApp::new(config, storage, None, None);

        app.load_memories().await.expect("Failed to load");

        let loaded = app.memories();
        assert_eq!(loaded.len(), 3);
        assert_eq!(loaded[0].importance, 9);
        assert_eq!(loaded[1].importance, 6);
        assert_eq!(loaded[2].importance, 3);
    }
}
