//! End-to-End tests for ICS (Integrated Context Studio)
//!
//! Comprehensive E2E testing covering:
//! - Human interaction workflows
//! - Agent interaction workflows
//! - Multi-agent collaboration
//! - Full workflow integration
//! - Edge cases and error handling
//!
//! Test infrastructure provides:
//! - Mock agent system for simulating agent behavior
//! - Test fixtures for documents, memories, proposals
//! - Custom assertions for E2E validation
//! - Helpers for common test operations

pub mod helpers;
pub mod human_workflows;
pub mod agent_workflows;
pub mod collaborative;
pub mod integration;
pub mod edge_cases;

// Re-export test helpers for use in test modules
#[allow(unused_imports)]
pub use helpers::{
    fixtures,
    actors,
    assertions::*,
    TestContext,
};
