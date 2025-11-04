//! Clarification artifact for resolving ambiguities

use super::types::{Artifact, ArtifactMetadata, ArtifactType};
use super::storage::{parse_frontmatter, serialize_frontmatter};
use crate::error::{MnemosyneError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Parse clarification items from markdown
///
/// Extracts items with format:
/// ## Q001 - Question
/// Question text
/// ### Context
/// Context text
/// ### Decision
/// Decision text
/// **Rationale**: Rationale text
/// **Spec Updates**:
/// - Update 1
fn parse_clarification_items(markdown: &str) -> Vec<ClarificationItem> {
    let mut items = Vec::new();
    let lines: Vec<&str> = markdown.lines().collect();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();

        // Look for item header: "## Q001 - Question" or "## QXXX - Question"
        if let Some(header) = line.strip_prefix("##") {
            let header = header.trim();

            // Skip non-question headers (like "Status")
            if !header.contains(" - ") || header.starts_with("*") {
                i += 1;
                continue;
            }

            // Extract question ID
            let id = if let Some(dash_pos) = header.find(" - ") {
                header[..dash_pos].trim().to_string()
            } else {
                i += 1;
                continue;
            };

            let mut question = String::new();
            let mut context = String::new();
            let mut decision: Option<String> = None;
            let mut rationale: Option<String> = None;
            let mut spec_updates = Vec::new();

            // Parse question text (lines after header until first ###)
            i += 1;
            while i < lines.len() {
                let content_line = lines[i].trim();

                if content_line.starts_with("###") || content_line.starts_with("##") {
                    break;
                }

                if !content_line.is_empty() {
                    if !question.is_empty() {
                        question.push(' ');
                    }
                    question.push_str(content_line);
                }

                i += 1;
            }

            // Parse sections (Context, Decision)
            while i < lines.len() {
                let section_line = lines[i].trim();

                // Stop at next item
                if section_line.starts_with("## ") && section_line.contains(" - ") {
                    break;
                }

                if section_line == "### Context" {
                    i += 1;
                    while i < lines.len() {
                        let ctx_line = lines[i].trim();
                        if ctx_line.starts_with("###") || ctx_line.starts_with("##") {
                            break;
                        }
                        if !ctx_line.is_empty() {
                            if !context.is_empty() {
                                context.push(' ');
                            }
                            context.push_str(ctx_line);
                        }
                        i += 1;
                    }
                    continue;
                }

                if section_line == "### Decision" {
                    i += 1;
                    let mut decision_text = String::new();

                    while i < lines.len() {
                        let dec_line = lines[i].trim();

                        // Stop at next section
                        if dec_line.starts_with("###") || dec_line.starts_with("##") {
                            break;
                        }

                        // Check for *Pending*
                        if dec_line == "*Pending*" {
                            decision = None;
                            i += 1;
                            break;
                        }

                        // Check for rationale
                        if let Some(rat_text) = dec_line.strip_prefix("**Rationale**:") {
                            rationale = Some(rat_text.trim().to_string());
                            i += 1;
                            continue;
                        }

                        // Check for spec updates
                        if dec_line == "**Spec Updates**:" {
                            i += 1;
                            while i < lines.len() {
                                let update_line = lines[i].trim();
                                if update_line.starts_with("###") || update_line.starts_with("##") ||
                                   update_line.starts_with("**") {
                                    break;
                                }
                                if let Some(content) = update_line.strip_prefix('-').or_else(|| update_line.strip_prefix('*')) {
                                    let cleaned = content.trim();
                                    if !cleaned.is_empty() {
                                        spec_updates.push(cleaned.to_string());
                                    }
                                }
                                i += 1;
                            }
                            continue;
                        }

                        // Regular decision text
                        if !dec_line.is_empty() {
                            if !decision_text.is_empty() {
                                decision_text.push(' ');
                            }
                            decision_text.push_str(dec_line);
                        }

                        i += 1;
                    }

                    if !decision_text.is_empty() {
                        decision = Some(decision_text);
                    }
                    continue;
                }

                i += 1;
            }

            items.push(ClarificationItem {
                id,
                question,
                context,
                decision,
                rationale,
                spec_updates,
            });
            continue;
        }

        i += 1;
    }

    items
}

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

        // Extract feature_id from metadata.id which has format "{feature_id}-clarifications"
        let feature_id = if let Some(suffix_pos) = metadata.id.rfind("-clarifications") {
            metadata.id[..suffix_pos].to_string()
        } else {
            metadata.id.clone()
        };

        // Parse clarification items from markdown
        let items = parse_clarification_items(&markdown);

        Ok(Self {
            metadata,
            feature_id,
            items,
            content: markdown.to_string(),
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

    #[test]
    fn test_clarification_round_trip_simple() {
        // Create simple clarification with one resolved item
        let mut original = Clarification::new(
            "test-feature".to_string(),
            "Test Feature Clarifications".to_string(),
        );

        original.add_item(ClarificationItem {
            id: "Q001".to_string(),
            question: "Should we use caching?".to_string(),
            context: "Performance requirements unclear".to_string(),
            decision: Some("Yes, use Redis caching".to_string()),
            rationale: Some("Improves response times significantly".to_string()),
            spec_updates: vec![],
        });

        // Serialize to markdown
        let markdown = original.to_markdown().unwrap();

        // Deserialize back
        let loaded = Clarification::from_markdown(&markdown).unwrap();

        // Verify fields preserved
        assert_eq!(loaded.feature_id, "test-feature");
        assert_eq!(loaded.items.len(), 1);
        assert_eq!(loaded.items[0].id, "Q001");
        assert_eq!(loaded.items[0].question, "Should we use caching?");
        assert_eq!(loaded.items[0].context, "Performance requirements unclear");
        assert_eq!(loaded.items[0].decision, Some("Yes, use Redis caching".to_string()));
        assert_eq!(loaded.items[0].rationale, Some("Improves response times significantly".to_string()));
        assert!(loaded.is_complete());
    }

    #[test]
    fn test_clarification_round_trip_complete() {
        // Create complete clarification with multiple items
        let mut original = Clarification::new(
            "user-auth".to_string(),
            "User Authentication Clarifications".to_string(),
        );

        // Resolved item with spec updates
        original.add_item(ClarificationItem {
            id: "Q001".to_string(),
            question: "Should we support refresh tokens or only short-lived access tokens?".to_string(),
            context: "The spec mentions stateless sessions but doesn't specify refresh mechanism.".to_string(),
            decision: Some("Use refresh tokens stored in HTTP-only cookies".to_string()),
            rationale: Some("Improves security (refresh tokens can be revoked) and UX (no manual re-auth every 24h)".to_string()),
            spec_updates: vec![
                "Added P2 scenario for refresh token flow".to_string(),
                "Updated security requirements to include token revocation".to_string(),
            ],
        });

        // Resolved item without rationale
        original.add_item(ClarificationItem {
            id: "Q002".to_string(),
            question: "Which JWT algorithm should we use?".to_string(),
            context: "Security best practices need to be clarified".to_string(),
            decision: Some("Use RS256 asymmetric signing".to_string()),
            rationale: None,
            spec_updates: vec!["Added RS256 requirement to spec".to_string()],
        });

        // Pending item
        original.add_item(ClarificationItem {
            id: "Q003".to_string(),
            question: "Should we implement password recovery?".to_string(),
            context: "Out of scope for MVP?".to_string(),
            decision: None,
            rationale: None,
            spec_updates: vec![],
        });

        // Serialize to markdown
        let markdown = original.to_markdown().unwrap();

        // Deserialize back
        let loaded = Clarification::from_markdown(&markdown).unwrap();

        // Verify all fields preserved
        assert_eq!(loaded.feature_id, "user-auth");
        assert_eq!(loaded.items.len(), 3);

        // Verify Q001 (complete with spec updates)
        assert_eq!(loaded.items[0].id, "Q001");
        assert_eq!(loaded.items[0].question, "Should we support refresh tokens or only short-lived access tokens?");
        assert_eq!(loaded.items[0].context, "The spec mentions stateless sessions but doesn't specify refresh mechanism.");
        assert_eq!(loaded.items[0].decision, Some("Use refresh tokens stored in HTTP-only cookies".to_string()));
        assert_eq!(loaded.items[0].rationale, Some("Improves security (refresh tokens can be revoked) and UX (no manual re-auth every 24h)".to_string()));
        assert_eq!(loaded.items[0].spec_updates.len(), 2);
        assert_eq!(loaded.items[0].spec_updates[0], "Added P2 scenario for refresh token flow");

        // Verify Q002 (no rationale)
        assert_eq!(loaded.items[1].id, "Q002");
        assert_eq!(loaded.items[1].decision, Some("Use RS256 asymmetric signing".to_string()));
        assert_eq!(loaded.items[1].rationale, None);
        assert_eq!(loaded.items[1].spec_updates.len(), 1);

        // Verify Q003 (pending)
        assert_eq!(loaded.items[2].id, "Q003");
        assert_eq!(loaded.items[2].question, "Should we implement password recovery?");
        assert_eq!(loaded.items[2].decision, None);
        assert_eq!(loaded.items[2].rationale, None);
        assert_eq!(loaded.items[2].spec_updates.len(), 0);

        // Check completion status
        assert!(!loaded.is_complete());
    }

    #[test]
    fn test_parse_clarification_items() {
        let markdown = r#"
# Clarifications

**Status**: Some clarifications pending

## Q001 - Question

Should we use caching?

### Context

Performance requirements need clarification

### Decision

Yes, use Redis caching

**Rationale**: Improves response times

**Spec Updates**:
- Added caching requirement
- Updated performance targets

## Q002 - Question

Use WebSockets or polling?

### Context

Real-time requirements unclear

### Decision

*Pending*
"#;

        let items = parse_clarification_items(markdown);
        assert_eq!(items.len(), 2);

        // Verify Q001
        assert_eq!(items[0].id, "Q001");
        assert_eq!(items[0].question, "Should we use caching?");
        assert_eq!(items[0].context, "Performance requirements need clarification");
        assert_eq!(items[0].decision, Some("Yes, use Redis caching".to_string()));
        assert_eq!(items[0].rationale, Some("Improves response times".to_string()));
        assert_eq!(items[0].spec_updates.len(), 2);
        assert_eq!(items[0].spec_updates[0], "Added caching requirement");

        // Verify Q002 (pending)
        assert_eq!(items[1].id, "Q002");
        assert_eq!(items[1].question, "Use WebSockets or polling?");
        assert_eq!(items[1].context, "Real-time requirements unclear");
        assert_eq!(items[1].decision, None);
        assert_eq!(items[1].rationale, None);
        assert_eq!(items[1].spec_updates.len(), 0);
    }
}
