//! Test helpers for ICS E2E tests

pub mod actors;
pub mod assertions;
pub mod fixtures;

use mnemosyne_core::ics::editor::{BufferId, Diagnostic, IcsEditor};
use mnemosyne_core::ics::{ChangeProposal, SemanticAnalysis, SemanticAnalyzer};
use mnemosyne_core::types::MemoryNote;

/// Test context that holds all necessary components for E2E tests
pub struct TestContext {
    /// ICS editor instance
    pub editor: IcsEditor,
    /// Semantic analyzer
    pub analyzer: SemanticAnalyzer,
    /// Mock agents
    pub agents: Vec<actors::MockAgent>,
    /// Test memories
    pub memories: Vec<MemoryNote>,
    /// Test proposals
    pub proposals: Vec<ChangeProposal>,
    /// Test diagnostics
    pub diagnostics: Vec<Diagnostic>,
}

impl TestContext {
    /// Create new test context with defaults
    pub fn new() -> Self {
        Self {
            editor: IcsEditor::new(),
            analyzer: SemanticAnalyzer::new(),
            agents: Vec::new(),
            memories: Vec::new(),
            proposals: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    /// Create test context with sample data
    pub fn with_fixtures() -> Self {
        Self {
            editor: IcsEditor::new(),
            analyzer: SemanticAnalyzer::new(),
            agents: actors::create_mock_agents(),
            memories: fixtures::sample_memories(),
            proposals: fixtures::sample_proposals(),
            diagnostics: Vec::new(),
        }
    }

    /// Add text to active buffer
    pub fn add_text(&mut self, text: &str) {
        let buffer = self.editor.active_buffer_mut();
        buffer.insert(text);
    }

    /// Get active buffer content as string
    pub fn buffer_content(&self) -> String {
        self.editor.active_buffer().content.to_string()
    }

    /// Trigger semantic analysis and wait for result
    pub async fn analyze(&mut self) -> anyhow::Result<SemanticAnalysis> {
        let text = self.buffer_content();
        self.analyzer.analyze(text)?;

        // Poll for result with timeout
        for _ in 0..100 {
            if let Some(analysis) = self.analyzer.try_recv() {
                return Ok(analysis);
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }

        anyhow::bail!("Analysis timed out")
    }

    /// Create new buffer and return its ID
    pub fn create_buffer(&mut self) -> BufferId {
        self.editor.new_buffer(None)
    }

    /// Switch to buffer by ID
    pub fn switch_buffer(&mut self, id: BufferId) {
        self.editor.set_active_buffer(id);
    }
}

impl Default for TestContext {
    fn default() -> Self {
        Self::new()
    }
}
