//! File storage and YAML parsing for artifacts

use crate::error::{MnemosyneError, Result};
use std::path::{Path, PathBuf};
use tokio::fs;

/// Artifact storage manager
pub struct ArtifactStorage {
    base_path: PathBuf,
}

impl ArtifactStorage {
    /// Create a new artifact storage manager
    pub fn new<P: AsRef<Path>>(base_path: P) -> Result<Self> {
        let base_path = base_path.as_ref().to_path_buf();
        if !base_path.exists() {
            return Err(MnemosyneError::Other(format!(
                "Artifact directory does not exist: {}",
                base_path.display()
            )));
        }

        Ok(Self { base_path })
    }

    /// Get the base path for artifacts
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    /// Get the subdirectory for a specific artifact type
    pub fn type_directory(&self, artifact_type: &str) -> PathBuf {
        self.base_path.join(artifact_type)
    }

    /// Write artifact to file
    pub async fn write_artifact<P: AsRef<Path>>(
        &self,
        path: P,
        content: &str,
    ) -> Result<()> {
        let full_path = self.base_path.join(path);

        // Ensure parent directory exists
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                MnemosyneError::Other(format!(
                    "Failed to create directory {}: {}",
                    parent.display(),
                    e
                ))
            })?;
        }

        // Write file
        fs::write(&full_path, content).await.map_err(|e| {
            MnemosyneError::Other(format!(
                "Failed to write artifact {}: {}",
                full_path.display(),
                e
            ))
        })?;

        Ok(())
    }

    /// Read artifact from file
    pub async fn read_artifact<P: AsRef<Path>>(&self, path: P) -> Result<String> {
        let full_path = self.base_path.join(path);

        fs::read_to_string(&full_path).await.map_err(|e| {
            MnemosyneError::Other(format!(
                "Failed to read artifact {}: {}",
                full_path.display(),
                e
            ))
        })
    }

    /// List all artifacts of a specific type
    pub async fn list_artifacts(&self, artifact_type: &str) -> Result<Vec<PathBuf>> {
        let type_dir = self.type_directory(artifact_type);

        if !type_dir.exists() {
            return Ok(Vec::new());
        }

        let mut entries = fs::read_dir(&type_dir).await.map_err(|e| {
            MnemosyneError::Other(format!(
                "Failed to read directory {}: {}",
                type_dir.display(),
                e
            ))
        })?;

        let mut artifacts = Vec::new();
        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            MnemosyneError::Other(format!("Failed to read directory entry: {}", e))
        })? {
            let path = entry.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "md") {
                artifacts.push(path);
            }
        }

        Ok(artifacts)
    }

    /// Check if artifact exists
    pub async fn exists<P: AsRef<Path>>(&self, path: P) -> bool {
        self.base_path.join(path).exists()
    }

    /// Delete artifact file
    pub async fn delete_artifact<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let full_path = self.base_path.join(path);

        fs::remove_file(&full_path).await.map_err(|e| {
            MnemosyneError::Other(format!(
                "Failed to delete artifact {}: {}",
                full_path.display(),
                e
            ))
        })
    }
}

/// Parse YAML frontmatter and markdown content
pub fn parse_frontmatter(content: &str) -> Result<(serde_yaml::Value, String)> {
    let lines: Vec<&str> = content.lines().collect();

    // Check for YAML frontmatter delimiter
    if !lines.first().map_or(false, |line| line.trim() == "---") {
        return Err(MnemosyneError::Other(
            "Missing YAML frontmatter delimiter".to_string(),
        ));
    }

    // Find closing delimiter
    let closing_index = lines[1..]
        .iter()
        .position(|line| line.trim() == "---")
        .ok_or_else(|| {
            MnemosyneError::Other("Missing closing YAML frontmatter delimiter".to_string())
        })?
        + 1;

    // Extract frontmatter
    let frontmatter_lines = &lines[1..closing_index];
    let frontmatter_str = frontmatter_lines.join("\n");
    let frontmatter: serde_yaml::Value =
        serde_yaml::from_str(&frontmatter_str).map_err(|e| {
            MnemosyneError::Other(format!("Failed to parse YAML frontmatter: {}", e))
        })?;

    // Extract markdown content (skip frontmatter + empty line)
    let content_start = closing_index + 1;
    let markdown = if content_start < lines.len() {
        lines[content_start..].join("\n")
    } else {
        String::new()
    };

    Ok((frontmatter, markdown.trim().to_string()))
}

/// Serialize YAML frontmatter and markdown content
pub fn serialize_frontmatter(
    frontmatter: &serde_yaml::Value,
    content: &str,
) -> Result<String> {
    let yaml = serde_yaml::to_string(frontmatter).map_err(|e| {
        MnemosyneError::Other(format!("Failed to serialize YAML frontmatter: {}", e))
    })?;

    Ok(format!("---\n{}---\n\n{}", yaml, content.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_frontmatter() {
        let content = r#"---
type: feature_spec
feature_id: test
version: 1.0.0
---

# Test Content

This is markdown content.
"#;

        let (frontmatter, markdown) = parse_frontmatter(content).unwrap();

        assert!(frontmatter.get("type").is_some());
        assert_eq!(frontmatter["type"].as_str().unwrap(), "feature_spec");
        assert!(markdown.contains("# Test Content"));
    }

    #[test]
    fn test_serialize_frontmatter() {
        let mut frontmatter = serde_yaml::Mapping::new();
        frontmatter.insert(
            serde_yaml::Value::String("type".to_string()),
            serde_yaml::Value::String("feature_spec".to_string()),
        );
        let frontmatter = serde_yaml::Value::Mapping(frontmatter);

        let content = "# Test\n\nContent here";
        let result = serialize_frontmatter(&frontmatter, content).unwrap();

        assert!(result.starts_with("---\n"));
        assert!(result.contains("type: feature_spec"));
        assert!(result.contains("# Test"));
    }

    #[test]
    fn test_parse_frontmatter_missing_delimiter() {
        let content = "type: feature_spec\n# Test";
        let result = parse_frontmatter(content);
        assert!(result.is_err());
    }
}
