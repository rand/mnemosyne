//! Integration Modules
//!
//! Connects orchestration system to other Mnemosyne subsystems:
//! - Evolution: Background optimization jobs
//! - MCP: Tool server integration
//! - Evaluation: Quality metrics and learning
//! - Embeddings: Semantic similarity (via storage)

pub mod evolution;

pub use evolution::EvolutionIntegration;
