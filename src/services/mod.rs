//! Services layer for Mnemosyne memory system
//!
//! Provides LLM integration, embedding generation, and memory intelligence.

pub mod dspy_llm;
pub mod embeddings;
pub mod llm;

pub use dspy_llm::{DspyConfig, DspyLlmService};
pub use embeddings::{cosine_similarity, EmbeddingService, EMBEDDING_DIM};
pub use llm::{LlmConfig, LlmService};
