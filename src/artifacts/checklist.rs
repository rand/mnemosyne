//! Quality checklist artifact

use super::types::{Artifact, ArtifactMetadata, ArtifactType};
use super::storage::{parse_frontmatter, serialize_frontmatter};
use crate::error::{MnemosyneError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Quality checklist for validation and acceptance criteria
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityChecklist {
    #[serde(flatten)]
    pub metadata: ArtifactMetadata,

    /// Feature ID this checklist validates
    pub feature_id: String,

    /// Checklist sections
    pub sections: Vec<ChecklistSection>,

    /// Full markdown content
    #[serde(skip)]
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistSection {
    /// Section name
    pub name: String,

    /// Checklist items
    pub items: Vec<ChecklistItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChecklistItem {
    /// Item description
    pub description: String,

    /// Completion status
    #[serde(default)]
    pub completed: bool,

    /// Notes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl QualityChecklist {
    pub fn new(feature_id: String, checklist_name: String) -> Self {
        let metadata = ArtifactMetadata::new(
            ArtifactType::QualityChecklist,
            format!("{}-checklist", feature_id),
            checklist_name,
        );

        Self {
            metadata,
            feature_id,
            sections: Vec::new(),
            content: String::new(),
        }
    }

    pub fn add_section(&mut self, section: ChecklistSection) {
        self.sections.push(section);
        self.metadata.update_timestamp();
    }

    pub fn completion_percentage(&self) -> f32 {
        let total: usize = self.sections.iter().map(|s| s.items.len()).sum();
        if total == 0 {
            return 0.0;
        }

        let completed: usize = self
            .sections
            .iter()
            .flat_map(|s| &s.items)
            .filter(|item| item.completed)
            .count();

        (completed as f32 / total as f32) * 100.0
    }
}

impl Artifact for QualityChecklist {
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
        PathBuf::from(format!("checklists/{}-checklist.md", self.feature_id))
    }

    fn to_markdown(&self) -> Result<String> {
        let frontmatter = serde_yaml::to_value(&self.metadata)
            .map_err(|e| MnemosyneError::Other(format!("Failed to serialize metadata: {}", e)))?;

        let content = if self.content.is_empty() {
            // Generate default content
            let mut md = format!("# Quality Checklist: {}\n\n", self.metadata.name);

            let completion = self.completion_percentage();
            md.push_str(&format!("**Completion**: {:.1}%\n\n", completion));

            for section in &self.sections {
                md.push_str(&format!("## {}\n\n", section.name));
                for item in &section.items {
                    let checkbox = if item.completed { "x" } else { " " };
                    md.push_str(&format!("- [{}] {}\n", checkbox, item.description));

                    if let Some(ref notes) = item.notes {
                        md.push_str(&format!("  - *Note*: {}\n", notes));
                    }
                }
                md.push_str("\n");
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
            sections: Vec::new(),
            content: markdown,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_checklist_creation() {
        let checklist = QualityChecklist::new(
            "user-auth".to_string(),
            "User Auth Checklist".to_string(),
        );

        assert_eq!(checklist.feature_id, "user-auth");
        assert_eq!(checklist.completion_percentage(), 0.0);
    }

    #[test]
    fn test_completion_percentage() {
        let mut checklist = QualityChecklist::new(
            "user-auth".to_string(),
            "User Auth Checklist".to_string(),
        );

        checklist.add_section(ChecklistSection {
            name: "Tests".to_string(),
            items: vec![
                ChecklistItem {
                    description: "Unit tests".to_string(),
                    completed: true,
                    notes: None,
                },
                ChecklistItem {
                    description: "Integration tests".to_string(),
                    completed: false,
                    notes: None,
                },
            ],
        });

        assert_eq!(checklist.completion_percentage(), 50.0);
    }
}
