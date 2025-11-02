//! Clarification artifact for resolving ambiguities

use super::types::{Artifact, ArtifactMetadata, ArtifactType};
use super::storage::{parse_frontmatter, serialize_frontmatter};
use crate::error::{MnemosyneError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Clarification resolving ambiguities in specs or requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clarification {
    #[serde(flatten)]
    pub metadata: ArtifactMetadata,

    /// Feature ID this clarification relates to
    pub feature_id: String,

    /// Clarification questions and answers
    pub items: Vec<ClarificationItem>,

    /// Full markdown content
    #[serde(skip)]
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarificationItem {
    /// Question ID (e.g., "Q001")
    pub id: String,

    /// Question text
    pub question: String,

    /// Context/background for the question
    pub context: String,

    /// Decision/answer
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decision: Option<String>,

    /// Rationale for the decision
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,

    /// Spec sections updated
    #[serde(default)]
    pub spec_updates: Vec<String>,
}

impl Clarification {
    pub fn new(feature_id: String, clarification_name: String) -> Self {
        let metadata = ArtifactMetadata::new(
            ArtifactType::Clarification,
            format!("{}-clarifications", feature_id),
            clarification_name,
        );

        Self {
            metadata,
            feature_id,
            items: Vec::new(),
            content: String::new(),
        }
    }

    pub fn add_item(&mut self, item: ClarificationItem) {
        self.items.push(item);
        self.metadata.update_timestamp();
    }

    pub fn is_complete(&self) -> bool {
        self.items
            .iter()
            .all(|item| item.decision.is_some())
    }
}

impl Artifact for Clarification {
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
        PathBuf::from(format!(
            "clarifications/{}-clarifications.md",
            self.feature_id
        ))
    }

    fn to_markdown(&self) -> Result<String> {
        let frontmatter = serde_yaml::to_value(&self.metadata)
            .map_err(|e| MnemosyneError::Other(format!("Failed to serialize metadata: {}", e)))?;

        let content = if self.content.is_empty() {
            // Generate default content
            let mut md = format!("# Clarifications: {}\n\n", self.metadata.name);

            let complete = if self.is_complete() {
                "All clarifications resolved"
            } else {
                "Some clarifications pending"
            };
            md.push_str(&format!("**Status**: {}\n\n", complete));

            for item in &self.items {
                md.push_str(&format!("## {} - Question\n\n", item.id));
                md.push_str(&format!("{}\n\n", item.question));

                md.push_str("### Context\n\n");
                md.push_str(&format!("{}\n\n", item.context));

                if let Some(ref decision) = item.decision {
                    md.push_str("### Decision\n\n");
                    md.push_str(&format!("{}\n\n", decision));

                    if let Some(ref rationale) = item.rationale {
                        md.push_str("**Rationale**: ");
                        md.push_str(&format!("{}\n\n", rationale));
                    }

                    if !item.spec_updates.is_empty() {
                        md.push_str("**Spec Updates**:\n");
                        for update in &item.spec_updates {
                            md.push_str(&format!("- {}\n", update));
                        }
                        md.push_str("\n");
                    }
                } else {
                    md.push_str("### Decision\n\n");
                    md.push_str("*Pending*\n\n");
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
            items: Vec::new(),
            content: markdown,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clarification_creation() {
        let clarification = Clarification::new(
            "user-auth".to_string(),
            "User Auth Clarifications".to_string(),
        );

        assert_eq!(clarification.feature_id, "user-auth");
        assert!(clarification.is_complete());
    }

    #[test]
    fn test_clarification_completion() {
        let mut clarification = Clarification::new(
            "user-auth".to_string(),
            "User Auth Clarifications".to_string(),
        );

        clarification.add_item(ClarificationItem {
            id: "Q001".to_string(),
            question: "Use refresh tokens?".to_string(),
            context: "Spec unclear on token lifecycle".to_string(),
            decision: None,
            rationale: None,
            spec_updates: Vec::new(),
        });

        assert!(!clarification.is_complete());

        // Resolve the clarification
        clarification.items[0].decision = Some("Use refresh tokens".to_string());
        assert!(clarification.is_complete());
    }
}
