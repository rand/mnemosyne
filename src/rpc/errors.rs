//! Error conversion from MnemosyneError to gRPC Status

use crate::error::MnemosyneError;
use tonic::{Code, Status};

impl From<MnemosyneError> for Status {
    fn from(err: MnemosyneError) -> Self {
        let (code, message) = match &err {
            MnemosyneError::Database(msg) => (Code::Internal, format!("Database error: {}", msg)),
            MnemosyneError::Migration(msg) => (Code::Internal, format!("Migration error: {}", msg)),
            MnemosyneError::LlmApi(msg) => (Code::Unavailable, format!("LLM API error: {}", msg)),
            MnemosyneError::LlmTimeout(secs) => (
                Code::DeadlineExceeded,
                format!("LLM request timed out after {} seconds", secs),
            ),
            MnemosyneError::LlmRetryExhausted(attempts, msg) => (
                Code::Unavailable,
                format!("LLM retry exhausted after {} attempts: {}", attempts, msg),
            ),
            MnemosyneError::PythonInterop(msg) => {
                (Code::Internal, format!("Python interop error: {}", msg))
            }
            MnemosyneError::Embedding(msg) => (Code::Internal, format!("Embedding error: {}", msg)),
            MnemosyneError::EmbeddingError(msg) => {
                (Code::Internal, format!("Embedding error: {}", msg))
            }
            MnemosyneError::AuthenticationError(msg) => (Code::Unauthenticated, msg.clone()),
            MnemosyneError::RateLimitExceeded(msg) => (Code::ResourceExhausted, msg.clone()),
            MnemosyneError::NetworkError(msg) => {
                (Code::Unavailable, format!("Network error: {}", msg))
            }
            MnemosyneError::SerializationError(msg) => (
                Code::InvalidArgument,
                format!("Serialization error: {}", msg),
            ),
            MnemosyneError::ValidationError(msg) => (Code::InvalidArgument, msg.clone()),
            MnemosyneError::InvalidMemoryId(_) => (Code::InvalidArgument, err.to_string()),
            MnemosyneError::InvalidId(id) => (Code::InvalidArgument, format!("Invalid ID: {}", id)),
            MnemosyneError::MemoryNotFound(id) => {
                (Code::NotFound, format!("Memory not found: {}", id))
            }
            MnemosyneError::InvalidNamespace(ns) => {
                (Code::InvalidArgument, format!("Invalid namespace: {}", ns))
            }
            MnemosyneError::InvalidAgentRole(role) => (
                Code::InvalidArgument,
                format!("Invalid agent role: {}", role),
            ),
            MnemosyneError::McpProtocol(msg) => {
                (Code::InvalidArgument, format!("Protocol error: {}", msg))
            }
            MnemosyneError::InvalidOperation(msg) => (Code::FailedPrecondition, msg.clone()),
            MnemosyneError::AlreadyExists(msg) => (Code::AlreadyExists, msg.clone()),
            MnemosyneError::PermissionDenied(msg) => (Code::PermissionDenied, msg.clone()),
            MnemosyneError::ActorError(msg) => (Code::Internal, format!("Actor error: {}", msg)),
            MnemosyneError::BranchConflict(msg) => {
                (Code::Aborted, format!("Branch conflict: {}", msg))
            }
            MnemosyneError::NotFound(msg) => (Code::NotFound, msg.clone()),
            MnemosyneError::Config(err) => (
                Code::FailedPrecondition,
                format!("Configuration error: {}", err),
            ),
            MnemosyneError::Io(err) => (Code::Internal, format!("I/O error: {}", err)),
            MnemosyneError::Serialization(err) => (
                Code::InvalidArgument,
                format!("Serialization error: {}", err),
            ),
            MnemosyneError::Http(err) => (Code::Unavailable, format!("HTTP error: {}", err)),
            MnemosyneError::AccessControl(msg) => (
                Code::PermissionDenied,
                format!("Access control error: {}", msg),
            ),
            MnemosyneError::AuditLog(msg) => (Code::Internal, format!("Audit log error: {}", msg)),
            MnemosyneError::Other(msg) => (Code::Unknown, msg.clone()),
        };

        Status::new(code, message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_conversion() {
        let err = MnemosyneError::MemoryNotFound("test-id".to_string());
        let status: Status = err.into();
        assert_eq!(status.code(), Code::NotFound);
        assert!(status.message().contains("test-id"));
    }

    #[test]
    fn test_authentication_error() {
        let err = MnemosyneError::AuthenticationError("Invalid API key".to_string());
        let status: Status = err.into();
        assert_eq!(status.code(), Code::Unauthenticated);
    }

    #[test]
    fn test_rate_limit_error() {
        let err = MnemosyneError::RateLimitExceeded("Too many requests".to_string());
        let status: Status = err.into();
        assert_eq!(status.code(), Code::ResourceExhausted);
    }
}
