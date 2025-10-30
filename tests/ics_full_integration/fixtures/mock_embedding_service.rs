//! Mock embedding service for testing without model downloads

use async_trait::async_trait;
use mnemosyne_core::{embeddings::EmbeddingService, error::Result};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Mock embedding service that generates deterministic fake embeddings
///
/// This allows vector search tests to run without downloading models.
/// Embeddings are deterministic based on text content hash.
pub struct MockEmbeddingService {
    dimensions: usize,
}

impl MockEmbeddingService {
    /// Create new mock embedding service
    pub fn new(dimensions: usize) -> Self {
        Self { dimensions }
    }

    /// Create with standard dimensions (384 for all-MiniLM-L6-v2 compatibility)
    pub fn new_standard() -> Self {
        Self::new(384)
    }

    /// Generate deterministic embedding from text hash
    fn generate_embedding(&self, text: &str) -> Vec<f32> {
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let base_hash = hasher.finish();

        // Generate deterministic but varied embeddings
        let mut embedding = Vec::with_capacity(self.dimensions);

        for i in 0..self.dimensions {
            // Use hash + dimension index for variety
            let mut dim_hasher = DefaultHasher::new();
            base_hash.hash(&mut dim_hasher);
            i.hash(&mut dim_hasher);
            let dim_hash = dim_hasher.finish();

            // Convert to float in range [-1, 1]
            let value = ((dim_hash % 2000) as f32 - 1000.0) / 1000.0;
            embedding.push(value);
        }

        // Normalize to unit length (like real embeddings)
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        if magnitude > 0.0 {
            for value in &mut embedding {
                *value /= magnitude;
            }
        }

        embedding
    }
}

#[async_trait]
impl EmbeddingService for MockEmbeddingService {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        Ok(self.generate_embedding(text))
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        Ok(texts
            .iter()
            .map(|text| self.generate_embedding(text))
            .collect())
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn model_name(&self) -> &str {
        "mock-embedding-service"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_embedding_consistency() {
        let service = MockEmbeddingService::new_standard();

        let text = "Test text";
        let embedding1 = service.embed(text).await.unwrap();
        let embedding2 = service.embed(text).await.unwrap();

        // Should be identical (deterministic)
        assert_eq!(embedding1, embedding2);
    }

    #[tokio::test]
    async fn test_mock_embedding_dimensions() {
        let service = MockEmbeddingService::new_standard();

        let embedding = service.embed("Test").await.unwrap();
        assert_eq!(embedding.len(), 384);
    }

    #[tokio::test]
    async fn test_mock_embedding_normalized() {
        let service = MockEmbeddingService::new_standard();

        let embedding = service.embed("Test").await.unwrap();
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();

        // Should be unit length (normalized)
        assert!((magnitude - 1.0).abs() < 0.001, "magnitude={}", magnitude);
    }

    #[tokio::test]
    async fn test_mock_embedding_different_texts() {
        let service = MockEmbeddingService::new_standard();

        let embedding1 = service.embed("Hello world").await.unwrap();
        let embedding2 = service.embed("Different text").await.unwrap();

        // Should be different for different texts
        assert_ne!(embedding1, embedding2);
    }

    #[tokio::test]
    async fn test_mock_embedding_batch() {
        let service = MockEmbeddingService::new_standard();

        let texts = vec!["Text 1", "Text 2", "Text 3"];
        let batch = service.embed_batch(&texts).await.unwrap();

        assert_eq!(batch.len(), 3);

        // Each should match individual embed
        for (i, text) in texts.iter().enumerate() {
            let individual = service.embed(text).await.unwrap();
            assert_eq!(batch[i], individual);
        }
    }
}
