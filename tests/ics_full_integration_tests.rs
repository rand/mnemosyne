//! Full Integration E2E Tests: ICS + Mnemosyne
//!
//! Comprehensive integration testing covering:
//! - Storage integration (S1-S10)
//! - LLM service integration (L1-L8)
//! - Evolution system (E1-E6)
//! - Orchestration (O1-O8)
//! - Evaluation system (V1-V6)
//! - Vector search (V1-V8)
//! - PTY mode (P1-P6)
//! - Full workflows (W1-W8)

#[path = "ics_full_integration/mod.rs"]
mod ics_full_integration;

#[path = "common/mod.rs"]
mod common;
