//! Namespace detection and project context discovery
//!
//! This module provides automatic namespace detection by analyzing the current
//! directory for git repositories and CLAUDE.md project metadata files. It enables
//! the memory system to automatically scope memories to the appropriate project
//! without requiring explicit namespace specification.
//!
//! # Detection Strategy
//!
//! 1. Walk up directory tree to find `.git` folder (repository root)
//! 2. Extract project name from git remote URL or directory name
//! 3. Parse `CLAUDE.md` if present for project metadata
//! 4. Generate session namespace with unique session ID
//! 5. Fall back to Global namespace if no git repository found
//!
//! # Example
//!
//! ```ignore
//! use mnemosyne::namespace::NamespaceDetector;
//!
//! let detector = NamespaceDetector::new();
//! let namespace = detector.detect_namespace()?;
//! println!("Detected namespace: {}", namespace);
//! ```

use crate::error::{MnemosyneError, Result};
use crate::types::Namespace;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Project metadata extracted from CLAUDE.md or git repository
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectMetadata {
    /// Project name (from git remote or directory name)
    pub name: String,

    /// Optional description from CLAUDE.md
    pub description: Option<String>,

    /// Repository root path
    pub root: PathBuf,
}

/// Namespace detector with caching for performance
///
/// The detector walks up the directory tree to find git repositories and
/// parses CLAUDE.md files for project metadata. Results are cached to avoid
/// repeated filesystem operations.
#[derive(Debug, Clone)]
pub struct NamespaceDetector {
    /// Cached project metadata (None = not yet detected, Some(None) = no project)
    cached_metadata: Option<Option<ProjectMetadata>>,

    /// Base directory to start detection from
    base_dir: PathBuf,
}

impl NamespaceDetector {
    /// Create a new namespace detector starting from the current directory
    pub fn new() -> Self {
        Self::with_base_dir(env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
    }

    /// Create a namespace detector starting from a specific directory
    pub fn with_base_dir(base_dir: PathBuf) -> Self {
        Self {
            cached_metadata: None,
            base_dir,
        }
    }

    /// Detect the appropriate namespace for the current context
    ///
    /// Returns:
    /// - `Namespace::Session` if in a git repository (preferred for active work)
    /// - `Namespace::Project` if in a git repository but session not needed
    /// - `Namespace::Global` if no git repository found
    pub fn detect_namespace(&mut self) -> Result<Namespace> {
        self.detect_session_namespace()
    }

    /// Detect namespace and return a session-scoped namespace
    ///
    /// This is the primary method for active development sessions, as session
    /// namespaces provide isolation for temporary context while maintaining
    /// access to project and global memories.
    pub fn detect_session_namespace(&mut self) -> Result<Namespace> {
        match self.detect_project_metadata()? {
            Some(metadata) => {
                let session_id = Self::generate_session_id();
                Ok(Namespace::Session {
                    project: metadata.name,
                    session_id,
                })
            }
            None => Ok(Namespace::Global),
        }
    }

    /// Detect namespace and return a project-scoped namespace
    ///
    /// Use this for project-level memories that should persist across sessions
    /// (e.g., architectural decisions, configuration, constraints).
    pub fn detect_project_namespace(&mut self) -> Result<Namespace> {
        match self.detect_project_metadata()? {
            Some(metadata) => Ok(Namespace::Project { name: metadata.name }),
            None => Ok(Namespace::Global),
        }
    }

    /// Detect project metadata (git root + CLAUDE.md)
    ///
    /// This method walks up the directory tree to find a `.git` folder,
    /// then attempts to parse CLAUDE.md for additional metadata.
    pub fn detect_project_metadata(&mut self) -> Result<Option<ProjectMetadata>> {
        // Return cached result if available
        if let Some(cached) = &self.cached_metadata {
            return Ok(cached.clone());
        }

        let result = self.detect_project_metadata_uncached()?;
        self.cached_metadata = Some(result.clone());
        Ok(result)
    }

    /// Detect project root by walking up directory tree
    ///
    /// Returns the path containing `.git` folder, or None if not in a git repository.
    pub fn detect_project_root(&self) -> Result<Option<PathBuf>> {
        let mut current = self.base_dir.clone();

        loop {
            let git_dir = current.join(".git");
            if git_dir.exists() {
                return Ok(Some(current));
            }

            // Move to parent directory
            match current.parent() {
                Some(parent) => current = parent.to_path_buf(),
                None => return Ok(None), // Reached filesystem root
            }
        }
    }

    /// Read and parse project metadata from CLAUDE.md
    ///
    /// Extracts project name and description from the CLAUDE.md file at the
    /// repository root. Returns None if the file doesn't exist or can't be parsed.
    pub fn read_project_metadata(&self, root: &Path) -> Result<Option<ProjectMetadata>> {
        let claude_md_path = root.join("CLAUDE.md");

        // If CLAUDE.md doesn't exist, derive metadata from directory
        if !claude_md_path.exists() {
            return self.derive_metadata_from_directory(root);
        }

        // Read CLAUDE.md
        let content = fs::read_to_string(&claude_md_path).map_err(|e| {
            MnemosyneError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to read CLAUDE.md at {:?}: {}", claude_md_path, e),
            ))
        })?;

        // Parse metadata from CLAUDE.md
        self.parse_claude_md(&content, root)
    }

    /// Generate a unique session ID using UUID v4
    ///
    /// Session IDs are used to isolate temporary context within a project,
    /// allowing multiple concurrent sessions without interference.
    fn generate_session_id() -> String {
        // Use a short UUID for more readable session IDs
        Uuid::new_v4().to_string()[..8].to_string()
    }

    // === Private Helper Methods ===

    /// Detect project metadata without caching (internal implementation)
    fn detect_project_metadata_uncached(&self) -> Result<Option<ProjectMetadata>> {
        // Find git repository root
        let root = match self.detect_project_root()? {
            Some(r) => r,
            None => return Ok(None),
        };

        // Try to read CLAUDE.md for metadata
        self.read_project_metadata(&root)
    }

    /// Parse CLAUDE.md content to extract project metadata
    ///
    /// Looks for:
    /// - Title (first # heading)
    /// - Description (paragraph following title)
    /// - Frontmatter (YAML block at start)
    fn parse_claude_md(&self, content: &str, root: &Path) -> Result<Option<ProjectMetadata>> {
        let mut name: Option<String> = None;
        let mut description: Option<String> = None;

        // Split into lines
        let lines: Vec<&str> = content.lines().collect();

        // Check for YAML frontmatter
        if !lines.is_empty() && lines[0] == "---" {
            if let Some(end_idx) = lines[1..].iter().position(|&line| line == "---") {
                // Parse YAML frontmatter (simple key-value extraction)
                for line in &lines[1..=end_idx] {
                    if let Some((key, value)) = line.split_once(':') {
                        let key = key.trim();
                        let value = value.trim().trim_matches('"').trim_matches('\'');

                        match key {
                            "project" | "name" | "title" => {
                                name = Some(value.to_string());
                            }
                            "description" | "desc" => {
                                description = Some(value.to_string());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        // Extract from Markdown headers if no frontmatter
        if name.is_none() {
            for line in &lines {
                let trimmed = line.trim();
                if trimmed.starts_with("# ") {
                    name = Some(trimmed[2..].trim().to_string());
                    break;
                }
            }
        }

        // Extract description from first paragraph after title
        if description.is_none() {
            let mut found_title = false;
            for line in &lines {
                let trimmed = line.trim();

                if trimmed.starts_with("# ") {
                    found_title = true;
                    continue;
                }

                if found_title && !trimmed.is_empty() && !trimmed.starts_with('#') {
                    description = Some(trimmed.to_string());
                    break;
                }
            }
        }

        // Fall back to directory name if no name found
        let final_name = name.unwrap_or_else(|| self.extract_dir_name(root));

        Ok(Some(ProjectMetadata {
            name: final_name,
            description,
            root: root.to_path_buf(),
        }))
    }

    /// Derive project metadata from directory name when CLAUDE.md is absent
    fn derive_metadata_from_directory(&self, root: &Path) -> Result<Option<ProjectMetadata>> {
        let name = self.extract_dir_name(root);

        Ok(Some(ProjectMetadata {
            name,
            description: None,
            root: root.to_path_buf(),
        }))
    }

    /// Extract a clean project name from directory path
    ///
    /// Handles cases like:
    /// - `/path/to/my-project` -> "my-project"
    /// - `/path/to/project.git` -> "project"
    fn extract_dir_name(&self, root: &Path) -> String {
        root.file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.trim_end_matches(".git").to_string())
            .unwrap_or_else(|| "unknown".to_string())
    }
}

impl Default for NamespaceDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper to create a test git repository structure
    fn create_test_repo(with_claude_md: bool, claude_content: Option<&str>) -> TempDir {
        let temp = TempDir::new().unwrap();
        let git_dir = temp.path().join(".git");
        fs::create_dir(&git_dir).unwrap();

        if with_claude_md {
            let claude_md = temp.path().join("CLAUDE.md");
            let content = claude_content.unwrap_or("# Test Project\n\nA test project description.");
            fs::write(&claude_md, content).unwrap();
        }

        temp
    }

    #[test]
    fn test_detect_git_root() {
        let temp = create_test_repo(false, None);
        let subdir = temp.path().join("src").join("nested");
        fs::create_dir_all(&subdir).unwrap();

        let detector = NamespaceDetector::with_base_dir(subdir);
        let root = detector.detect_project_root().unwrap();

        assert!(root.is_some());
        assert_eq!(root.unwrap(), temp.path());
    }

    #[test]
    fn test_detect_git_root_not_found() {
        let temp = TempDir::new().unwrap();
        let detector = NamespaceDetector::with_base_dir(temp.path().to_path_buf());
        let root = detector.detect_project_root().unwrap();

        assert!(root.is_none());
    }

    #[test]
    fn test_parse_claude_md_with_title() {
        let temp = create_test_repo(true, Some("# My Awesome Project\n\nThis is a great project."));
        let detector = NamespaceDetector::with_base_dir(temp.path().to_path_buf());

        let metadata = detector
            .read_project_metadata(temp.path())
            .unwrap()
            .unwrap();

        assert_eq!(metadata.name, "My Awesome Project");
        assert_eq!(metadata.description, Some("This is a great project.".to_string()));
    }

    #[test]
    fn test_parse_claude_md_with_frontmatter() {
        let content = r#"---
project: mnemosyne
description: "Memory system for Claude Code"
---

# Mnemosyne

Additional content here.
"#;
        let temp = create_test_repo(true, Some(content));
        let detector = NamespaceDetector::with_base_dir(temp.path().to_path_buf());

        let metadata = detector
            .read_project_metadata(temp.path())
            .unwrap()
            .unwrap();

        assert_eq!(metadata.name, "mnemosyne");
        assert_eq!(
            metadata.description,
            Some("Memory system for Claude Code".to_string())
        );
    }

    #[test]
    fn test_derive_metadata_from_directory_name() {
        let temp = create_test_repo(false, None);
        let detector = NamespaceDetector::with_base_dir(temp.path().to_path_buf());

        let metadata = detector
            .read_project_metadata(temp.path())
            .unwrap()
            .unwrap();

        // TempDir names are random, just verify we got something
        assert!(!metadata.name.is_empty());
        assert_eq!(metadata.description, None);
    }

    #[test]
    fn test_detect_session_namespace() {
        let temp = create_test_repo(true, Some("# test-project\n\nTest description."));
        let mut detector = NamespaceDetector::with_base_dir(temp.path().to_path_buf());

        let namespace = detector.detect_session_namespace().unwrap();

        match namespace {
            Namespace::Session { project, session_id } => {
                assert_eq!(project, "test-project");
                assert_eq!(session_id.len(), 8); // Short UUID
            }
            _ => panic!("Expected Session namespace"),
        }
    }

    #[test]
    fn test_detect_project_namespace() {
        let temp = create_test_repo(true, Some("# test-project\n\nTest description."));
        let mut detector = NamespaceDetector::with_base_dir(temp.path().to_path_buf());

        let namespace = detector.detect_project_namespace().unwrap();

        match namespace {
            Namespace::Project { name } => {
                assert_eq!(name, "test-project");
            }
            _ => panic!("Expected Project namespace"),
        }
    }

    #[test]
    fn test_detect_global_namespace_no_git() {
        let temp = TempDir::new().unwrap();
        let mut detector = NamespaceDetector::with_base_dir(temp.path().to_path_buf());

        let namespace = detector.detect_namespace().unwrap();

        assert_eq!(namespace, Namespace::Global);
    }

    #[test]
    fn test_caching() {
        let temp = create_test_repo(true, Some("# cached-project\n\nDescription."));
        let mut detector = NamespaceDetector::with_base_dir(temp.path().to_path_buf());

        // First call should populate cache
        let metadata1 = detector.detect_project_metadata().unwrap();
        assert!(metadata1.is_some());

        // Second call should return cached result
        let metadata2 = detector.detect_project_metadata().unwrap();
        assert_eq!(metadata1, metadata2);
    }

    #[test]
    fn test_session_id_generation() {
        let id1 = NamespaceDetector::generate_session_id();
        let id2 = NamespaceDetector::generate_session_id();

        assert_eq!(id1.len(), 8);
        assert_eq!(id2.len(), 8);
        assert_ne!(id1, id2); // Should be unique
    }

    #[test]
    fn test_extract_dir_name_removes_git_suffix() {
        let temp = TempDir::new().unwrap();
        let git_dir = temp.path().join("myproject.git");
        fs::create_dir(&git_dir).unwrap();

        let detector = NamespaceDetector::new();
        let name = detector.extract_dir_name(&git_dir);

        assert_eq!(name, "myproject");
    }
}
