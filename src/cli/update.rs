//! Tool update command

use mnemosyne_core::{error::Result, icons, Tool, UpdateManager, VersionChecker};
use tracing::{debug, info};

/// Handle update command
pub async fn handle(tools: Vec<String>, install: bool, check_only: bool) -> Result<()> {
    debug!("Running update command...");

    if check_only {
        // Just check for updates without installing
        handle_check_only().await
    } else if install {
        // Show installation instructions
        handle_install_instructions(tools).await
    } else {
        // Perform updates
        handle_updates(tools).await
    }
}

/// Check for updates without installing
async fn handle_check_only() -> Result<()> {
    println!("{}  Checking for updates...\n", icons::system::gear());

    let checker = VersionChecker::new()?;
    let results = checker.check_all_tools().await?;

    let mut has_updates = false;

    for info in results {
        if info.is_installed {
            if info.update_available {
                has_updates = true;
                if let (Some(installed), Some(latest)) = (&info.installed, &info.latest) {
                    println!(
                        "{} {} update available: {} → {}",
                        icons::status::warning(),
                        info.tool.display_name(),
                        installed,
                        latest
                    );
                    if let Some(url) = &info.release_url {
                        println!("   Release notes: {}", url);
                    }
                }
            } else if let Some(version) = &info.installed {
                println!(
                    "{} {} is up to date ({})",
                    icons::status::success(),
                    info.tool.display_name(),
                    version
                );
            }
        } else {
            println!(
                "{} {} is not installed",
                icons::status::warning(),
                info.tool.display_name()
            );
        }
    }

    if has_updates {
        println!("\nRun 'mnemosyne update' to install all available updates");
    }

    Ok(())
}

/// Show installation instructions for tools
async fn handle_install_instructions(tools: Vec<String>) -> Result<()> {
    let manager = UpdateManager::new()?;

    if tools.is_empty() {
        // Show instructions for all tools
        println!("{}  Installation Instructions\n", icons::system::lightbulb());
        for tool in Tool::all() {
            println!("{}  {}:", icons::system::package(), tool.display_name());
            for line in manager.get_install_instructions(tool).lines() {
                println!("   {}", line);
            }
            println!();
        }
    } else {
        // Show instructions for specific tools
        for tool_name in &tools {
            let tool = match tool_name.as_str() {
                "mnemosyne" => Tool::Mnemosyne,
                "claude" | "claude-code" => Tool::ClaudeCode,
                "beads" | "bd" => Tool::Beads,
                _ => {
                    eprintln!("Unknown tool: {}", tool_name);
                    continue;
                }
            };

            println!("{}  {}:", icons::system::package(), tool.display_name());
            for line in manager.get_install_instructions(tool).lines() {
                println!("   {}", line);
            }
            println!();
        }
    }

    Ok(())
}

/// Perform tool updates
async fn handle_updates(tools: Vec<String>) -> Result<()> {
    let manager = UpdateManager::new()?;
    let checker = VersionChecker::new()?;

    // Determine which tools to update
    let tools_to_update: Vec<Tool> = if tools.is_empty() {
        // Check all tools for updates
        info!("Checking all tools for updates...");
        let results = checker.check_all_tools().await?;
        results
            .into_iter()
            .filter(|info| info.is_installed && info.update_available)
            .map(|info| info.tool)
            .collect()
    } else {
        // Parse specific tools from command line
        tools
            .iter()
            .filter_map(|name| match name.as_str() {
                "mnemosyne" => Some(Tool::Mnemosyne),
                "claude" | "claude-code" => Some(Tool::ClaudeCode),
                "beads" | "bd" => Some(Tool::Beads),
                _ => {
                    eprintln!("Unknown tool: {}", name);
                    None
                }
            })
            .collect()
    };

    if tools_to_update.is_empty() {
        println!("{} All tools are up to date!", icons::status::success());
        return Ok(());
    }

    // Confirm with user
    println!("\nThe following tools will be updated:");
    for tool in &tools_to_update {
        println!("  • {}", tool.display_name());
    }
    println!();

    if !confirm_update() {
        println!("Update cancelled.");
        return Ok(());
    }

    // Perform updates
    let mut success_count = 0;
    let mut fail_count = 0;

    for tool in tools_to_update {
        println!("\n{}  Updating {}...", icons::system::gear(), tool.display_name());

        match manager.update_tool(tool).await {
            Ok(result) => {
                if result.success {
                    success_count += 1;
                    let version_info = if let (Some(old), Some(new)) =
                        (&result.old_version, &result.new_version)
                    {
                        format!(" ({} → {})", old, new)
                    } else if let Some(new) = &result.new_version {
                        format!(" ({})", new)
                    } else {
                        String::new()
                    };

                    println!(
                        "{} {} updated successfully{}",
                        icons::status::success(),
                        tool.display_name(),
                        version_info
                    );
                } else {
                    fail_count += 1;
                    println!("{} Failed: {}", icons::status::error(), result.message);
                }
            }
            Err(e) => {
                fail_count += 1;
                println!("{} Error: {}", icons::status::error(), e);
            }
        }
    }

    // Summary
    println!();
    if fail_count == 0 {
        println!(
            "{} All updates completed successfully! ({} tools updated)",
            icons::status::success(),
            success_count
        );
    } else {
        println!(
            "{} Updates completed with errors ({} succeeded, {} failed)",
            icons::status::warning(),
            success_count,
            fail_count
        );
    }

    Ok(())
}

/// Prompt user for confirmation
fn confirm_update() -> bool {
    use std::io::{self, Write};

    print!("Proceed with update? [Y/n]: ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();

    let input = input.trim().to_lowercase();
    input.is_empty() || input == "y" || input == "yes"
}
