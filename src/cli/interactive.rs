//! Interactive work submission mode for multi-agent orchestration
//!
//! Provides a REPL-style interface for submitting work to the orchestration engine
//! and interacting with the memory system.

use mnemosyne_core::{
    api::StateManager,
    error::Result,
    launcher::agents::AgentRole,
    orchestration::{
        events::AgentEvent,
        messages::OrchestratorMessage,
        state::{AgentState, Phase, WorkItem, WorkItemId},
        OrchestrationEngine,
    },
};
use std::io::{self, Write};
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};
use tracing::{debug, error, info};
use super::event_helpers;

/// Run interactive mode with orchestration engine
pub async fn run(
    mut engine: OrchestrationEngine,
    state_manager: Option<Arc<StateManager>>,
) -> Result<()> {
    let start_time = std::time::Instant::now();
    let commands_executed = Arc::new(AtomicU64::new(0));

    // Emit interactive mode started event
    event_helpers::emit_domain_event(AgentEvent::InteractiveModeStarted {
        mode: "orchestration".to_string(),
        timestamp: chrono::Utc::now(),
    }).await;

    println!();
    println!("📋 Mnemosyne Multi-Agent System");
    println!("════════════════════════════════════════════════════════════");
    println!("Dashboard:    http://127.0.0.1:3000");
    println!("Commands:     help, quit, status, recall: <query>");
    println!("Submit work:  <description> or work: <description>");
    println!("════════════════════════════════════════════════════════════");
    println!();

    // Get orchestrator reference before entering loop
    let orchestrator = engine.orchestrator().clone();

    // Give actors time to fully initialize and wire together
    // The start() method uses fire-and-forget .cast() messages for:
    // - Initialize messages to all actors
    // - RegisterAgents wiring
    // - RegisterEventBroadcaster setup
    // - RegisterPythonBridge initialization
    // All of these must complete before work can be submitted
    tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;
    info!("Agents initialized, ready for work submission");

    loop {
        print!("mnemosyne> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let trimmed = input.trim();

        match trimmed {
            "" => continue,
            "help" => {
                show_help();
                commands_executed.fetch_add(1, Ordering::Relaxed);
            }
            "quit" | "exit" => {
                info!("Shutting down...");
                commands_executed.fetch_add(1, Ordering::Relaxed);
                break;
            }
            "status" => {
                show_status(&state_manager).await?;
                commands_executed.fetch_add(1, Ordering::Relaxed);
            }
            cmd if cmd.starts_with("recall:") => {
                let query = cmd.strip_prefix("recall:").unwrap().trim();
                println!("Memory recall: {} (not yet implemented)", query);
                commands_executed.fetch_add(1, Ordering::Relaxed);
            }
            cmd if cmd.starts_with("work:") => {
                let desc = cmd.strip_prefix("work:").unwrap().trim();
                submit_work(&orchestrator, desc).await?;
                commands_executed.fetch_add(1, Ordering::Relaxed);
            }
            desc => {
                // Treat any other input as work description
                submit_work(&orchestrator, desc).await?;
                commands_executed.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    // Shutdown orchestration engine
    info!("Stopping orchestration engine...");
    engine.stop().await?;

    println!();
    println!("✓ Shutdown complete");

    // Emit interactive mode ended event
    let duration_ms = start_time.elapsed().as_millis() as u64;
    let total_commands = commands_executed.load(Ordering::Relaxed) as usize;
    event_helpers::emit_domain_event(AgentEvent::InteractiveModeEnded {
        commands_executed: total_commands,
        duration_ms,
    }).await;

    Ok(())
}

/// Submit work to the orchestration engine
async fn submit_work(
    orchestrator: &ractor::ActorRef<OrchestratorMessage>,
    description: &str,
) -> Result<()> {
    info!("Submitting work: {}", description);

    // Create work item using constructor
    let item = WorkItem::new(
        description.to_string(),
        AgentRole::Executor,       // Default to executor
        Phase::PromptToSpec,       // Default phase
        5,                          // Default priority
    );

    let item_id = item.id.clone();

    // Send to orchestrator
    match orchestrator
        .send_message(OrchestratorMessage::SubmitWork(Box::new(item)))
    {
        Ok(()) => {
            println!("✓ Work submitted: {}", item_id);
            println!("  View progress at: http://127.0.0.1:3000");
        }
        Err(e) => {
            error!("Failed to submit work: {}", e);
            println!("✗ Failed to submit work: {}", e);
        }
    }

    Ok(())
}

/// Show agent status
async fn show_status(state_manager: &Option<Arc<StateManager>>) -> Result<()> {
    if let Some(state_manager) = state_manager {
        let agents = state_manager.list_agents().await;

        if agents.is_empty() {
            println!("No agents registered yet");
            return Ok(());
        }

        println!();
        println!("Agent Status:");
        println!("─────────────────────────────────────────────────────");
        for agent in agents {
            println!(
                "  • {} - {} (updated: {})",
                agent.id,
                format!("{:?}", agent.state),
                agent.updated_at.format("%H:%M:%S")
            );

            if let Some(health) = agent.health {
                let status = if health.is_healthy { "✓" } else { "✗" };
                println!("    Health: {} {} errors",
                    status, health.error_count);
            }
        }
        println!("─────────────────────────────────────────────────────");
        println!();
    } else {
        println!("State manager not available");
    }

    Ok(())
}

/// Show help text
fn show_help() {
    println!();
    println!("Available Commands:");
    println!("─────────────────────────────────────────────────────");
    println!("  help                Show this help message");
    println!("  quit, exit          Exit the system");
    println!("  status              Show agent status");
    println!("  recall: <query>     Search memories");
    println!("  work: <description> Submit work to agents");
    println!("  <description>       Submit work (shorthand)");
    println!("─────────────────────────────────────────────────────");
    println!();
    println!("Examples:");
    println!("  work: Create a new feature for authentication");
    println!("  Refactor the database layer");
    println!("  recall: authentication patterns");
    println!("─────────────────────────────────────────────────────");
    println!();
}
