//! Database initialization command

use mnemosyne_core::{error::Result, ConnectionMode, LibsqlStorage};
use std::path::PathBuf;
use tracing::debug;

use super::helpers::get_default_db_path;

/// Handle database initialization command
pub async fn handle(database: Option<String>, global_db_path: Option<String>) -> Result<()> {
    debug!("Initializing database...");

    // Use provided database path or fall back to global/default
    let db_path = database
        .or_else(|| global_db_path)
        .unwrap_or_else(|| get_default_db_path().to_string_lossy().to_string());

    debug!("Database path: {}", db_path);

    // Create parent directory if it doesn't exist
    if let Some(parent) = PathBuf::from(&db_path).parent() {
        std::fs::create_dir_all(parent)?;
        debug!("Created directory: {}", parent.display());
    }

    // Initialize storage (this will create the database and run migrations)
    // Init command explicitly creates database if missing
    let _storage =
        LibsqlStorage::new_with_validation(ConnectionMode::Local(db_path.clone()), true).await?;

    println!(" Database initialized: {}", db_path);
    Ok(())
}
