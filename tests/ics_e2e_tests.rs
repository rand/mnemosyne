//! ICS End-to-End Test Suite
//!
//! Comprehensive E2E testing for the Integrated Context Studio.
//! Tests are organized into categories:
//! - Human workflows: Typical user interactions
//! - Agent workflows: AI agent interactions
//! - Collaborative: Multi-agent scenarios
//! - Integration: Full workflow tests
//! - Edge cases: Error handling and limits

#[path = "ics_e2e/mod.rs"]
mod ics_e2e;

#[path = "common/mod.rs"]
mod common;
