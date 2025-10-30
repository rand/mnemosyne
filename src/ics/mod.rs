//! Integrated Context Studio (ICS)
//!
//! AI-assisted context engineering environment for Claude Code.
//!
//! ICS provides:
//! - Real-time semantic analysis of context files
//! - Typed hole tracking for ambiguities
//! - Symbol resolution (#file/path, @symbol_name)
//! - AI-powered suggestions and rewrites
//! - Interactive editor with syntax highlighting
//!
//! Can be used as:
//! - Embedded panel in PTY mode (Ctrl+E to toggle)
//! - Standalone mode: `mnemosyne --ics context.md`

pub mod actors;
pub mod editor;
pub mod rendering;
pub mod semantic;
pub mod memory_panel;
pub mod agent_status;
pub mod attribution;
pub mod proposals;
pub mod diagnostics_panel;
pub mod symbols;
pub mod holes;
pub mod suggestions;
pub mod completion_popup;
pub mod commands;
pub mod views;
pub mod layout;
pub mod storage;
pub mod input;
pub mod diagnostics;

mod app;
mod config;
mod events;

pub use app::IcsApp;
pub use config::IcsConfig;
pub use editor::IcsEditor;
pub use events::{EditorEvent, AnalysisEvent, IcsEvent};
pub use memory_panel::{MemoryPanel, MemoryPanelState, MemoryAction};
pub use semantic::{SemanticAnalyzer, SemanticAnalysis, Triple, TypedHole, HoleKind};
pub use agent_status::{AgentStatusWidget, AgentStatusState, AgentInfo, AgentActivity};
pub use attribution::{AttributionPanel, AttributionPanelState, AttributionEntry, ChangeType};
pub use proposals::{ProposalsPanel, ProposalsPanelState, ChangeProposal, ProposalStatus};
pub use diagnostics_panel::{DiagnosticsPanel, DiagnosticsPanelState};
pub use completion_popup::CompletionPopup;
pub use suggestions::CompletionEngine;
pub use symbols::{SymbolRegistry, SharedSymbolRegistry};
pub use holes::{HoleNavigator, HoleResolution, ResolutionStrategy};
