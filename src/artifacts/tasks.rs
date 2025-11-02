//! Task breakdown artifact

use super::types::{Artifact, ArtifactMetadata, ArtifactType};
use super::storage::{parse_frontmatter, serialize_frontmatter};
use crate::error::{MnemosyneError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Task breakdown with dependencies and parallelization markers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskBreakdown {
    #[serde(flatten)]
    pub metadata: ArtifactMetadata,

    /// Feature ID this task breakdown implements
    pub feature_id: String,

    /// Task phases
    pub phases: Vec<TaskPhase>,

    /// Full markdown content
    #[serde(skip)]
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPhase {
    /// Phase name (e.g., "Setup", "P1: Core Features")
    pub name: String,

    /// Tasks in this phase
    pub tasks: Vec<Task>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Task ID (e.g., "T001")
    pub id: String,

    /// Task description
    pub description: String,

    /// Can be parallelized
    #[serde(default)]
    pub parallelizable: bool,

    /// User story this task implements
    #[serde(skip_serializing_if = "Option::is_none")]
    pub story: Option<String>,

    /// Task status
    #[serde(default)]
    pub completed: bool,

    /// Dependencies (other task IDs)
    #[serde(default)]
    pub depends_on: Vec<String>,
}

impl TaskBreakdown {
    pub fn new(feature_id: String, breakdown_name: String) -> Self {
        let metadata = ArtifactMetadata::new(
            ArtifactType::TaskBreakdown,
            format!("{}-tasks", feature_id),
            breakdown_name,
        );

        Self {
            metadata,
            feature_id,
            phases: Vec::new(),
            content: String::new(),
        }
    }

    pub fn add_phase(&mut self, phase: TaskPhase) {
        self.phases.push(phase);
        self.metadata.update_timestamp();
    }
}

impl Artifact for TaskBreakdown {
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
        PathBuf::from(format!("tasks/{}-tasks.md", self.feature_id))
    }

    fn to_markdown(&self) -> Result<String> {
        let frontmatter = serde_yaml::to_value(&self.metadata)
            .map_err(|e| MnemosyneError::Other(format!("Failed to serialize metadata: {}", e)))?;

        let content = if self.content.is_empty() {
            // Generate default content
            let mut md = format!("# Task Breakdown: {}\n\n", self.metadata.name);

            for phase in &self.phases {
                md.push_str(&format!("## {}\n\n", phase.name));
                for task in &phase.tasks {
                    let checkbox = if task.completed { "x" } else { " " };
                    let parallel = if task.parallelizable { " [P]" } else { "" };
                    let story = if let Some(ref s) = task.story {
                        format!(" [{}]", s)
                    } else {
                        String::new()
                    };

                    md.push_str(&format!(
                        "- [{}] [{}]{}{} {}\n",
                        checkbox, task.id, parallel, story, task.description
                    ));

                    if !task.depends_on.is_empty() {
                        md.push_str(&format!(
                            "  - Depends on: {}\n",
                            task.depends_on.join(", ")
                        ));
                    }
                }
                md.push_str("\n");
            }

            md.push_str("**Legend**: `[P]` = Parallelizable\n");
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
            phases: Vec::new(),
            content: markdown,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_breakdown_creation() {
        let tasks = TaskBreakdown::new(
            "user-auth".to_string(),
            "User Auth Tasks".to_string(),
        );

        assert_eq!(tasks.feature_id, "user-auth");
    }
}
