//! PTY wrapper for Claude Code with enhanced observability
//!
//! This module provides:
//! - PTY session management
//! - Stream interception for agent detection
//! - Output parsing for semantic highlighting
//! - Bidirectional communication with wrapped process

mod session;
mod parser;
mod wrapper;

pub use session::{PtySession, PtyConfig, PtyOutput};
pub use parser::{AgentMarker, OutputParser, ParsedChunk};
pub use wrapper::ClaudeCodeWrapper;
