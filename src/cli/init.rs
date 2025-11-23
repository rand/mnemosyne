//! Database initialization command

use mnemosyne_core::{
    error::Result, orchestration::events::AgentEvent, ConnectionMode, LibsqlStorage,
};
use std::path::PathBuf;
use tracing::debug;

use super::event_helpers;
use super::helpers::get_default_db_path;

/// Handle database initialization command
pub async fn handle(database: Option<String>, global_db_path: Option<String>) -> Result<()> {
    // Use provided database path or fall back to global/default
    let db_path = database
        .or(global_db_path)
        .unwrap_or_else(|| get_default_db_path().to_string_lossy().to_string());

    event_helpers::with_event_lifecycle(
        "init",
        vec![format!("--database={}", db_path)],
        async move {
            debug!("Initializing database...");
            debug!("Database path: {}", db_path);

            // Create parent directory if it doesn't exist
            if let Some(parent) = PathBuf::from(&db_path).parent() {
                std::fs::create_dir_all(parent)?;
                debug!("Created directory: {}", parent.display());
            }

            // Initialize storage (this will create the database and run migrations)
            // Init command explicitly creates database if missing
            let _storage =
                LibsqlStorage::new_with_validation(ConnectionMode::Local(db_path.clone()), true)
                    .await?;

            // Count migrations that would be applied for a new database
            // Based on libsql.rs::run_migrations() - LibSQL has 6 migrations, StandardSQLite has 7
            // Since this is a new database, all applicable migrations are run
            let migrations_applied = if cfg!(feature = "libsql") { 6 } else { 7 };

            // Emit domain event
            event_helpers::emit_domain_event(AgentEvent::DatabaseInitialized {
                database_path: db_path.clone(),
                migrations_applied,
            })
            .await;

            println!(" Database initialized: {}", db_path);
            Ok(())
        },
    )
    .await
}
