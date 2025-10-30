//! Core editor for ICS
//!
//! Manages multiple text buffers with syntax highlighting

mod buffer;
mod completion;
mod crdt_buffer;
mod cursor;
mod highlight;
mod sync;
mod syntax;
mod validation;
mod widget;

pub use buffer::BufferId;
pub use completion::{CompletionEngine, CompletionItem, CompletionKind};
pub use crdt_buffer::{Actor, Attribution, CrdtBuffer};
pub use cursor::{CursorState, Movement, Position};
pub use highlight::{HighlightKind as HighlightKindEnum, HighlightSpan, Highlighter};
pub use sync::{Awareness, AwarenessTracker, SyncCoordinator, SyncMessage, SyncPayload};
pub use syntax::{HighlightKind, Language};
pub use validation::{Diagnostic, Severity, Validator};
pub use widget::{EditorState, EditorWidget};

use std::collections::HashMap;
use std::path::PathBuf;

/// Main editor managing multiple buffers
///
/// # Invariants
///
/// - `active_buffer` always refers to a buffer that exists in `buffers`
/// - Buffer 0 always exists (created on initialization)
/// - Buffers are never removed (only added)
///
/// These invariants ensure that `active_buffer()` and `active_buffer_mut()` never panic.
pub struct IcsEditor {
    buffers: HashMap<BufferId, CrdtBuffer>,
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

        // Create initial empty buffer (maintains invariant: buffer 0 always exists)
        let initial_buffer =
            CrdtBuffer::new(0, Actor::Human, None).expect("Failed to create initial buffer");
        editor.buffers.insert(0, initial_buffer);

        editor
    }

    /// Create new buffer
    pub fn new_buffer(&mut self, path: Option<PathBuf>) -> BufferId {
        let id = self.next_buffer_id;
        self.next_buffer_id += 1;

        let buffer = CrdtBuffer::new(id, Actor::Human, path).expect("Failed to create buffer");
        self.buffers.insert(id, buffer);

        id
    }

    /// Get active buffer
    ///
    /// # Panics
    ///
    /// Never panics due to maintained invariant that active_buffer always exists
    pub fn active_buffer(&self) -> &CrdtBuffer {
        self.buffers
            .get(&self.active_buffer)
            .expect("INVARIANT VIOLATION: active_buffer should always exist")
    }

    /// Get mutable active buffer
    ///
    /// # Panics
    ///
    /// Never panics due to maintained invariant that active_buffer always exists
    pub fn active_buffer_mut(&mut self) -> &mut CrdtBuffer {
        self.buffers
            .get_mut(&self.active_buffer)
            .expect("INVARIANT VIOLATION: active_buffer should always exist")
    }

    /// Get buffer by ID
    pub fn buffer(&self, id: BufferId) -> Option<&CrdtBuffer> {
        self.buffers.get(&id)
    }

    /// Get mutable buffer by ID
    pub fn buffer_mut(&mut self, id: BufferId) -> Option<&mut CrdtBuffer> {
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
