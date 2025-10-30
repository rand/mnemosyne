//! Skills Discovery System
//!
//! Integrates with cc-polymath for progressive skill discovery.
//! Discovers, scores, and loads skills based on task relevance.

use crate::error::{MnemosyneError, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Skill metadata from YAML frontmatter
#[derive(Debug, Clone)]
pub struct SkillMetadata {
    pub name: String,
    pub category: String,
    pub keywords: Vec<String>,
    pub description: String,
    pub file_path: PathBuf,
}

/// Skill discovery result with relevance score
#[derive(Debug, Clone)]
pub struct SkillMatch {
    pub metadata: SkillMetadata,
    pub score: f32,
}

/// Skills discovery engine
pub struct SkillsDiscovery {
    /// Base directory for skills (cc-polymath location)
    skills_dir: PathBuf,

    /// Cached skill metadata
    skill_cache: HashMap<String, SkillMetadata>,
}

impl SkillsDiscovery {
    /// Create new skills discovery engine
    pub fn new(skills_dir: PathBuf) -> Self {
        Self {
            skills_dir,
            skill_cache: HashMap::new(),
        }
    }

    /// Discover skills relevant to a task description
    pub async fn discover_skills(
        &mut self,
        task_description: &str,
        max_skills: usize,
    ) -> Result<Vec<SkillMatch>> {
        info!("Discovering skills for: {}", task_description);

        // Extract keywords from task description
        let keywords = self.extract_keywords(task_description);
        debug!("Extracted keywords: {:?}", keywords);

        // Scan for skills if cache is empty
        if self.skill_cache.is_empty() {
            self.scan_skills_directory().await?;
        }

        // Score all skills against keywords
        let mut matches: Vec<SkillMatch> = self
            .skill_cache
            .values()
            .map(|metadata| {
                let score = self.score_skill(metadata, &keywords);
                SkillMatch {
                    metadata: metadata.clone(),
                    score,
                }
            })
            .collect();

        // Sort by score descending and take top N
        matches.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        matches.truncate(max_skills);

        info!("Discovered {} relevant skills", matches.len());
        for (i, skill_match) in matches.iter().enumerate() {
            debug!(
                "{}. {} (score: {:.2})",
                i + 1,
                skill_match.metadata.name,
                skill_match.score
            );
        }

        Ok(matches)
    }

    /// Scan skills directory and populate cache
    async fn scan_skills_directory(&mut self) -> Result<()> {
        if !self.skills_dir.exists() {
            warn!("Skills directory not found: {:?}", self.skills_dir);
            return Ok(());
        }

        info!("Scanning skills directory: {:?}", self.skills_dir);

        self.scan_directory_recursive(&self.skills_dir)?;

        info!("Loaded {} skills into cache", self.skill_cache.len());
        Ok(())
    }

    /// Recursively scan directory for .md files
    fn scan_directory_recursive(&mut self, dir: &Path) -> Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }

        let entries = fs::read_dir(dir).map_err(|e| {
            MnemosyneError::Other(format!("Failed to read directory {:?}: {}", dir, e))
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                MnemosyneError::Other(format!("Failed to read directory entry: {}", e))
            })?;

            let path = entry.path();

            if path.is_dir() {
                // Recurse into subdirectories
                self.scan_directory_recursive(&path)?;
            } else if path.extension().and_then(|s| s.to_str()) == Some("md") {
                // Parse markdown skill file
                if let Ok(metadata) = self.parse_skill_file(&path) {
                    self.skill_cache.insert(metadata.name.clone(), metadata);
                }
            }
        }

        Ok(())
    }

    /// Parse skill file and extract metadata
    fn parse_skill_file(&self, path: &Path) -> Result<SkillMetadata> {
        let content = fs::read_to_string(path).map_err(|e| {
            MnemosyneError::Other(format!("Failed to read skill file {:?}: {}", path, e))
        })?;

        // Extract YAML frontmatter if present
        let (frontmatter, body) = if content.starts_with("---") {
            let parts: Vec<&str> = content.splitn(3, "---").collect();
            if parts.len() >= 3 {
                (parts[1], parts[2])
            } else {
                ("", content.as_str())
            }
        } else {
            ("", content.as_str())
        };

        // Parse frontmatter (simple key: value format)
        let mut name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        let mut category = path
            .parent()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .unwrap_or("general")
            .to_string();
        let mut keywords = Vec::new();
        let mut description = String::new();

        for line in frontmatter.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "name" => name = value.to_string(),
                    "category" => category = value.to_string(),
                    "keywords" => {
                        keywords = value
                            .split(',')
                            .map(|s| s.trim().to_lowercase())
                            .collect();
                    }
                    "description" => description = value.to_string(),
                    _ => {}
                }
            }
        }

        // Extract description from body if not in frontmatter
        if description.is_empty() {
            // Use first paragraph as description
            description = body
                .lines()
                .skip_while(|l| l.trim().is_empty())
                .take_while(|l| !l.trim().is_empty())
                .collect::<Vec<_>>()
                .join(" ")
                .chars()
                .take(200)
                .collect();
        }

        // Extract keywords from body if not in frontmatter
        if keywords.is_empty() {
            keywords = self.extract_keywords(body);
        }

        Ok(SkillMetadata {
            name,
            category,
            keywords,
            description,
            file_path: path.to_path_buf(),
        })
    }

    /// Extract keywords from text
    fn extract_keywords(&self, text: &str) -> Vec<String> {
        let text_lower = text.to_lowercase();

        // Common technical keywords to look for
        let keyword_patterns = [
            // Languages
            "rust", "python", "typescript", "javascript", "go", "zig",
            // Frameworks
            "react", "nextjs", "vue", "svelte", "django", "flask", "fastapi",
            // Databases
            "postgres", "mongodb", "redis", "sqlite", "mysql",
            // DevOps
            "docker", "kubernetes", "aws", "gcp", "azure", "terraform",
            // Testing
            "pytest", "jest", "vitest", "cargo test",
            // ML/AI
            "pytorch", "tensorflow", "transformers", "embeddings", "llm",
            // Tools
            "git", "github", "gitlab", "ci/cd", "mcp",
        ];

        keyword_patterns
            .iter()
            .filter(|&&keyword| text_lower.contains(keyword))
            .map(|&s| s.to_string())
            .collect()
    }

    /// Score a skill against keywords (0.0-1.0)
    fn score_skill(&self, metadata: &SkillMetadata, keywords: &[String]) -> f32 {
        if keywords.is_empty() {
            return 0.0;
        }

        let mut score = 0.0;
        let mut matches = 0;

        for keyword in keywords {
            // Check skill keywords
            if metadata.keywords.iter().any(|k| k.contains(keyword)) {
                score += 1.0;
                matches += 1;
            }

            // Check skill name
            if metadata.name.to_lowercase().contains(keyword) {
                score += 0.5;
            }

            // Check category
            if metadata.category.to_lowercase().contains(keyword) {
                score += 0.3;
            }

            // Check description
            if metadata.description.to_lowercase().contains(keyword) {
                score += 0.2;
            }
        }

        // Normalize by number of keywords
        score / keywords.len() as f32
    }

    /// Load skill content from file
    pub async fn load_skill(&self, skill_match: &SkillMatch) -> Result<String> {
        let content = fs::read_to_string(&skill_match.metadata.file_path).map_err(|e| {
            MnemosyneError::Other(format!(
                "Failed to load skill {:?}: {}",
                skill_match.metadata.file_path, e
            ))
        })?;

        debug!("Loaded skill: {}", skill_match.metadata.name);
        Ok(content)
    }
}

/// Get default skills directory
pub fn get_skills_directory() -> PathBuf {
    // Check for cc-polymath in common locations
    if let Ok(home) = std::env::var("HOME") {
        let home_path = PathBuf::from(home);

        // Check ~/.claude/skills
        let claude_skills = home_path.join(".claude").join("skills");
        if claude_skills.exists() {
            return claude_skills;
        }

        // Check ~/src/cc-polymath/skills
        let src_polymath = home_path.join("src").join("cc-polymath").join("skills");
        if src_polymath.exists() {
            return src_polymath;
        }

        // Check ~/cc-polymath/skills
        let home_polymath = home_path.join("cc-polymath").join("skills");
        if home_polymath.exists() {
            return home_polymath;
        }
    }

    // Check SKILLS_DIR environment variable
    if let Ok(skills_dir) = std::env::var("SKILLS_DIR") {
        let path = PathBuf::from(skills_dir);
        if path.exists() {
            return path;
        }
    }

    // Fall back to ./skills in current directory
    PathBuf::from("skills")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_extract_keywords() {
        let discovery = SkillsDiscovery::new(PathBuf::from("."));

        let text = "This is a Rust project using PostgreSQL and Redis for caching.";
        let keywords = discovery.extract_keywords(text);

        assert!(keywords.contains(&"rust".to_string()));
        assert!(keywords.contains(&"postgres".to_string()));
        assert!(keywords.contains(&"redis".to_string()));
    }

    #[test]
    fn test_score_skill() {
        let discovery = SkillsDiscovery::new(PathBuf::from("."));

        let metadata = SkillMetadata {
            name: "database-postgres".to_string(),
            category: "database".to_string(),
            keywords: vec!["postgres".to_string(), "sql".to_string()],
            description: "PostgreSQL database skills".to_string(),
            file_path: PathBuf::from("test.md"),
        };

        let keywords = vec!["postgres".to_string(), "database".to_string()];
        let score = discovery.score_skill(&metadata, &keywords);

        assert!(score > 0.5); // Should have good relevance
    }

    #[tokio::test]
    async fn test_parse_skill_file() {
        let temp_dir = TempDir::new().unwrap();
        let skill_file = temp_dir.path().join("test-skill.md");

        let content = r#"---
name: test-skill
category: testing
keywords: rust, testing, cargo
description: Testing skills for Rust
---

# Test Skill

This is a test skill for testing purposes.
"#;

        fs::write(&skill_file, content).unwrap();

        let discovery = SkillsDiscovery::new(temp_dir.path().to_path_buf());
        let metadata = discovery.parse_skill_file(&skill_file).unwrap();

        assert_eq!(metadata.name, "test-skill");
        assert_eq!(metadata.category, "testing");
        assert!(metadata.keywords.contains(&"rust".to_string()));
    }
}
