//! Implementation plan artifact

use super::types::{Artifact, ArtifactMetadata, ArtifactType};
use super::storage::{parse_frontmatter, serialize_frontmatter};
use crate::error::{MnemosyneError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Implementation plan with technical architecture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImplementationPlan {
    #[serde(flatten)]
    pub metadata: ArtifactMetadata,

    /// Feature ID this plan implements
    pub feature_id: String,

    /// Technical approach summary
    pub approach: String,

    /// Architecture decisions
    pub architecture: Vec<ArchitectureDecision>,

    /// Dependencies (crates, libraries, services)
    pub dependencies: Vec<String>,

    /// Full markdown content
    #[serde(skip)]
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureDecision {
    /// Decision title
    pub title: String,

    /// Chosen approach
    pub decision: String,

    /// Rationale
    pub rationale: String,

    /// Alternatives considered
    #[serde(default)]
    pub alternatives: Vec<String>,
}

impl ImplementationPlan {
    pub fn new(feature_id: String, plan_name: String, approach: String) -> Self {
        let metadata = ArtifactMetadata::new(
            ArtifactType::ImplementationPlan,
            format!("{}-plan", feature_id),
            plan_name,
        );

        Self {
            metadata,
            feature_id,
            approach,
            architecture: Vec::new(),
            dependencies: Vec::new(),
            content: String::new(),
        }
    }

    pub fn add_architecture_decision(&mut self, decision: ArchitectureDecision) {
        self.architecture.push(decision);
        self.metadata.update_timestamp();
    }

    pub fn add_dependency(&mut self, dependency: String) {
        self.dependencies.push(dependency);
        self.metadata.update_timestamp();
    }
}

impl Artifact for ImplementationPlan {
    fn metadata(&self) -> &ArtifactMetadata {
        &self.metadata
    }

    fn metadata_mut(&mut self) -> &mut ArtifactMetadata {
        &mut self.metadata
    }

    fn content(&self) -> &str {
        &self.content
    }

    fn file_path(&self) -> PathBuf {
        PathBuf::from(format!("plans/{}-plan.md", self.feature_id))
    }

    fn to_markdown(&self) -> Result<String> {
        let frontmatter = serde_yaml::to_value(&self.metadata)
            .map_err(|e| MnemosyneError::Other(format!("Failed to serialize metadata: {}", e)))?;

        let content = if self.content.is_empty() {
            // Generate default content
            let mut md = format!("# Implementation Plan: {}\n\n", self.metadata.name);

            md.push_str("## Technical Approach\n\n");
            md.push_str(&format!("{}\n\n", self.approach));

            if !self.architecture.is_empty() {
                md.push_str("## Architecture Decisions\n\n");
                for (i, decision) in self.architecture.iter().enumerate() {
                    md.push_str(&format!("### AD-{:03}: {}\n\n", i + 1, decision.title));
                    md.push_str(&format!("**Decision**: {}\n\n", decision.decision));
                    md.push_str(&format!("**Rationale**: {}\n\n", decision.rationale));

                    if !decision.alternatives.is_empty() {
                        md.push_str("**Alternatives Considered**:\n");
                        for alt in &decision.alternatives {
                            md.push_str(&format!("- {}\n", alt));
                        }
                        md.push_str("\n");
                    }
                }
            }

            if !self.dependencies.is_empty() {
                md.push_str("## Dependencies\n\n");
                for dep in &self.dependencies {
                    md.push_str(&format!("- {}\n", dep));
                }
            }

            md
        } else {
            self.content.clone()
        };

        serialize_frontmatter(&frontmatter, &content)
    }

    fn from_markdown(content: &str) -> Result<Self> {
        let (frontmatter, markdown) = parse_frontmatter(content)?;

        let metadata: ArtifactMetadata = serde_yaml::from_value(frontmatter)
            .map_err(|e| MnemosyneError::Other(format!("Failed to parse metadata: {}", e)))?;

        Ok(Self {
            metadata,
            feature_id: String::new(),
            approach: String::new(),
            architecture: Vec::new(),
            dependencies: Vec::new(),
            content: markdown,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_implementation_plan_creation() {
        let plan = ImplementationPlan::new(
            "user-auth".to_string(),
            "User Auth Plan".to_string(),
            "Use JWT with RS256".to_string(),
        );

        assert_eq!(plan.feature_id, "user-auth");
        assert!(plan.approach.contains("JWT"));
    }
}
