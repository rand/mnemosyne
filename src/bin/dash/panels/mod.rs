//! Dashboard panels - Modular btop-inspired UI components
//!
//! Each panel is a self-contained module responsible for rendering
//! a specific aspect of the system (agents, memory, skills, work, context, events).

pub mod agents;
pub mod context;
pub mod events;
pub mod memory;
pub mod skills;
pub mod work;

pub use agents::{AgentInfo, AgentState, AgentsPanel};
pub use context::{ContextPanel, ContextState};
pub use events::{EventEntry, EventLogPanel};
pub use memory::{MemoryOpsMetrics, MemoryPanel};
pub use skills::{SkillsMetrics, SkillsPanel};
pub use work::{WorkMetrics, WorkPanel};
