//! Interactive work submission mode for multi-agent orchestration
//!
//! Provides a REPL-style interface for submitting work to the orchestration engine
//! and interacting with the memory system.

use mnemosyne_core::{
    api::StateManager,
    error::Result,
    orchestration::{
        messages::OrchestratorMessage,
        state::{AgentRole, AgentState, Phase, WorkItem, WorkItemId, WorkItemStatus},
        OrchestrationEngine,
    },
    storage::MemoryStore,
};
use std::io::{self, Write};
use std::sync::Arc;
use tracing::{debug, error, info};

/// Run interactive mode with orchestration engine and memory store
pub async fn run(
    mut engine: OrchestrationEngine,
    memory: MemoryStore,
    state_manager: Option<Arc<StateManager>>,
) -> Result<()> {
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

    loop {
        print!("mnemosyne> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let trimmed = input.trim();

        match trimmed {
            "" => continue,
            "help" => show_help(),
            "quit" | "exit" => {
                info!("Shutting down...");
                break;
            }
            "status" => {
                show_status(&state_manager).await?;
            }
            cmd if cmd.starts_with("recall:") => {
                let query = cmd.strip_prefix("recall:").unwrap().trim();
                recall_memories(&memory, query).await?;
            }
            cmd if cmd.starts_with("work:") => {
                let desc = cmd.strip_prefix("work:").unwrap().trim();
                submit_work(&orchestrator, desc).await?;
            }
            desc => {
                // Treat any other input as work description
                submit_work(&orchestrator, desc).await?;
            }
        }
    }

    // Shutdown orchestration engine
    info!("Stopping orchestration engine...");
    engine.stop().await?;

    println!();
    println!("✓ Shutdown complete");
    Ok(())
}

/// Submit work to the orchestration engine
async fn submit_work(
    orchestrator: &ractor::ActorRef<OrchestratorMessage>,
    description: &str,
) -> Result<()> {
    info!("Submitting work: {}", description);

    // Create work item
    let item = WorkItem {
        id: WorkItemId::new(),
        description: description.to_string(),
        agent: AgentRole::Executor, // Default to executor
        state: AgentState::Idle,
        phase: Phase::Spec,
        priority: 5,
        dependencies: vec![],
        status: WorkItemStatus::Pending,
        result: None,
        error: None,
        started_at: None,
        completed_at: None,
        metadata: Default::default(),
        context: None,
        artifacts: vec![],
    };

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
                println!("    Health: {} errors, {} warnings",
                    health.error_count, health.warning_count);
            }
        }
        println!("─────────────────────────────────────────────────────");
        println!();
    } else {
        println!("State manager not available");
    }

    Ok(())
}

/// Recall memories from the memory store
async fn recall_memories(memory: &MemoryStore, query: &str) -> Result<()> {
    debug!("Recalling memories for query: {}", query);

    // TODO: Implement memory recall
    // This requires MemoryStore to expose recall functionality
    println!("Memory recall: {} (not yet implemented)", query);

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
