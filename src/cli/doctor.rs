//! Health diagnostics command

use mnemosyne_core::{
    error::Result,
    health::{run_health_checks, print_health_summary, CheckStatus},
    LibsqlStorage,
};
use tracing::debug;

use super::helpers::get_db_path;

/// Handle doctor command
pub async fn handle(verbose: bool, fix: bool, json: bool, global_db_path: Option<String>) -> Result<()> {
    debug!("Running health checks...");

    // Get database path
    let db_path = get_db_path(global_db_path);

    // Create storage instance
    let storage = LibsqlStorage::from_path(&db_path).await?;

    // Run health checks
    let summary = run_health_checks(&storage, verbose, fix).await?;

    // Output results
    if json {
        println!("{}", serde_json::to_string_pretty(&summary)?);
    } else {
        print_health_summary(&summary, verbose);
    }

    // Exit with appropriate code
    match summary.status {
        CheckStatus::Pass => std::process::exit(0),
        CheckStatus::Warn => std::process::exit(1),
        CheckStatus::Fail => std::process::exit(2),
    }
}
