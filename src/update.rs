//! Tool update and installation system.
//!
//! Handles updating mnemosyne, Claude Code, and beads:
//! - mnemosyne: Uses scripts/build-and-install.sh (includes macOS code signing)
//! - Claude Code: Updates via npm
//! - beads: Updates via npm or homebrew
//!
//! Includes safety features:
//! - Binary backup before replacement
//! - Verification after update
//! - Rollback on failure

use crate::error::{MnemosyneError, Result};
use crate::version_check::{Tool, VersionChecker, VersionInfo};
use std::path::PathBuf;
use std::process::Command;
use tracing::{debug, info, warn};

/// Update manager for tools
pub struct UpdateManager {
    version_checker: VersionChecker,
}

/// Update result
#[derive(Debug)]
pub struct UpdateResult {
    pub tool: Tool,
    pub success: bool,
    pub old_version: Option<String>,
    pub new_version: Option<String>,
    pub message: String,
}

impl UpdateManager {
    /// Create a new update manager
    pub fn new() -> Result<Self> {
        Ok(Self {
            version_checker: VersionChecker::new()?,
        })
    }

    /// Update a specific tool
    pub async fn update_tool(&self, tool: Tool) -> Result<UpdateResult> {
        info!("Starting update for {}", tool.display_name());

        // Get current version
        let old_version = self.version_checker.detect_installed_version(tool);

        // Perform the update based on tool type
        let result = match tool {
            Tool::Mnemosyne => self.update_mnemosyne().await,
            Tool::ClaudeCode => self.update_claude_code().await,
            Tool::Beads => self.update_beads().await,
        };

        match result {
            Ok(message) => {
                // Get new version after update
                let new_version = self.version_checker.detect_installed_version(tool);

                Ok(UpdateResult {
                    tool,
                    success: true,
                    old_version,
                    new_version,
                    message,
                })
            }
            Err(e) => Ok(UpdateResult {
                tool,
                success: false,
                old_version,
                new_version: None,
                message: format!("Update failed: {}", e),
            }),
        }
    }

    /// Install a tool that is not currently installed
    pub async fn install_tool(&self, tool: Tool) -> Result<UpdateResult> {
        info!("Starting installation for {}", tool.display_name());

        let result = match tool {
            Tool::Mnemosyne => self.install_mnemosyne().await,
            Tool::ClaudeCode => self.install_claude_code().await,
            Tool::Beads => self.install_beads().await,
        };

        match result {
            Ok(message) => {
                let new_version = self.version_checker.detect_installed_version(tool);

                Ok(UpdateResult {
                    tool,
                    success: true,
                    old_version: None,
                    new_version,
                    message,
                })
            }
            Err(e) => Ok(UpdateResult {
                tool,
                success: false,
                old_version: None,
                new_version: None,
                message: format!("Installation failed: {}", e),
            }),
        }
    }

    /// Get installation instructions for a tool
    pub fn get_install_instructions(&self, tool: Tool) -> String {
        match tool {
            Tool::Mnemosyne => "To install mnemosyne:\n\
                 1. Clone the repository: git clone https://github.com/rand/mnemosyne.git\n\
                 2. cd mnemosyne\n\
                 3. ./scripts/build-and-install.sh"
                .to_string(),
            Tool::ClaudeCode => "To install Claude Code:\n\
                 npm install -g @anthropic-ai/claude-code"
                .to_string(),
            Tool::Beads => "To install Beads:\n\
                 Option 1 (npm): npm install -g @beads/bd\n\
                 Option 2 (homebrew): brew tap steveyegge/beads && brew install bd"
                .to_string(),
        }
    }

    /// Update mnemosyne using the build-and-install script
    async fn update_mnemosyne(&self) -> Result<String> {
        // Find the mnemosyne repository
        let repo_path = self.find_mnemosyne_repo()?;

        info!("Found mnemosyne repository at: {}", repo_path.display());

        // Pull latest changes
        debug!("Pulling latest changes...");
        let pull_output = Command::new("git")
            .current_dir(&repo_path)
            .args(["pull", "origin", "main"])
            .output()
            .map_err(|e| {
                MnemosyneError::InvalidOperation(format!("Failed to run git pull: {}", e))
            })?;

        if !pull_output.status.success() {
            return Err(MnemosyneError::InvalidOperation(format!(
                "git pull failed: {}",
                String::from_utf8_lossy(&pull_output.stderr)
            )));
        }

        // Backup current binary
        let binary_path = self.version_checker.detect_tool_path(Tool::Mnemosyne);
        if let Some(bin_path) = &binary_path {
            let backup_path = bin_path.with_extension("backup");
            debug!(
                "Backing up binary: {} -> {}",
                bin_path.display(),
                backup_path.display()
            );
            std::fs::copy(bin_path, &backup_path)?;
        }

        // Run build-and-install script
        let script_path = repo_path.join("scripts/build-and-install.sh");
        if !script_path.exists() {
            return Err(MnemosyneError::InvalidOperation(
                "build-and-install.sh script not found".to_string(),
            ));
        }

        info!("Running build-and-install.sh...");
        let build_output = Command::new(&script_path)
            .current_dir(&repo_path)
            .output()
            .map_err(|e| {
                MnemosyneError::InvalidOperation(format!(
                    "Failed to run build-and-install.sh: {}",
                    e
                ))
            })?;

        if !build_output.status.success() {
            // Restore backup on failure
            if let Some(bin_path) = binary_path {
                let backup_path = bin_path.with_extension("backup");
                if backup_path.exists() {
                    warn!("Build failed, restoring backup...");
                    std::fs::copy(&backup_path, &bin_path).ok();
                }
            }

            return Err(MnemosyneError::InvalidOperation(format!(
                "build-and-install.sh failed: {}",
                String::from_utf8_lossy(&build_output.stderr)
            )));
        }

        // Verify new binary works
        let verify = Command::new("mnemosyne")
            .arg("--version")
            .output()
            .map_err(|e| {
                MnemosyneError::InvalidOperation(format!("Failed to verify new binary: {}", e))
            })?;

        if !verify.status.success() {
            return Err(MnemosyneError::InvalidOperation(
                "New binary failed verification".to_string(),
            ));
        }

        // Clean up backup
        if let Some(bin_path) = binary_path {
            let backup_path = bin_path.with_extension("backup");
            if backup_path.exists() {
                std::fs::remove_file(&backup_path).ok();
            }
        }

        Ok("Successfully updated mnemosyne".to_string())
    }

    /// Install mnemosyne from source
    async fn install_mnemosyne(&self) -> Result<String> {
        // Check if already installed
        if self.version_checker.is_tool_installed(Tool::Mnemosyne) {
            return Err(MnemosyneError::InvalidOperation(
                "mnemosyne is already installed. Use update instead.".to_string(),
            ));
        }

        Err(MnemosyneError::InvalidOperation(
            "Automatic installation of mnemosyne is not yet supported. \
             Please clone the repository and run ./scripts/build-and-install.sh manually."
                .to_string(),
        ))
    }

    /// Update Claude Code via npm
    async fn update_claude_code(&self) -> Result<String> {
        info!("Updating Claude Code via npm...");

        let output = Command::new("npm")
            .args(["update", "-g", "@anthropic-ai/claude-code"])
            .output()
            .map_err(|e| {
                MnemosyneError::InvalidOperation(format!("Failed to run npm update: {}", e))
            })?;

        if !output.status.success() {
            return Err(MnemosyneError::InvalidOperation(format!(
                "npm update failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok("Successfully updated Claude Code".to_string())
    }

    /// Install Claude Code via npm
    async fn install_claude_code(&self) -> Result<String> {
        info!("Installing Claude Code via npm...");

        let output = Command::new("npm")
            .args(["install", "-g", "@anthropic-ai/claude-code"])
            .output()
            .map_err(|e| {
                MnemosyneError::InvalidOperation(format!("Failed to run npm install: {}", e))
            })?;

        if !output.status.success() {
            return Err(MnemosyneError::InvalidOperation(format!(
                "npm install failed: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok("Successfully installed Claude Code".to_string())
    }

    /// Update beads via npm
    /// Note: Beads is distributed via npm (@beads/bd), even when the binary
    /// ends up in paths like /opt/homebrew/bin (due to Homebrew-managed npm)
    async fn update_beads(&self) -> Result<String> {
        info!("Updating beads via npm...");

        let output = Command::new("npm")
            .args(["install", "-g", "@beads/bd@latest"])
            .output()
            .map_err(|e| MnemosyneError::InvalidOperation(format!("Failed to run npm: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(MnemosyneError::InvalidOperation(format!(
                "Failed to update beads via npm: {}",
                stderr
            )));
        }

        Ok("Successfully updated beads via npm".to_string())
    }

    /// Install beads (try npm first, then provide homebrew instructions)
    async fn install_beads(&self) -> Result<String> {
        info!("Installing beads via npm...");

        let output = Command::new("npm")
            .args(["install", "-g", "@beads/bd"])
            .output()
            .map_err(|e| {
                MnemosyneError::InvalidOperation(format!("Failed to run npm install: {}", e))
            })?;

        if !output.status.success() {
            return Err(MnemosyneError::InvalidOperation(format!(
                "npm install failed. Try homebrew: brew tap steveyegge/beads && brew install bd\n\
                 Error: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok("Successfully installed beads".to_string())
    }

    /// Find the mnemosyne repository on the system
    fn find_mnemosyne_repo(&self) -> Result<PathBuf> {
        // Try common locations
        let home = std::env::var("HOME").map_err(|_| {
            MnemosyneError::InvalidOperation("HOME environment variable not set".to_string())
        })?;

        let search_paths = vec![
            PathBuf::from(format!("{}/src/mnemosyne", home)),
            PathBuf::from(format!("{}/projects/mnemosyne", home)),
            PathBuf::from(format!("{}/code/mnemosyne", home)),
            PathBuf::from(format!("{}/mnemosyne", home)),
        ];

        for path in search_paths {
            if path.exists() && path.join(".git").exists() {
                return Ok(path);
            }
        }

        Err(MnemosyneError::InvalidOperation(
            "Could not find mnemosyne repository. Please ensure it's cloned in a standard location (~/src/mnemosyne)".to_string(),
        ))
    }
}

impl Default for UpdateManager {
    fn default() -> Self {
        Self::new().expect("Failed to create update manager")
    }
}

/// Interactive update prompt
pub async fn prompt_for_update(info: &VersionInfo) -> bool {
    use std::io::{self, Write};

    println!("\n📦 Update available for {}:", info.tool.display_name());
    if let Some(installed) = &info.installed {
        println!("   Current version: {}", installed);
    }
    if let Some(latest) = &info.latest {
        println!("   Latest version:  {}", latest);
    }
    if let Some(url) = &info.release_url {
        println!("   Release notes:   {}", url);
    }

    print!("\nWould you like to update? [y/N]: ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}

/// Interactive install prompt
pub async fn prompt_for_install(tool: Tool) -> bool {
    use std::io::{self, Write};

    println!("\n📦 {} is not installed", tool.display_name());
    print!("Would you like to install it? [y/N]: ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
}
