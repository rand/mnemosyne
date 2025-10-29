//! ICS fixture for integration tests

use mnemosyne_core::ics::{IcsEditor, SemanticAnalyzer, SemanticAnalysis};
use mnemosyne_core::ics::editor::BufferId;
use mnemosyne_core::types::MemoryNote;
use std::sync::Arc;

/// ICS test fixture with editor and analyzer
pub struct IcsFixture {
    /// ICS editor instance
    pub editor: IcsEditor,
    /// Semantic analyzer
    pub analyzer: SemanticAnalyzer,
    /// Test memories for memory panel
    pub memories: Vec<MemoryNote>,
}

impl IcsFixture {
    /// Create new ICS fixture
    pub fn new() -> Self {
        Self {
            editor: IcsEditor::new(),
            analyzer: SemanticAnalyzer::new(),
            memories: Vec::new(),
        }
    }

    /// Create ICS fixture with pre-loaded memories
    pub fn with_memories(memories: Vec<MemoryNote>) -> Self {
        Self {
            editor: IcsEditor::new(),
            analyzer: SemanticAnalyzer::new(),
            memories,
        }
    }

    /// Add text to active buffer
    pub fn add_text(&mut self, text: &str) {
        self.editor.active_buffer_mut().insert(text);
    }

    /// Get buffer content
    pub fn buffer_content(&self) -> String {
        self.editor.active_buffer().content.to_string()
    }

    /// Trigger semantic analysis
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

    /// Create new buffer
    pub fn create_buffer(&mut self) -> BufferId {
        self.editor.new_buffer(None)
    }

    /// Switch to buffer
    pub fn switch_buffer(&mut self, id: BufferId) {
        self.editor.set_active_buffer(id);
    }

    /// Set memories for panel
    pub fn set_memories(&mut self, memories: Vec<MemoryNote>) {
        self.memories = memories;
    }

    /// Search memories (simple filter)
    pub fn search_memories(&self, query: &str) -> Vec<&MemoryNote> {
        self.memories
            .iter()
            .filter(|m| {
                m.content.contains(query)
                    || m.keywords.iter().any(|k| k.contains(query))
                    || m.tags.iter().any(|t| t.contains(query))
            })
            .collect()
    }
}

impl Default for IcsFixture {
    fn default() -> Self {
        Self::new()
    }
}
