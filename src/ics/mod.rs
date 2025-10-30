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
pub mod agent_status;
pub mod attribution;
pub mod commands;
pub mod completion_popup;
pub mod diagnostics;
pub mod diagnostics_panel;
pub mod editor;
pub mod holes;
pub mod input;
pub mod layout;
pub mod memory_panel;
pub mod proposals;
pub mod rendering;
pub mod semantic;
pub mod storage;
pub mod suggestions;
pub mod symbols;
pub mod views;

mod app;
mod config;
mod events;

pub use agent_status::{AgentActivity, AgentInfo, AgentStatusState, AgentStatusWidget};
pub use app::IcsApp;
pub use attribution::{AttributionEntry, AttributionPanel, AttributionPanelState, ChangeType};
pub use completion_popup::CompletionPopup;
pub use config::IcsConfig;
pub use diagnostics_panel::{DiagnosticsPanel, DiagnosticsPanelState};
pub use editor::IcsEditor;
pub use events::{AnalysisEvent, EditorEvent, IcsEvent};
pub use holes::{HoleNavigator, HoleResolution, ResolutionStrategy};
pub use memory_panel::{MemoryAction, MemoryPanel, MemoryPanelState};
pub use proposals::{ChangeProposal, ProposalStatus, ProposalsPanel, ProposalsPanelState};
pub use semantic::{HoleKind, SemanticAnalysis, SemanticAnalyzer, Triple, TypedHole};
pub use suggestions::CompletionEngine;
pub use symbols::{SharedSymbolRegistry, SymbolRegistry};
