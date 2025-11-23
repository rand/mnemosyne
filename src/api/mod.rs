//! HTTP API for event streaming and state coordination
//!
//! Provides:
//! - Server-Sent Events (SSE) for real-time updates
//! - State coordination endpoints
//! - Agent registry
//! - Memory activity monitoring
//! - Time-series metrics collection

pub mod events;
pub mod metrics;
pub mod server;
pub mod state;

pub use events::{Event, EventBroadcaster, EventType};
pub use metrics::{
    AgentStateCounts, CircularBuffer, MemoryOpRates, MetricsCollector, MetricsSnapshot, SkillUsage,
    WorkProgress,
};
pub use server::{ApiServer, ApiServerConfig};
pub use state::{AgentState, StateManager};
