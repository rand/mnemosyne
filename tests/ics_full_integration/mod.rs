//! Full integration test modules

pub mod fixtures;
pub mod helpers;
pub mod storage_integration;
pub mod storage_integration_s7_s10;
pub mod llm_integration;
pub mod evolution_integration;
pub mod vector_search_integration;
pub mod pty_mode_integration;
pub mod workflow_integration;

// Re-export test fixtures and helpers
pub use fixtures::*;
pub use helpers::*;

// Re-export commonly used types
pub use mnemosyne_core::types::MemoryNote;
