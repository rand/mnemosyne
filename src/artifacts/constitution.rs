//! Project constitution artifact
#![allow(clippy::useless_format, clippy::single_char_add_str)]

use super::types::{Artifact, ArtifactMetadata, ArtifactType};
use super::storage::{parse_frontmatter, serialize_frontmatter};
use crate::error::{MnemosyneError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Parse numbered list from markdown section
///
/// Extracts items from a section like:
/// ```markdown
/// ## Section Name
/// 1. First item
/// 2. Second item
/// ```
fn parse_numbered_list(markdown: &str, section_name: &str) -> Vec<String> {
    let mut items = Vec::new();
    let section_header = format!("## {}", section_name);

    if let Some(start_idx) = markdown.find(&section_header) {
        let after_header = &markdown[start_idx + section_header.len()..];

        for line in after_header.lines() {
            let trimmed = line.trim();

            // Stop at next section header
            if trimmed.starts_with("##") {
                break;
            }

            // Match numbered list items: "1. Item" or "1) Item"
            if let Some(item) = trimmed.strip_prefix(|c: char| c.is_numeric()) {
                if let Some(content) = item.strip_prefix('.').or_else(|| item.strip_prefix(')')) {
                    let cleaned = content.trim();
                    if !cleaned.is_empty() {
                        items.push(cleaned.to_string());
                    }
                }
            }
        }
    }

    items
}

/// Parse bullet list from markdown section
///
/// Extracts items from a section like:
/// ```markdown
/// ## Section Name
/// - First item
/// - Second item
/// ```
fn parse_bullet_list(markdown: &str, section_name: &str) -> Vec<String> {
    let mut items = Vec::new();
    let section_header = format!("## {}", section_name);

    if let Some(start_idx) = markdown.find(&section_header) {
        let after_header = &markdown[start_idx + section_header.len()..];

        for line in after_header.lines() {
            let trimmed = line.trim();

            // Stop at next section header
            if trimmed.starts_with("##") {
                break;
            }

            // Match bullet items: "- Item" or "* Item" or "[ ] Item" (checkbox)
            if let Some(content) = trimmed.strip_prefix('-').or_else(|| trimmed.strip_prefix('*')) {
                let mut cleaned = content.trim();

                // Strip checkboxes if present
                if let Some(after_checkbox) = cleaned.strip_prefix("[ ]").or_else(|| cleaned.strip_prefix("[x]")) {
                    cleaned = after_checkbox.trim();
                }

                if !cleaned.is_empty() {
                    items.push(cleaned.to_string());
                }
            }
        }
    }

    items
}

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

        // Parse principles, quality gates, and constraints from markdown
        let principles = parse_numbered_list(&markdown, "Core Principles");
        let quality_gates = parse_bullet_list(&markdown, "Quality Gates");
        let constraints = parse_bullet_list(&markdown, "Constraints");

        Ok(Self {
            metadata,
            principles,
            quality_gates,
            constraints,
            content: markdown.to_string(),
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

    #[test]
    fn test_constitution_round_trip_simple() {
        // Create constitution
        let original = Constitution::new(
            "TestProject".to_string(),
            vec!["Principle One".to_string(), "Principle Two".to_string()],
        );

        // Serialize to markdown
        let markdown = original.to_markdown().unwrap();

        // Deserialize back
        let loaded = Constitution::from_markdown(&markdown).unwrap();

        // Verify principles were preserved
        assert_eq!(loaded.principles.len(), 2);
        assert_eq!(loaded.principles[0], "Principle One");
        assert_eq!(loaded.principles[1], "Principle Two");
        assert_eq!(loaded.metadata.id, "project-constitution");
    }

    #[test]
    fn test_constitution_round_trip_complete() {
        // Create constitution with all fields
        let mut original = Constitution::new(
            "CompleteProject".to_string(),
            vec![
                "Performance First: Sub-200ms latency".to_string(),
                "Type Safety: Leverage strong typing".to_string(),
                "Developer Experience: Clear error messages".to_string(),
            ],
        );
        original.add_quality_gate("90%+ test coverage".to_string());
        original.add_quality_gate("All warnings addressed".to_string());
        original.add_constraint("Rust-only for core".to_string());
        original.add_constraint("No external APIs".to_string());

        // Serialize to markdown
        let markdown = original.to_markdown().unwrap();

        // Deserialize back
        let loaded = Constitution::from_markdown(&markdown).unwrap();

        // Verify all fields preserved
        assert_eq!(loaded.principles.len(), 3);
        assert_eq!(loaded.principles[0], "Performance First: Sub-200ms latency");
        assert_eq!(loaded.principles[1], "Type Safety: Leverage strong typing");
        assert_eq!(loaded.principles[2], "Developer Experience: Clear error messages");

        assert_eq!(loaded.quality_gates.len(), 2);
        assert_eq!(loaded.quality_gates[0], "90%+ test coverage");
        assert_eq!(loaded.quality_gates[1], "All warnings addressed");

        assert_eq!(loaded.constraints.len(), 2);
        assert_eq!(loaded.constraints[0], "Rust-only for core");
        assert_eq!(loaded.constraints[1], "No external APIs");
    }

    #[test]
    fn test_parse_numbered_list() {
        let markdown = r#"
## Core Principles

1. First principle
2. Second principle
3. Third principle

## Next Section
"#;

        let items = parse_numbered_list(markdown, "Core Principles");
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], "First principle");
        assert_eq!(items[1], "Second principle");
        assert_eq!(items[2], "Third principle");
    }

    #[test]
    fn test_parse_bullet_list() {
        let markdown = r#"
## Quality Gates

- First gate
- Second gate
- Third gate

## Next Section
"#;

        let items = parse_bullet_list(markdown, "Quality Gates");
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], "First gate");
        assert_eq!(items[1], "Second gate");
        assert_eq!(items[2], "Third gate");
    }

    #[test]
    fn test_parse_bullet_list_with_checkboxes() {
        let markdown = r#"
## Quality Gates

- [ ] Unchecked item
- [x] Checked item
- Regular item

## Next Section
"#;

        let items = parse_bullet_list(markdown, "Quality Gates");
        assert_eq!(items.len(), 3);
        assert_eq!(items[0], "Unchecked item");
        assert_eq!(items[1], "Checked item");
        assert_eq!(items[2], "Regular item");
    }
}
