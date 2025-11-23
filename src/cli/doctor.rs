//! Health diagnostics command

use mnemosyne_core::{
    error::Result,
    health::{print_health_summary, run_health_checks, CheckStatus},
    orchestration::events::AgentEvent,
    LibsqlStorage,
};
use tracing::debug;

use super::event_helpers;
use super::helpers::get_db_path;

/// Handle doctor command
pub async fn handle(
    verbose: bool,
    fix: bool,
    json: bool,
    global_db_path: Option<String>,
) -> Result<()> {
    let start_time = std::time::Instant::now();

    debug!("Running health checks...");

    // Emit HealthCheckStarted event
    event_helpers::emit_domain_event(AgentEvent::HealthCheckStarted {
        timestamp: chrono::Utc::now(),
    })
    .await;

    // Get database path
    let db_path = get_db_path(global_db_path);

    // Create storage instance
    let storage = LibsqlStorage::from_path(&db_path).await?;

    // Run health checks
    let summary = run_health_checks(&storage, verbose, fix).await?;

    // Extract check counts
    let checks_passed = summary
        .checks
        .iter()
        .filter(|r| matches!(r.status, CheckStatus::Pass))
        .count();
    let checks_failed = summary
        .checks
        .iter()
        .filter(|r| matches!(r.status, CheckStatus::Fail))
        .count();
    let checks_warned = summary
        .checks
        .iter()
        .filter(|r| matches!(r.status, CheckStatus::Warn))
        .count();

    // Output results
    if json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        print_health_summary(&summary, verbose);
    }

    // Emit HealthCheckCompleted event
    let duration_ms = start_time.elapsed().as_millis() as u64;
    event_helpers::emit_domain_event(AgentEvent::HealthCheckCompleted {
        checks_passed,
        checks_failed,
        checks_warned,
        duration_ms,
    })
    .await;

    // Exit with appropriate code
    match summary.status {
        CheckStatus::Pass => std::process::exit(0),
        CheckStatus::Warn => std::process::exit(1),
        CheckStatus::Fail => std::process::exit(2),
    }
}
