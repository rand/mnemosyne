//! Embedding generation service for vector search
//!
//! Generates fixed-size vector embeddings for memory content to enable
//! semantic similarity search.

use crate::error::Result;
use crate::services::llm::LlmConfig;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use tracing::{debug, warn};

/// Embedding dimension (using 384 for compatibility with all-MiniLM-L6-v2)
pub const EMBEDDING_DIM: usize = 384;

/// Embedding generation service
pub struct EmbeddingService {
    client: Client,
    api_key: String,
    config: LlmConfig,
}

#[derive(Serialize)]
struct EmbeddingRequest {
    model: String,
    messages: Vec<Message>,
    max_tokens: u32,
    temperature: f32,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    text: String,
}

impl EmbeddingService {
    /// Create a new embedding service
    pub fn new(api_key: String, config: LlmConfig) -> Self {
        Self {
            client: Client::new(),
            api_key,
            config,
        }
    }

    /// Generate embedding vector for text
    ///
    /// Uses LLM to extract semantic features, then converts to fixed-size vector.
    /// This is a pragmatic approach using existing infrastructure.
    /// Can be upgraded to dedicated embedding models (Voyage AI, OpenAI, etc.) later.
    pub async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        debug!("Generating embedding for text ({} chars)", text.len());

        // For very short text, use simpler approach
        if text.len() < 50 {
            return Ok(Self::simple_embedding(text));
        }

        // Use LLM to extract semantic features
        let prompt = format!(
            "Extract 10 key semantic concepts from this text, as single words or short phrases. \
             Focus on: main topics, entities, actions, and domain. \
             Return only a comma-separated list.\n\nText: {}\n\nConcepts:",
            text.chars().take(1000).collect::<String>()
        );

        match self.call_llm(&prompt).await {
            Ok(concepts) => {
                // Convert concepts to embedding vector
                Ok(Self::concepts_to_embedding(&concepts, text))
            }
            Err(e) => {
                warn!("LLM embedding failed, using fallback: {}", e);
                // Fallback to simple embedding
                Ok(Self::simple_embedding(text))
            }
        }
    }

    /// Call LLM for concept extraction
    async fn call_llm(&self, prompt: &str) -> Result<String> {
        let request = EmbeddingRequest {
            model: self.config.model.clone(),
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
            max_tokens: 200,
            temperature: 0.3, // Low temperature for consistent extraction
        };

        // Always use x-api-key header (OAuth tokens work with this header)
        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        let body: EmbeddingResponse = response.json().await?;

        Ok(body
            .content
            .first()
            .map(|c| c.text.clone())
            .unwrap_or_default())
    }

    /// Convert semantic concepts to embedding vector
    fn concepts_to_embedding(concepts: &str, original_text: &str) -> Vec<f32> {
        let mut embedding = vec![0.0; EMBEDDING_DIM];

        // Hash concepts to generate vector components
        let concept_words: Vec<&str> = concepts
            .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
            .filter(|s| !s.is_empty())
            .collect();

        // Use multiple hash functions for different dimensions
        for (i, concept) in concept_words.iter().enumerate() {
            let mut hasher = DefaultHasher::new();
            concept.to_lowercase().hash(&mut hasher);

            let hash = hasher.finish();

            // Spread influence across multiple dimensions
            let start_dim = (hash as usize % (EMBEDDING_DIM - 32)) as usize;
            for offset in 0..32 {
                let dim = (start_dim + offset) % EMBEDDING_DIM;
                // Use different hash bits for different dimensions
                let value = ((hash >> offset) & 0xFF) as f32 / 255.0;
                embedding[dim] += value * (1.0 - i as f32 / concept_words.len() as f32);
            }
        }

        // Add text length signal
        let length_signal = (original_text.len() as f32 / 1000.0).min(1.0);
        embedding[0] = length_signal;

        // Normalize vector
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            for val in &mut embedding {
                *val /= magnitude;
            }
        }

        embedding
    }

    /// Simple embedding for short text or fallback
    fn simple_embedding(text: &str) -> Vec<f32> {
        let mut embedding = vec![0.0; EMBEDDING_DIM];

        // Character n-grams hashing
        let text_lower = text.to_lowercase();
        let chars: Vec<char> = text_lower.chars().collect();

        for window_size in 2..=4 {
            for window in chars.windows(window_size) {
                let mut hasher = DefaultHasher::new();
                window.iter().collect::<String>().hash(&mut hasher);
                let hash = hasher.finish();

                let dim = (hash as usize) % EMBEDDING_DIM;
                embedding[dim] += 1.0;
            }
        }

        // Word-level hashing
        for word in text_lower.split_whitespace() {
            let mut hasher = DefaultHasher::new();
            word.hash(&mut hasher);
            let hash = hasher.finish();

            let dim = (hash as usize) % EMBEDDING_DIM;
            embedding[dim] += 2.0; // Words weighted more than character n-grams
        }

        // Normalize
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            for val in &mut embedding {
                *val /= magnitude;
            }
        }

        embedding
    }
}

/// Calculate cosine similarity between two vectors
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let magnitude_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let magnitude_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if magnitude_a == 0.0 || magnitude_b == 0.0 {
        return 0.0;
    }

    dot_product / (magnitude_a * magnitude_b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_embedding() {
        let text = "Rust programming language";
        let embedding = EmbeddingService::simple_embedding(text);

        assert_eq!(embedding.len(), EMBEDDING_DIM);

        // Check normalization
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 0.01, "Vector should be normalized");
    }

    #[test]
    fn test_cosine_similarity() {
        let vec1 = vec![1.0, 0.0, 0.0];
        let vec2 = vec![1.0, 0.0, 0.0];
        let vec3 = vec![0.0, 1.0, 0.0];

        assert!((cosine_similarity(&vec1, &vec2) - 1.0).abs() < 0.01);
        assert!((cosine_similarity(&vec1, &vec3) - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_similar_texts_have_similar_embeddings() {
        let text1 = "database architecture decisions";
        let text2 = "database design choices";
        let text3 = "cooking recipes";

        let emb1 = EmbeddingService::simple_embedding(text1);
        let emb2 = EmbeddingService::simple_embedding(text2);
        let emb3 = EmbeddingService::simple_embedding(text3);

        let sim_12 = cosine_similarity(&emb1, &emb2);
        let sim_13 = cosine_similarity(&emb1, &emb3);

        assert!(
            sim_12 > sim_13,
            "Similar texts should have higher similarity"
        );
    }
}
