//! Core editor for ICS
//!
//! Manages multiple text buffers with syntax highlighting

mod buffer;
mod cursor;
mod syntax;

pub use buffer::{TextBuffer, BufferId};
pub use cursor::{CursorState, Position, Movement};
pub use syntax::{Language, HighlightKind};

use std::collections::HashMap;
use std::path::PathBuf;

/// Main editor managing multiple buffers
pub struct IcsEditor {
    buffers: HashMap<BufferId, TextBuffer>,
    active_buffer: BufferId,
    next_buffer_id: usize,
}

impl IcsEditor {
    /// Create new editor
    pub fn new() -> Self {
        let mut editor = Self {
            buffers: HashMap::new(),
            active_buffer: 0,
            next_buffer_id: 1,
        };

        // Create initial empty buffer
        let initial_buffer = TextBuffer::new(0, None);
        editor.buffers.insert(0, initial_buffer);

        editor
    }

    /// Create new buffer
    pub fn new_buffer(&mut self, path: Option<PathBuf>) -> BufferId {
        let id = self.next_buffer_id;
        self.next_buffer_id += 1;

        let buffer = TextBuffer::new(id, path);
        self.buffers.insert(id, buffer);

        id
    }

    /// Get active buffer
    pub fn active_buffer(&self) -> &TextBuffer {
        self.buffers.get(&self.active_buffer).unwrap()
    }

    /// Get mutable active buffer
    pub fn active_buffer_mut(&mut self) -> &mut TextBuffer {
        self.buffers.get_mut(&self.active_buffer).unwrap()
    }

    /// Get buffer by ID
    pub fn buffer(&self, id: BufferId) -> Option<&TextBuffer> {
        self.buffers.get(&id)
    }

    /// Get mutable buffer by ID
    pub fn buffer_mut(&mut self, id: BufferId) -> Option<&mut TextBuffer> {
        self.buffers.get_mut(&id)
    }

    /// Set active buffer
    pub fn set_active_buffer(&mut self, id: BufferId) {
        if self.buffers.contains_key(&id) {
            self.active_buffer = id;
        }
    }
}

impl Default for IcsEditor {
    fn default() -> Self {
        Self::new()
    }
}
