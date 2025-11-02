//! Task breakdown artifact

use super::types::{Artifact, ArtifactMetadata, ArtifactType};
use super::storage::{parse_frontmatter, serialize_frontmatter};
use crate::error::{MnemosyneError, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use regex::Regex;

/// Parse task phases from markdown
///
/// Extracts phases with format:
/// ## Phase Name
/// - [x] [T001] [P] [story] Description
///   - Depends on: T002, T003
fn parse_task_phases(markdown: &str) -> Vec<TaskPhase> {
    let mut phases = Vec::new();
    let lines: Vec<&str> = markdown.lines().collect();

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();

        // Look for phase header: "## Phase Name"
        if let Some(header) = line.strip_prefix("##") {
            let phase_name = header.trim();

            // Skip "Legend" section
            if phase_name.contains("Legend") {
                i += 1;
                continue;
            }

            let mut tasks = Vec::new();

            // Parse tasks in this phase
            i += 1;
            while i < lines.len() {
                let task_line = lines[i];
                let trimmed = task_line.trim();

                // Stop at next phase or end
                if trimmed.starts_with("##") || trimmed.starts_with("**Legend") {
                    break;
                }

                // Parse task line
                if let Some(task) = parse_task_line(trimmed) {
                    tasks.push(task);

                    // Check if next line is dependencies
                    if i + 1 < lines.len() {
                        let next_line = lines[i + 1].trim();
                        if let Some(deps_str) = next_line.strip_prefix("- Depends on:") {
                            if let Some(last_task) = tasks.last_mut() {
                                last_task.depends_on = deps_str
                                    .split(',')
                                    .map(|d| d.trim().to_string())
                                    .filter(|d| !d.is_empty())
                                    .collect();
                            }
                            i += 1; // Skip dependency line
                        }
                    }
                }

                i += 1;
            }

            if !tasks.is_empty() {
                phases.push(TaskPhase {
                    name: phase_name.to_string(),
                    tasks,
                });
            }
            continue;
        }

        i += 1;
    }

    phases
}

/// Parse a single task line
///
/// Format: - [x] [T001] [P] [story] Description
/// Returns Task with parsed markers
fn parse_task_line(line: &str) -> Option<Task> {
    // Must start with checkbox
    if !line.starts_with("- [") {
        return None;
    }

    // Parse completion status
    let completed = line.contains("- [x]");

    // Remove checkbox prefix
    let after_checkbox = if completed {
        line.strip_prefix("- [x]")?
    } else {
        line.strip_prefix("- [ ]")?
    };

    let rest = after_checkbox.trim();

    // Task ID regex: [T001] or [TXXXX]
    let id_regex = Regex::new(r"^\[([T][^\]]+)\]").ok()?;
    let id_match = id_regex.find(rest)?;
    let task_id = id_match.as_str().trim_matches(|c| c == '[' || c == ']').to_string();
    let after_id = &rest[id_match.end()..].trim();

    // Check for [P] marker
    let (parallelizable, after_parallel) = if after_id.starts_with("[P]") {
        (true, after_id.strip_prefix("[P]").unwrap().trim())
    } else {
        (false, *after_id)
    };

    // Check for story marker [story-name]
    let story_regex = Regex::new(r"^\[([^\]]+)\]").ok()?;
    let (story, description) = if let Some(story_match) = story_regex.find(after_parallel) {
        let story_name = story_match.as_str().trim_matches(|c| c == '[' || c == ']').to_string();
        let after_story = &after_parallel[story_match.end()..].trim();
        (Some(story_name), after_story.to_string())
    } else {
        (None, after_parallel.to_string())
    };

    Some(Task {
        id: task_id,
        description,
        parallelizable,
        story,
        completed,
        depends_on: Vec::new(), // Filled in by caller if dependency line exists
    })
}

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

        // Extract feature_id from metadata.id which has format "{feature_id}-tasks"
        let feature_id = if let Some(suffix_pos) = metadata.id.rfind("-tasks") {
            metadata.id[..suffix_pos].to_string()
        } else {
            metadata.id.clone()
        };

        // Parse phases from markdown
        let phases = parse_task_phases(&markdown);

        Ok(Self {
            metadata,
            feature_id,
            phases,
            content: markdown.to_string(),
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

    #[test]
    fn test_task_round_trip_simple() {
        // Create simple task breakdown
        let mut original = TaskBreakdown::new(
            "test-feature".to_string(),
            "Test Feature Tasks".to_string(),
        );

        original.add_phase(TaskPhase {
            name: "Setup".to_string(),
            tasks: vec![
                Task {
                    id: "T001".to_string(),
                    description: "Initialize project".to_string(),
                    parallelizable: false,
                    story: None,
                    completed: true,
                    depends_on: vec![],
                },
            ],
        });

        // Serialize to markdown
        let markdown = original.to_markdown().unwrap();

        // Deserialize back
        let loaded = TaskBreakdown::from_markdown(&markdown).unwrap();

        // Verify fields preserved
        assert_eq!(loaded.feature_id, "test-feature");
        assert_eq!(loaded.phases.len(), 1);
        assert_eq!(loaded.phases[0].name, "Setup");
        assert_eq!(loaded.phases[0].tasks.len(), 1);
        assert_eq!(loaded.phases[0].tasks[0].id, "T001");
        assert_eq!(loaded.phases[0].tasks[0].description, "Initialize project");
        assert!(loaded.phases[0].tasks[0].completed);
        assert!(!loaded.phases[0].tasks[0].parallelizable);
    }

    #[test]
    fn test_task_round_trip_complete() {
        // Create complete task breakdown with all markers
        let mut original = TaskBreakdown::new(
            "user-auth".to_string(),
            "User Authentication Tasks".to_string(),
        );

        original.add_phase(TaskPhase {
            name: "Setup".to_string(),
            tasks: vec![
                Task {
                    id: "T001".to_string(),
                    description: "Add jsonwebtoken dependency".to_string(),
                    parallelizable: false,
                    story: None,
                    completed: true,
                    depends_on: vec![],
                },
                Task {
                    id: "T002".to_string(),
                    description: "Create keys directory".to_string(),
                    parallelizable: false,
                    story: None,
                    completed: true,
                    depends_on: vec![],
                },
            ],
        });

        original.add_phase(TaskPhase {
            name: "P1: Core Features".to_string(),
            tasks: vec![
                Task {
                    id: "T003".to_string(),
                    description: "Implement token generation".to_string(),
                    parallelizable: true,
                    story: Some("user-login".to_string()),
                    completed: false,
                    depends_on: vec!["T001".to_string()],
                },
                Task {
                    id: "T004".to_string(),
                    description: "Implement token validation".to_string(),
                    parallelizable: true,
                    story: Some("user-login".to_string()),
                    completed: false,
                    depends_on: vec!["T001".to_string()],
                },
                Task {
                    id: "T005".to_string(),
                    description: "Integrate with login endpoint".to_string(),
                    parallelizable: false,
                    story: Some("user-login".to_string()),
                    completed: false,
                    depends_on: vec!["T003".to_string(), "T004".to_string()],
                },
            ],
        });

        // Serialize to markdown
        let markdown = original.to_markdown().unwrap();

        // Deserialize back
        let loaded = TaskBreakdown::from_markdown(&markdown).unwrap();

        // Verify all fields preserved
        assert_eq!(loaded.feature_id, "user-auth");
        assert_eq!(loaded.phases.len(), 2);

        // Verify Setup phase
        assert_eq!(loaded.phases[0].name, "Setup");
        assert_eq!(loaded.phases[0].tasks.len(), 2);
        assert_eq!(loaded.phases[0].tasks[0].id, "T001");
        assert!(loaded.phases[0].tasks[0].completed);
        assert_eq!(loaded.phases[0].tasks[1].id, "T002");

        // Verify P1 phase
        assert_eq!(loaded.phases[1].name, "P1: Core Features");
        assert_eq!(loaded.phases[1].tasks.len(), 3);

        // Verify T003 with all markers
        assert_eq!(loaded.phases[1].tasks[0].id, "T003");
        assert_eq!(loaded.phases[1].tasks[0].description, "Implement token generation");
        assert!(loaded.phases[1].tasks[0].parallelizable);
        assert_eq!(loaded.phases[1].tasks[0].story, Some("user-login".to_string()));
        assert!(!loaded.phases[1].tasks[0].completed);
        assert_eq!(loaded.phases[1].tasks[0].depends_on, vec!["T001"]);

        // Verify T004
        assert_eq!(loaded.phases[1].tasks[1].id, "T004");
        assert!(loaded.phases[1].tasks[1].parallelizable);

        // Verify T005 with multiple dependencies
        assert_eq!(loaded.phases[1].tasks[2].id, "T005");
        assert!(!loaded.phases[1].tasks[2].parallelizable);
        assert_eq!(loaded.phases[1].tasks[2].depends_on.len(), 2);
        assert_eq!(loaded.phases[1].tasks[2].depends_on[0], "T003");
        assert_eq!(loaded.phases[1].tasks[2].depends_on[1], "T004");
    }

    #[test]
    fn test_parse_task_line_minimal() {
        let line = "- [ ] [T001] Simple task";
        let task = parse_task_line(line).unwrap();

        assert_eq!(task.id, "T001");
        assert_eq!(task.description, "Simple task");
        assert!(!task.completed);
        assert!(!task.parallelizable);
        assert_eq!(task.story, None);
    }

    #[test]
    fn test_parse_task_line_completed() {
        let line = "- [x] [T002] Completed task";
        let task = parse_task_line(line).unwrap();

        assert_eq!(task.id, "T002");
        assert!(task.completed);
    }

    #[test]
    fn test_parse_task_line_parallel() {
        let line = "- [ ] [T003] [P] Parallelizable task";
        let task = parse_task_line(line).unwrap();

        assert_eq!(task.id, "T003");
        assert_eq!(task.description, "Parallelizable task");
        assert!(task.parallelizable);
    }

    #[test]
    fn test_parse_task_line_with_story() {
        let line = "- [ ] [T004] [user-login] Task with story";
        let task = parse_task_line(line).unwrap();

        assert_eq!(task.id, "T004");
        assert_eq!(task.description, "Task with story");
        assert_eq!(task.story, Some("user-login".to_string()));
    }

    #[test]
    fn test_parse_task_line_all_markers() {
        let line = "- [x] [T005] [P] [user-login] All markers present";
        let task = parse_task_line(line).unwrap();

        assert_eq!(task.id, "T005");
        assert_eq!(task.description, "All markers present");
        assert!(task.completed);
        assert!(task.parallelizable);
        assert_eq!(task.story, Some("user-login".to_string()));
    }

    #[test]
    fn test_parse_task_phases() {
        let markdown = r#"
# Task Breakdown

## Setup

- [x] [T001] Add dependency
- [x] [T002] Create directory

## P1: Features

- [ ] [T003] [P] [story] Implement feature
  - Depends on: T001, T002
- [ ] [T004] Another task

**Legend**: `[P]` = Parallelizable
"#;

        let phases = parse_task_phases(markdown);
        assert_eq!(phases.len(), 2);

        assert_eq!(phases[0].name, "Setup");
        assert_eq!(phases[0].tasks.len(), 2);
        assert_eq!(phases[0].tasks[0].id, "T001");
        assert!(phases[0].tasks[0].completed);

        assert_eq!(phases[1].name, "P1: Features");
        assert_eq!(phases[1].tasks.len(), 2);
        assert_eq!(phases[1].tasks[0].id, "T003");
        assert!(phases[1].tasks[0].parallelizable);
        assert_eq!(phases[1].tasks[0].story, Some("story".to_string()));
        assert_eq!(phases[1].tasks[0].depends_on, vec!["T001", "T002"]);
    }
}
