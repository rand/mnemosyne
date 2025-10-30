//! Text buffer with rope data structure
//!
//! Efficient text storage and manipulation using ropey

use super::{CursorState, Language};
use anyhow::{Context, Result};
use ropey::Rope;
use std::collections::VecDeque;
use std::fs;
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
            LineStart => {
                self.cursor.position.column = 0;
            }
            LineEnd => {
                if let Some(line) = self.line(self.cursor.position.line) {
                    // Trim newline characters from line length
                    self.cursor.position.column = line.trim_end().len();
                }
            }
            _ => {
                // TODO: Implement other movement commands
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
}
