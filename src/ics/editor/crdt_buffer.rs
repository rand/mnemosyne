//! CRDT-based text buffer using Automerge
//!
//! Provides conflict-free collaborative editing for multi-agent context engineering

use anyhow::{Context as AnyhowContext, Result};
use automerge::{transaction::Transactable, AutoCommit, ObjType, ReadDoc};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;

use super::{CursorState, Language, Movement};

/// Buffer identifier
pub type BufferId = usize;

/// Actor in the collaborative editing session
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Actor {
    /// Human user
    Human,
    /// Orchestrator agent
    Orchestrator,
    /// Optimizer agent
    Optimizer,
    /// Reviewer agent
    Reviewer,
    /// Executor agent
    Executor,
    /// Sub-agent spawned by executor
    SubAgent,
}

impl Actor {
    /// Get display name for actor
    pub fn display_name(&self) -> &'static str {
        match self {
            Actor::Human => "You",
            Actor::Orchestrator => "Orchestrator",
            Actor::Optimizer => "Optimizer",
            Actor::Reviewer => "Reviewer",
            Actor::Executor => "Executor",
            Actor::SubAgent => "Agent",
        }
    }

    /// Get color for actor attribution (RGB)
    pub fn color(&self) -> (u8, u8, u8) {
        match self {
            Actor::Human => (255, 255, 255),        // White
            Actor::Orchestrator => (255, 200, 100), // Orange
            Actor::Optimizer => (100, 200, 255),    // Blue
            Actor::Reviewer => (255, 100, 100),     // Red
            Actor::Executor => (100, 255, 100),     // Green
            Actor::SubAgent => (200, 100, 255),     // Purple
        }
    }
}

/// Change attribution for a text range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attribution {
    /// Actor who made the change
    pub actor: Actor,
    /// When the change was made
    pub timestamp: DateTime<Utc>,
    /// Character range (start, end)
    pub range: (usize, usize),
}

/// CRDT-based text buffer
pub struct CrdtBuffer {
    /// Buffer ID
    pub id: BufferId,

    /// Automerge document
    doc: AutoCommit,

    /// Text object ID in the document
    text_id: automerge::ObjId,

    /// Local actor (who is editing from this instance)
    local_actor: Actor,

    /// File path (if loaded from disk)
    pub path: Option<PathBuf>,

    /// Language for syntax highlighting
    pub language: Language,

    /// Whether buffer has unsaved changes
    pub dirty: bool,

    /// Cursor state
    pub cursor: CursorState,

    /// Change attributions
    attributions: Vec<Attribution>,

    /// Undo/redo stack (document snapshots before operations)
    /// We store full document state to enable reliable undo/redo
    /// Limited to 100 snapshots to manage memory usage
    undo_stack: VecDeque<Vec<u8>>,
    redo_stack: VecDeque<Vec<u8>>,
}

impl CrdtBuffer {
    /// Create new CRDT buffer
    pub fn new(id: BufferId, actor: Actor, path: Option<PathBuf>) -> Result<Self> {
        let mut doc = AutoCommit::new();

        // Create a text object in the document
        let text_id = doc
            .put_object(automerge::ROOT, "text", ObjType::Text)
            .context("Failed to create text object")?;

        // Detect language from path
        let language = path
            .as_ref()
            .and_then(|p| Language::from_path(p))
            .unwrap_or(Language::PlainText);

        Ok(Self {
            id,
            doc,
            text_id,
            local_actor: actor,
            path,
            language,
            dirty: false,
            cursor: CursorState::default(),
            attributions: Vec::new(),
            undo_stack: VecDeque::new(),
            redo_stack: VecDeque::new(),
        })
    }

    /// Load content into buffer
    pub fn load_content(&mut self, content: &str) -> Result<()> {
        // Clear existing text
        let len = self.text_len()?;
        if len > 0 {
            self.doc.splice_text(&self.text_id, 0, len as isize, "")?;
        }

        // Insert new content
        self.doc.splice_text(&self.text_id, 0, 0, content)?;

        // Mark as clean (just loaded from disk)
        self.dirty = false;

        // Add attribution for entire content
        self.attributions.push(Attribution {
            actor: self.local_actor,
            timestamp: Utc::now(),
            range: (0, content.len()),
        });

        Ok(())
    }

    /// Insert text at position
    pub fn insert(&mut self, pos: usize, text: &str) -> Result<()> {
        // Save current document state for undo (snapshot BEFORE operation)
        let snapshot = self.doc.save();
        self.undo_stack.push_back(snapshot);
        if self.undo_stack.len() > 100 {
            self.undo_stack.pop_front();
        }
        // Clear redo stack since we're making a new change
        self.redo_stack.clear();

        // Insert text
        self.doc.splice_text(&self.text_id, pos, 0, text)?;
        self.dirty = true;

        // Add attribution
        self.attributions.push(Attribution {
            actor: self.local_actor,
            timestamp: Utc::now(),
            range: (pos, pos + text.len()),
        });

        Ok(())
    }

    /// Delete text at position
    pub fn delete(&mut self, pos: usize, len: usize) -> Result<()> {
        // Check bounds
        let text_len = self.text_len()?;
        if pos >= text_len {
            return Ok(());
        }

        let delete_len = len.min(text_len - pos);

        // Save current document state for undo (snapshot BEFORE operation)
        let snapshot = self.doc.save();
        self.undo_stack.push_back(snapshot);
        if self.undo_stack.len() > 100 {
            self.undo_stack.pop_front();
        }
        // Clear redo stack since we're making a new change
        self.redo_stack.clear();

        // Delete text
        self.doc
            .splice_text(&self.text_id, pos, delete_len as isize, "")?;
        self.dirty = true;

        // Remove attributions in deleted range
        self.attributions.retain(|attr| {
            let (start, end) = attr.range;
            !(start >= pos && end <= pos + delete_len)
        });

        // Adjust remaining attributions
        for attr in &mut self.attributions {
            let (start, end) = attr.range;
            if start > pos {
                attr.range.0 = start.saturating_sub(delete_len);
                attr.range.1 = end.saturating_sub(delete_len);
            }
        }

        Ok(())
    }

    /// Get text content
    pub fn text(&self) -> Result<String> {
        let text = self.doc.text(&self.text_id)?;
        Ok(text)
    }

    /// Get text length
    pub fn text_len(&self) -> Result<usize> {
        Ok(self.doc.length(&self.text_id))
    }

    /// Merge changes from another buffer/agent
    pub fn merge_changes(&mut self, changes: &[u8]) -> Result<()> {
        self.doc.load_incremental(changes)?;
        self.dirty = true;
        Ok(())
    }

    /// Get changes to send to other buffers/agents
    pub fn get_changes(&mut self) -> Result<Vec<u8>> {
        if let Some(change) = self.doc.get_last_local_change() {
            Ok(change.raw_bytes().to_vec())
        } else {
            Ok(Vec::new())
        }
    }

    /// Get full document state (for sync)
    pub fn save_state(&mut self) -> Vec<u8> {
        self.doc.save()
    }

    /// Load document state (for sync)
    pub fn load_state(&mut self, state: &[u8]) -> Result<()> {
        // Create new document from saved state
        let new_doc = AutoCommit::load(state)?;
        self.doc = new_doc;

        // Re-get text_id from the loaded document
        let result = self.doc.get(automerge::ROOT, "text")?;
        if let Some((_, obj_id)) = result {
            self.text_id = obj_id;
        } else {
            return Err(anyhow::anyhow!("Text object not found in loaded state"));
        }

        Ok(())
    }

    /// Undo last operation
    ///
    /// Restores the document to the state before the last operation.
    /// Returns true if undo was performed, false if undo stack is empty.
    pub fn undo(&mut self) -> Result<bool> {
        // Check if there's anything to undo
        if self.undo_stack.is_empty() {
            return Ok(false);
        }

        // Save current state to redo stack
        let current_state = self.doc.save();
        self.redo_stack.push_back(current_state);
        if self.redo_stack.len() > 100 {
            self.redo_stack.pop_front();
        }

        // Restore previous state from undo stack
        if let Some(previous_state) = self.undo_stack.pop_back() {
            self.load_state(&previous_state)?;
            self.dirty = true;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Redo last undone operation
    ///
    /// Re-applies an operation that was undone.
    /// Returns true if redo was performed, false if redo stack is empty.
    pub fn redo(&mut self) -> Result<bool> {
        // Check if there's anything to redo
        if self.redo_stack.is_empty() {
            return Ok(false);
        }

        // Save current state to undo stack
        let current_state = self.doc.save();
        self.undo_stack.push_back(current_state);
        if self.undo_stack.len() > 100 {
            self.undo_stack.pop_front();
        }

        // Restore next state from redo stack
        if let Some(next_state) = self.redo_stack.pop_back() {
            self.load_state(&next_state)?;
            self.dirty = true;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get line count
    pub fn line_count(&self) -> Result<usize> {
        let text = self.text()?;
        Ok(text.lines().count().max(1))
    }

    /// Get line by index
    pub fn line(&self, idx: usize) -> Result<Option<String>> {
        let text = self.text()?;
        Ok(text.lines().nth(idx).map(String::from))
    }

    /// Get attributions for displaying in gutter
    pub fn attributions(&self) -> &[Attribution] {
        &self.attributions
    }

    /// Get attribution for a specific position
    pub fn attribution_at(&self, pos: usize) -> Option<&Attribution> {
        self.attributions
            .iter()
            .rev()
            .find(|attr| pos >= attr.range.0 && pos < attr.range.1)
    }

    /// Get all attributions
    pub fn get_attributions(&self) -> &[Attribution] {
        &self.attributions
    }

    // === File I/O Methods ===

    /// Load file from disk
    pub fn load_file(&mut self, path: PathBuf) -> Result<()> {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        self.load_content(&content)?;
        self.path = Some(path.clone());
        self.language = Language::from_path(&path).unwrap_or(Language::PlainText);
        self.dirty = false;

        Ok(())
    }

    /// Save buffer to file
    pub fn save_file(&mut self) -> Result<()> {
        let path = self
            .path
            .as_ref()
            .context("Cannot save buffer without path")?;

        let content = self.text()?;
        fs::write(path, content)
            .with_context(|| format!("Failed to write file: {}", path.display()))?;

        self.dirty = false;
        Ok(())
    }

    // === Cursor-Aware Editing Methods ===

    /// Convert cursor position to character index
    fn cursor_to_char_idx(&self) -> Result<usize> {
        let text = self.text()?;
        let mut char_idx = 0;
        let mut line = 0;
        let mut col = 0;

        for ch in text.chars() {
            if line == self.cursor.position.line && col == self.cursor.position.column {
                return Ok(char_idx);
            }

            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }

            char_idx += 1;
        }

        Ok(char_idx)
    }

    /// Insert text at cursor position (wrapper around insert)
    pub fn insert_at_cursor(&mut self, text: &str) -> Result<()> {
        let pos = self.cursor_to_char_idx()?;
        self.insert(pos, text)?;

        // Move cursor forward by text length
        self.cursor.position.column += text.chars().count();

        Ok(())
    }

    /// Delete character at cursor position (wrapper around delete)
    pub fn delete_at_cursor(&mut self) -> Result<()> {
        let pos = self.cursor_to_char_idx()?;
        let len = self.text_len()?;

        if pos >= len {
            return Ok(()); // Nothing to delete
        }

        self.delete(pos, 1)?;

        Ok(())
    }

    /// Move cursor
    pub fn move_cursor(&mut self, movement: Movement) -> Result<()> {
        let text = self.text()?;
        let lines: Vec<&str> = text.lines().collect();
        let line_count = lines.len().max(1);

        match movement {
            Movement::Up => {
                if self.cursor.position.line > 0 {
                    self.cursor.position.line -= 1;
                    // Clamp column to line length
                    if let Some(line) = lines.get(self.cursor.position.line) {
                        self.cursor.position.column = self.cursor.position.column.min(line.len());
                    }
                }
            }
            Movement::Down => {
                if self.cursor.position.line < line_count.saturating_sub(1) {
                    self.cursor.position.line += 1;
                    // Clamp column to line length
                    if let Some(line) = lines.get(self.cursor.position.line) {
                        self.cursor.position.column = self.cursor.position.column.min(line.len());
                    }
                }
            }
            Movement::Left => {
                if self.cursor.position.column > 0 {
                    self.cursor.position.column -= 1;
                } else if self.cursor.position.line > 0 {
                    // Move to end of previous line
                    self.cursor.position.line -= 1;
                    if let Some(line) = lines.get(self.cursor.position.line) {
                        self.cursor.position.column = line.len();
                    }
                }
            }
            Movement::Right => {
                if let Some(line) = lines.get(self.cursor.position.line) {
                    if self.cursor.position.column < line.len() {
                        self.cursor.position.column += 1;
                    } else if self.cursor.position.line < line_count.saturating_sub(1) {
                        // Move to start of next line
                        self.cursor.position.line += 1;
                        self.cursor.position.column = 0;
                    }
                }
            }
            Movement::LineStart => {
                self.cursor.position.column = 0;
            }
            Movement::LineEnd => {
                if let Some(line) = lines.get(self.cursor.position.line) {
                    self.cursor.position.column = line.len();
                }
            }
            Movement::WordLeft => {
                // Move left to the start of the current or previous word
                if let Some(line) = lines.get(self.cursor.position.line) {
                    let chars: Vec<char> = line.chars().collect();
                    let mut pos = self.cursor.position.column;

                    // Skip current whitespace
                    while pos > 0
                        && chars
                            .get(pos.saturating_sub(1))
                            .is_some_and(|c| c.is_whitespace())
                    {
                        pos = pos.saturating_sub(1);
                    }

                    // Skip to start of word
                    while pos > 0
                        && chars
                            .get(pos.saturating_sub(1))
                            .is_some_and(|c| !c.is_whitespace())
                    {
                        pos = pos.saturating_sub(1);
                    }

                    self.cursor.position.column = pos;
                } else if self.cursor.position.line > 0 {
                    // Move to end of previous line
                    self.cursor.position.line -= 1;
                    if let Some(line) = lines.get(self.cursor.position.line) {
                        self.cursor.position.column = line.len();
                    }
                }
            }
            Movement::WordRight => {
                // Move right to the start of the next word
                if let Some(line) = lines.get(self.cursor.position.line) {
                    let chars: Vec<char> = line.chars().collect();
                    let mut pos = self.cursor.position.column;

                    // Skip current word
                    while pos < chars.len() && !chars[pos].is_whitespace() {
                        pos += 1;
                    }

                    // Skip whitespace
                    while pos < chars.len() && chars[pos].is_whitespace() {
                        pos += 1;
                    }

                    if pos >= chars.len()
                        && self.cursor.position.line < line_count.saturating_sub(1)
                    {
                        // Move to start of next line
                        self.cursor.position.line += 1;
                        self.cursor.position.column = 0;
                    } else {
                        self.cursor.position.column = pos;
                    }
                }
            }
            Movement::PageUp => {
                // Move up approximately one page (20 lines)
                let page_size = 20;
                if self.cursor.position.line >= page_size {
                    self.cursor.position.line -= page_size;
                } else {
                    self.cursor.position.line = 0;
                }
                // Clamp column to line length
                if let Some(line) = lines.get(self.cursor.position.line) {
                    self.cursor.position.column = self.cursor.position.column.min(line.len());
                }
            }
            Movement::PageDown => {
                // Move down approximately one page (20 lines)
                let page_size = 20;
                let new_line = self.cursor.position.line + page_size;
                if new_line < line_count {
                    self.cursor.position.line = new_line;
                } else {
                    self.cursor.position.line = line_count.saturating_sub(1);
                }
                // Clamp column to line length
                if let Some(line) = lines.get(self.cursor.position.line) {
                    self.cursor.position.column = self.cursor.position.column.min(line.len());
                }
            }
            Movement::BufferStart => {
                self.cursor.position.line = 0;
                self.cursor.position.column = 0;
            }
            Movement::BufferEnd => {
                self.cursor.position.line = line_count.saturating_sub(1);
                if let Some(line) = lines.get(self.cursor.position.line) {
                    self.cursor.position.column = line.len();
                }
            }
            Movement::WordEnd => {
                // Move to the end of the current or next word (Helix-style)
                if let Some(line) = lines.get(self.cursor.position.line) {
                    let chars: Vec<char> = line.chars().collect();
                    let mut pos = self.cursor.position.column;

                    // If at end of word, move to next word
                    if pos < chars.len() && !chars[pos].is_whitespace() {
                        pos += 1;
                    }

                    // Skip whitespace
                    while pos < chars.len() && chars[pos].is_whitespace() {
                        pos += 1;
                    }

                    // Move to end of word
                    while pos < chars.len() && !chars[pos].is_whitespace() {
                        pos += 1;
                    }

                    pos = pos.saturating_sub(1);

                    if pos >= chars.len()
                        && self.cursor.position.line < line_count.saturating_sub(1)
                    {
                        // Move to start of next line
                        self.cursor.position.line += 1;
                        self.cursor.position.column = 0;
                    } else {
                        self.cursor.position.column = pos.min(chars.len().saturating_sub(1));
                    }
                }
            }
            Movement::FindChar(ch) => {
                // Find next occurrence of character on current line
                if let Some(line) = lines.get(self.cursor.position.line) {
                    let chars: Vec<char> = line.chars().collect();
                    for (i, &c) in chars
                        .iter()
                        .enumerate()
                        .skip(self.cursor.position.column + 1)
                    {
                        if c == ch {
                            self.cursor.position.column = i;
                            break;
                        }
                    }
                }
            }
            Movement::FindCharReverse(ch) => {
                // Find previous occurrence of character on current line
                if let Some(line) = lines.get(self.cursor.position.line) {
                    let chars: Vec<char> = line.chars().collect();
                    for i in (0..self.cursor.position.column).rev() {
                        if chars.get(i).copied() == Some(ch) {
                            self.cursor.position.column = i;
                            break;
                        }
                    }
                }
            }
            Movement::TillChar(ch) => {
                // Move till (before) character on current line
                if let Some(line) = lines.get(self.cursor.position.line) {
                    let chars: Vec<char> = line.chars().collect();
                    for (i, &c) in chars
                        .iter()
                        .enumerate()
                        .skip(self.cursor.position.column + 1)
                    {
                        if c == ch {
                            self.cursor.position.column = i.saturating_sub(1);
                            break;
                        }
                    }
                }
            }
            Movement::TillCharReverse(ch) => {
                // Move till (after) character reverse on current line
                if let Some(line) = lines.get(self.cursor.position.line) {
                    let chars: Vec<char> = line.chars().collect();
                    for i in (0..self.cursor.position.column).rev() {
                        if chars.get(i).copied() == Some(ch) {
                            self.cursor.position.column =
                                (i + 1).min(chars.len().saturating_sub(1));
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ics::editor::Position;

    #[test]
    fn test_crdt_buffer_creation() {
        let buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();
        assert_eq!(buffer.id, 0);
        assert_eq!(buffer.text_len().unwrap(), 0);
    }

    #[test]
    fn test_insert_text() {
        let mut buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();
        buffer.insert(0, "Hello, world!").unwrap();
        assert_eq!(buffer.text().unwrap(), "Hello, world!");
        assert_eq!(buffer.text_len().unwrap(), 13);
        assert!(buffer.dirty);
    }

    #[test]
    fn test_delete_text() {
        let mut buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();
        buffer.insert(0, "Hello, world!").unwrap();
        buffer.delete(7, 6).unwrap(); // Delete "world!"
        assert_eq!(buffer.text().unwrap(), "Hello, ");
    }

    #[test]
    fn test_concurrent_edits() {
        // Create two buffers
        let mut buffer1 = CrdtBuffer::new(0, Actor::Human, None).unwrap();
        let mut buffer2 = CrdtBuffer::new(0, Actor::Optimizer, None).unwrap();

        // Load same initial state
        buffer1.insert(0, "Initial text").unwrap();
        let state = buffer1.save_state();
        buffer2.load_state(&state).unwrap();

        // Make concurrent edits
        buffer1.insert(12, " from human").unwrap();
        buffer2.insert(12, " from agent").unwrap();

        // Merge changes
        let changes1 = buffer1.get_changes().unwrap();
        let changes2 = buffer2.get_changes().unwrap();

        buffer1.merge_changes(&changes2).unwrap();
        buffer2.merge_changes(&changes1).unwrap();

        // Both buffers should have both edits (order may vary)
        let text1 = buffer1.text().unwrap();
        let text2 = buffer2.text().unwrap();
        assert_eq!(text1, text2);
        assert!(text1.contains("from human"));
        assert!(text1.contains("from agent"));
    }

    #[test]
    fn test_attribution() {
        let mut buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();
        buffer.insert(0, "Hello").unwrap();

        let attr = buffer.attribution_at(2).unwrap();
        assert_eq!(attr.actor, Actor::Human);
        assert_eq!(attr.range, (0, 5));
    }

    #[test]
    fn test_word_movements() {
        let mut buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();
        buffer.insert(0, "hello world test").unwrap();
        buffer.cursor.position = Position { line: 0, column: 0 };

        // WordRight
        buffer.move_cursor(Movement::WordRight).unwrap();
        assert_eq!(buffer.cursor.position.column, 6); // After "hello "

        buffer.move_cursor(Movement::WordRight).unwrap();
        assert_eq!(buffer.cursor.position.column, 12); // After "world "

        // WordLeft
        buffer.move_cursor(Movement::WordLeft).unwrap();
        assert_eq!(buffer.cursor.position.column, 6); // Start of "world"

        buffer.move_cursor(Movement::WordLeft).unwrap();
        assert_eq!(buffer.cursor.position.column, 0); // Start of "hello"
    }

    #[test]
    fn test_word_end_movement() {
        let mut buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();
        buffer.insert(0, "hello world test").unwrap();
        buffer.cursor.position = Position { line: 0, column: 0 };

        // WordEnd
        buffer.move_cursor(Movement::WordEnd).unwrap();
        assert_eq!(buffer.cursor.position.column, 4); // End of "hello"

        buffer.move_cursor(Movement::WordEnd).unwrap();
        assert_eq!(buffer.cursor.position.column, 10); // End of "world"
    }

    #[test]
    fn test_page_movements() {
        let mut buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();
        let mut text = String::new();
        for i in 0..50 {
            text.push_str(&format!("Line {}\n", i));
        }
        buffer.insert(0, &text).unwrap();
        buffer.cursor.position = Position {
            line: 25,
            column: 0,
        };

        // PageUp
        buffer.move_cursor(Movement::PageUp).unwrap();
        assert_eq!(buffer.cursor.position.line, 5); // 25 - 20

        // PageDown
        buffer.move_cursor(Movement::PageDown).unwrap();
        assert_eq!(buffer.cursor.position.line, 25); // 5 + 20
    }

    #[test]
    fn test_buffer_start_end() {
        let mut buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();
        buffer.insert(0, "Line 1\nLine 2\nLine 3").unwrap();
        buffer.cursor.position = Position { line: 1, column: 3 };

        // BufferStart
        buffer.move_cursor(Movement::BufferStart).unwrap();
        assert_eq!(buffer.cursor.position.line, 0);
        assert_eq!(buffer.cursor.position.column, 0);

        // BufferEnd
        buffer.move_cursor(Movement::BufferEnd).unwrap();
        assert_eq!(buffer.cursor.position.line, 2);
        assert_eq!(buffer.cursor.position.column, 6); // End of "Line 3"
    }

    #[test]
    fn test_find_char_movement() {
        let mut buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();
        buffer.insert(0, "hello world test").unwrap();
        buffer.cursor.position = Position { line: 0, column: 0 };

        // FindChar
        buffer.move_cursor(Movement::FindChar('w')).unwrap();
        assert_eq!(buffer.cursor.position.column, 6); // 'w' in "world"

        // FindChar again
        buffer.move_cursor(Movement::FindChar('t')).unwrap();
        assert_eq!(buffer.cursor.position.column, 12); // 't' in "test"

        // FindCharReverse
        buffer.move_cursor(Movement::FindCharReverse('w')).unwrap();
        assert_eq!(buffer.cursor.position.column, 6); // Back to 'w'
    }

    #[test]
    fn test_till_char_movement() {
        let mut buffer = CrdtBuffer::new(0, Actor::Human, None).unwrap();
        buffer.insert(0, "hello world test").unwrap();
        buffer.cursor.position = Position { line: 0, column: 0 };

        // TillChar (move till before character)
        buffer.move_cursor(Movement::TillChar('w')).unwrap();
        assert_eq!(buffer.cursor.position.column, 5); // Before 'w' in "world"

        // TillCharReverse
        buffer.move_cursor(Movement::TillCharReverse('e')).unwrap();
        assert_eq!(buffer.cursor.position.column, 2); // After 'e' in "hello"
    }
}
