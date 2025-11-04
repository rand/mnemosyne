//! Core types and traits for artifact management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Artifact type classification matching memory types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactType {
    Constitution,
    FeatureSpec,
    ImplementationPlan,
    TaskBreakdown,
    QualityChecklist,
    Clarification,
}

/// Artifact status in the workflow
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArtifactStatus {
    Draft,
    Approved,
    Implemented,
    Deprecated,
}

/// Semantic versioning for artifacts
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl ArtifactVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self {
            major,
            minor,
            patch,
        }
    }

    pub fn initial() -> Self {
        Self::new(1, 0, 0)
    }

    pub fn bump_major(&mut self) {
        self.major += 1;
        self.minor = 0;
        self.patch = 0;
    }

    pub fn bump_minor(&mut self) {
        self.minor += 1;
        self.patch = 0;
    }

    pub fn bump_patch(&mut self) {
        self.patch += 1;
    }
}

impl std::fmt::Display for ArtifactVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

impl std::str::FromStr for ArtifactVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return Err(format!("Invalid version format: {}", s));
        }

        let major = parts[0]
            .parse()
            .map_err(|_| format!("Invalid major version: {}", parts[0]))?;
        let minor = parts[1]
            .parse()
            .map_err(|_| format!("Invalid minor version: {}", parts[1]))?;
        let patch = parts[2]
            .parse()
            .map_err(|_| format!("Invalid patch version: {}", parts[2]))?;

        Ok(Self::new(major, minor, patch))
    }
}

/// Common metadata for all artifacts (YAML frontmatter)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactMetadata {
    /// Type of artifact
    #[serde(rename = "type")]
    pub artifact_type: ArtifactType,

    /// Unique identifier (e.g., feature ID, "constitution")
    pub id: String,

    /// Human-readable name
    pub name: String,

    /// Git branch associated with this artifact
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,

    /// Semantic version
    pub version: ArtifactVersion,

    /// Current status
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<ArtifactStatus>,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    /// Mnemosyne memory ID for this artifact
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_id: Option<String>,

    /// Reference to parent/related memory IDs
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub references: Vec<String>,
}

impl ArtifactMetadata {
    pub fn new(
        artifact_type: ArtifactType,
        id: String,
        name: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            artifact_type,
            id,
            name,
            branch: None,
            version: ArtifactVersion::initial(),
            status: Some(ArtifactStatus::Draft),
            created_at: now,
            updated_at: now,
            memory_id: None,
            references: Vec::new(),
        }
    }

    pub fn update_timestamp(&mut self) {
        self.updated_at = Utc::now();
    }
}

/// Core artifact trait
pub trait Artifact {
    /// Get artifact metadata
    fn metadata(&self) -> &ArtifactMetadata;

    /// Get mutable metadata
    fn metadata_mut(&mut self) -> &mut ArtifactMetadata;

    /// Get artifact content as markdown
    fn content(&self) -> &str;

    /// Get artifact file path
    fn file_path(&self) -> PathBuf;

    /// Serialize to YAML frontmatter + markdown
    fn to_markdown(&self) -> Result<String, crate::error::MnemosyneError>;

    /// Parse from YAML frontmatter + markdown
    fn from_markdown(content: &str) -> Result<Self, crate::error::MnemosyneError>
    where
        Self: Sized;

    /// Validate artifact structure
    fn validate(&self) -> Result<(), crate::error::MnemosyneError> {
        // Default validation - can be overridden
        if self.metadata().id.is_empty() {
            return Err(crate::error::MnemosyneError::Other(
                "Artifact ID cannot be empty".to_string(),
            ));
        }
        if self.content().is_empty() {
            return Err(crate::error::MnemosyneError::Other(
                "Artifact content cannot be empty".to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_artifact_version_display() {
        let version = ArtifactVersion::new(1, 2, 3);
        assert_eq!(version.to_string(), "1.2.3");
    }

    #[test]
    fn test_artifact_version_parse() {
        let version: ArtifactVersion = "1.2.3".parse().unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
    }

    #[test]
    fn test_artifact_version_bump() {
        let mut version = ArtifactVersion::initial();
        assert_eq!(version.to_string(), "1.0.0");

        version.bump_patch();
        assert_eq!(version.to_string(), "1.0.1");

        version.bump_minor();
        assert_eq!(version.to_string(), "1.1.0");

        version.bump_major();
        assert_eq!(version.to_string(), "2.0.0");
    }

    #[test]
    fn test_metadata_creation() {
        let metadata = ArtifactMetadata::new(
            ArtifactType::FeatureSpec,
            "test-feature".to_string(),
            "Test Feature".to_string(),
        );

        assert_eq!(metadata.artifact_type, ArtifactType::FeatureSpec);
        assert_eq!(metadata.id, "test-feature");
        assert_eq!(metadata.version.to_string(), "1.0.0");
        assert_eq!(metadata.status, Some(ArtifactStatus::Draft));
    }
}
