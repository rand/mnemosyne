//! Integration Modules
//!
//! Connects orchestration system to other Mnemosyne subsystems:
//! - Evolution: Background optimization jobs
//! - MCP: Tool server integration
//! - Evaluation: Quality metrics and learning
//! - Embeddings: Semantic similarity (via storage)

pub mod embeddings;
pub mod evaluation;
pub mod evolution;
pub mod mcp;

pub use embeddings::EmbeddingIntegration;
pub use evaluation::EvaluationIntegration;
pub use evolution::EvolutionIntegration;
pub use mcp::McpIntegration;
