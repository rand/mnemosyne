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

pub mod activity_stream;
pub mod agents;
pub mod events;
pub mod operations;
pub mod system_overview;

pub use activity_stream::ActivityStreamPanel;
pub use agents::{AgentInfo, AgentsPanel};
pub use events::EventLogPanel;
pub use operations::{OperationEntry, OperationStatus, OperationsPanel};
pub use system_overview::{SystemMetrics, SystemOverviewPanel};
