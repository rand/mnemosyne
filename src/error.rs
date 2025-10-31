//! Error types for the Mnemosyne memory system
//!
//! This module provides comprehensive error handling using thiserror for
//! structured error definitions and anyhow for error propagation.

use thiserror::Error;

/// Main error type for Mnemosyne operations
#[derive(Error, Debug)]
pub enum MnemosyneError {
    /// Database operation failed
    #[error("Database error: {0}")]
    Database(String),

    /// Database migration failed
    #[error("Migration error: {0}")]
    Migration(String),

    /// LLM API request failed
    #[error("LLM API error: {0}")]
    LlmApi(String),

    /// LLM API timeout
    #[error("LLM API timeout after {0}s")]
    LlmTimeout(u64),

    /// LLM API retry limit exceeded
    #[error("LLM API retry limit exceeded after {0} attempts: {1}")]
    LlmRetryExhausted(u32, String),

    /// Python interop error (PyO3)
    #[error("Python interop error: {0}")]
    PythonInterop(String),

    /// Embedding generation failed
    #[error("Embedding error: {0}")]
    Embedding(String),

    /// Embedding service error (replaces Embedding for v2.0)
    #[error("Embedding error: {0}")]
    EmbeddingError(String),

    /// Authentication error (invalid API key, etc.)
    #[error("Authentication error: {0}")]
    AuthenticationError(String),

    /// Rate limit exceeded
    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    /// Network communication error
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Validation error (invalid input)
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Invalid memory ID format
    #[error("Invalid memory ID: {0}")]
    InvalidMemoryId(#[from] uuid::Error),

    /// Invalid ID format (generic)
    #[error("Invalid ID: {0}")]
    InvalidId(String),

    /// Memory not found
    #[error("Memory not found: {0}")]
    MemoryNotFound(String),

    /// Invalid namespace
    #[error("Invalid namespace: {0}")]
    InvalidNamespace(String),

    /// Invalid agent role
    #[error("Invalid agent role: {0}")]
    InvalidAgentRole(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(#[from] config::ConfigError),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// HTTP request error
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// MCP protocol error
    #[error("MCP protocol error: {0}")]
    McpProtocol(String),

    /// Invalid operation (e.g., updating archived memory)
    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    /// Resource already exists
    #[error("Resource already exists: {0}")]
    AlreadyExists(String),

    /// Permission denied for access control
    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    /// Access control operation failed
    #[error("Access control error: {0}")]
    AccessControl(String),

    /// Audit log operation failed
    #[error("Audit log error: {0}")]
    AuditLog(String),

    /// Actor system error (Ractor)
    #[error("Actor error: {0}")]
    ActorError(String),

    /// Branch assignment conflict
    #[error("Branch conflict: {0}")]
    BranchConflict(String),

    /// Resource not found (generic)
    #[error("Not found: {0}")]
    NotFound(String),

    /// Generic error with context
    #[error("{0}")]
    Other(String),
}

/// Result type alias for Mnemosyne operations
pub type Result<T> = std::result::Result<T, MnemosyneError>;

/// Convert anyhow::Error to MnemosyneError
impl From<anyhow::Error> for MnemosyneError {
    fn from(err: anyhow::Error) -> Self {
        MnemosyneError::Other(err.to_string())
    }
}

/// Convert libsql::Error to MnemosyneError
impl From<libsql::Error> for MnemosyneError {
    fn from(err: libsql::Error) -> Self {
        MnemosyneError::Database(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = MnemosyneError::MemoryNotFound("test-id".to_string());
        assert_eq!(err.to_string(), "Memory not found: test-id");
    }

    #[test]
    fn test_error_conversion() {
        let uuid_err = uuid::Uuid::parse_str("invalid");
        assert!(uuid_err.is_err());

        let mnemosyne_err: MnemosyneError = uuid_err.unwrap_err().into();
        assert!(matches!(mnemosyne_err, MnemosyneError::InvalidMemoryId(_)));
    }
}
