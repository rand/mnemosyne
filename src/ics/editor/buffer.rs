//! Text buffer with rope data structure
//!
//! Efficient text storage and manipulation using ropey

use super::{CursorState, Language};
use ropey::Rope;
use std::collections::VecDeque;
use std::path::PathBuf;

/// Buffer identifier
pub type BufferId = usize;

/// Edit operation for undo/redo
#[derive(Debug, Clone)]
pub struct Edit {
    /// Position where edit occurred
    pub position: usize,
    /// Text that was inserted (empty if deletion)
    pub inserted: String,
    /// Text that was deleted (empty if insertion)
    pub deleted: String,
}

/// Text buffer with undo/redo support
pub struct TextBuffer {
    /// Buffer ID
    pub id: BufferId,

    /// Text content (rope for efficient editing)
    pub content: Rope,

    /// File path (if loaded from disk)
    pub path: Option<PathBuf>,

    /// Language for syntax highlighting
    pub language: Language,

    /// Whether buffer has unsaved changes
    pub dirty: bool,

    /// Cursor state
    pub cursor: CursorState,

    /// Undo stack
    undo_stack: VecDeque<Edit>,

    /// Redo stack
    redo_stack: VecDeque<Edit>,
}

impl TextBuffer {
    /// Create new text buffer
    pub fn new(id: BufferId, path: Option<PathBuf>) -> Self {
        let language = path
            .as_ref()
            .and_then(|p| Language::from_path(p))
            .unwrap_or(Language::PlainText);

        Self {
            id,
            content: Rope::new(),
            path,
            language,
            dirty: false,
            cursor: CursorState::default(),
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
        }
    }

    /// Insert text at cursor position
    pub fn insert(&mut self, text: &str) {
        let pos = self.cursor_to_char_idx();

        // Record edit for undo
        self.undo_stack.push_back(Edit {
            position: pos,
            inserted: text.to_string(),
            deleted: String::new(),
        });
        self.redo_stack.clear();

        // Insert text
        self.content.insert(pos, text);
        self.dirty = true;

        // Move cursor forward
        self.cursor.position.column += text.len();
    }

    /// Delete character at cursor
    pub fn delete(&mut self) {
        let pos = self.cursor_to_char_idx();
        if pos >= self.content.len_chars() {
            return;
        }

        // Get character to delete
        let ch = self.content.char(pos);

        // Record edit for undo
        self.undo_stack.push_back(Edit {
            position: pos,
            inserted: String::new(),
            deleted: ch.to_string(),
        });
        self.redo_stack.clear();

        // Delete character
        self.content.remove(pos..pos + 1);
        self.dirty = true;
    }

    /// Undo last edit
    pub fn undo(&mut self) -> Option<Edit> {
        let edit = self.undo_stack.pop_back()?;

        // Reverse the edit
        if !edit.inserted.is_empty() {
            // Was insertion, remove it
            self.content.remove(edit.position..edit.position + edit.inserted.len());
        }
        if !edit.deleted.is_empty() {
            // Was deletion, reinsert it
            self.content.insert(edit.position, &edit.deleted);
        }

        self.redo_stack.push_back(edit.clone());
        self.dirty = true;

        Some(edit)
    }

    /// Redo last undone edit
    pub fn redo(&mut self) -> Option<Edit> {
        let edit = self.redo_stack.pop_back()?;

        // Replay the edit
        if !edit.deleted.is_empty() {
            // Was deletion, delete again
            self.content.remove(edit.position..edit.position + edit.deleted.len());
        }
        if !edit.inserted.is_empty() {
            // Was insertion, insert again
            self.content.insert(edit.position, &edit.inserted);
        }

        self.undo_stack.push_back(edit.clone());
        self.dirty = true;

        Some(edit)
    }

    /// Convert cursor position to character index
    fn cursor_to_char_idx(&self) -> usize {
        let line_idx = self.content.line_to_char(self.cursor.position.line.min(self.content.len_lines() - 1));
        line_idx + self.cursor.position.column
    }

    /// Get line count
    pub fn line_count(&self) -> usize {
        self.content.len_lines()
    }

    /// Get line by index
    pub fn line(&self, idx: usize) -> Option<String> {
        if idx >= self.content.len_lines() {
            return None;
        }
        Some(self.content.line(idx).to_string())
    }
}
