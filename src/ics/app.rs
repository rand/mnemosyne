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
    tui::{EventLoop, TerminalConfig, TerminalManager, TuiEvent},
    types::MemoryNote,
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
    /// Memory panel state
    memory_panel: MemoryPanelState,
    /// Placeholder memories (will be fetched from storage in real implementation)
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
    pub fn new(config: IcsConfig) -> Self {
        Self {
            config,
            editor: IcsEditor::new(),
            editor_state: EditorState::default(),
            state: AppState::Running,
            status: "ICS | Ctrl+Q: quit | Ctrl+M: memories | Ctrl+P: proposals | Ctrl+D: diagnostics | Ctrl+A: agents".to_string(),

            // Phase 3: Memory Integration
            memory_panel: MemoryPanelState::new(),
            memories: Vec::new(), // TODO: fetch from storage

            // Phase 4: Semantic Analysis
            semantic_analyzer: SemanticAnalyzer::new(),
            semantic_analysis: None,

            // Phase 5: Agent Collaboration
            agent_status_panel: AgentStatusState::new(),
            agents: Vec::new(), // TODO: track active agents
            attribution_panel: AttributionPanelState::new(),
            attributions: Vec::new(), // TODO: extract from CRDT
            proposals_panel: ProposalsPanelState::new(),
            proposals: Vec::new(), // TODO: agent proposals

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
        let text = buffer.text().to_string();

        // Trigger background analysis
        if let Err(e) = self.semantic_analyzer.analyze(text) {
            eprintln!("Error triggering semantic analysis: {}", e);
        }

        // Try to get result if ready
        if let Some(analysis) = self.semantic_analyzer.try_recv() {
            self.semantic_analysis = Some(analysis);
        }
    }

    /// Run validation on current buffer
    fn run_validation(&mut self) {
        let buffer = self.editor.active_buffer();
        let text = buffer.text().to_string();

        // Run validation
        self.diagnostics = self.validator.validate(&text);
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
                        self.status = if self.memory_panel.is_visible() {
                            "Memory panel: visible".to_string()
                        } else {
                            "Memory panel: hidden".to_string()
                        };
                    }

                    // Toggle proposals panel
                    (KeyCode::Char('p'), true) => {
                        self.proposals_panel.toggle();
                        self.status = if self.proposals_panel.is_visible() {
                            "Proposals panel: visible".to_string()
                        } else {
                            "Proposals panel: hidden".to_string()
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
                        self.status = if self.agent_status_panel.is_visible() {
                            "Agent status: visible".to_string()
                        } else {
                            "Agent status: hidden".to_string()
                        };
                    }

                    // Toggle attribution panel
                    (KeyCode::Char('t'), true) => {
                        self.attribution_panel.toggle();
                        self.status = if self.attribution_panel.is_visible() {
                            "Attribution: visible".to_string()
                        } else {
                            "Attribution: hidden".to_string()
                        };
                    }

                    // Undo/Redo
                    (KeyCode::Char('z'), true) => {
                        buffer.undo();
                        self.status = "Undo".to_string();
                    }
                    (KeyCode::Char('y'), true) => {
                        buffer.redo();
                        self.status = "Redo".to_string();
                    }

                    // Text input
                    (KeyCode::Char(c), false) => {
                        buffer.insert(&c.to_string());
                        self.trigger_semantic_analysis();
                        self.run_validation();
                    }

                    // Newline
                    (KeyCode::Enter, _) => {
                        buffer.insert("\n");
                        self.trigger_semantic_analysis();
                        self.run_validation();
                    }

                    // Backspace
                    (KeyCode::Backspace, _) => {
                        let pos = buffer.cursor.position.column;
                        if pos > 0 {
                            buffer.move_cursor(Movement::Left);
                            buffer.delete();
                            self.trigger_semantic_analysis();
                            self.run_validation();
                        }
                    }

                    // Delete
                    (KeyCode::Delete, _) => {
                        buffer.delete();
                        self.trigger_semantic_analysis();
                        self.run_validation();
                    }

                    // Cursor movement
                    (KeyCode::Left, _) => {
                        buffer.move_cursor(Movement::Left);
                    }
                    (KeyCode::Right, _) => {
                        buffer.move_cursor(Movement::Right);
                    }
                    (KeyCode::Up, _) => {
                        buffer.move_cursor(Movement::Up);
                    }
                    (KeyCode::Down, _) => {
                        buffer.move_cursor(Movement::Down);
                    }
                    (KeyCode::Home, _) => {
                        buffer.move_cursor(Movement::LineStart);
                    }
                    (KeyCode::End, _) => {
                        buffer.move_cursor(Movement::LineEnd);
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
            let semantic_info = if let Some(analysis) = &self.semantic_analysis {
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

impl Default for IcsApp {
    fn default() -> Self {
        Self::new(IcsConfig::default())
    }
}
