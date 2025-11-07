//! Main ICS application
#![allow(dead_code)]
//!
//! Standalone ICS application that can be run with `mnemosyne --ics`

use super::{
    agent_status::{AgentInfo, AgentStatusState, AgentStatusWidget},
    attribution::{AttributionEntry, AttributionPanel, AttributionPanelState},
    diagnostics_panel::{DiagnosticsPanel, DiagnosticsPanelState},
    editor::{Diagnostic, EditorState, EditorWidget, IcsEditor, Movement, Position, Validator},
    memory_panel::{MemoryPanel, MemoryPanelState},
    proposals::{ChangeProposal, ProposalsPanel, ProposalsPanelState},
    semantic::{SemanticAnalysis, SemanticAnalyzer},
    IcsConfig,
};
use crate::{
    orchestration::{AgentRegistry, ProposalQueue},
    storage::{MemorySortOrder, StorageBackend},
    tui::{EventLoop, TerminalConfig, TerminalManager, TuiEvent},
    types::{MemoryId, MemoryNote, MemoryType, Namespace},
    utils::string::truncate_at_char_boundary,
};
use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph},
};
use std::path::PathBuf;
use std::sync::Arc;

/// Application state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppState {
    Running,
    Quitting,
}

/// Panel types for programmatic opening (--panel flag)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelType {
    Memory,
    Diagnostics,
    Proposals,
    Holes,
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

    // Phase 7: Real-time Completion
    /// Symbol registry for completion
    symbol_registry: super::SharedSymbolRegistry,
    /// Completion engine
    completion_engine: Option<super::CompletionEngine>,
    /// Completion popup widget
    completion_popup: super::CompletionPopup,

    // Phase 8: Typed Holes Navigation
    /// Hole navigator for jumping between typed holes
    hole_navigator: super::HoleNavigator,

    // Phase 2.1: Event Broadcasting
    /// Optional event broadcaster for real-time API updates
    event_broadcaster: Option<crate::api::EventBroadcaster>,
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
        Self::new_with_broadcaster(config, storage, agent_registry, proposal_queue, None)
    }

    /// Create new ICS application with event broadcasting
    ///
    /// # Arguments
    /// * `config` - ICS configuration
    /// * `storage` - Storage backend for memory retrieval
    /// * `agent_registry` - Optional agent registry for orchestration mode
    /// * `proposal_queue` - Optional proposal queue for orchestration mode
    /// * `event_broadcaster` - Optional event broadcaster for real-time API updates
    pub fn new_with_broadcaster(
        config: IcsConfig,
        storage: Arc<dyn StorageBackend>,
        agent_registry: Option<AgentRegistry>,
        proposal_queue: Option<ProposalQueue>,
        event_broadcaster: Option<crate::api::EventBroadcaster>,
    ) -> Self {
        Self {
            config,
            editor: IcsEditor::new(),
            editor_state: EditorState::default(),
            state: AppState::Running,
            status: "ICS | Ctrl+Q: quit | Ctrl+S: save | Ctrl+M: memories | Ctrl+N: next hole | Ctrl+H: holes list | Ctrl+P: proposals | Ctrl+D: diagnostics".to_string(),

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

            // Phase 7: Real-time Completion
            symbol_registry: Arc::new(std::sync::RwLock::new(super::SymbolRegistry::new())),
            completion_engine: None, // Initialized lazily when first needed
            completion_popup: super::CompletionPopup::new(),

            // Phase 8: Typed Holes Navigation
            hole_navigator: super::HoleNavigator::new(),

            // Phase 2.1: Event Broadcasting
            event_broadcaster,
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
        // Check read-only mode
        if self.config.read_only {
            self.status = "Cannot save: Read-only mode".to_string();
            return Err(anyhow::anyhow!("Cannot save in read-only mode"));
        }

        let buffer = self.editor.active_buffer_mut();
        let file_path = buffer
            .path
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "untitled".to_string());

        buffer.save_file()?;
        self.status = format!("Saved: {}", file_path);

        // Emit context_modified event if broadcaster available
        if let Some(broadcaster) = &self.event_broadcaster {
            let event = crate::api::Event::context_modified(file_path.clone());
            if let Err(e) = broadcaster.broadcast(event) {
                tracing::debug!("Failed to broadcast context_modified event: {}", e);
                // Don't fail save if broadcasting fails
            }
        }

        Ok(())
    }

    /// Show a specific panel (for --panel CLI flag)
    pub fn show_panel(&mut self, panel: PanelType) {
        match panel {
            PanelType::Memory => {
                if !self.memory_panel.is_visible() {
                    self.memory_panel.toggle();
                }
            }
            PanelType::Diagnostics => {
                if !self.diagnostics_panel.is_visible() {
                    self.diagnostics_panel.toggle();
                }
            }
            PanelType::Proposals => {
                if !self.proposals_panel.is_visible() {
                    self.proposals_panel.toggle();
                }
            }
            PanelType::Holes => {
                // Show holes count in status (no actual panel for holes)
                let hole_count = self.hole_navigator.hole_count();
                let unresolved = self.hole_navigator.unresolved_holes().len();
                self.status = format!(
                    "Holes: {} total, {} unresolved | Use Ctrl+N/Ctrl+Shift+N to navigate",
                    hole_count, unresolved
                );
            }
        }
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
            // Update hole navigator with new holes
            self.hole_navigator.update_holes(analysis.holes.clone());

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
                    summary: format!(
                        "Relationship: {} {} {}",
                        triple.subject, triple.predicate, triple.object
                    ),
                    keywords: vec![
                        triple.subject.clone(),
                        triple.predicate.clone(),
                        triple.object.clone(),
                    ],
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

            let truncated = truncate_at_char_boundary(&changed_text, 47);
            let description = format!("\"{}\"", truncated);

            // Convert actor to author name
            let author = format!("{:?}", attr.actor);

            // Convert timestamp
            let timestamp = SystemTime::UNIX_EPOCH
                + std::time::Duration::from_secs(attr.timestamp.timestamp() as u64);

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
        let memories = self
            .storage
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

    /// Initialize completion engine (lazy)
    fn ensure_completion_engine(&mut self) {
        if self.completion_engine.is_none() {
            let namespace = Namespace::Session {
                project: "ics".to_string(),
                session_id: format!("{}", chrono::Utc::now().timestamp()),
            };

            let engine = super::CompletionEngine::new(
                self.symbol_registry.clone(),
                self.storage.clone(),
                namespace,
                None, // Project root not set yet
            );

            self.completion_engine = Some(engine);
        }
    }

    /// Trigger completion check at current cursor position
    ///
    /// Checks if user is typing after @ or # and shows completion popup
    async fn trigger_completion(&mut self) {
        self.ensure_completion_engine();

        let buffer = self.editor.active_buffer();
        let cursor = buffer.cursor.position;

        // Get current line text
        let text = match buffer.text() {
            Ok(text) => text,
            Err(_) => return,
        };

        let lines: Vec<&str> = text.lines().collect();
        if cursor.line >= lines.len() {
            return;
        }

        let line = lines[cursor.line];

        // Get completions from engine
        if let Some(ref engine) = self.completion_engine {
            let candidates = engine.get_completions(line, cursor.column).await;

            if !candidates.is_empty() {
                // Extract prefix from line
                let (_, prefix) = engine.detect_context(line, cursor.column);

                // Show popup
                self.completion_popup.show(candidates, cursor, prefix);
            } else {
                // Hide popup if no candidates
                self.completion_popup.hide();
            }
        }
    }

    /// Insert selected completion at cursor
    fn insert_completion(&mut self) {
        if let Some(candidate) = self.completion_popup.selected_completion() {
            let buffer = self.editor.active_buffer_mut();
            let cursor_pos = buffer.cursor.position;

            // Get current line
            let text = match buffer.text() {
                Ok(text) => text,
                Err(_) => {
                    self.completion_popup.hide();
                    return;
                }
            };

            let lines: Vec<&str> = text.lines().collect();
            if cursor_pos.line >= lines.len() {
                self.completion_popup.hide();
                return;
            }

            let line = lines[cursor_pos.line];

            // Find the start of the completion (@ or # trigger)
            let prefix_len = self.completion_popup.prefix().len();
            let trigger_col = cursor_pos.column.saturating_sub(prefix_len);

            // Check if there's a @ or # before the prefix
            let has_trigger = if trigger_col > 0 {
                let char_before = line.chars().nth(trigger_col - 1);
                char_before == Some('@') || char_before == Some('#')
            } else {
                false
            };

            let start_col = if has_trigger && trigger_col > 0 {
                trigger_col - 1 // Include the @ or #
            } else {
                trigger_col
            };

            // Delete characters from start to cursor (delete the prefix + trigger)
            let chars_to_delete = cursor_pos.column - start_col;
            for _ in 0..chars_to_delete {
                // Move left and delete
                if let Err(e) = buffer.move_cursor(Movement::Left) {
                    eprintln!("Failed to move cursor for deletion: {}", e);
                    break;
                }
            }
            for _ in 0..chars_to_delete {
                if let Err(e) = buffer.delete_at_cursor() {
                    eprintln!("Failed to delete character: {}", e);
                    break;
                }
            }

            // Insert completion text
            if let Err(e) = buffer.insert_at_cursor(&candidate.text) {
                eprintln!("Failed to insert completion: {}", e);
            }

            self.completion_popup.hide();
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
        // Pre-flight: Check if running in a terminal
        if !atty::is(atty::Stream::Stdin) || !atty::is(atty::Stream::Stdout) {
            eprintln!("\n❌ ICS requires a terminal (TTY)");
            eprintln!();
            eprintln!(
                "Current mode: {}",
                if !atty::is(atty::Stream::Stdin) {
                    "stdin is piped/redirected"
                } else {
                    "stdout is piped/redirected"
                }
            );
            eprintln!();
            eprintln!("Solutions:");
            eprintln!("  • Run in a terminal emulator");
            eprintln!("  • Redirect: mnemosyne ics file.md < /dev/tty");
            eprintln!();
            return Err(crate::error::MnemosyneError::Other("Not a TTY".into()).into());
        }

        // Check terminal size
        let (width, height) = crossterm::terminal::size().map_err(|e| {
            eprintln!("\n❌ Cannot determine terminal size: {}", e);
            eprintln!();
            eprintln!("Common causes:");
            eprintln!("  • SSH without TERM variable");
            eprintln!("  • tmux/screen misconfiguration");
            eprintln!();
            eprintln!("Try: export TERM=xterm-256color");
            eprintln!();
            e
        })?;

        if width < 80 || height < 24 {
            eprintln!("⚠️  Small terminal: {}x{}", width, height);
            eprintln!("   Recommended: 80x24 minimum");
            eprintln!();
        }

        // Initialize terminal with better error messages
        let mut terminal = TerminalManager::new(TerminalConfig::default()).map_err(|e| {
            eprintln!("\n❌ Terminal initialization failed");
            eprintln!();
            eprintln!("Error: {}", e);
            eprintln!();
            eprintln!("Troubleshooting:");
            eprintln!("  • Check TERM variable: echo $TERM");
            eprintln!("  • Try: export TERM=xterm-256color");
            eprintln!("  • Verify terminal supports ANSI colors");
            eprintln!();
            e
        })?;
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
                    (KeyCode::Char('M'), true) => match self.store_semantic_memories().await {
                        Ok(ids) => {
                            self.status = format!("Stored {} semantic memories", ids.len());
                        }
                        Err(e) => {
                            self.status = format!("Error storing memories: {}", e);
                        }
                    },

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
                            self.status = format!(
                                "Attribution: visible ({} entries)",
                                self.attributions.len()
                            );
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

                    // Tab - accept completion if popup is visible
                    (KeyCode::Tab, _) if self.completion_popup.is_visible() => {
                        self.insert_completion();
                    }

                    // Escape - cancel completion
                    (KeyCode::Esc, _) if self.completion_popup.is_visible() => {
                        self.completion_popup.hide();
                        self.status = "Completion cancelled".to_string();
                    }

                    // Text input
                    (KeyCode::Char(c), false) => {
                        if let Err(e) = buffer.insert_at_cursor(&c.to_string()) {
                            self.status = format!("Insert failed: {}", e);
                        } else {
                            self.trigger_semantic_analysis();
                            self.run_validation();
                            // Trigger completion on @ or # or continue existing completion
                            self.trigger_completion().await;
                        }
                    }

                    // Enter - accept completion if visible, otherwise newline
                    (KeyCode::Enter, _) if self.completion_popup.is_visible() => {
                        self.insert_completion();
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

                    // Cursor movement - navigate completion if visible
                    (KeyCode::Up, _) if self.completion_popup.is_visible() => {
                        self.completion_popup.select_previous();
                    }
                    (KeyCode::Down, _) if self.completion_popup.is_visible() => {
                        self.completion_popup.select_next();
                    }

                    // Regular cursor movement
                    (KeyCode::Left, false) => {
                        let _ = buffer.move_cursor(Movement::Left);
                        self.completion_popup.hide(); // Hide on cursor movement
                    }
                    (KeyCode::Right, false) => {
                        let _ = buffer.move_cursor(Movement::Right);
                        self.completion_popup.hide(); // Hide on cursor movement
                    }
                    (KeyCode::Up, _) => {
                        let _ = buffer.move_cursor(Movement::Up);
                        self.completion_popup.hide(); // Hide on cursor movement
                    }
                    (KeyCode::Down, _) => {
                        let _ = buffer.move_cursor(Movement::Down);
                        self.completion_popup.hide(); // Hide on cursor movement
                    }
                    (KeyCode::Home, false) => {
                        let _ = buffer.move_cursor(Movement::LineStart);
                    }
                    (KeyCode::End, false) => {
                        let _ = buffer.move_cursor(Movement::LineEnd);
                    }

                    // Word navigation (Ctrl+Left/Right)
                    (KeyCode::Left, true) => {
                        let _ = buffer.move_cursor(Movement::WordLeft);
                        self.completion_popup.hide();
                    }
                    (KeyCode::Right, true) => {
                        let _ = buffer.move_cursor(Movement::WordRight);
                        self.completion_popup.hide();
                    }

                    // Word end (Alt+E, Helix-style)
                    (KeyCode::Char('e'), _) if key.modifiers.contains(KeyModifiers::ALT) => {
                        let _ = buffer.move_cursor(Movement::WordEnd);
                        self.completion_popup.hide();
                    }

                    // Page navigation
                    (KeyCode::PageUp, _) => {
                        let _ = buffer.move_cursor(Movement::PageUp);
                        self.completion_popup.hide();
                    }
                    (KeyCode::PageDown, _) => {
                        let _ = buffer.move_cursor(Movement::PageDown);
                        self.completion_popup.hide();
                    }

                    // Buffer start/end (Ctrl+Home/End)
                    (KeyCode::Home, true) => {
                        let _ = buffer.move_cursor(Movement::BufferStart);
                        self.completion_popup.hide();
                    }
                    (KeyCode::End, true) => {
                        let _ = buffer.move_cursor(Movement::BufferEnd);
                        self.completion_popup.hide();
                    }

                    // Hole navigation (Ctrl+N for next hole, Ctrl+Shift+N for previous hole)
                    (KeyCode::Char('n'), true) if !key.modifiers.contains(KeyModifiers::SHIFT) => {
                        // Next hole
                        let cursor_pos = buffer.cursor.position;
                        if let Some(hole) = self.hole_navigator.next_hole(cursor_pos) {
                            // Clone hole to avoid borrow issues
                            let hole_clone = hole.clone();

                            // Jump to hole position
                            let target = Position {
                                line: hole_clone.line,
                                column: hole_clone.column,
                            };
                            buffer.cursor.position = target;

                            // Generate and show suggestions
                            let suggestions = self.hole_navigator.generate_suggestions(&hole_clone);
                            self.status = format!(
                                "Hole: {} - {} ({} suggestions)",
                                hole_clone.kind.icon(),
                                hole_clone.name,
                                suggestions.len()
                            );
                        } else {
                            self.status = "No holes found".to_string();
                        }
                    }

                    // Previous hole (Ctrl+Shift+N)
                    (KeyCode::Char('N'), true) => {
                        let cursor_pos = buffer.cursor.position;
                        if let Some(hole) = self.hole_navigator.previous_hole(cursor_pos) {
                            // Clone to avoid holding reference while calling generate_suggestions
                            let hole_clone = hole.clone();

                            // Jump to hole position
                            let target = Position {
                                line: hole_clone.line,
                                column: hole_clone.column,
                            };
                            buffer.cursor.position = target;

                            // Generate and show suggestions
                            let suggestions = self.hole_navigator.generate_suggestions(&hole_clone);
                            self.status = format!(
                                "Hole: {} - {} ({} suggestions)",
                                hole_clone.kind.icon(),
                                hole_clone.name,
                                suggestions.len()
                            );
                        } else {
                            self.status = "No holes found".to_string();
                        }
                    }

                    // Show holes list (Ctrl+H)
                    (KeyCode::Char('h'), true) => {
                        let hole_count = self.hole_navigator.hole_count();
                        let unresolved = self.hole_navigator.unresolved_holes().len();
                        self.status = format!(
                            "Holes: {} total, {} unresolved | Use Ctrl+N/Ctrl+Shift+N to navigate",
                            hole_count, unresolved
                        );

                        // If there are holes, show the first one's details
                        if let Some(hole) = self.hole_navigator.go_to_hole(0) {
                            // Clone to avoid holding reference while calling generate_suggestions
                            let hole_clone = hole.clone();
                            let suggestions = self.hole_navigator.generate_suggestions(&hole_clone);
                            eprintln!(
                                "{}",
                                super::holes::format_hole_with_suggestions(
                                    &hole_clone,
                                    &suggestions
                                )
                            );
                        }
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
            let bottom_panels_visible =
                self.diagnostics_panel.is_visible() || self.proposals_panel.is_visible();
            let right_panels_visible = self.memory_panel.is_visible()
                || self.agent_status_panel.is_visible()
                || self.attribution_panel.is_visible();

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
                        frame.render_stateful_widget(
                            panel_widget,
                            right_chunks[chunk_idx],
                            &mut self.memory_panel,
                        );
                        chunk_idx += 1;
                    }

                    if self.agent_status_panel.is_visible() {
                        let panel_widget = AgentStatusWidget::new(&self.agents);
                        frame.render_stateful_widget(
                            panel_widget,
                            right_chunks[chunk_idx],
                            &mut self.agent_status_panel,
                        );
                        chunk_idx += 1;
                    }

                    if self.attribution_panel.is_visible() {
                        let panel_widget = AttributionPanel::new(&self.attributions);
                        frame.render_stateful_widget(
                            panel_widget,
                            right_chunks[chunk_idx],
                            &mut self.attribution_panel,
                        );
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
                        frame.render_stateful_widget(
                            panel_widget,
                            bottom_chunks[chunk_idx],
                            &mut self.diagnostics_panel,
                        );
                        chunk_idx += 1;
                    }

                    if self.proposals_panel.is_visible() {
                        let selected_proposal = self
                            .proposals_panel
                            .selected()
                            .and_then(|idx| self.proposals.get(idx));
                        let panel_widget = ProposalsPanel::new(&self.proposals, selected_proposal);
                        frame.render_stateful_widget(
                            panel_widget,
                            bottom_chunks[chunk_idx],
                            &mut self.proposals_panel,
                        );
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

            // Add hole navigation info
            let hole_info = if self.hole_navigator.hole_count() > 0 {
                let unresolved = self.hole_navigator.unresolved_holes().len();
                format!(
                    "| Holes: {}/{} unresolved ",
                    unresolved,
                    self.hole_navigator.hole_count()
                )
            } else {
                String::new()
            };

            let info_text = format!("{} | {}{}{}", cursor_pos, lang, semantic_info, hole_info);

            let info_widget = Paragraph::new(info_text).style(Style::default().fg(Color::DarkGray));
            let info_bar_index = if bottom_panels_visible { 3 } else { 2 };
            frame.render_widget(info_widget, main_chunks[info_bar_index]);

            // Render completion popup (overlay on top of everything)
            if self.completion_popup.is_visible() {
                let cursor_pos = buffer.cursor.position;
                let popup_area = self.completion_popup.popup_area(
                    size,
                    cursor_pos.line as u16,
                    cursor_pos.column as u16,
                );
                let buf = frame.buffer_mut();
                self.completion_popup.render(popup_area, buf);
            }
        })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ics::proposals::ProposalStatus,
        launcher::agents::AgentRole,
        orchestration::{AgentRegistry, ProposalQueue},
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
        let storage = crate::storage::test_utils::create_test_storage_with_embedded_schema()
            .await
            .expect("Failed to create storage");

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
        registry
            .register(
                "test-optimizer".to_string(),
                "Optimizer".to_string(),
                AgentRole::Optimizer,
            )
            .await;
        registry
            .register(
                "test-reviewer".to_string(),
                "Reviewer".to_string(),
                AgentRole::Reviewer,
            )
            .await;

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
        buffer
            .insert_at_cursor("\nAdded line")
            .expect("Failed to insert");

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
        buffer
            .insert_at_cursor("Line 1\n")
            .expect("Failed to insert");
        buffer
            .insert_at_cursor("Line 2\n")
            .expect("Failed to insert");

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
        registry
            .register("test".to_string(), "Test".to_string(), AgentRole::Optimizer)
            .await;

        let queue = ProposalQueue::new();
        queue
            .send(ChangeProposal {
                id: "test".to_string(),
                agent: "Test".to_string(),
                description: "Test".to_string(),
                original: "old".to_string(),
                proposed: "new".to_string(),
                line_range: (1, 2),
                created_at: SystemTime::now(),
                status: ProposalStatus::Pending,
                rationale: "Test".to_string(),
            })
            .expect("Failed to send");

        let config2 = IcsConfig::default();
        let mut app2 = IcsApp::new(config2, storage2, Some(registry), Some(queue));

        app2.load_agents().await;
        app2.poll_proposals().await;

        assert_eq!(app2.agents().len(), 1);
        assert_eq!(app2.proposals().len(), 1);
    }

    #[tokio::test]
    async fn test_memory_sorting_by_importance() {
        let storage = crate::storage::test_utils::create_test_storage_with_embedded_schema()
            .await
            .expect("Failed to create storage");

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
