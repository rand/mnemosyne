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
use super::{Constitution, FeatureSpec};
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
    use super::*;

    #[test]
    fn test_workflow_coordinator_creation() {
        // Basic creation test
        // Full integration tests require StorageBackend mock
    }
}
