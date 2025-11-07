//! Dashboard panels - Modular btop-inspired UI components
//!
//! Each panel is a self-contained module responsible for rendering
//! a specific aspect of the system (agents, memory, skills, work, context, events).

pub mod agents;

pub use agents::AgentsPanel;
