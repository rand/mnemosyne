//! Workflow coordinator for artifact creation with memory integration
//!
//! This module provides high-level workflow functions that coordinate:
//! - Artifact file creation (via ArtifactStorage)
//! - Memory entry creation (via MemoryLinker)
//! - Graph linking between artifacts and memories
//!
//! Use these functions to implement complete specification workflow operations.

use super::storage::ArtifactStorage;
use super::memory_link::MemoryLinker;
use super::types::Artifact;
use super::{Constitution, FeatureSpec, ImplementationPlan, TaskBreakdown, QualityChecklist, Clarification};
use crate::error::Result;
use crate::types::{MemoryId, MemoryType, Namespace};
use std::sync::Arc;
use crate::storage::StorageBackend;

/// Workflow coordinator for artifact operations
pub struct ArtifactWorkflow {
    storage: ArtifactStorage,
    memory_linker: MemoryLinker,
}

impl ArtifactWorkflow {
    /// Create a new workflow coordinator
    pub fn new(
        artifact_base_path: std::path::PathBuf,
        memory_backend: Arc<dyn StorageBackend>,
    ) -> Result<Self> {
        let storage = ArtifactStorage::new(artifact_base_path)?;
        let memory_linker = MemoryLinker::new(memory_backend);

        Ok(Self {
            storage,
            memory_linker,
        })
    }

    /// Save constitution artifact and create memory entry
    ///
    /// This performs a complete constitution workflow:
    /// 1. Serialize constitution to markdown with YAML frontmatter
    /// 2. Write to .mnemosyne/artifacts/constitution/project-constitution.md
    /// 3. Create memory entry with high importance (9)
    /// 4. Update constitution metadata with memory ID
    /// 5. Re-save constitution with memory ID reference
    ///
    /// # Returns
    /// Memory ID of the created memory entry
    pub async fn save_constitution(
        &self,
        constitution: &mut Constitution,
        namespace: Namespace,
    ) -> Result<MemoryId> {
        // 1. Generate markdown
        let markdown = constitution.to_markdown()?;

        // 2. Save to file
        let file_path = constitution.file_path();
        self.storage.write_artifact(&file_path, &markdown).await?;

        // 3. Create memory entry
        let artifact_path = format!(".mnemosyne/artifacts/{}", file_path.display());
        let content = format!(
            "{}\n\nFull constitution: {}",
            constitution.principles.join(" | "),
            artifact_path
        );

        let memory_id = self
            .memory_linker
            .create_artifact_memory(
                MemoryType::Constitution,
                content,
                namespace,
                artifact_path,
                9, // High importance for constitution
                vec!["constitution".to_string(), "principles".to_string()],
            )
            .await?;

        // 4. Update constitution with memory ID
        constitution.metadata.memory_id = Some(memory_id.to_string());

        // 5. Re-save with memory ID
        let updated_markdown = constitution.to_markdown()?;
        self.storage
            .write_artifact(&file_path, &updated_markdown)
            .await?;

        Ok(memory_id)
    }

    /// Save feature spec artifact and create memory entry with constitution link
    ///
    /// This performs a complete feature spec workflow:
    /// 1. Serialize spec to markdown with YAML frontmatter
    /// 2. Write to .mnemosyne/artifacts/specs/{feature-id}.md
    /// 3. Create memory entry with importance 8
    /// 4. Link to constitution memory if provided
    /// 5. Update spec metadata with memory ID
    /// 6. Re-save spec with memory ID reference
    ///
    /// # Arguments
    /// * `spec` - Feature specification to save
    /// * `namespace` - Memory namespace
    /// * `constitution_memory_id` - Optional constitution memory to link to
    ///
    /// # Returns
    /// Memory ID of the created memory entry
    pub async fn save_feature_spec(
        &self,
        spec: &mut FeatureSpec,
        namespace: Namespace,
        constitution_memory_id: Option<String>,
    ) -> Result<MemoryId> {
        // 1. Generate markdown
        let markdown = spec.to_markdown()?;

        // 2. Save to file
        let file_path = spec.file_path();
        self.storage.write_artifact(&file_path, &markdown).await?;

        // 3. Create memory entry
        let artifact_path = format!(".mnemosyne/artifacts/{}", file_path.display());

        // Build summary from first scenario or requirements
        let summary = if let Some(scenario) = spec.scenarios.first() {
            format!("{}: {}", scenario.priority, scenario.goal)
        } else if let Some(req) = spec.requirements.first() {
            req.clone()
        } else {
            spec.metadata.name.clone()
        };

        let content = format!(
            "{}\n\nFull specification: {}",
            summary,
            artifact_path
        );

        let tags = vec![
            "spec".to_string(),
            "feature".to_string(),
            spec.feature_id.clone(),
        ];

        let memory_id = self
            .memory_linker
            .create_artifact_memory(
                MemoryType::FeatureSpec,
                content,
                namespace,
                artifact_path,
                8, // High importance for specs
                tags,
            )
            .await?;

        // 4. Update spec with memory ID and constitution reference
        spec.metadata.memory_id = Some(memory_id.to_string());
        if let Some(constitution_id) = constitution_memory_id {
            spec.metadata.references.push(constitution_id);
        }

        // 5. Re-save with memory ID
        let updated_markdown = spec.to_markdown()?;
        self.storage
            .write_artifact(&file_path, &updated_markdown)
            .await?;

        Ok(memory_id)
    }

    /// Load constitution from file
    pub async fn load_constitution(&self) -> Result<Constitution> {
        let file_path = "constitution/project-constitution.md";
        let content = self.storage.read_artifact(file_path).await?;
        Constitution::from_markdown(&content)
    }

    /// Load feature spec from file
    pub async fn load_feature_spec(&self, feature_id: &str) -> Result<FeatureSpec> {
        let file_path = format!("specs/{}.md", feature_id);
        let content = self.storage.read_artifact(&file_path).await?;
        FeatureSpec::from_markdown(&content)
    }

    /// Save implementation plan artifact and create memory entry
    ///
    /// This performs a complete plan workflow:
    /// 1. Serialize plan to markdown with YAML frontmatter
    /// 2. Write to .mnemosyne/artifacts/plans/{feature-id}-plan.md
    /// 3. Create memory entry with importance 7
    /// 4. Link to spec memory if provided
    /// 5. Update plan metadata with memory ID
    /// 6. Re-save plan with memory ID reference
    ///
    /// # Arguments
    /// * `plan` - Implementation plan to save
    /// * `namespace` - Memory namespace
    /// * `spec_memory_id` - Optional spec memory to link to
    ///
    /// # Returns
    /// Memory ID of the created memory entry
    pub async fn save_implementation_plan(
        &self,
        plan: &mut ImplementationPlan,
        namespace: Namespace,
        spec_memory_id: Option<String>,
    ) -> Result<MemoryId> {
        // 1. Generate markdown
        let markdown = plan.to_markdown()?;

        // 2. Save to file
        let file_path = plan.file_path();
        self.storage.write_artifact(&file_path, &markdown).await?;

        // 3. Create memory entry
        let artifact_path = format!(".mnemosyne/artifacts/{}", file_path.display());

        // Build summary from approach
        let summary = if plan.approach.len() > 100 {
            format!("{}...", &plan.approach[..100])
        } else {
            plan.approach.clone()
        };

        let content = format!(
            "{}\n\nFull plan: {}",
            summary,
            artifact_path
        );

        let tags = vec![
            "plan".to_string(),
            "implementation".to_string(),
            plan.feature_id.clone(),
        ];

        let memory_id = self
            .memory_linker
            .create_artifact_memory(
                MemoryType::ImplementationPlan,
                content,
                namespace,
                artifact_path,
                7, // Medium-high importance
                tags,
            )
            .await?;

        // 4. Update plan with memory ID and spec reference
        plan.metadata.memory_id = Some(memory_id.to_string());
        if let Some(spec_id) = spec_memory_id {
            plan.metadata.references.push(spec_id);
        }

        // 5. Re-save with memory ID
        let updated_markdown = plan.to_markdown()?;
        self.storage
            .write_artifact(&file_path, &updated_markdown)
            .await?;

        Ok(memory_id)
    }

    /// Load implementation plan from file
    pub async fn load_implementation_plan(&self, feature_id: &str) -> Result<ImplementationPlan> {
        let file_path = format!("plans/{}-plan.md", feature_id);
        let content = self.storage.read_artifact(&file_path).await?;
        ImplementationPlan::from_markdown(&content)
    }

    /// Save task breakdown artifact and create memory entry
    ///
    /// This performs a complete task breakdown workflow:
    /// 1. Serialize tasks to markdown with YAML frontmatter
    /// 2. Write to .mnemosyne/artifacts/tasks/{feature-id}-tasks.md
    /// 3. Create memory entry with importance 7
    /// 4. Link to plan memory if provided
    /// 5. Update tasks metadata with memory ID
    /// 6. Re-save tasks with memory ID reference
    ///
    /// # Arguments
    /// * `tasks` - Task breakdown to save
    /// * `namespace` - Memory namespace
    /// * `plan_memory_id` - Optional plan memory to link to
    ///
    /// # Returns
    /// Memory ID of the created memory entry
    pub async fn save_task_breakdown(
        &self,
        tasks: &mut TaskBreakdown,
        namespace: Namespace,
        plan_memory_id: Option<String>,
    ) -> Result<MemoryId> {
        // 1. Generate markdown
        let markdown = tasks.to_markdown()?;

        // 2. Save to file
        let file_path = tasks.file_path();
        self.storage.write_artifact(&file_path, &markdown).await?;

        // 3. Create memory entry
        let artifact_path = format!(".mnemosyne/artifacts/{}", file_path.display());

        // Build summary from phases
        let total_tasks: usize = tasks.phases.iter().map(|p| p.tasks.len()).sum();
        let summary = format!(
            "{} phases, {} tasks",
            tasks.phases.len(),
            total_tasks
        );

        let content = format!(
            "{}\n\nFull tasks: {}",
            summary,
            artifact_path
        );

        let tags = vec![
            "tasks".to_string(),
            "breakdown".to_string(),
            tasks.feature_id.clone(),
        ];

        let memory_id = self
            .memory_linker
            .create_artifact_memory(
                MemoryType::TaskBreakdown,
                content,
                namespace,
                artifact_path,
                7, // Medium-high importance
                tags,
            )
            .await?;

        // 4. Update tasks with memory ID and plan reference
        tasks.metadata.memory_id = Some(memory_id.to_string());
        if let Some(plan_id) = plan_memory_id {
            tasks.metadata.references.push(plan_id);
        }

        // 5. Re-save with memory ID
        let updated_markdown = tasks.to_markdown()?;
        self.storage
            .write_artifact(&file_path, &updated_markdown)
            .await?;

        Ok(memory_id)
    }

    /// Load task breakdown from file
    pub async fn load_task_breakdown(&self, feature_id: &str) -> Result<TaskBreakdown> {
        let file_path = format!("tasks/{}-tasks.md", feature_id);
        let content = self.storage.read_artifact(&file_path).await?;
        TaskBreakdown::from_markdown(&content)
    }

    /// Save quality checklist artifact and create memory entry
    ///
    /// This performs a complete checklist workflow:
    /// 1. Serialize checklist to markdown with YAML frontmatter
    /// 2. Write to .mnemosyne/artifacts/checklists/{feature-id}-checklist.md
    /// 3. Create memory entry with importance 7
    /// 4. Link to spec memory if provided
    /// 5. Update checklist metadata with memory ID
    /// 6. Re-save checklist with memory ID reference
    ///
    /// # Arguments
    /// * `checklist` - Quality checklist to save
    /// * `namespace` - Memory namespace
    /// * `spec_memory_id` - Optional spec memory to link to
    ///
    /// # Returns
    /// Memory ID of the created memory entry
    pub async fn save_quality_checklist(
        &self,
        checklist: &mut QualityChecklist,
        namespace: Namespace,
        spec_memory_id: Option<String>,
    ) -> Result<MemoryId> {
        // 1. Generate markdown
        let markdown = checklist.to_markdown()?;

        // 2. Save to file
        let file_path = checklist.file_path();
        self.storage.write_artifact(&file_path, &markdown).await?;

        // 3. Create memory entry
        let artifact_path = format!(".mnemosyne/artifacts/{}", file_path.display());

        // Build summary from completion
        let completion = checklist.completion_percentage();
        let summary = format!(
            "{} sections, {:.1}% complete",
            checklist.sections.len(),
            completion
        );

        let content = format!(
            "{}\n\nFull checklist: {}",
            summary,
            artifact_path
        );

        let tags = vec![
            "checklist".to_string(),
            "quality".to_string(),
            checklist.feature_id.clone(),
        ];

        let memory_id = self
            .memory_linker
            .create_artifact_memory(
                MemoryType::QualityChecklist,
                content,
                namespace,
                artifact_path,
                7, // Medium-high importance
                tags,
            )
            .await?;

        // 4. Update checklist with memory ID and spec reference
        checklist.metadata.memory_id = Some(memory_id.to_string());
        if let Some(spec_id) = spec_memory_id {
            checklist.metadata.references.push(spec_id);
        }

        // 5. Re-save with memory ID
        let updated_markdown = checklist.to_markdown()?;
        self.storage
            .write_artifact(&file_path, &updated_markdown)
            .await?;

        Ok(memory_id)
    }

    /// Load quality checklist from file
    pub async fn load_quality_checklist(&self, feature_id: &str) -> Result<QualityChecklist> {
        let file_path = format!("checklists/{}-checklist.md", feature_id);
        let content = self.storage.read_artifact(&file_path).await?;
        QualityChecklist::from_markdown(&content)
    }

    /// Save clarification artifact and create memory entry
    ///
    /// This performs a complete clarification workflow:
    /// 1. Serialize clarification to markdown with YAML frontmatter
    /// 2. Write to .mnemosyne/artifacts/clarifications/{feature-id}-clarifications.md
    /// 3. Create memory entry with importance 6
    /// 4. Link to spec memory if provided
    /// 5. Update clarification metadata with memory ID
    /// 6. Re-save clarification with memory ID reference
    ///
    /// # Arguments
    /// * `clarification` - Clarification to save
    /// * `namespace` - Memory namespace
    /// * `spec_memory_id` - Optional spec memory to link to
    ///
    /// # Returns
    /// Memory ID of the created memory entry
    pub async fn save_clarification(
        &self,
        clarification: &mut Clarification,
        namespace: Namespace,
        spec_memory_id: Option<String>,
    ) -> Result<MemoryId> {
        // 1. Generate markdown
        let markdown = clarification.to_markdown()?;

        // 2. Save to file
        let file_path = clarification.file_path();
        self.storage.write_artifact(&file_path, &markdown).await?;

        // 3. Create memory entry
        let artifact_path = format!(".mnemosyne/artifacts/{}", file_path.display());

        // Build summary from completion status
        let total = clarification.items.len();
        let resolved = clarification.items.iter().filter(|i| i.decision.is_some()).count();
        let summary = format!(
            "{} clarifications: {} resolved, {} pending",
            total,
            resolved,
            total - resolved
        );

        let content = format!(
            "{}\n\nFull clarifications: {}",
            summary,
            artifact_path
        );

        let tags = vec![
            "clarification".to_string(),
            "questions".to_string(),
            clarification.feature_id.clone(),
        ];

        let memory_id = self
            .memory_linker
            .create_artifact_memory(
                MemoryType::Clarification,
                content,
                namespace,
                artifact_path,
                6, // Medium importance (supplementary artifact)
                tags,
            )
            .await?;

        // 4. Update clarification with memory ID and spec reference
        clarification.metadata.memory_id = Some(memory_id.to_string());
        if let Some(spec_id) = spec_memory_id {
            clarification.metadata.references.push(spec_id);
        }

        // 5. Re-save with memory ID
        let updated_markdown = clarification.to_markdown()?;
        self.storage
            .write_artifact(&file_path, &updated_markdown)
            .await?;

        Ok(memory_id)
    }

    /// Load clarification from file
    pub async fn load_clarification(&self, feature_id: &str) -> Result<Clarification> {
        let file_path = format!("clarifications/{}-clarifications.md", feature_id);
        let content = self.storage.read_artifact(&file_path).await?;
        Clarification::from_markdown(&content)
    }

    /// Check if constitution exists
    pub async fn constitution_exists(&self) -> bool {
        self.storage
            .exists("constitution/project-constitution.md")
            .await
    }

    /// Check if feature spec exists
    pub async fn feature_spec_exists(&self, feature_id: &str) -> bool {
        self.storage
            .exists(format!("specs/{}.md", feature_id))
            .await
    }

    /// List all feature specs
    pub async fn list_feature_specs(&self) -> Result<Vec<std::path::PathBuf>> {
        self.storage.list_artifacts("specs").await
    }

    /// Get reference to storage for advanced operations
    pub fn storage(&self) -> &ArtifactStorage {
        &self.storage
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_workflow_coordinator_creation() {
        // Basic creation test
        // Full integration tests require StorageBackend mock
    }
}
