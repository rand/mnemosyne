//! Embedding generation services for vector similarity search
//!
//! Provides both remote (Voyage AI) and local (fastembed) embedding generation.

pub mod local;
pub mod remote;

pub use local::LocalEmbeddingService;
pub use remote::{EmbeddingService, RemoteEmbeddingService, VOYAGE_EMBEDDING_DIM};

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
    fn test_cosine_similarity() {
        let vec1 = vec![1.0, 0.0, 0.0];
        let vec2 = vec![1.0, 0.0, 0.0];
        let vec3 = vec![0.0, 1.0, 0.0];

        // Same vectors
        assert!((cosine_similarity(&vec1, &vec2) - 1.0).abs() < 0.01);

        // Orthogonal vectors
        assert!((cosine_similarity(&vec1, &vec3) - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_cosine_similarity_different_lengths() {
        let vec1 = vec![1.0, 2.0, 3.0];
        let vec2 = vec![1.0, 2.0];

        assert_eq!(cosine_similarity(&vec1, &vec2), 0.0);
    }

    #[test]
    fn test_cosine_similarity_zero_vectors() {
        let vec1 = vec![0.0, 0.0, 0.0];
        let vec2 = vec![1.0, 2.0, 3.0];

        assert_eq!(cosine_similarity(&vec1, &vec2), 0.0);
    }
}
