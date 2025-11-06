//! Quality checklist artifact
#![allow(clippy::useless_format, clippy::single_char_add_str)]

use super::storage::{parse_frontmatter, serialize_frontmatter};
use super::types::{Artifact, ArtifactMetadata, ArtifactType};
use crate::error::{MnemosyneError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Parse checklist sections from markdown
///
/// Extracts sections with format:
/// ## Section Name
/// - [x] Item description
///   - *Note*: Item notes
fn parse_checklist_sections(markdown: &str) -> Vec<ChecklistSection> {
    let mut sections = Vec::new();
    let lines: Vec<&str> = markdown.lines().collect();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();

        // Look for section header: "## Section Name"
        if let Some(header) = line.strip_prefix("##") {
            let section_name = header.trim();

            // Skip "Completion" line or other special headers
            if section_name.starts_with("*") {
                i += 1;
                continue;
            }

            let mut items = Vec::new();

            // Parse items in this section
            i += 1;
            while i < lines.len() {
                let item_line = lines[i];
                let trimmed = item_line.trim();

                // Stop at next section
                if trimmed.starts_with("##") {
                    break;
                }

                // Parse checklist item
                if trimmed.starts_with("- [") {
                    // Check completion status
                    let completed = trimmed.contains("- [x]");

                    // Extract description
                    let after_checkbox = if completed {
                        trimmed.strip_prefix("- [x]")
                    } else {
                        trimmed.strip_prefix("- [ ]")
                    };

                    if let Some(description) = after_checkbox {
                        let description = description.trim().to_string();

                        // Check if next line is a note
                        let mut notes = None;
                        if i + 1 < lines.len() {
                            let next_line = lines[i + 1].trim();
                            if let Some(note_text) = next_line.strip_prefix("- *Note*:") {
                                notes = Some(note_text.trim().to_string());
                                i += 1; // Skip note line
                            }
                        }

                        items.push(ChecklistItem {
                            description,
                            completed,
                            notes,
                        });
                    }
                }

                i += 1;
            }

            if !items.is_empty() {
                sections.push(ChecklistSection {
                    name: section_name.to_string(),
                    items,
                });
            }
            continue;
        }

        i += 1;
    }

    sections
}

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

        // Extract feature_id from metadata.id which has format "{feature_id}-checklist"
        let feature_id = if let Some(suffix_pos) = metadata.id.rfind("-checklist") {
            metadata.id[..suffix_pos].to_string()
        } else {
            metadata.id.clone()
        };

        // Parse sections from markdown
        let sections = parse_checklist_sections(&markdown);

        Ok(Self {
            metadata,
            feature_id,
            sections,
            content: markdown.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quality_checklist_creation() {
        let checklist =
            QualityChecklist::new("user-auth".to_string(), "User Auth Checklist".to_string());

        assert_eq!(checklist.feature_id, "user-auth");
        assert_eq!(checklist.completion_percentage(), 0.0);
    }

    #[test]
    fn test_completion_percentage() {
        let mut checklist =
            QualityChecklist::new("user-auth".to_string(), "User Auth Checklist".to_string());

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

    #[test]
    fn test_checklist_round_trip_simple() {
        // Create simple checklist
        let mut original = QualityChecklist::new(
            "test-feature".to_string(),
            "Test Feature Checklist".to_string(),
        );

        original.add_section(ChecklistSection {
            name: "Tests".to_string(),
            items: vec![
                ChecklistItem {
                    description: "Unit tests passing".to_string(),
                    completed: true,
                    notes: None,
                },
                ChecklistItem {
                    description: "Integration tests passing".to_string(),
                    completed: false,
                    notes: None,
                },
            ],
        });

        // Serialize to markdown
        let markdown = original.to_markdown().unwrap();

        // Deserialize back
        let loaded = QualityChecklist::from_markdown(&markdown).unwrap();

        // Verify fields preserved
        assert_eq!(loaded.feature_id, "test-feature");
        assert_eq!(loaded.sections.len(), 1);
        assert_eq!(loaded.sections[0].name, "Tests");
        assert_eq!(loaded.sections[0].items.len(), 2);
        assert_eq!(
            loaded.sections[0].items[0].description,
            "Unit tests passing"
        );
        assert!(loaded.sections[0].items[0].completed);
        assert_eq!(
            loaded.sections[0].items[1].description,
            "Integration tests passing"
        );
        assert!(!loaded.sections[0].items[1].completed);
    }

    #[test]
    fn test_checklist_round_trip_complete() {
        // Create complete checklist with multiple sections and notes
        let mut original = QualityChecklist::new(
            "user-auth".to_string(),
            "User Authentication Checklist".to_string(),
        );

        original.add_section(ChecklistSection {
            name: "Functional Requirements".to_string(),
            items: vec![
                ChecklistItem {
                    description: "Token generation works".to_string(),
                    completed: true,
                    notes: Some("Using RS256 algorithm".to_string()),
                },
                ChecklistItem {
                    description: "Token validation works".to_string(),
                    completed: true,
                    notes: None,
                },
                ChecklistItem {
                    description: "Refresh token flow works".to_string(),
                    completed: false,
                    notes: Some("Waiting on cookie implementation".to_string()),
                },
            ],
        });

        original.add_section(ChecklistSection {
            name: "Testing".to_string(),
            items: vec![
                ChecklistItem {
                    description: "Unit tests: 90%+ coverage".to_string(),
                    completed: true,
                    notes: None,
                },
                ChecklistItem {
                    description: "Integration tests passing".to_string(),
                    completed: false,
                    notes: None,
                },
            ],
        });

        original.add_section(ChecklistSection {
            name: "Security".to_string(),
            items: vec![ChecklistItem {
                description: "No secrets in git".to_string(),
                completed: true,
                notes: Some("Keys stored in .env".to_string()),
            }],
        });

        // Serialize to markdown
        let markdown = original.to_markdown().unwrap();

        // Deserialize back
        let loaded = QualityChecklist::from_markdown(&markdown).unwrap();

        // Verify all fields preserved
        assert_eq!(loaded.feature_id, "user-auth");
        assert_eq!(loaded.sections.len(), 3);

        // Verify Functional Requirements section
        assert_eq!(loaded.sections[0].name, "Functional Requirements");
        assert_eq!(loaded.sections[0].items.len(), 3);
        assert_eq!(
            loaded.sections[0].items[0].description,
            "Token generation works"
        );
        assert!(loaded.sections[0].items[0].completed);
        assert_eq!(
            loaded.sections[0].items[0].notes,
            Some("Using RS256 algorithm".to_string())
        );
        assert_eq!(
            loaded.sections[0].items[1].description,
            "Token validation works"
        );
        assert!(loaded.sections[0].items[1].completed);
        assert_eq!(loaded.sections[0].items[1].notes, None);
        assert_eq!(
            loaded.sections[0].items[2].description,
            "Refresh token flow works"
        );
        assert!(!loaded.sections[0].items[2].completed);
        assert_eq!(
            loaded.sections[0].items[2].notes,
            Some("Waiting on cookie implementation".to_string())
        );

        // Verify Testing section
        assert_eq!(loaded.sections[1].name, "Testing");
        assert_eq!(loaded.sections[1].items.len(), 2);
        assert!(loaded.sections[1].items[0].completed);
        assert!(!loaded.sections[1].items[1].completed);

        // Verify Security section
        assert_eq!(loaded.sections[2].name, "Security");
        assert_eq!(loaded.sections[2].items.len(), 1);
        assert_eq!(
            loaded.sections[2].items[0].notes,
            Some("Keys stored in .env".to_string())
        );
    }

    #[test]
    fn test_parse_checklist_sections() {
        let markdown = r#"
# Quality Checklist

**Completion**: 50.0%

## Functional Requirements

- [x] Feature A works
  - *Note*: Tested manually
- [ ] Feature B works

## Testing

- [x] Unit tests passing
- [ ] Integration tests passing
"#;

        let sections = parse_checklist_sections(markdown);
        assert_eq!(sections.len(), 2);

        assert_eq!(sections[0].name, "Functional Requirements");
        assert_eq!(sections[0].items.len(), 2);
        assert_eq!(sections[0].items[0].description, "Feature A works");
        assert!(sections[0].items[0].completed);
        assert_eq!(
            sections[0].items[0].notes,
            Some("Tested manually".to_string())
        );
        assert_eq!(sections[0].items[1].description, "Feature B works");
        assert!(!sections[0].items[1].completed);
        assert_eq!(sections[0].items[1].notes, None);

        assert_eq!(sections[1].name, "Testing");
        assert_eq!(sections[1].items.len(), 2);
        assert!(sections[1].items[0].completed);
        assert!(!sections[1].items[1].completed);
    }
}
