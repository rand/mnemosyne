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

pub mod checklist;
pub mod clarification;
pub mod constitution;
pub mod feature_spec;
pub mod memory_link;
pub mod plan;
pub mod storage;
pub mod tasks;
pub mod types;
pub mod workflow;

// Re-export core types
pub use checklist::{ChecklistItem, ChecklistSection, QualityChecklist};
pub use clarification::{Clarification, ClarificationItem};
pub use constitution::{Constitution, ConstitutionBuilder};
pub use feature_spec::{FeatureSpec, FeatureSpecBuilder, UserScenario};
pub use memory_link::MemoryLinker;
pub use plan::{ArchitectureDecision, ImplementationPlan};
pub use storage::{parse_frontmatter, serialize_frontmatter, ArtifactStorage};
pub use tasks::{Task, TaskBreakdown, TaskPhase};
pub use types::{Artifact, ArtifactMetadata, ArtifactStatus, ArtifactType, ArtifactVersion};
pub use workflow::ArtifactWorkflow;
