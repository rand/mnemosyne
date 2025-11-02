//! Project constitution artifact

use super::types::{Artifact, ArtifactMetadata, ArtifactType};
use super::storage::{parse_frontmatter, serialize_frontmatter};
use crate::error::{MnemosyneError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Project constitution defining principles and quality gates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constitution {
    #[serde(flatten)]
    pub metadata: ArtifactMetadata,

    /// Core principles
    pub principles: Vec<String>,

    /// Quality gates that must be met
    pub quality_gates: Vec<String>,

    /// Constraints that must be satisfied
    pub constraints: Vec<String>,

    /// Full markdown content
    #[serde(skip)]
    pub content: String,
}

impl Constitution {
    pub fn new(project_name: String, principles: Vec<String>) -> Self {
        let metadata = ArtifactMetadata::new(
            ArtifactType::Constitution,
            "project-constitution".to_string(),
            format!("{} Constitution", project_name),
        );

        Self {
            metadata,
            principles,
            quality_gates: Vec::new(),
            constraints: Vec::new(),
            content: String::new(),
        }
    }

    /// Create a constitution with builder pattern
    pub fn builder(project_name: String) -> ConstitutionBuilder {
        ConstitutionBuilder::new(project_name)
    }

    pub fn add_quality_gate(&mut self, gate: String) {
        self.quality_gates.push(gate);
        self.metadata.update_timestamp();
    }

    pub fn add_constraint(&mut self, constraint: String) {
        self.constraints.push(constraint);
        self.metadata.update_timestamp();
    }

    pub fn add_principle(&mut self, principle: String) {
        self.principles.push(principle);
        self.metadata.update_timestamp();
    }
}

/// Builder for Constitution to enable fluent API
pub struct ConstitutionBuilder {
    project_name: String,
    principles: Vec<String>,
    quality_gates: Vec<String>,
    constraints: Vec<String>,
}

impl ConstitutionBuilder {
    pub fn new(project_name: String) -> Self {
        Self {
            project_name,
            principles: Vec::new(),
            quality_gates: Vec::new(),
            constraints: Vec::new(),
        }
    }

    pub fn principle(mut self, principle: impl Into<String>) -> Self {
        self.principles.push(principle.into());
        self
    }

    pub fn quality_gate(mut self, gate: impl Into<String>) -> Self {
        self.quality_gates.push(gate.into());
        self
    }

    pub fn constraint(mut self, constraint: impl Into<String>) -> Self {
        self.constraints.push(constraint.into());
        self
    }

    pub fn build(self) -> Constitution {
        let mut constitution = Constitution::new(self.project_name, self.principles);
        constitution.quality_gates = self.quality_gates;
        constitution.constraints = self.constraints;
        constitution
    }
}

impl Artifact for Constitution {
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
        PathBuf::from("constitution/project-constitution.md")
    }

    fn to_markdown(&self) -> Result<String> {
        let frontmatter = serde_yaml::to_value(&self.metadata)
            .map_err(|e| MnemosyneError::Other(format!("Failed to serialize metadata: {}", e)))?;

        let content = if self.content.is_empty() {
            // Generate default content
            let mut md = format!("# Project Constitution\n\n");
            md.push_str("## Core Principles\n\n");
            for (i, principle) in self.principles.iter().enumerate() {
                md.push_str(&format!("{}. {}\n", i + 1, principle));
            }

            if !self.quality_gates.is_empty() {
                md.push_str("\n## Quality Gates\n\n");
                for gate in &self.quality_gates {
                    md.push_str(&format!("- {}\n", gate));
                }
            }

            if !self.constraints.is_empty() {
                md.push_str("\n## Constraints\n\n");
                for constraint in &self.constraints {
                    md.push_str(&format!("- {}\n", constraint));
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

        // Parse principles from markdown (simple extraction)
        let principles = Vec::new(); // TODO: Parse from markdown
        let quality_gates = Vec::new();
        let constraints = Vec::new();

        Ok(Self {
            metadata,
            principles,
            quality_gates,
            constraints,
            content: markdown,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constitution_creation() {
        let constitution = Constitution::new(
            "Mnemosyne".to_string(),
            vec!["Performance First".to_string()],
        );

        assert_eq!(constitution.metadata.id, "project-constitution");
        assert_eq!(constitution.principles.len(), 1);
    }

    #[test]
    fn test_constitution_to_markdown() {
        let mut constitution = Constitution::new(
            "Mnemosyne".to_string(),
            vec!["Performance First".to_string()],
        );
        constitution.add_quality_gate("Sub-200ms latency".to_string());

        let markdown = constitution.to_markdown().unwrap();
        assert!(markdown.contains("---"));
        assert!(markdown.contains("type: constitution"));
        assert!(markdown.contains("# Project Constitution"));
        assert!(markdown.contains("Performance First"));
    }
}
