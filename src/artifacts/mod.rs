//! Artifact Management for Specification Workflow
//!
//! This module provides structured artifact storage for specification-driven development,
//! inspired by GitHub's Spec-Kit (adapted for Mnemosyne). Artifacts are markdown files with YAML frontmatter that
//! integrate with Mnemosyne's memory system.
//!
//! # Artifact Types
//!
//! - **Constitution**: Project principles and quality gates
//! - **FeatureSpec**: Feature specifications with user scenarios
//! - **ImplementationPlan**: Technical architecture and design
//! - **TaskBreakdown**: Task lists with dependencies
//! - **QualityChecklist**: Validation and acceptance criteria
//! - **Clarification**: Resolved ambiguities
//!
//! # Architecture
//!
//! Artifacts are stored as:
//! 1. **Files**: Human-readable markdown in `.mnemosyne/artifacts/`
//! 2. **Memory Entries**: Searchable entries in database with `artifact_path` field
//! 3. **Graph Links**: Relationships between artifacts and code
//!
//! # Example
//!
//! ```no_run
//! use mnemosyne_core::artifacts::{Constitution, ArtifactStorage};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let storage = ArtifactStorage::new(".mnemosyne/artifacts")?;
//!
//! // Create constitution
//! let constitution = Constitution::new(
//!     "mnemosyne".to_string(),
//!     vec!["Performance First".to_string()],
//! );
//!
//! // Save to file and create memory entry
//! storage.save_constitution(&constitution).await?;
//! # Ok(())
//! # }
//! ```

pub mod types;
pub mod storage;
pub mod memory_link;
pub mod constitution;
pub mod feature_spec;
pub mod plan;
pub mod tasks;
pub mod checklist;
pub mod clarification;

// Re-export core types
pub use types::{
    Artifact, ArtifactMetadata, ArtifactType, ArtifactStatus, ArtifactVersion,
};
pub use storage::{ArtifactStorage, parse_frontmatter, serialize_frontmatter};
pub use memory_link::MemoryLinker;
pub use constitution::Constitution;
pub use feature_spec::FeatureSpec;
pub use plan::ImplementationPlan;
pub use tasks::TaskBreakdown;
pub use checklist::QualityChecklist;
pub use clarification::Clarification;
