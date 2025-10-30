//! Cursor and selection management

/// Position in text buffer
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[derive(Default)]
pub struct Position {
    /// Line number (0-indexed)
    pub line: usize,
    /// Column number (0-indexed, UTF-8 byte offset)
    pub column: usize,
}


/// Cursor state
#[derive(Debug, Clone)]
#[derive(Default)]
pub struct CursorState {
    /// Current cursor position
    pub position: Position,

    /// Selection anchor (for selections)
    /// If Some, there is an active selection from anchor to position
    pub anchor: Option<Position>,

    /// Virtual column (for vertical movement)
    /// Preserves horizontal position when moving through shorter lines
    pub virtual_column: usize,
}


/// Cursor movement commands
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Movement {
    /// Move left one character
    Left,
    /// Move right one character
    Right,
    /// Move up one line
    Up,
    /// Move down one line
    Down,

    /// Move left one word
    WordLeft,
    /// Move right one word
    WordRight,
    /// Move to end of word (Helix-style)
    WordEnd,

    /// Move to start of line
    LineStart,
    /// Move to end of line
    LineEnd,

    /// Move up one page
    PageUp,
    /// Move down one page
    PageDown,

    /// Move to start of buffer
    BufferStart,
    /// Move to end of buffer
    BufferEnd,

    /// Find next occurrence of character (Helix f)
    FindChar(char),
    /// Find previous occurrence of character (Helix F)
    FindCharReverse(char),
    /// Move till (before) character (Helix t)
    TillChar(char),
    /// Move till (after) character reverse (Helix T)
    TillCharReverse(char),
}

impl CursorState {
    /// Start selection at current position
    pub fn start_selection(&mut self) {
        self.anchor = Some(self.position);
    }

    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.anchor = None;
    }

    /// Check if there is an active selection
    pub fn has_selection(&self) -> bool {
        self.anchor.is_some()
    }

    /// Get selection range (start, end)
    pub fn selection_range(&self) -> Option<(Position, Position)> {
        let anchor = self.anchor?;
        let (start, end) = if anchor.line < self.position.line
            || (anchor.line == self.position.line && anchor.column < self.position.column)
        {
            (anchor, self.position)
        } else {
            (self.position, anchor)
        };
        Some((start, end))
    }
}
