//! HTTP API for event streaming and state coordination
//!
//! Provides:
//! - Server-Sent Events (SSE) for real-time updates
//! - State coordination endpoints
//! - Agent registry
//! - Memory activity monitoring

pub mod events;
pub mod server;
pub mod state;

pub use events::{Event, EventBroadcaster, EventType};
pub use server::{ApiServer, ApiServerConfig};
pub use state::{AgentState, StateManager};
