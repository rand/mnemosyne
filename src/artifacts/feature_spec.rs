//! Feature specification artifact
#![allow(clippy::useless_format, clippy::single_char_add_str)]

use super::types::{Artifact, ArtifactMetadata, ArtifactType};
use super::storage::{parse_frontmatter, serialize_frontmatter};
use crate::error::{MnemosyneError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Parse user scenarios from markdown
///
/// Format:
/// ```markdown
/// ### P0: Goal text
/// **As a** actor
/// **I want** goal
/// **So that** benefit
/// **Acceptance Criteria**:
/// - [ ] criterion
/// ```
fn parse_scenarios(markdown: &str) -> Vec<UserScenario> {
    let mut scenarios = Vec::new();

    if let Some(start_idx) = markdown.find("## User Scenarios") {
        let after_header = &markdown[start_idx..];
        let lines: Vec<&str> = after_header.lines().collect();

        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();

            // Look for scenario header: "### P0: Goal text"
            if let Some(header) = line.strip_prefix("###") {
                let header = header.trim();
                if let Some((priority, goal)) = header.split_once(':') {
                    let priority = priority.trim().to_string();
                    let goal = goal.trim().to_string();

                    // Parse actor, benefit, and criteria
                    let mut actor = String::new();
                    let mut benefit = String::new();
                    let mut criteria = Vec::new();

                    i += 1;
                    while i < lines.len() {
                        let content_line = lines[i].trim();

                        // Stop at next scenario or section
                        if content_line.starts_with("###") || content_line.starts_with("##") {
                            break;
                        }

                        // Parse fields
                        if let Some(a) = content_line.strip_prefix("**As a**") {
                            actor = a.trim().to_string();
                        } else if let Some(b) = content_line.strip_prefix("**So that**") {
                            benefit = b.trim().to_string();
                        } else if content_line == "**Acceptance Criteria**:" {
                            // Parse criteria list
                            i += 1;
                            while i < lines.len() {
                                let crit_line = lines[i].trim();
                                if crit_line.starts_with("###") || crit_line.starts_with("##") ||
                                   crit_line.starts_with("**") {
                                    i -= 1; // Back up to let outer loop handle it
                                    break;
                                }
                                if let Some(content) = crit_line.strip_prefix('-').or_else(|| crit_line.strip_prefix('*')) {
                                    let mut cleaned = content.trim();
                                    if let Some(after) = cleaned.strip_prefix("[ ]").or_else(|| cleaned.strip_prefix("[x]")) {
                                        cleaned = after.trim();
                                    }
                                    if !cleaned.is_empty() {
                                        criteria.push(cleaned.to_string());
                                    }
                                } else if crit_line.is_empty() {
                                    // Empty line might end criteria
                                } else {
                                    i -= 1;
                                    break;
                                }
                                i += 1;
                            }
                        }

                        i += 1;
                    }

                    scenarios.push(UserScenario {
                        priority,
                        actor,
                        goal: goal.clone(),
                        benefit,
                        acceptance_criteria: criteria,
                    });
                    continue;
                }
            }

            i += 1;
        }
    }

    scenarios
}

/// Parse functional requirements from markdown
///
/// Format: **FR-001**: requirement text
fn parse_requirements(markdown: &str) -> Vec<String> {
    let mut requirements = Vec::new();

    if let Some(start_idx) = markdown.find("## Functional Requirements") {
        let after_header = &markdown[start_idx..];

        for line in after_header.lines() {
            let trimmed = line.trim();

            // Stop at next section
            if trimmed.starts_with("##") && !trimmed.contains("Functional Requirements") {
                break;
            }

            // Match **FR-NNN**: text
            if let Some(content) = trimmed.strip_prefix("**FR-") {
                if let Some(req_text) = content.split_once("**:") {
                    requirements.push(req_text.1.trim().to_string());
                }
            }
        }
    }

    requirements
}

/// Parse success criteria from markdown (bullet list)
fn parse_success_criteria(markdown: &str) -> Vec<String> {
    let mut criteria = Vec::new();

    if let Some(start_idx) = markdown.find("## Success Criteria") {
        let after_header = &markdown[start_idx..];

        for line in after_header.lines() {
            let trimmed = line.trim();

            // Stop at next section
            if trimmed.starts_with("##") && !trimmed.contains("Success Criteria") {
                break;
            }

            // Match bullet items with optional checkboxes
            if let Some(content) = trimmed.strip_prefix('-').or_else(|| trimmed.strip_prefix('*')) {
                let mut cleaned = content.trim();
                if let Some(after) = cleaned.strip_prefix("[ ]").or_else(|| cleaned.strip_prefix("[x]")) {
                    cleaned = after.trim();
                }
                if !cleaned.is_empty() {
                    criteria.push(cleaned.to_string());
                }
            }
        }
    }

    criteria
}

/// Feature specification with user scenarios and requirements
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureSpec {
    #[serde(flatten)]
    pub metadata: ArtifactMetadata,

    /// Feature ID (kebab-case, e.g., "user-auth-jwt")
    pub feature_id: String,

    /// Parent feature (for sub-features)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_feature: Option<String>,

    /// User scenarios (prioritized)
    pub scenarios: Vec<UserScenario>,

    /// Functional requirements
    pub requirements: Vec<String>,

    /// Success criteria
    pub success_criteria: Vec<String>,

    /// Full markdown content
    #[serde(skip)]
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserScenario {
    /// Priority (P0, P1, P2, P3)
    pub priority: String,

    /// As a...
    pub actor: String,

    /// I want...
    pub goal: String,

    /// So that...
    pub benefit: String,

    /// Acceptance criteria
    pub acceptance_criteria: Vec<String>,
}

impl FeatureSpec {
    pub fn new(feature_id: String, feature_name: String) -> Self {
        let metadata = ArtifactMetadata::new(
            ArtifactType::FeatureSpec,
            feature_id.clone(),
            feature_name,
        );

        Self {
            metadata,
            feature_id,
            parent_feature: None,
            scenarios: Vec::new(),
            requirements: Vec::new(),
            success_criteria: Vec::new(),
            content: String::new(),
        }
    }

    /// Create a feature spec with builder pattern
    pub fn builder(feature_id: String, feature_name: String) -> FeatureSpecBuilder {
        FeatureSpecBuilder::new(feature_id, feature_name)
    }

    pub fn add_scenario(&mut self, scenario: UserScenario) {
        self.scenarios.push(scenario);
        self.metadata.update_timestamp();
    }

    pub fn add_requirement(&mut self, requirement: String) {
        self.requirements.push(requirement);
        self.metadata.update_timestamp();
    }

    pub fn add_success_criterion(&mut self, criterion: String) {
        self.success_criteria.push(criterion);
        self.metadata.update_timestamp();
    }
}

/// Builder for FeatureSpec to enable fluent API
pub struct FeatureSpecBuilder {
    feature_id: String,
    feature_name: String,
    parent_feature: Option<String>,
    scenarios: Vec<UserScenario>,
    requirements: Vec<String>,
    success_criteria: Vec<String>,
}

impl FeatureSpecBuilder {
    pub fn new(feature_id: String, feature_name: String) -> Self {
        Self {
            feature_id,
            feature_name,
            parent_feature: None,
            scenarios: Vec::new(),
            requirements: Vec::new(),
            success_criteria: Vec::new(),
        }
    }

    pub fn parent_feature(mut self, parent: impl Into<String>) -> Self {
        self.parent_feature = Some(parent.into());
        self
    }

    pub fn scenario(mut self, scenario: UserScenario) -> Self {
        self.scenarios.push(scenario);
        self
    }

    pub fn requirement(mut self, requirement: impl Into<String>) -> Self {
        self.requirements.push(requirement.into());
        self
    }

    pub fn success_criterion(mut self, criterion: impl Into<String>) -> Self {
        self.success_criteria.push(criterion.into());
        self
    }

    pub fn build(self) -> FeatureSpec {
        let mut spec = FeatureSpec::new(self.feature_id, self.feature_name);
        spec.parent_feature = self.parent_feature;
        spec.scenarios = self.scenarios;
        spec.requirements = self.requirements;
        spec.success_criteria = self.success_criteria;
        spec
    }
}

impl Artifact for FeatureSpec {
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
        PathBuf::from(format!("specs/{}.md", self.feature_id))
    }

    fn to_markdown(&self) -> Result<String> {
        let frontmatter = serde_yaml::to_value(&self.metadata)
            .map_err(|e| MnemosyneError::Other(format!("Failed to serialize metadata: {}", e)))?;

        let content = if self.content.is_empty() {
            // Generate default content
            let mut md = format!("# Feature: {}\n\n", self.metadata.name);

            if !self.scenarios.is_empty() {
                md.push_str("## User Scenarios (Prioritized)\n\n");
                for scenario in &self.scenarios {
                    md.push_str(&format!("### {}: {}\n\n", scenario.priority, scenario.goal));
                    md.push_str(&format!("**As a** {}\n", scenario.actor));
                    md.push_str(&format!("**I want** {}\n", scenario.goal));
                    md.push_str(&format!("**So that** {}\n\n", scenario.benefit));

                    if !scenario.acceptance_criteria.is_empty() {
                        md.push_str("**Acceptance Criteria**:\n");
                        for criterion in &scenario.acceptance_criteria {
                            md.push_str(&format!("- [ ] {}\n", criterion));
                        }
                        md.push_str("\n");
                    }
                }
            }

            if !self.requirements.is_empty() {
                md.push_str("## Functional Requirements\n\n");
                for (i, req) in self.requirements.iter().enumerate() {
                    md.push_str(&format!("**FR-{:03}**: {}\n\n", i + 1, req));
                }
            }

            if !self.success_criteria.is_empty() {
                md.push_str("## Success Criteria\n\n");
                for criterion in &self.success_criteria {
                    md.push_str(&format!("- [ ] {}\n", criterion));
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

        let feature_id = metadata.id.clone();

        // Parse structured content from markdown
        let scenarios = parse_scenarios(&markdown);
        let requirements = parse_requirements(&markdown);
        let success_criteria = parse_success_criteria(&markdown);

        Ok(Self {
            metadata,
            feature_id,
            parent_feature: None,
            scenarios,
            requirements,
            success_criteria,
            content: markdown.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_spec_creation() {
        let spec = FeatureSpec::new(
            "user-auth".to_string(),
            "User Authentication".to_string(),
        );

        assert_eq!(spec.feature_id, "user-auth");
        assert_eq!(spec.metadata.name, "User Authentication");
    }

    #[test]
    fn test_feature_spec_to_markdown() {
        let mut spec = FeatureSpec::new(
            "user-auth".to_string(),
            "User Authentication".to_string(),
        );

        spec.add_scenario(UserScenario {
            priority: "P1".to_string(),
            actor: "developer".to_string(),
            goal: "authenticate with JWT".to_string(),
            benefit: "maintain stateless sessions".to_string(),
            acceptance_criteria: vec!["Token issued on login".to_string()],
        });

        let markdown = spec.to_markdown().unwrap();
        assert!(markdown.contains("type: feature_spec"));
        assert!(markdown.contains("# Feature: User Authentication"));
        assert!(markdown.contains("**As a** developer"));
    }

    #[test]
    fn test_feature_spec_round_trip_simple() {
        let mut original = FeatureSpec::new(
            "user-auth".to_string(),
            "User Authentication".to_string(),
        );

        original.add_scenario(UserScenario {
            priority: "P0".to_string(),
            actor: "developer".to_string(),
            goal: "authenticate with JWT tokens".to_string(),
            benefit: "maintain stateless sessions".to_string(),
            acceptance_criteria: vec![
                "Token issued on successful login".to_string(),
                "Token validated on protected endpoints".to_string(),
            ],
        });

        // Serialize
        let markdown = original.to_markdown().unwrap();

        // Deserialize
        let loaded = FeatureSpec::from_markdown(&markdown).unwrap();

        // Verify scenario preserved
        assert_eq!(loaded.scenarios.len(), 1);
        assert_eq!(loaded.scenarios[0].priority, "P0");
        assert_eq!(loaded.scenarios[0].actor, "developer");
        assert_eq!(loaded.scenarios[0].goal, "authenticate with JWT tokens");
        assert_eq!(loaded.scenarios[0].benefit, "maintain stateless sessions");
        assert_eq!(loaded.scenarios[0].acceptance_criteria.len(), 2);
        assert_eq!(loaded.scenarios[0].acceptance_criteria[0], "Token issued on successful login");
        assert_eq!(loaded.scenarios[0].acceptance_criteria[1], "Token validated on protected endpoints");
    }

    #[test]
    fn test_feature_spec_round_trip_complete() {
        let mut original = FeatureSpec::new(
            "user-auth".to_string(),
            "User Authentication".to_string(),
        );

        // Add multiple scenarios
        original.add_scenario(UserScenario {
            priority: "P0".to_string(),
            actor: "developer".to_string(),
            goal: "authenticate with JWT tokens".to_string(),
            benefit: "maintain stateless sessions".to_string(),
            acceptance_criteria: vec![
                "Token issued on successful login".to_string(),
                "Token validated on protected endpoints".to_string(),
            ],
        });

        original.add_scenario(UserScenario {
            priority: "P1".to_string(),
            actor: "developer".to_string(),
            goal: "refresh authentication tokens".to_string(),
            benefit: "avoid frequent re-authentication".to_string(),
            acceptance_criteria: vec![
                "Refresh token provided with access token".to_string(),
                "Refresh endpoint validates refresh token".to_string(),
            ],
        });

        // Add requirements
        original.add_requirement("Use RS256 algorithm for JWT signing".to_string());
        original.add_requirement("Store refresh tokens in HTTP-only cookies".to_string());

        // Add success criteria
        original.add_success_criterion("Authentication latency < 100ms p95".to_string());
        original.add_success_criterion("Support 10,000 concurrent sessions".to_string());

        // Serialize
        let markdown = original.to_markdown().unwrap();

        // Deserialize
        let loaded = FeatureSpec::from_markdown(&markdown).unwrap();

        // Verify all fields
        assert_eq!(loaded.scenarios.len(), 2);
        assert_eq!(loaded.scenarios[0].priority, "P0");
        assert_eq!(loaded.scenarios[1].priority, "P1");
        assert_eq!(loaded.scenarios[1].goal, "refresh authentication tokens");

        assert_eq!(loaded.requirements.len(), 2);
        assert_eq!(loaded.requirements[0], "Use RS256 algorithm for JWT signing");
        assert_eq!(loaded.requirements[1], "Store refresh tokens in HTTP-only cookies");

        assert_eq!(loaded.success_criteria.len(), 2);
        assert_eq!(loaded.success_criteria[0], "Authentication latency < 100ms p95");
        assert_eq!(loaded.success_criteria[1], "Support 10,000 concurrent sessions");
    }

    #[test]
    fn test_parse_scenarios() {
        let markdown = r#"
## User Scenarios (Prioritized)

### P0: authenticate with JWT tokens
**As a** developer
**I want** authenticate with JWT tokens
**So that** maintain stateless sessions
**Acceptance Criteria**:
- [ ] Token issued on successful login
- [ ] Token validated on protected endpoints

### P1: refresh tokens
**As a** user
**I want** refresh tokens
**So that** avoid re-authentication
**Acceptance Criteria**:
- [ ] Refresh endpoint works
"#;

        let scenarios = parse_scenarios(markdown);
        assert_eq!(scenarios.len(), 2);
        assert_eq!(scenarios[0].priority, "P0");
        assert_eq!(scenarios[0].actor, "developer");
        assert_eq!(scenarios[0].acceptance_criteria.len(), 2);
        assert_eq!(scenarios[1].priority, "P1");
        assert_eq!(scenarios[1].actor, "user");
    }

    #[test]
    fn test_parse_requirements() {
        let markdown = r#"
## Functional Requirements

**FR-001**: Use RS256 algorithm for JWT signing

**FR-002**: Store refresh tokens in HTTP-only cookies

**FR-003**: Implement token revocation endpoint
"#;

        let reqs = parse_requirements(markdown);
        assert_eq!(reqs.len(), 3);
        assert_eq!(reqs[0], "Use RS256 algorithm for JWT signing");
        assert_eq!(reqs[1], "Store refresh tokens in HTTP-only cookies");
        assert_eq!(reqs[2], "Implement token revocation endpoint");
    }

    #[test]
    fn test_parse_success_criteria() {
        let markdown = r#"
## Success Criteria

- [ ] Authentication latency < 100ms p95
- [ ] Support 10,000 concurrent sessions
- [x] Tests passing
"#;

        let criteria = parse_success_criteria(markdown);
        assert_eq!(criteria.len(), 3);
        assert_eq!(criteria[0], "Authentication latency < 100ms p95");
        assert_eq!(criteria[1], "Support 10,000 concurrent sessions");
        assert_eq!(criteria[2], "Tests passing");
    }
}
