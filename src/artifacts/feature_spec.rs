//! Feature specification artifact

use super::types::{Artifact, ArtifactMetadata, ArtifactType};
use super::storage::{parse_frontmatter, serialize_frontmatter};
use crate::error::{MnemosyneError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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

    pub fn add_scenario(&mut self, scenario: UserScenario) {
        self.scenarios.push(scenario);
        self.metadata.update_timestamp();
    }

    pub fn add_requirement(&mut self, requirement: String) {
        self.requirements.push(requirement);
        self.metadata.update_timestamp();
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

        Ok(Self {
            metadata,
            feature_id,
            parent_feature: None,
            scenarios: Vec::new(),
            requirements: Vec::new(),
            success_criteria: Vec::new(),
            content: markdown,
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
}
