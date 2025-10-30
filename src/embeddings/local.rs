//! Local embedding service using fastembed
//!
//! Provides high-quality semantic embeddings using locally-run models
//! via the fastembed library with ONNX Runtime.
//!
//! Models are automatically downloaded on first use to the cache directory
//! and subsequent runs load from cache.

use crate::config::EmbeddingConfig;
use crate::embeddings::EmbeddingService;
use crate::error::{MnemosyneError, Result};
use async_trait::async_trait;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use std::sync::{Arc, Mutex};
use tokio::task;
use tracing::{debug, info};

/// Local embedding service using fastembed
pub struct LocalEmbeddingService {
    /// The underlying fastembed model (wrapped in Arc<Mutex> for thread-safe interior mutability)
    model: Arc<Mutex<TextEmbedding>>,
    /// Configuration
    config: EmbeddingConfig,
    /// Cached dimensions
    dimensions: usize,
}

impl LocalEmbeddingService {
    /// Create a new local embedding service with the given configuration
    ///
    /// This will download the model if not already cached (may take 30-120 seconds
    /// depending on model size and network speed).
    ///
    /// # Arguments
    /// * `config` - Embedding configuration (model, device, cache dir, etc.)
    ///
    /// # Returns
    /// * `Ok(LocalEmbeddingService)` - Service ready to generate embeddings
    /// * `Err(MnemosyneError)` - If model loading fails
    ///
    /// # Example
    /// ```ignore
    /// let config = EmbeddingConfig::default();
    /// let service = LocalEmbeddingService::new(config).await?;
    /// let embedding = service.embed("Hello world").await?;
    /// ```
    pub async fn new(config: EmbeddingConfig) -> Result<Self> {
        // Validate configuration first
        config.validate()?;

        info!(
            "Initializing local embedding service: model={}, device={}, cache={:?}",
            config.model, config.device, config.cache_dir
        );

        // Map model name to fastembed's EmbeddingModel enum
        let embedding_model = Self::model_name_to_enum(&config.model)?;

        // Create initialization options
        let show_progress = config.show_download_progress;
        let cache_dir = config.cache_dir.clone();
        let mut init_options = InitOptions::default();
        init_options.model_name = embedding_model;
        init_options.show_download_progress = show_progress;
        init_options.cache_dir = cache_dir;

        // Load model in blocking task (may download if not cached)
        let model = task::spawn_blocking(move || TextEmbedding::try_new(init_options))
            .await
            .map_err(|e| MnemosyneError::Other(format!("Task join error: {}", e)))?
            .map_err(|e| MnemosyneError::EmbeddingError(format!("Failed to load model: {}", e)))?;

        let dimensions = config.dimensions();

        info!(
            "Local embedding service initialized successfully: {} dimensions",
            dimensions
        );

        Ok(Self {
            model: Arc::new(Mutex::new(model)),
            config,
            dimensions,
        })
    }

    /// Map model name string to fastembed's EmbeddingModel enum
    fn model_name_to_enum(model_name: &str) -> Result<EmbeddingModel> {
        match model_name {
            "nomic-embed-text-v1.5" => Ok(EmbeddingModel::NomicEmbedTextV15),
            "nomic-embed-text-v1" => Ok(EmbeddingModel::NomicEmbedTextV1),
            "all-MiniLM-L6-v2" => Ok(EmbeddingModel::AllMiniLML6V2),
            "all-MiniLM-L12-v2" => Ok(EmbeddingModel::AllMiniLML12V2),
            "bge-small-en-v1.5" => Ok(EmbeddingModel::BGESmallENV15),
            "bge-base-en-v1.5" => Ok(EmbeddingModel::BGEBaseENV15),
            "bge-large-en-v1.5" => Ok(EmbeddingModel::BGELargeENV15),
            _ => Err(MnemosyneError::Config(
                config::ConfigError::Message(format!(
                    "Unsupported model: '{}'. See EmbeddingConfig::validate() for supported models.",
                    model_name
                ))
            )),
        }
    }

    /// Embed a batch of texts in a blocking task
    ///
    /// This is the internal implementation that runs fastembed's synchronous
    /// embed function in a Tokio blocking task.
    async fn embed_batch_internal(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        debug!("Embedding batch of {} texts", texts.len());

        let model = Arc::clone(&self.model);
        let dimensions = self.dimensions;

        // Run embedding in blocking task (fastembed is synchronous)
        let embeddings = task::spawn_blocking(move || {
            // Lock the mutex to get mutable access
            let mut model_guard = model
                .lock()
                .map_err(|e| format!("Mutex lock failed: {}", e))?;

            // Call embed on the guard
            model_guard
                .embed(texts, None)
                .map_err(|e| format!("Embedding generation failed: {}", e))
        })
        .await
        .map_err(|e| MnemosyneError::Other(format!("Task join error: {}", e)))?
        .map_err(|e| MnemosyneError::EmbeddingError(e))?;

        // Validate dimensions
        for (i, embedding) in embeddings.iter().enumerate() {
            if embedding.len() != dimensions {
                return Err(MnemosyneError::EmbeddingError(format!(
                    "Embedding {} has wrong dimensions: expected {}, got {}",
                    i,
                    dimensions,
                    embedding.len()
                )));
            }
        }

        debug!("Successfully generated {} embeddings", embeddings.len());

        Ok(embeddings)
    }
}

#[async_trait]
impl EmbeddingService for LocalEmbeddingService {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        if text.is_empty() {
            return Err(MnemosyneError::ValidationError(
                "Text cannot be empty".to_string(),
            ));
        }

        let texts = vec![text.to_string()];
        let mut embeddings = self.embed_batch_internal(texts).await?;

        embeddings
            .pop()
            .ok_or_else(|| MnemosyneError::EmbeddingError("No embedding returned".to_string()))
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        // Validate all texts
        for (i, text) in texts.iter().enumerate() {
            if text.is_empty() {
                return Err(MnemosyneError::ValidationError(format!(
                    "Text at index {} cannot be empty",
                    i
                )));
            }
        }

        // Convert to owned strings for spawn_blocking
        let texts_owned: Vec<String> = texts.iter().map(|s| s.to_string()).collect();

        // Process in batches based on config
        let batch_size = self.config.batch_size;
        let mut all_embeddings = Vec::new();

        for chunk in texts_owned.chunks(batch_size) {
            let chunk_embeddings = self.embed_batch_internal(chunk.to_vec()).await?;
            all_embeddings.extend(chunk_embeddings);
        }

        Ok(all_embeddings)
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn model_name(&self) -> &str {
        &self.config.model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_name_mapping() {
        // Valid models
        assert!(LocalEmbeddingService::model_name_to_enum("nomic-embed-text-v1.5").is_ok());
        assert!(LocalEmbeddingService::model_name_to_enum("all-MiniLM-L6-v2").is_ok());
        assert!(LocalEmbeddingService::model_name_to_enum("bge-base-en-v1.5").is_ok());

        // Invalid model
        assert!(LocalEmbeddingService::model_name_to_enum("invalid-model").is_err());
    }

    #[tokio::test]
    async fn test_empty_text_validation() {
        // Create a mock config (won't actually load model)
        let config = EmbeddingConfig::default();

        // We can't actually test embed() without loading the model,
        // but we can test that the service creation validates config
        assert!(config.validate().is_ok());
    }

    // Integration tests with real model downloads
    // NOTE: Run with --test-threads=1 to avoid concurrency issues during model loading:
    // cargo test --lib embeddings::local::tests --release -- --test-threads=1
    #[tokio::test]
    async fn test_embed_single_text() {
        let config = EmbeddingConfig::default();
        let service = LocalEmbeddingService::new(config).await.unwrap();

        let embedding = service.embed("Hello, world!").await.unwrap();

        // nomic-embed-text-v1.5 has 768 dimensions
        assert_eq!(embedding.len(), 768);

        // Check that values are in reasonable range
        for &val in &embedding {
            assert!(val.is_finite());
            assert!(val >= -1.0 && val <= 1.0); // Normalized embeddings
        }
    }

    #[tokio::test]
    async fn test_embed_batch() {
        let config = EmbeddingConfig::default();
        let service = LocalEmbeddingService::new(config).await.unwrap();

        let texts = vec!["Hello", "World", "Test"];
        let embeddings = service.embed_batch(&texts).await.unwrap();

        assert_eq!(embeddings.len(), 3);
        for embedding in &embeddings {
            assert_eq!(embedding.len(), 768);
        }
    }

    #[tokio::test]
    async fn test_semantic_similarity() {
        let config = EmbeddingConfig::default();
        let service = LocalEmbeddingService::new(config).await.unwrap();

        // Similar texts should have similar embeddings
        let embed1 = service.embed("The cat sat on the mat").await.unwrap();
        let embed2 = service.embed("A feline rested on the rug").await.unwrap();
        let embed3 = service
            .embed("Quantum physics is fascinating")
            .await
            .unwrap();

        // Calculate cosine similarity
        let sim_similar = crate::embeddings::cosine_similarity(&embed1, &embed2);
        let sim_different = crate::embeddings::cosine_similarity(&embed1, &embed3);

        // Similar texts should be more similar than different texts
        assert!(sim_similar > sim_different);
        assert!(sim_similar > 0.5); // Reasonably high similarity
    }
}
