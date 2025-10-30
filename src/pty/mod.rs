//! PTY wrapper for Claude Code with enhanced observability
//!
//! This module provides:
//! - PTY session management
//! - Stream interception for agent detection
//! - Output parsing for semantic highlighting
//! - Bidirectional communication with wrapped process

mod parser;
mod session;
mod wrapper;

pub use parser::{AgentMarker, OutputParser, ParsedChunk};
pub use session::{PtyConfig, PtyOutput, PtySession};
pub use wrapper::ClaudeCodeWrapper;
