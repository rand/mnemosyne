//! Coordination module for handoff between Claude Code and ICS
//!
//! Provides file-based coordination protocol for seamless context handoff:
//! 1. Claude Code writes intent (what to edit, template, etc.)
//! 2. ICS reads intent, opens editor
//! 3. User edits in ICS
//! 4. ICS writes result (changes, analysis, etc.)
//! 5. Claude Code reads result, continues conversation

mod handoff;

pub use handoff::{EditIntent, EditResult, ExitReason, HandoffCoordinator, SemanticAnalysisSummary};
