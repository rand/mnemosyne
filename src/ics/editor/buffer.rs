//! Text buffer with rope data structure
#![allow(dead_code)]
//!
//! Efficient text storage and manipulation using ropey

use super::{CursorState, Language};
use anyhow::{Context, Result};
use ropey::Rope;
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;

/// Check if character is a word character (alphanumeric or underscore)
fn is_word_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
}

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
            self.content
                .remove(edit.position..edit.position + edit.inserted.len());
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
            self.content
                .remove(edit.position..edit.position + edit.deleted.len());
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
        let line_idx = self
            .content
            .line_to_char(self.cursor.position.line.min(self.content.len_lines() - 1));
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

    /// Load file from disk
    pub fn load_file(&mut self, path: PathBuf) -> Result<()> {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        self.content = Rope::from_str(&content);
        self.path = Some(path.clone());
        self.language = Language::from_path(&path).unwrap_or(Language::PlainText);
        self.dirty = false;

        // Reset cursor to start
        self.cursor.position.line = 0;
        self.cursor.position.column = 0;

        // Clear undo/redo stacks
        self.undo_stack.clear();
        self.redo_stack.clear();

        Ok(())
    }

    /// Save buffer to disk
    pub fn save_file(&mut self) -> Result<()> {
        let path = self
            .path
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No file path set"))?;

        let content = self.content.to_string();
        fs::write(path, content)
            .with_context(|| format!("Failed to write file: {}", path.display()))?;

        self.dirty = false;

        Ok(())
    }

    /// Save buffer to a new path
    pub fn save_file_as(&mut self, path: PathBuf) -> Result<()> {
        self.path = Some(path.clone());
        self.language = Language::from_path(&path).unwrap_or(Language::PlainText);
        self.save_file()
    }

    /// Get text content as string
    pub fn text(&self) -> String {
        self.content.to_string()
    }

    /// Move cursor
    pub fn move_cursor(&mut self, movement: super::Movement) {
        use super::Movement::*;

        match movement {
            Left => {
                if self.cursor.position.column > 0 {
                    self.cursor.position.column -= 1;
                } else if self.cursor.position.line > 0 {
                    // Move to end of previous line
                    self.cursor.position.line -= 1;
                    if let Some(line) = self.line(self.cursor.position.line) {
                        self.cursor.position.column = line.trim_end().len();
                    }
                }
            }
            Right => {
                if let Some(line) = self.line(self.cursor.position.line) {
                    let line_len = line.trim_end().len();
                    if self.cursor.position.column < line_len {
                        self.cursor.position.column += 1;
                    } else if self.cursor.position.line < self.line_count() - 1 {
                        // Move to start of next line
                        self.cursor.position.line += 1;
                        self.cursor.position.column = 0;
                    }
                }
            }
            Up => {
                if self.cursor.position.line > 0 {
                    self.cursor.position.line -= 1;
                    // Clamp column to line length
                    if let Some(line) = self.line(self.cursor.position.line) {
                        let line_len = line.trim_end().len();
                        self.cursor.position.column = self.cursor.position.column.min(line_len);
                    }
                }
            }
            Down => {
                if self.cursor.position.line < self.line_count() - 1 {
                    self.cursor.position.line += 1;
                    // Clamp column to line length
                    if let Some(line) = self.line(self.cursor.position.line) {
                        let line_len = line.trim_end().len();
                        self.cursor.position.column = self.cursor.position.column.min(line_len);
                    }
                }
            }
            WordLeft => {
                if let Some(line) = self.line(self.cursor.position.line) {
                    self.move_word_left(&line);
                }
            }
            WordRight => {
                if let Some(line) = self.line(self.cursor.position.line) {
                    self.move_word_right(&line);
                }
            }
            WordEnd => {
                if let Some(line) = self.line(self.cursor.position.line) {
                    self.move_word_end(&line);
                }
            }
            LineStart => {
                self.cursor.position.column = 0;
            }
            LineEnd => {
                if let Some(line) = self.line(self.cursor.position.line) {
                    // Trim newline characters from line length
                    self.cursor.position.column = line.trim_end().len();
                }
            }
            PageUp => {
                // Move up one page (20 lines)
                const PAGE_SIZE: usize = 20;
                if self.cursor.position.line > PAGE_SIZE {
                    self.cursor.position.line -= PAGE_SIZE;
                } else {
                    self.cursor.position.line = 0;
                }
                // Clamp column to line length
                if let Some(line) = self.line(self.cursor.position.line) {
                    let line_len = line.trim_end().len();
                    self.cursor.position.column = self.cursor.position.column.min(line_len);
                }
            }
            PageDown => {
                // Move down one page (20 lines)
                const PAGE_SIZE: usize = 20;
                let max_line = self.line_count().saturating_sub(1);
                self.cursor.position.line = (self.cursor.position.line + PAGE_SIZE).min(max_line);
                // Clamp column to line length
                if let Some(line) = self.line(self.cursor.position.line) {
                    let line_len = line.trim_end().len();
                    self.cursor.position.column = self.cursor.position.column.min(line_len);
                }
            }
            BufferStart => {
                self.cursor.position.line = 0;
                self.cursor.position.column = 0;
            }
            BufferEnd => {
                let max_line = self.line_count().saturating_sub(1);
                self.cursor.position.line = max_line;
                if let Some(line) = self.line(max_line) {
                    self.cursor.position.column = line.trim_end().len();
                }
            }
            FindChar(ch) => {
                if let Some(line) = self.line(self.cursor.position.line) {
                    self.find_char(&line, ch, false);
                }
            }
            FindCharReverse(ch) => {
                if let Some(line) = self.line(self.cursor.position.line) {
                    self.find_char_reverse(&line, ch, false);
                }
            }
            TillChar(ch) => {
                if let Some(line) = self.line(self.cursor.position.line) {
                    self.find_char(&line, ch, true);
                }
            }
            TillCharReverse(ch) => {
                if let Some(line) = self.line(self.cursor.position.line) {
                    self.find_char_reverse(&line, ch, true);
                }
            }
        }
    }

    /// Helper: Move to start of previous word
    fn move_word_left(&mut self, line: &str) {
        if self.cursor.position.column == 0 {
            // Move to end of previous line
            if self.cursor.position.line > 0 {
                self.cursor.position.line -= 1;
                if let Some(prev_line) = self.line(self.cursor.position.line) {
                    self.cursor.position.column = prev_line.trim_end().len();
                }
            }
            return;
        }

        let chars: Vec<char> = line.chars().collect();
        let mut col = self.cursor.position.column.min(chars.len());

        // Skip current position if at word boundary
        col = col.saturating_sub(1);

        // Skip whitespace
        while col > 0 && chars[col].is_whitespace() {
            col -= 1;
        }

        // Skip word characters
        if col > 0 && is_word_char(chars[col]) {
            while col > 0 && is_word_char(chars[col]) {
                col -= 1;
            }
            if !is_word_char(chars[col]) {
                col += 1;
            }
        } else if col > 0 {
            // Skip non-word, non-whitespace characters (punctuation)
            while col > 0 && !is_word_char(chars[col]) && !chars[col].is_whitespace() {
                col -= 1;
            }
            if is_word_char(chars[col]) || chars[col].is_whitespace() {
                col += 1;
            }
        }

        self.cursor.position.column = col;
    }

    /// Helper: Move to start of next word
    fn move_word_right(&mut self, line: &str) {
        let trimmed_len = line.trim_end().len();
        let chars: Vec<char> = line.chars().collect();
        let line_len = chars.len();

        if self.cursor.position.column >= trimmed_len {
            // At or past end of line content, move to start of next line
            if self.cursor.position.line < self.line_count() - 1 {
                self.cursor.position.line += 1;
                self.cursor.position.column = 0;
            }
            return;
        }

        let mut col = self.cursor.position.column;

        // Skip current word
        if is_word_char(chars[col]) {
            while col < line_len && is_word_char(chars[col]) {
                col += 1;
            }
        } else if !chars[col].is_whitespace() {
            // Skip punctuation
            while col < line_len && !is_word_char(chars[col]) && !chars[col].is_whitespace() {
                col += 1;
            }
        }

        // Skip whitespace to next word
        while col < line_len && chars[col].is_whitespace() {
            col += 1;
        }

        // If we've gone past the line content, move to next line
        if col >= trimmed_len && self.cursor.position.line < self.line_count() - 1 {
            self.cursor.position.line += 1;
            self.cursor.position.column = 0;
        } else {
            self.cursor.position.column = col.min(trimmed_len);
        }
    }

    /// Helper: Move to end of current/next word
    fn move_word_end(&mut self, line: &str) {
        let chars: Vec<char> = line.chars().collect();
        let line_len = chars.len();

        if self.cursor.position.column >= line_len.saturating_sub(1) {
            // Move to end of first word on next line
            if self.cursor.position.line < self.line_count() - 1 {
                self.cursor.position.line += 1;
                self.cursor.position.column = 0;
                if let Some(next_line) = self.line(self.cursor.position.line) {
                    self.move_word_end(&next_line);
                }
            }
            return;
        }

        let mut col = self.cursor.position.column;

        // If we're at the end of a word, advance past it
        if col < line_len
            && is_word_char(chars[col])
            && (col + 1 >= line_len || !is_word_char(chars[col + 1]))
        {
            col += 1;
        }

        // Skip whitespace
        while col < line_len && chars[col].is_whitespace() {
            col += 1;
        }

        if col >= line_len {
            // At end of line, go to next line
            if self.cursor.position.line < self.line_count() - 1 {
                self.cursor.position.line += 1;
                self.cursor.position.column = 0;
                if let Some(next_line) = self.line(self.cursor.position.line) {
                    self.move_word_end(&next_line);
                }
            } else {
                self.cursor.position.column = line_len.saturating_sub(1);
            }
            return;
        }

        // Move to end of word
        if is_word_char(chars[col]) {
            while col < line_len - 1 && is_word_char(chars[col + 1]) {
                col += 1;
            }
        } else {
            // Move to end of punctuation sequence
            while col < line_len - 1
                && !is_word_char(chars[col + 1])
                && !chars[col + 1].is_whitespace()
            {
                col += 1;
            }
        }

        self.cursor.position.column = col;
    }

    /// Helper: Find next occurrence of character on current line
    fn find_char(&mut self, line: &str, target: char, till: bool) {
        let chars: Vec<char> = line.chars().collect();
        let start = self.cursor.position.column + 1;

        for (i, &ch) in chars.iter().enumerate().skip(start) {
            if ch == target {
                self.cursor.position.column = if till { i.saturating_sub(1) } else { i };
                return;
            }
        }
    }

    /// Helper: Find previous occurrence of character on current line
    fn find_char_reverse(&mut self, line: &str, target: char, till: bool) {
        let chars: Vec<char> = line.chars().collect();
        let end = self.cursor.position.column;

        for i in (0..end).rev() {
            if chars[i] == target {
                self.cursor.position.column = if till { i + 1 } else { i };
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ics::editor::{Movement, Position};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_file_load_save() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.txt");

        // Write initial content
        fs::write(&file_path, "Hello, world!").unwrap();

        // Load into buffer
        let mut buffer = TextBuffer::new(0, None);
        buffer.load_file(file_path.clone()).unwrap();

        assert_eq!(buffer.text(), "Hello, world!");
        assert_eq!(buffer.path, Some(file_path.clone()));
        assert!(!buffer.dirty);

        // Modify buffer
        buffer.insert(" More text.");
        assert!(buffer.dirty);

        // Save back
        buffer.save_file().unwrap();
        assert!(!buffer.dirty);

        // Read back from disk
        let content = fs::read_to_string(&file_path).unwrap();
        assert!(content.contains("Hello, world!"));
        assert!(content.contains("More text."));
    }

    #[test]
    fn test_cursor_movement() {
        let mut buffer = TextBuffer::new(0, None);
        buffer.insert("Line 1\nLine 2\nLine 3");

        // Move to start
        buffer.cursor.position = Position { line: 0, column: 0 };

        // Move right
        buffer.move_cursor(Movement::Right);
        assert_eq!(buffer.cursor.position.column, 1);

        // Move down
        buffer.move_cursor(Movement::Down);
        assert_eq!(buffer.cursor.position.line, 1);

        // Move to line end
        buffer.move_cursor(Movement::LineEnd);
        // Rope lines include newlines, so line length includes '\n'
        let line_len = buffer
            .line(buffer.cursor.position.line)
            .unwrap()
            .trim_end()
            .len();
        assert_eq!(buffer.cursor.position.column, line_len);

        // Move to line start
        buffer.move_cursor(Movement::LineStart);
        assert_eq!(buffer.cursor.position.column, 0);
    }

    #[test]
    fn test_undo_redo() {
        let mut buffer = TextBuffer::new(0, None);

        buffer.insert("Hello");
        buffer.insert(" World");

        assert_eq!(buffer.text(), "Hello World");

        // Undo
        buffer.undo();
        assert_eq!(buffer.text(), "Hello");

        // Redo
        buffer.redo();
        assert_eq!(buffer.text(), "Hello World");
    }

    #[test]
    fn test_word_movement() {
        let mut buffer = TextBuffer::new(0, None);
        buffer.insert("hello world foo_bar");
        buffer.cursor.position = Position { line: 0, column: 0 };

        // WordRight: hello -> world
        buffer.move_cursor(Movement::WordRight);
        assert_eq!(buffer.cursor.position.column, 6); // Start of "world"

        // WordRight: world -> foo_bar
        buffer.move_cursor(Movement::WordRight);
        assert_eq!(buffer.cursor.position.column, 12); // Start of "foo_bar"

        // WordLeft: foo_bar -> world
        buffer.move_cursor(Movement::WordLeft);
        assert_eq!(buffer.cursor.position.column, 6); // Start of "world"

        // WordLeft: world -> hello
        buffer.move_cursor(Movement::WordLeft);
        assert_eq!(buffer.cursor.position.column, 0); // Start of "hello"
    }

    #[test]
    fn test_word_end_movement() {
        let mut buffer = TextBuffer::new(0, None);
        buffer.insert("hello world");
        buffer.cursor.position = Position { line: 0, column: 0 };

        // Move to end of "hello"
        buffer.move_cursor(Movement::WordEnd);
        assert_eq!(buffer.cursor.position.column, 4); // 'o' of "hello"

        // Move to end of "world"
        buffer.move_cursor(Movement::WordEnd);
        assert_eq!(buffer.cursor.position.column, 10); // 'd' of "world"
    }

    #[test]
    fn test_page_movement() {
        let mut buffer = TextBuffer::new(0, None);
        // Create 50 lines
        for i in 0..50 {
            buffer.insert(&format!("Line {}\n", i));
        }
        buffer.cursor.position = Position {
            line: 25,
            column: 0,
        };

        // Page up
        buffer.move_cursor(Movement::PageUp);
        assert_eq!(buffer.cursor.position.line, 5); // 25 - 20

        // Page down
        buffer.move_cursor(Movement::PageDown);
        assert_eq!(buffer.cursor.position.line, 25); // 5 + 20

        // Page down near end
        buffer.move_cursor(Movement::PageDown);
        assert_eq!(buffer.cursor.position.line, 45); // 25 + 20

        // Page down at end (should clamp)
        buffer.move_cursor(Movement::PageDown);
        assert_eq!(buffer.cursor.position.line, 50); // Clamped to max line
    }

    #[test]
    fn test_buffer_start_end() {
        let mut buffer = TextBuffer::new(0, None);
        buffer.insert("Line 1\nLine 2\nLine 3");
        buffer.cursor.position = Position { line: 1, column: 3 };

        // Move to buffer start
        buffer.move_cursor(Movement::BufferStart);
        assert_eq!(buffer.cursor.position.line, 0);
        assert_eq!(buffer.cursor.position.column, 0);

        // Move to buffer end
        buffer.move_cursor(Movement::BufferEnd);
        assert_eq!(buffer.cursor.position.line, 2);
        assert_eq!(buffer.cursor.position.column, 6); // "Line 3"
    }

    #[test]
    fn test_find_char() {
        let mut buffer = TextBuffer::new(0, None);
        buffer.insert("hello world");
        buffer.cursor.position = Position { line: 0, column: 0 };

        // Find 'o' (first occurrence after cursor)
        buffer.move_cursor(Movement::FindChar('o'));
        assert_eq!(buffer.cursor.position.column, 4); // 'o' in "hello"

        // Find next 'o'
        buffer.move_cursor(Movement::FindChar('o'));
        assert_eq!(buffer.cursor.position.column, 7); // 'o' in "world"

        // Find 'x' (not found, cursor shouldn't move)
        let prev_col = buffer.cursor.position.column;
        buffer.move_cursor(Movement::FindChar('x'));
        assert_eq!(buffer.cursor.position.column, prev_col);
    }

    #[test]
    fn test_find_char_reverse() {
        let mut buffer = TextBuffer::new(0, None);
        buffer.insert("hello world");
        buffer.cursor.position = Position {
            line: 0,
            column: 10,
        }; // 'd' in "world"

        // Find 'o' backwards
        buffer.move_cursor(Movement::FindCharReverse('o'));
        assert_eq!(buffer.cursor.position.column, 7); // 'o' in "world"

        // Find previous 'o' backwards
        buffer.move_cursor(Movement::FindCharReverse('o'));
        assert_eq!(buffer.cursor.position.column, 4); // 'o' in "hello"
    }

    #[test]
    fn test_till_char() {
        let mut buffer = TextBuffer::new(0, None);
        buffer.insert("hello world");
        buffer.cursor.position = Position { line: 0, column: 0 };

        // Till 'w' (move to position before 'w')
        buffer.move_cursor(Movement::TillChar('w'));
        assert_eq!(buffer.cursor.position.column, 5); // Space before "world"

        // Till 'd' (move to position before 'd')
        buffer.move_cursor(Movement::TillChar('d'));
        assert_eq!(buffer.cursor.position.column, 9); // 'l' in "world"
    }

    #[test]
    fn test_till_char_reverse() {
        let mut buffer = TextBuffer::new(0, None);
        buffer.insert("hello world");
        buffer.cursor.position = Position {
            line: 0,
            column: 10,
        }; // 'd' in "world"

        // Till 'w' backwards (move to position after 'w')
        buffer.move_cursor(Movement::TillCharReverse('w'));
        assert_eq!(buffer.cursor.position.column, 7); // 'o' in "world"

        // Till 'h' backwards (move to position after 'h')
        buffer.move_cursor(Movement::TillCharReverse('h'));
        assert_eq!(buffer.cursor.position.column, 1); // 'e' in "hello"
    }

    #[test]
    fn test_word_movement_with_punctuation() {
        let mut buffer = TextBuffer::new(0, None);
        buffer.insert("hello, world! foo");
        buffer.cursor.position = Position { line: 0, column: 0 };

        // WordRight: hello -> ,
        buffer.move_cursor(Movement::WordRight);
        assert_eq!(buffer.cursor.position.column, 5); // ','

        // WordRight: , -> world
        buffer.move_cursor(Movement::WordRight);
        assert_eq!(buffer.cursor.position.column, 7); // Start of "world"

        // WordRight: world -> !
        buffer.move_cursor(Movement::WordRight);
        assert_eq!(buffer.cursor.position.column, 12); // '!'

        // WordRight: ! -> foo
        buffer.move_cursor(Movement::WordRight);
        assert_eq!(buffer.cursor.position.column, 14); // Start of "foo"
    }

    #[test]
    fn test_word_movement_multiline() {
        let mut buffer = TextBuffer::new(0, None);
        buffer.insert("hello\nworld");
        buffer.cursor.position = Position { line: 0, column: 5 }; // End of "hello"

        // WordRight: should move to "world" on next line
        buffer.move_cursor(Movement::WordRight);
        assert_eq!(buffer.cursor.position.line, 1);
        assert_eq!(buffer.cursor.position.column, 0);

        // WordLeft: should move back to "hello"
        buffer.move_cursor(Movement::WordLeft);
        assert_eq!(buffer.cursor.position.line, 0);
        assert_eq!(buffer.cursor.position.column, 5); // End of previous line
    }
}
