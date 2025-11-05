//! Multi-agent orchestration command

use mnemosyne_core::{error::Result, icons, launcher};
use tracing::debug;

use super::helpers::{get_db_path, process_structured_plan};

/// Handle multi-agent orchestration command
pub async fn handle(
    plan: String,
    database: Option<String>,
    dashboard: bool,
    max_concurrent: u8,
) -> Result<()> {
    debug!("Launching multi-agent orchestration system...");

    let db_path = get_db_path(database);

    println!("> Mnemosyne Multi-Agent Orchestration");
    println!("");
    println!("Configuration:");
    println!("  Database: {}", db_path);
    println!("  Max concurrent agents: {}", max_concurrent);
    println!(
        "  Dashboard: {}",
        if dashboard {
            "enabled (future)"
        } else {
            "disabled"
        }
    );
    println!("  Work plan: {}", plan);
    println!();

    // Create launcher configuration
    let mut config = launcher::LauncherConfig::default();
    config.mnemosyne_db_path = Some(db_path.clone());
    config.max_concurrent_agents = max_concurrent;

    // Parse plan as JSON or treat as prompt
    if let Ok(plan_json) = serde_json::from_str::<serde_json::Value>(&plan) {
        debug!("Parsed work plan as JSON");
        debug!("Plan: {:?}", plan_json);

        // Process structured work plan
        println!("{} Structured work plan detected:", icons::data::chart());
        println!();
        process_structured_plan(&plan_json);
        println!();
    } else {
        debug!("Treating plan as plain text prompt");
        println!("{} Prompt-based orchestration:", icons::action::edit());
        println!("   {}", plan);
        println!();
    }

    // Launch orchestrated session
    println!(
        "{} Starting orchestration engine...",
        icons::action::launch()
    );
    println!();

    launcher::launch_orchestrated_session(Some(db_path), Some(plan), None, None).await?;

    println!();
    println!("{} Orchestration session complete", icons::status::ready());
    Ok(())
}
