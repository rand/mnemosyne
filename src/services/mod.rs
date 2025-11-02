//! Services layer for Mnemosyne memory system
//!
//! Provides LLM integration, embedding generation, and memory intelligence.

pub mod embeddings;
pub mod llm;

pub use embeddings::{cosine_similarity, EmbeddingService, EMBEDDING_DIM};
pub use llm::{LlmConfig, LlmService};
