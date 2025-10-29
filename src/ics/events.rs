//! Event types for ICS
//!
//! Defines all events that can occur in the Integrated Context Studio

use std::path::PathBuf;

/// Editor events (user actions)
#[derive(Debug, Clone)]
pub enum EditorEvent {
    /// Insert text at cursor
    Insert(String),
    /// Delete character at cursor
    Delete,
    /// Move cursor
    CursorMove { line: usize, column: usize },
    /// Open buffer
    BufferOpen(PathBuf),
    /// Close buffer
    BufferClose(usize),
    /// Save buffer
    BufferSave(usize),
}

/// Analysis events (background processing)
#[derive(Debug, Clone)]
pub enum AnalysisEvent {
    /// Analysis started
    Started,
    /// Analysis completed successfully
    Completed,
    /// Analysis failed
    Failed(String),
}

/// Top-level ICS event
#[derive(Debug, Clone)]
pub enum IcsEvent {
    /// Editor event
    Editor(EditorEvent),
    /// Analysis event
    Analysis(AnalysisEvent),
    /// Quit ICS
    Quit,
}
