//! Implementation plan artifact
#![allow(clippy::useless_format, clippy::single_char_add_str)]

use super::types::{Artifact, ArtifactMetadata, ArtifactType};
use super::storage::{parse_frontmatter, serialize_frontmatter};
use crate::error::{MnemosyneError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Parse technical approach from markdown section
///
/// Extracts text from "## Technical Approach" section until next ## header
fn parse_technical_approach(markdown: &str) -> String {
    if let Some(start_idx) = markdown.find("## Technical Approach") {
        let after_header = &markdown[start_idx + "## Technical Approach".len()..];

        let mut approach_lines = Vec::new();
        for line in after_header.lines() {
            let trimmed = line.trim();

            // Stop at next section header
            if trimmed.starts_with("##") {
                break;
            }

            approach_lines.push(line);
        }

        approach_lines.join("\n").trim().to_string()
    } else {
        String::new()
    }
}

/// Parse architecture decisions from markdown
///
/// Extracts decisions in format:
/// ### AD-001: Title
/// **Decision**: text
/// **Rationale**: text
/// **Alternatives Considered**:
/// - Alternative 1
fn parse_architecture_decisions(markdown: &str) -> Vec<ArchitectureDecision> {
    let mut decisions = Vec::new();

    if let Some(start_idx) = markdown.find("## Architecture Decisions") {
        let after_header = &markdown[start_idx..];
        let lines: Vec<&str> = after_header.lines().collect();

        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();

            // Look for decision header: "### AD-NNN: Title"
            if let Some(header) = line.strip_prefix("###") {
                let header = header.trim();
                // Extract title after "AD-NNN: "
                let title = if let Some(colon_pos) = header.find(':') {
                    header[colon_pos + 1..].trim().to_string()
                } else {
                    header.to_string()
                };

                let mut decision = String::new();
                let mut rationale = String::new();
                let mut alternatives = Vec::new();

                i += 1;
                while i < lines.len() {
                    let content_line = lines[i].trim();

                    // Stop at next decision or section
                    if content_line.starts_with("###") ||
                       (content_line.starts_with("##") && !content_line.contains("Architecture Decisions")) {
                        break;
                    }

                    // Parse fields
                    if let Some(d) = content_line.strip_prefix("**Decision**:") {
                        decision = d.trim().to_string();
                    } else if let Some(r) = content_line.strip_prefix("**Rationale**:") {
                        rationale = r.trim().to_string();
                    } else if content_line == "**Alternatives Considered**:" {
                        // Parse alternatives list
                        i += 1;
                        while i < lines.len() {
                            let alt_line = lines[i].trim();
                            if alt_line.starts_with("###") || alt_line.starts_with("##") ||
                               alt_line.starts_with("**") {
                                i -= 1; // Back up
                                break;
                            }
                            if let Some(content) = alt_line.strip_prefix('-').or_else(|| alt_line.strip_prefix('*')) {
                                let cleaned = content.trim();
                                if !cleaned.is_empty() {
                                    alternatives.push(cleaned.to_string());
                                }
                            } else if alt_line.is_empty() {
                                // Empty line might end alternatives
                            } else {
                                i -= 1;
                                break;
                            }
                            i += 1;
                        }
                    }

                    i += 1;
                }

                decisions.push(ArchitectureDecision {
                    title,
                    decision,
                    rationale,
                    alternatives,
                });
                continue;
            }

            i += 1;
        }
    }

    decisions
}

/// Parse dependencies from markdown section
///
/// Extracts bullet list from "## Dependencies" section
fn parse_dependencies(markdown: &str) -> Vec<String> {
    let mut dependencies = Vec::new();

    if let Some(start_idx) = markdown.find("## Dependencies") {
        let after_header = &markdown[start_idx + "## Dependencies".len()..];

        for line in after_header.lines() {
            let trimmed = line.trim();

            // Stop at next section header
            if trimmed.starts_with("##") {
                break;
            }

            // Match bullet items
            if let Some(content) = trimmed.strip_prefix('-').or_else(|| trimmed.strip_prefix('*')) {
                let cleaned = content.trim();
                if !cleaned.is_empty() {
                    dependencies.push(cleaned.to_string());
                }
            }
        }
    }

    dependencies
}

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

        // Extract feature_id from metadata.id which has format "{feature_id}-plan"
        let feature_id = if let Some(suffix_pos) = metadata.id.rfind("-plan") {
            metadata.id[..suffix_pos].to_string()
        } else {
            metadata.id.clone()
        };

        // Parse sections from markdown
        let approach = parse_technical_approach(&markdown);
        let architecture = parse_architecture_decisions(&markdown);
        let dependencies = parse_dependencies(&markdown);

        Ok(Self {
            metadata,
            feature_id,
            approach,
            architecture,
            dependencies,
            content: markdown.to_string(),
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

    #[test]
    fn test_plan_round_trip_simple() {
        // Create plan with minimal fields
        let original = ImplementationPlan::new(
            "test-feature".to_string(),
            "Test Feature Plan".to_string(),
            "Use simple approach with minimal dependencies".to_string(),
        );

        // Serialize to markdown
        let markdown = original.to_markdown().unwrap();

        // Deserialize back
        let loaded = ImplementationPlan::from_markdown(&markdown).unwrap();

        // Verify fields were preserved
        assert_eq!(loaded.feature_id, "test-feature");
        assert_eq!(loaded.approach, "Use simple approach with minimal dependencies");
        assert_eq!(loaded.architecture.len(), 0);
        assert_eq!(loaded.dependencies.len(), 0);
    }

    #[test]
    fn test_plan_round_trip_complete() {
        // Create plan with all fields
        let mut original = ImplementationPlan::new(
            "user-auth".to_string(),
            "User Authentication Plan".to_string(),
            "Implement JWT-based authentication with RS256 signing and refresh tokens".to_string(),
        );

        // Add architecture decisions
        original.add_architecture_decision(ArchitectureDecision {
            title: "JWT Algorithm".to_string(),
            decision: "Use RS256 asymmetric signing".to_string(),
            rationale: "Better security than HS256, enables distributed verification".to_string(),
            alternatives: vec![
                "HS256 symmetric signing".to_string(),
                "Opaque session tokens".to_string(),
            ],
        });

        original.add_architecture_decision(ArchitectureDecision {
            title: "Token Storage".to_string(),
            decision: "Store refresh tokens in HTTP-only cookies".to_string(),
            rationale: "Prevents XSS attacks on refresh tokens".to_string(),
            alternatives: vec![
                "LocalStorage".to_string(),
            ],
        });

        // Add dependencies
        original.add_dependency("jsonwebtoken = \"9.0\"".to_string());
        original.add_dependency("ring = \"0.17\"".to_string());
        original.add_dependency("tower-sessions = \"0.10\"".to_string());

        // Serialize to markdown
        let markdown = original.to_markdown().unwrap();

        // Deserialize back
        let loaded = ImplementationPlan::from_markdown(&markdown).unwrap();

        // Verify all fields preserved
        assert_eq!(loaded.feature_id, "user-auth");
        assert_eq!(loaded.approach, "Implement JWT-based authentication with RS256 signing and refresh tokens");

        assert_eq!(loaded.architecture.len(), 2);
        assert_eq!(loaded.architecture[0].title, "JWT Algorithm");
        assert_eq!(loaded.architecture[0].decision, "Use RS256 asymmetric signing");
        assert_eq!(loaded.architecture[0].rationale, "Better security than HS256, enables distributed verification");
        assert_eq!(loaded.architecture[0].alternatives.len(), 2);
        assert_eq!(loaded.architecture[0].alternatives[0], "HS256 symmetric signing");
        assert_eq!(loaded.architecture[0].alternatives[1], "Opaque session tokens");

        assert_eq!(loaded.architecture[1].title, "Token Storage");
        assert_eq!(loaded.architecture[1].decision, "Store refresh tokens in HTTP-only cookies");
        assert_eq!(loaded.architecture[1].alternatives.len(), 1);

        assert_eq!(loaded.dependencies.len(), 3);
        assert_eq!(loaded.dependencies[0], "jsonwebtoken = \"9.0\"");
        assert_eq!(loaded.dependencies[1], "ring = \"0.17\"");
        assert_eq!(loaded.dependencies[2], "tower-sessions = \"0.10\"");
    }

    #[test]
    fn test_parse_technical_approach() {
        let markdown = r#"
# Implementation Plan: User Auth

## Technical Approach

Implement JWT-based authentication with RS256 signing.
This provides strong security guarantees and enables
distributed token verification.

## Architecture Decisions
"#;

        let approach = parse_technical_approach(markdown);
        assert!(approach.contains("JWT-based authentication"));
        assert!(approach.contains("RS256 signing"));
        assert!(approach.contains("distributed token verification"));
    }

    #[test]
    fn test_parse_architecture_decisions() {
        let markdown = r#"
## Architecture Decisions

### AD-001: JWT Algorithm

**Decision**: Use RS256 asymmetric signing

**Rationale**: Better security than HS256

**Alternatives Considered**:
- HS256 symmetric signing
- Opaque session tokens

### AD-002: Token Storage

**Decision**: HTTP-only cookies

**Rationale**: Prevents XSS attacks

## Dependencies
"#;

        let decisions = parse_architecture_decisions(markdown);
        assert_eq!(decisions.len(), 2);

        assert_eq!(decisions[0].title, "JWT Algorithm");
        assert_eq!(decisions[0].decision, "Use RS256 asymmetric signing");
        assert_eq!(decisions[0].rationale, "Better security than HS256");
        assert_eq!(decisions[0].alternatives.len(), 2);
        assert_eq!(decisions[0].alternatives[0], "HS256 symmetric signing");

        assert_eq!(decisions[1].title, "Token Storage");
        assert_eq!(decisions[1].decision, "HTTP-only cookies");
        assert_eq!(decisions[1].rationale, "Prevents XSS attacks");
        assert_eq!(decisions[1].alternatives.len(), 0);
    }

    #[test]
    fn test_parse_dependencies() {
        let markdown = r#"
## Dependencies

- jsonwebtoken = "9.0"
- ring = "0.17"
- tower-sessions = "0.10"

## Next Section
"#;

        let deps = parse_dependencies(markdown);
        assert_eq!(deps.len(), 3);
        assert_eq!(deps[0], "jsonwebtoken = \"9.0\"");
        assert_eq!(deps[1], "ring = \"0.17\"");
        assert_eq!(deps[2], "tower-sessions = \"0.10\"");
    }
}
