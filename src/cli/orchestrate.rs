//! Multi-agent orchestration command

use mnemosyne_core::{
    api::{ApiServer, ApiServerConfig},
    error::Result,
    icons,
    launcher,
    orchestration::events::AgentEvent,
};
use std::sync::Arc;
use tracing::debug;

use super::helpers::{get_db_path, process_structured_plan};
use super::event_helpers;

/// Handle multi-agent orchestration command
pub async fn handle(
    plan: String,
    database: Option<String>,
    dashboard: bool,
    max_concurrent: u8,
) -> Result<()> {
    let start_time = std::time::Instant::now();

    debug!("Launching multi-agent orchestration system...");

    let db_path = get_db_path(database);

    println!("> Mnemosyne Multi-Agent Orchestration");
    println!("");
    println!("Configuration:");
    println!("  Database: {}", db_path);
    println!("  Max concurrent agents: {}", max_concurrent);
    println!("  Dashboard: {}", if dashboard { "enabled" } else { "disabled" });
    println!("  Work plan: {}", plan);
    println!();

    // Note: max_concurrent_agents is currently not used by launch_orchestrated_session
    // TODO: Add max_concurrent support to orchestration engine
    let _ = max_concurrent; // Acknowledge parameter

    // Emit OrchestrationStarted event
    let plan_description = if plan.len() > 100 {
        format!("{}...", plan.chars().take(100).collect::<String>())
    } else {
        plan.clone()
    };

    event_helpers::emit_domain_event(AgentEvent::OrchestrationStarted {
        plan_description,
        max_concurrent,
        timestamp: chrono::Utc::now(),
    }).await;

    // Start embedded API server if dashboard requested
    let (event_broadcaster, state_manager, api_task) = if dashboard {
        debug!("Starting embedded API server for dashboard");

        let api_config = ApiServerConfig {
            addr: ([127, 0, 0, 1], 3000).into(),
            event_capacity: 1000,
        };

        let api_server = ApiServer::new(api_config);
        let broadcaster = api_server.broadcaster().clone();
        let state_manager = Arc::clone(api_server.state_manager());

        // Spawn API server in background task
        let api_task = tokio::spawn(async move {
            match api_server.serve().await {
                Ok(()) => {
                    debug!("API server stopped gracefully");
                }
                Err(e) => {
                    tracing::error!("API server error: {}", e);
                }
            }
        });

        println!("{} Dashboard API: http://127.0.0.1:3000", icons::action::link());
        println!("   Connect dashboard: mnemosyne-dash --api http://127.0.0.1:3000");
        println!();

        (Some(broadcaster), Some(state_manager), Some(api_task))
    } else {
        (None, None, None)
    };

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

    // Launch orchestrated session with dashboard support
    println!(
        "{} Starting orchestration engine...",
        icons::action::launch()
    );
    println!();

    launcher::launch_orchestrated_session(
        Some(db_path),
        Some(plan),
        event_broadcaster,
        state_manager,
    )
    .await?;

    // Gracefully shutdown API server if it was started
    if let Some(task) = api_task {
        debug!("Shutting down API server");
        task.abort();
    }

    println!();
    println!("{} Orchestration session complete", icons::status::ready());

    // Emit OrchestrationCompleted event
    let duration_ms = start_time.elapsed().as_millis() as u64;

    // For now, we don't have work item counts from launch_orchestrated_session
    // TODO: Update launch_orchestrated_session to return work item stats
    event_helpers::emit_domain_event(AgentEvent::OrchestrationCompleted {
        work_items_completed: 0,  // TODO: extract from results
        work_items_failed: 0,     // TODO: extract from results
        duration_ms,
    }).await;

    Ok(())
}
