//! Test fixtures for full integration tests

pub mod data_generators;
pub mod ics_fixture;
pub mod mock_embedding_service;
pub mod storage_fixture;

pub use data_generators::*;
pub use ics_fixture::*;
pub use mock_embedding_service::MockEmbeddingService;
pub use storage_fixture::*;
