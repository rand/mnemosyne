//! Remote embedding service using Voyage AI API
//!
//! Provides high-quality semantic embeddings via Voyage AI's
//! text embedding models for vector similarity search.

use crate::error::{MnemosyneError, Result};
use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::sleep;
use tracing::{debug, warn};

/// Embedding dimension for Voyage AI models (1536 for voyage-3-large)
pub const VOYAGE_EMBEDDING_DIM: usize = 1536;

/// Maximum texts per batch request
const MAX_BATCH_SIZE: usize = 128;

/// Maximum retry attempts for rate limiting
const MAX_RETRIES: usize = 3;

/// Backoff base duration in milliseconds
const BACKOFF_BASE_MS: u64 = 1000;

/// Request timeout duration
const REQUEST_TIMEOUT_SECS: u64 = 30;

/// Embedding service trait defining required operations
#[async_trait]
pub trait EmbeddingService: Send + Sync {
    /// Generate embedding for a single text
    async fn embed(&self, text: &str) -> Result<Vec<f32>>;

    /// Generate embeddings for multiple texts (batched)
    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;

    /// Get embedding dimensionality (e.g., 1536 for Voyage AI)
    fn dimensions(&self) -> usize;

    /// Get model name
    fn model_name(&self) -> &str;
}

/// Voyage AI embedding service
pub struct RemoteEmbeddingService {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
    dimensions: usize,
}

/// Voyage AI API request structure
#[derive(Debug, Serialize)]
struct VoyageRequest {
    input: Vec<String>,
    model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    input_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    truncation: Option<bool>,
}

/// Voyage AI API response structure
#[derive(Debug, Deserialize)]
struct VoyageResponse {
    data: Vec<EmbeddingData>,
    usage: Usage,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
    index: usize,
}

#[derive(Debug, Deserialize)]
struct Usage {
    total_tokens: usize,
}

/// API error response
#[derive(Debug, Deserialize)]
struct ErrorResponse {
    error: Option<ErrorDetail>,
}

#[derive(Debug, Deserialize)]
struct ErrorDetail {
    message: String,
    #[serde(rename = "type")]
    error_type: Option<String>,
}

impl RemoteEmbeddingService {
    /// Create a new remote embedding service
    ///
    /// # Arguments
    /// * `api_key` - Voyage AI API key
    /// * `model` - Model name (e.g., "voyage-3-large", "voyage-3.5")
    /// * `base_url` - API base URL (defaults to Voyage AI endpoint)
    pub fn new(api_key: String, model: Option<String>, base_url: Option<String>) -> Result<Self> {
        if api_key.is_empty() {
            return Err(MnemosyneError::ValidationError(
                "API key cannot be empty".to_string(),
            ));
        }

        let client = Client::builder()
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .build()
            .map_err(|e| MnemosyneError::NetworkError(e.to_string()))?;

        let model = model.unwrap_or_else(|| "voyage-3-large".to_string());
        let base_url = base_url.unwrap_or_else(|| "https://api.voyageai.com/v1".to_string());

        Ok(Self {
            client,
            api_key,
            model,
            base_url,
            dimensions: VOYAGE_EMBEDDING_DIM,
        })
    }

    /// Call Voyage AI API with retry logic and rate limiting
    async fn call_api_with_retry(&self, texts: &[String]) -> Result<VoyageResponse> {
        let mut retries = 0;

        loop {
            match self.call_api(texts).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    if retries >= MAX_RETRIES {
                        return Err(e);
                    }

                    // Check if error is retryable
                    let should_retry = match &e {
                        MnemosyneError::RateLimitExceeded(_) => true,
                        MnemosyneError::NetworkError(msg) if msg.contains("timeout") => true,
                        _ => false,
                    };

                    if !should_retry {
                        return Err(e);
                    }

                    // Exponential backoff
                    let backoff_ms = BACKOFF_BASE_MS * 2_u64.pow(retries as u32);
                    warn!(
                        "API call failed, retrying after {}ms (attempt {}/{})",
                        backoff_ms,
                        retries + 1,
                        MAX_RETRIES
                    );

                    sleep(Duration::from_millis(backoff_ms)).await;
                    retries += 1;
                }
            }
        }
    }

    /// Call Voyage AI API once (no retry)
    async fn call_api(&self, texts: &[String]) -> Result<VoyageResponse> {
        debug!(
            "Calling Voyage AI API: {} texts, model: {}",
            texts.len(),
            self.model
        );

        let request = VoyageRequest {
            input: texts.to_vec(),
            model: self.model.clone(),
            input_type: Some("document".to_string()),
            truncation: Some(true),
        };

        let response = self
            .client
            .post(format!("{}/embeddings", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| MnemosyneError::NetworkError(e.to_string()))?;

        let status = response.status();

        match status {
            StatusCode::OK => {
                let voyage_response = response
                    .json::<VoyageResponse>()
                    .await
                    .map_err(|e| MnemosyneError::SerializationError(e.to_string()))?;

                debug!(
                    "Successfully generated {} embeddings ({} tokens)",
                    voyage_response.data.len(),
                    voyage_response.usage.total_tokens
                );

                Ok(voyage_response)
            }
            StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                Err(MnemosyneError::AuthenticationError(
                    "Invalid or missing API key".to_string(),
                ))
            }
            StatusCode::TOO_MANY_REQUESTS => Err(MnemosyneError::RateLimitExceeded(
                "Voyage AI rate limit exceeded".to_string(),
            )),
            StatusCode::BAD_REQUEST => {
                // Try to parse error message
                let error_msg = if let Ok(error_response) = response.json::<ErrorResponse>().await {
                    error_response
                        .error
                        .map(|e| e.message)
                        .unwrap_or_else(|| "Bad request".to_string())
                } else {
                    "Bad request".to_string()
                };

                Err(MnemosyneError::EmbeddingError(error_msg))
            }
            _ => {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());

                Err(MnemosyneError::EmbeddingError(format!(
                    "API error (status {}): {}",
                    status, error_text
                )))
            }
        }
    }

    /// Validate text input
    fn validate_text(&self, text: &str) -> Result<()> {
        if text.is_empty() {
            return Err(MnemosyneError::ValidationError(
                "Text cannot be empty".to_string(),
            ));
        }

        Ok(())
    }

    /// Validate embedding dimensions
    fn validate_embedding(&self, embedding: &[f32]) -> Result<()> {
        if embedding.len() != self.dimensions {
            return Err(MnemosyneError::EmbeddingError(format!(
                "Expected {} dimensions, got {}",
                self.dimensions,
                embedding.len()
            )));
        }

        // Check for invalid values (NaN, Inf)
        if embedding.iter().any(|&x| !x.is_finite()) {
            return Err(MnemosyneError::EmbeddingError(
                "Embedding contains invalid values (NaN or Inf)".to_string(),
            ));
        }

        Ok(())
    }
}

#[async_trait]
impl EmbeddingService for RemoteEmbeddingService {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        self.validate_text(text)?;

        let texts = vec![text.to_string()];
        let response = self.call_api_with_retry(&texts).await?;

        let embedding = response
            .data
            .into_iter()
            .next()
            .ok_or_else(|| MnemosyneError::EmbeddingError("Empty response from API".to_string()))?
            .embedding;

        self.validate_embedding(&embedding)?;

        Ok(embedding)
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        // Validate all texts
        for text in texts {
            self.validate_text(text)?;
        }

        let mut all_embeddings = Vec::new();

        // Process in chunks to respect batch size limit
        for chunk in texts.chunks(MAX_BATCH_SIZE) {
            let text_strings: Vec<String> = chunk.iter().map(|s| s.to_string()).collect();
            let response = self.call_api_with_retry(&text_strings).await?;

            // Sort by index to maintain order
            let mut embeddings: Vec<_> = response.data.into_iter().collect();
            embeddings.sort_by_key(|e| e.index);

            for embedding_data in embeddings {
                self.validate_embedding(&embedding_data.embedding)?;
                all_embeddings.push(embedding_data.embedding);
            }
        }

        Ok(all_embeddings)
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_creation() {
        let service = RemoteEmbeddingService::new(
            "test-key".to_string(),
            Some("voyage-3-large".to_string()),
            None,
        );

        assert!(service.is_ok());
        let service = service.unwrap();
        assert_eq!(service.dimensions(), VOYAGE_EMBEDDING_DIM);
        assert_eq!(service.model_name(), "voyage-3-large");
    }

    #[test]
    fn test_empty_api_key_error() {
        let result = RemoteEmbeddingService::new("".to_string(), None, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_text() {
        let service = RemoteEmbeddingService::new("test-key".to_string(), None, None).unwrap();

        assert!(service.validate_text("valid text").is_ok());
        assert!(service.validate_text("").is_err());
    }

    #[test]
    fn test_validate_embedding() {
        let service = RemoteEmbeddingService::new("test-key".to_string(), None, None).unwrap();

        // Valid embedding
        let valid = vec![0.5; VOYAGE_EMBEDDING_DIM];
        assert!(service.validate_embedding(&valid).is_ok());

        // Wrong dimensions
        let wrong_dims = vec![0.5; 512];
        assert!(service.validate_embedding(&wrong_dims).is_err());

        // NaN values
        let mut nan_embedding = vec![0.5; VOYAGE_EMBEDDING_DIM];
        nan_embedding[0] = f32::NAN;
        assert!(service.validate_embedding(&nan_embedding).is_err());

        // Inf values
        let mut inf_embedding = vec![0.5; VOYAGE_EMBEDDING_DIM];
        inf_embedding[0] = f32::INFINITY;
        assert!(service.validate_embedding(&inf_embedding).is_err());
    }

    // Integration tests (require API key)
    #[tokio::test]
    #[ignore] // Run with: cargo test -- --ignored
    async fn test_embed_single_text() {
        let api_key = std::env::var("VOYAGE_API_KEY").expect("VOYAGE_API_KEY not set");
        let service = RemoteEmbeddingService::new(api_key, None, None).unwrap();

        let embedding = service.embed("Rust programming language").await;
        assert!(embedding.is_ok());

        let embedding = embedding.unwrap();
        assert_eq!(embedding.len(), VOYAGE_EMBEDDING_DIM);

        // Check normalization (approximately unit length)
        let magnitude: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < 0.1);
    }

    #[tokio::test]
    #[ignore]
    async fn test_embed_batch() {
        let api_key = std::env::var("VOYAGE_API_KEY").expect("VOYAGE_API_KEY not set");
        let service = RemoteEmbeddingService::new(api_key, None, None).unwrap();

        let texts = vec![
            "Rust programming language",
            "Python data science",
            "JavaScript web development",
        ];

        let embeddings = service.embed_batch(&texts).await;
        assert!(embeddings.is_ok());

        let embeddings = embeddings.unwrap();
        assert_eq!(embeddings.len(), 3);

        for embedding in &embeddings {
            assert_eq!(embedding.len(), VOYAGE_EMBEDDING_DIM);
        }
    }

    #[tokio::test]
    #[ignore]
    async fn test_empty_text_error() {
        let api_key = std::env::var("VOYAGE_API_KEY").expect("VOYAGE_API_KEY not set");
        let service = RemoteEmbeddingService::new(api_key, None, None).unwrap();

        let result = service.embed("").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[ignore]
    async fn test_invalid_api_key() {
        let service = RemoteEmbeddingService::new("invalid-key".to_string(), None, None).unwrap();

        let result = service.embed("test text").await;
        assert!(result.is_err());

        match result.unwrap_err() {
            MnemosyneError::AuthenticationError(_) => (),
            _ => panic!("Expected AuthenticationError"),
        }
    }

    #[tokio::test]
    async fn test_batch_chunking() {
        // Test that large batches are split correctly
        let service = RemoteEmbeddingService::new("test-key".to_string(), None, None).unwrap();

        let large_batch: Vec<&str> = (0..MAX_BATCH_SIZE * 2)
            .map(|i| if i % 2 == 0 { "text A" } else { "text B" })
            .collect();

        // This will fail with invalid API key, but we're testing the chunking logic doesn't panic
        let _result = service.embed_batch(&large_batch).await;
    }
}
