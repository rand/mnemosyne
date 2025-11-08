//! Dashboard panels - Modular btop-inspired UI components
//!
//! Each panel is a self-contained module responsible for rendering
//! a specific aspect of the system.
//!
//! Current panels:
//! - System Overview: At-a-glance health summary
//! - Activity Stream: Intelligent event log with filtering
//! - Agent Details: Deep-dive into agent activity
//! - Operations: CLI command history and stats

pub mod agents;
pub mod events;
pub mod operations;

pub use agents::{AgentInfo, AgentsPanel};
pub use events::EventLogPanel;
pub use operations::{OperationEntry, OperationStatus, OperationsPanel};
