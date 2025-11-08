//! Version checking and update system for mnemosyne and dependencies.
//!
//! This module provides functionality to:
//! - Check for updates to mnemosyne, Claude Code, and beads
//! - Compare semantic versions
//! - Cache check results (24-hour throttling)
//! - Detect installed tool versions

use crate::error::{MnemosyneError, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{debug, warn};

/// Tool that can be version-checked
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Tool {
    Mnemosyne,
    ClaudeCode,
    Beads,
}

impl Tool {
    pub fn name(&self) -> &'static str {
        match self {
            Tool::Mnemosyne => "mnemosyne",
            Tool::ClaudeCode => "claude",
            Tool::Beads => "bd",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Tool::Mnemosyne => "Mnemosyne",
            Tool::ClaudeCode => "Claude Code",
            Tool::Beads => "Beads",
        }
    }

    pub fn all() -> Vec<Tool> {
        vec![Tool::Mnemosyne, Tool::ClaudeCode, Tool::Beads]
    }
}

/// Information about a tool's version status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    pub tool: Tool,
    pub installed: Option<String>,
    pub latest: Option<String>,
    pub update_available: bool,
    pub is_installed: bool,
    pub release_url: Option<String>,
}

/// Cached version check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionCheckCache {
    pub tool: Tool,
    pub latest_version: String,
    pub checked_at: u64,
    pub release_url: String,
}

impl VersionCheckCache {
    pub fn is_stale(&self, max_age_hours: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let age_hours = (now - self.checked_at) / 3600;
        age_hours >= max_age_hours
    }
}

/// GitHub release response
#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
}

/// npm registry response (simplified)
#[derive(Debug, Deserialize)]
struct NpmPackage {
    #[serde(rename = "dist-tags")]
    dist_tags: NpmDistTags,
}

#[derive(Debug, Deserialize)]
struct NpmDistTags {
    latest: String,
}

/// Version checker service
pub struct VersionChecker {
    client: Client,
    _cache_max_age_hours: u64,
}

impl VersionChecker {
    /// Create a new version checker
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(5))
            .user_agent("mnemosyne-version-checker")
            .build()
            .map_err(|e| MnemosyneError::NetworkError(e.to_string()))?;

        Ok(Self {
            client,
            _cache_max_age_hours: 24,
        })
    }

    /// Check for updates to all tools
    pub async fn check_all_tools(&self) -> Result<Vec<VersionInfo>> {
        let mut results = Vec::new();

        for tool in Tool::all() {
            match self.check_tool(tool).await {
                Ok(info) => results.push(info),
                Err(e) => {
                    warn!("Failed to check {} version: {}", tool.display_name(), e);
                    // Add a failed entry
                    results.push(VersionInfo {
                        tool,
                        installed: self.detect_installed_version(tool),
                        latest: None,
                        update_available: false,
                        is_installed: self.is_tool_installed(tool),
                        release_url: None,
                    });
                }
            }
        }

        Ok(results)
    }

    /// Check for updates to a specific tool
    pub async fn check_tool(&self, tool: Tool) -> Result<VersionInfo> {
        let installed = self.detect_installed_version(tool);
        let is_installed = installed.is_some();

        // Fetch latest version from appropriate source
        // Note: Use npm for Beads since that's the primary distribution method,
        // even though GitHub releases may have newer versions not yet published to npm
        let (latest, release_url) = match tool {
            Tool::Mnemosyne => self.fetch_github_latest("rand", "mnemosyne").await?,
            Tool::ClaudeCode => self.fetch_npm_latest("@anthropic-ai/claude-code").await?,
            Tool::Beads => self.fetch_npm_latest("@beads/bd").await?,
        };

        let update_available = if let Some(installed_ver) = &installed {
            Self::is_newer_version(&latest, installed_ver)
        } else {
            false
        };

        Ok(VersionInfo {
            tool,
            installed,
            latest: Some(latest),
            update_available,
            is_installed,
            release_url: Some(release_url),
        })
    }

    /// Fetch latest version from GitHub releases
    async fn fetch_github_latest(&self, owner: &str, repo: &str) -> Result<(String, String)> {
        let url = format!("https://api.github.com/repos/{}/{}/releases/latest", owner, repo);

        debug!("Fetching latest release from: {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MnemosyneError::NetworkError(format!("GitHub API request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(MnemosyneError::NetworkError(format!(
                "GitHub API returned status: {}",
                response.status()
            )));
        }

        let release: GitHubRelease = response
            .json()
            .await
            .map_err(|e| MnemosyneError::NetworkError(format!("Failed to parse GitHub response: {}", e)))?;

        // Strip 'v' prefix if present
        let version = release.tag_name.strip_prefix('v').unwrap_or(&release.tag_name).to_string();

        Ok((version, release.html_url))
    }

    /// Fetch latest version from npm registry
    async fn fetch_npm_latest(&self, package: &str) -> Result<(String, String)> {
        let url = format!("https://registry.npmjs.org/{}", package);

        debug!("Fetching npm package info from: {}", url);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| MnemosyneError::NetworkError(format!("npm registry request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(MnemosyneError::NetworkError(format!(
                "npm registry returned status: {}",
                response.status()
            )));
        }

        let package_info: NpmPackage = response
            .json()
            .await
            .map_err(|e| MnemosyneError::NetworkError(format!("Failed to parse npm response: {}", e)))?;

        let version = package_info.dist_tags.latest;
        let release_url = format!("https://www.npmjs.com/package/{}", package);

        Ok((version, release_url))
    }

    /// Detect if a tool is installed
    pub fn is_tool_installed(&self, tool: Tool) -> bool {
        self.detect_installed_version(tool).is_some()
    }

    /// Detect the installed version of a tool
    pub fn detect_installed_version(&self, tool: Tool) -> Option<String> {
        let binary_name = tool.name();

        // Try executing with --version
        let output = Command::new(binary_name)
            .arg("--version")
            .output()
            .ok()?;

        if !output.status.success() {
            return None;
        }

        let version_output = String::from_utf8_lossy(&output.stdout);
        Self::parse_version_output(&version_output, tool)
    }

    /// Detect the path to a tool's binary
    pub fn detect_tool_path(&self, tool: Tool) -> Option<PathBuf> {
        let binary_name = tool.name();

        // Common installation locations
        let search_paths = vec![
            PathBuf::from(format!("{}/bin/{}", std::env::var("HOME").ok()?, binary_name)),
            PathBuf::from(format!("{}/.cargo/bin/{}", std::env::var("HOME").ok()?, binary_name)),
            PathBuf::from(format!("{}/.local/bin/{}", std::env::var("HOME").ok()?, binary_name)),
            PathBuf::from(format!("/usr/local/bin/{}", binary_name)),
            PathBuf::from(format!("/opt/homebrew/bin/{}", binary_name)),
        ];

        for path in search_paths {
            if path.exists() && path.is_file() {
                return Some(path);
            }
        }

        // Try using 'which'
        let which_output = Command::new("which")
            .arg(binary_name)
            .output()
            .ok()?;

        if which_output.status.success() {
            let path_str = String::from_utf8_lossy(&which_output.stdout);
            let path = PathBuf::from(path_str.trim());
            if path.exists() {
                return Some(path);
            }
        }

        None
    }

    /// Parse version from command output
    fn parse_version_output(output: &str, tool: Tool) -> Option<String> {
        let output = output.trim();

        match tool {
            Tool::Mnemosyne => {
                // Output format: "mnemosyne 2.1.1" or just "2.1.1"
                if let Some(version) = output.strip_prefix("mnemosyne") {
                    Some(version.trim().to_string())
                } else {
                    Some(output.to_string())
                }
            }
            Tool::ClaudeCode => {
                // Output format may vary, look for version pattern
                output
                    .split_whitespace()
                    .find(|s| s.chars().next().is_some_and(|c| c.is_ascii_digit()))
                    .map(|s| s.to_string())
            }
            Tool::Beads => {
                // Output format: "bd version 0.20.1" or similar
                output
                    .split_whitespace()
                    .find(|s| s.chars().next().is_some_and(|c| c.is_ascii_digit()))
                    .map(|s| s.to_string())
            }
        }
    }

    /// Compare versions using semantic versioning rules
    /// Returns true if `latest` is newer than `current`
    pub fn is_newer_version(latest: &str, current: &str) -> bool {
        let latest = Self::normalize_version(latest);
        let current = Self::normalize_version(current);

        let latest_parts = Self::parse_version_parts(&latest);
        let current_parts = Self::parse_version_parts(&current);

        latest_parts > current_parts
    }

    /// Normalize version string (remove v prefix, etc.)
    fn normalize_version(version: &str) -> String {
        version
            .trim()
            .strip_prefix('v')
            .unwrap_or(version)
            .to_string()
    }

    /// Parse version into comparable parts (major, minor, patch)
    fn parse_version_parts(version: &str) -> Vec<u32> {
        version
            .split('.')
            .filter_map(|part| {
                // Handle versions like "1.2.3-beta"
                part.split('-')
                    .next()
                    .and_then(|p| p.parse::<u32>().ok())
            })
            .collect()
    }
}

impl Default for VersionChecker {
    fn default() -> Self {
        Self::new().expect("Failed to create version checker")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_comparison() {
        assert!(VersionChecker::is_newer_version("2.1.0", "2.0.0"));
        assert!(VersionChecker::is_newer_version("2.0.1", "2.0.0"));
        assert!(VersionChecker::is_newer_version("3.0.0", "2.9.9"));
        assert!(!VersionChecker::is_newer_version("2.0.0", "2.0.0"));
        assert!(!VersionChecker::is_newer_version("2.0.0", "2.1.0"));
        assert!(!VersionChecker::is_newer_version("1.9.9", "2.0.0"));
    }

    #[test]
    fn test_version_normalization() {
        assert_eq!(VersionChecker::normalize_version("v2.1.0"), "2.1.0");
        assert_eq!(VersionChecker::normalize_version("2.1.0"), "2.1.0");
        assert_eq!(VersionChecker::normalize_version("  v2.1.0  "), "2.1.0");
    }

    #[test]
    fn test_parse_version_output() {
        assert_eq!(
            VersionChecker::parse_version_output("mnemosyne 2.1.1", Tool::Mnemosyne),
            Some("2.1.1".to_string())
        );
        assert_eq!(
            VersionChecker::parse_version_output("2.1.1", Tool::Mnemosyne),
            Some("2.1.1".to_string())
        );
    }

    #[test]
    fn test_tool_names() {
        assert_eq!(Tool::Mnemosyne.name(), "mnemosyne");
        assert_eq!(Tool::ClaudeCode.name(), "claude");
        assert_eq!(Tool::Beads.name(), "bd");
    }
}
