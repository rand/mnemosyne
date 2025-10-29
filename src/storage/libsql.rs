//! LibSQL storage backend implementation
//!
//! Provides persistent storage using Turso/libSQL with native vector search,
//! FTS5 for keyword search, and efficient indexing for graph traversal.

use crate::embeddings::{EmbeddingService, LocalEmbeddingService};
use crate::error::{MnemosyneError, Result};
use crate::storage::StorageBackend;
use crate::types::{MemoryId, MemoryLink, MemoryNote, Namespace, SearchResult};
use async_trait::async_trait;
use chrono::Utc;
use libsql::{params, Builder, Connection, Database};
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Parse SQL file into individual statements, handling multi-line constructs like triggers
fn parse_sql_statements(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut depth: i32 = 0; // Track BEGIN/END nesting depth

    for line in sql.lines() {
        let trimmed = line.trim();

        // Skip comment-only and empty lines when not building a statement
        if current.is_empty() && (trimmed.is_empty() || trimmed.starts_with("--")) {
            continue;
        }

        // Add line to current statement
        if !current.is_empty() {
            current.push('\n');
        }
        current.push_str(line);

        // Track BEGIN/END depth for triggers
        let upper = trimmed.to_uppercase();
        if upper.starts_with("BEGIN") || upper.contains(" BEGIN") {
            depth += 1;
        }
        if upper.starts_with("END") {
            depth = depth.saturating_sub(1);
        }

        // Statement is complete when we hit ; and depth is 0
        if trimmed.ends_with(';') && depth == 0 {
            statements.push(current.clone());
            current.clear();
        }
    }

    // Add any remaining statement
    if !current.trim().is_empty() {
        statements.push(current);
    }

    statements
}

/// LibSQL storage backend
pub struct LibsqlStorage {
    db: Database,
    embedding_service: Option<Arc<LocalEmbeddingService>>,
    search_config: crate::config::SearchConfig,
}

/// Database connection mode
#[derive(Debug, Clone)]
pub enum ConnectionMode {
    /// Local file-based database
    Local(String),
    /// In-memory database (for testing)
    InMemory,
    /// Remote database (Turso Cloud)
    Remote { url: String, token: String },
    /// Embedded replica with sync
    EmbeddedReplica {
        path: String,
        url: String,
        token: String,
    },
}

impl LibsqlStorage {
    /// Validate database file before opening
    ///
    /// Checks:
    /// 1. Database file exists (for local SQLite paths)
    /// 2. Database is not corrupted (basic SQLite header check)
    /// 3. File is readable
    ///
    /// # Arguments
    /// * `db_path` - Path to the database file
    /// * `must_exist` - If true, error if database doesn't exist. If false, skip existence check.
    ///
    /// # Returns
    /// * `Ok(true)` if database exists and is valid
    /// * `Ok(false)` if database doesn't exist and must_exist=false
    /// * `Err(MnemosyneError)` with actionable message if validation fails
    fn validate_database_file(db_path: &str, must_exist: bool) -> Result<bool> {
        use std::fs;
        use std::path::Path;

        let path = Path::new(db_path);

        // Check if database file exists
        if !path.exists() {
            if must_exist {
                return Err(MnemosyneError::Database(format!(
                    "Database file not found at '{}'. Please run 'mnemosyne init' first or check your DATABASE_URL configuration.",
                    db_path
                )));
            } else {
                // Database doesn't exist, but that's ok - caller will create it
                return Ok(false);
            }
        }

        // Database exists - validate it's a valid SQLite database
        // SQLite files start with "SQLite format 3\0" (16 bytes)
        match fs::read(path) {
            Ok(bytes) => {
                if bytes.len() < 16 {
                    return Err(MnemosyneError::Database(format!(
                        "Database file at '{}' is corrupted or invalid (file too small). Please delete it and run 'mnemosyne init' to reinitialize.",
                        db_path
                    )));
                }

                let header = &bytes[0..16];
                let expected_header = b"SQLite format 3\0";

                if header != expected_header {
                    return Err(MnemosyneError::Database(format!(
                        "Database file at '{}' is corrupted or not a valid SQLite database. Please delete it and run 'mnemosyne init' to reinitialize.",
                        db_path
                    )));
                }

                debug!("Database file validation passed: {}", db_path);
                Ok(true)
            }
            Err(e) => {
                // Check if it's a permission error
                let error_msg = e.to_string();
                if error_msg.contains("permission") || error_msg.contains("Permission") {
                    Err(MnemosyneError::Database(format!(
                        "Cannot read database file at '{}': Permission denied. Please check file permissions.",
                        db_path
                    )))
                } else {
                    Err(MnemosyneError::Database(format!(
                        "Cannot read database file at '{}': {}. The file may be corrupted or inaccessible.",
                        db_path, e
                    )))
                }
            }
        }
    }

    /// Create a new LibSQL storage backend with validation
    ///
    /// # Arguments
    /// * `mode` - Connection mode (local, in-memory, remote, or replica)
    /// * `create_if_missing` - If true, create database if it doesn't exist. If false, error on missing database.
    ///
    /// # Example
    /// ```ignore
    /// // Normal use (database must exist)
    /// let storage = LibsqlStorage::new_with_validation(ConnectionMode::Local("mnemosyne.db".into()), false).await?;
    ///
    /// // Init mode (create if missing)
    /// let storage = LibsqlStorage::new_with_validation(ConnectionMode::Local("mnemosyne.db".into()), true).await?;
    /// ```
    pub async fn new_with_validation(mode: ConnectionMode, create_if_missing: bool) -> Result<Self> {
        info!("Connecting to LibSQL database: {:?} (create_if_missing: {})", mode, create_if_missing);

        // Validate database before connecting (for local paths only)
        match &mode {
            ConnectionMode::Local(ref path) => {
                // Validate database file
                let exists = Self::validate_database_file(path, !create_if_missing)?;

                // If creating and doesn't exist, check parent directory
                if create_if_missing && !exists {
                    if let Some(parent) = std::path::Path::new(path).parent() {
                        if !parent.exists() {
                            return Err(MnemosyneError::Database(format!(
                                "Database directory '{}' does not exist. Please create it first.",
                                parent.display()
                            )));
                        }
                    }
                }
            }
            ConnectionMode::EmbeddedReplica { ref path, .. } => {
                // Validate replica database file
                let exists = Self::validate_database_file(path, !create_if_missing)?;

                // If creating and doesn't exist, check parent directory
                if create_if_missing && !exists {
                    if let Some(parent) = std::path::Path::new(path).parent() {
                        if !parent.exists() {
                            return Err(MnemosyneError::Database(format!(
                                "Database directory '{}' does not exist. Please create it first.",
                                parent.display()
                            )));
                        }
                    }
                }
            }
            ConnectionMode::InMemory | ConnectionMode::Remote { .. } => {
                // Skip validation for in-memory and remote databases
                // Remote validation happens server-side
            }
        }

        let db = match mode {
            ConnectionMode::Local(ref path) => {
                // Create parent directory only if create_if_missing is true
                if create_if_missing {
                    if let Some(parent) = std::path::Path::new(path).parent() {
                        std::fs::create_dir_all(parent).map_err(|e| {
                            MnemosyneError::Database(format!(
                                "Failed to create database directory {}: {}",
                                parent.display(), e
                            ))
                        })?;
                    }
                }

                Builder::new_local(path)
                    .build()
                    .await
                    .map_err(|e| MnemosyneError::Database(format!("Failed to create local database: {}", e)))?
            }
            ConnectionMode::InMemory => {
                Builder::new_local(":memory:")
                    .build()
                    .await
                    .map_err(|e| MnemosyneError::Database(format!("Failed to create in-memory database: {}", e)))?
            }
            ConnectionMode::Remote { ref url, ref token } => {
                Builder::new_remote(url.clone(), token.clone())
                    .build()
                    .await
                    .map_err(|e| MnemosyneError::Database(format!("Failed to create remote database: {}", e)))?
            }
            ConnectionMode::EmbeddedReplica { ref path, ref url, ref token } => {
                // Create parent directory only if create_if_missing is true
                if create_if_missing {
                    if let Some(parent) = std::path::Path::new(path).parent() {
                        std::fs::create_dir_all(parent).map_err(|e| {
                            MnemosyneError::Database(format!(
                                "Failed to create replica directory {}: {}",
                                parent.display(), e
                            ))
                        })?;
                    }
                }

                Builder::new_remote_replica(path, url.clone(), token.clone())
                    .build()
                    .await
                    .map_err(|e| MnemosyneError::Database(format!("Failed to create embedded replica: {}", e)))?
            }
        };

        info!("LibSQL database connection established");

        let storage = Self {
            db,
            embedding_service: None,
            search_config: crate::config::SearchConfig::default(),
        };

        // Verify database health before running migrations
        storage.verify_database_health().await?;

        // Run migrations
        storage.run_migrations().await?;

        // Verify database file exists for local modes
        match &mode {
            ConnectionMode::Local(path) | ConnectionMode::EmbeddedReplica { path, .. } => {
                if !std::path::Path::new(path).exists() {
                    return Err(MnemosyneError::Database(format!(
                        "Database file not created after initialization: {}",
                        path
                    )));
                }
                debug!("Verified database file exists: {}", path);
            }
            _ => {} // In-memory and remote don't have local files
        }

        Ok(storage)
    }

    /// Create a new LibSQL storage backend
    ///
    /// Default behavior: requires database to exist (secure by default).
    /// Returns clear error if database not found or corrupted.
    ///
    /// For explicit database creation, use `new_with_validation(..., true)`.
    ///
    /// # Arguments
    /// * `mode` - Connection mode (local, in-memory, remote, or replica)
    ///
    /// # Example
    /// ```ignore
    /// // Normal use (requires database to exist)
    /// let storage = LibsqlStorage::new(ConnectionMode::Local("mnemosyne.db".into())).await?;
    /// ```
    pub async fn new(mode: ConnectionMode) -> Result<Self> {
        // Default behavior: database must exist (secure by default, clear errors)
        // This prevents accidental database creation and ensures explicit initialization
        // Use new_with_validation(..., true) for database creation (init/serve commands)
        Self::new_with_validation(mode, false).await
    }

    /// Create a new local file-based storage (convenience method)
    ///
    /// # Arguments
    /// * `path` - Path to the database file
    ///
    /// # Example
    /// ```ignore
    /// let storage = LibsqlStorage::new_local("mnemosyne.db").await?;
    /// ```
    pub async fn new_local(path: &str) -> Result<Self> {
        Self::new(ConnectionMode::Local(path.to_string())).await
    }

    /// Create from string path (backward compatibility)
    ///
    /// Parses database path and creates appropriate connection mode:
    /// - ":memory:" → InMemory
    /// - "libsql://..." → Remote (requires token in environment)
    /// - Other → Local file path
    pub async fn from_path(database_url: &str) -> Result<Self> {
        let mode = if database_url == ":memory:" {
            ConnectionMode::InMemory
        } else if database_url.starts_with("libsql://") {
            let token = std::env::var("TURSO_AUTH_TOKEN")
                .map_err(|_| MnemosyneError::Other("TURSO_AUTH_TOKEN not found".into()))?;
            ConnectionMode::Remote {
                url: database_url.to_string(),
                token,
            }
        } else {
            ConnectionMode::Local(database_url.to_string())
        };

        Self::new(mode).await
    }

    /// Verify database health before operations
    async fn verify_database_health(&self) -> Result<()> {
        let conn = self.get_conn()?;

        // Test 1: Basic query to detect corruption
        let test_query = "SELECT 1";
        conn.query(test_query, params![])
            .await
            .map_err(|e| {
                MnemosyneError::Database(format!(
                    "Database corruption detected or invalid database file: {}",
                    e
                ))
            })?;

        // Test 2: Check if database is writable
        // Try to create a test table and drop it
        let write_test = r#"
            CREATE TABLE IF NOT EXISTS _health_check (id INTEGER PRIMARY KEY);
            DROP TABLE IF EXISTS _health_check;
        "#;

        if let Err(e) = conn.execute_batch(write_test).await {
            // Check if it's a read-only error
            let error_msg = e.to_string().to_lowercase();
            if error_msg.contains("read") && error_msg.contains("only")
                || error_msg.contains("readonly")
                || error_msg.contains("permission")
            {
                return Err(MnemosyneError::Database(format!(
                    "Database is read-only or lacks write permissions: {}",
                    e
                )));
            }
            // Other write errors
            return Err(MnemosyneError::Database(format!(
                "Database write test failed: {}",
                e
            )));
        }

        debug!("Database health check passed");
        Ok(())
    }

    /// Run database migrations
    pub async fn run_migrations(&self) -> Result<()> {
        info!("Running database migrations...");

        // Get a connection for migrations
        let conn = self.get_conn()?;

        // Create migrations tracking table if it doesn't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS _migrations_applied (
                migration_name TEXT PRIMARY KEY,
                applied_at INTEGER NOT NULL
            )",
            params![]
        ).await.map_err(|e| {
            MnemosyneError::Migration(format!("Failed to create migrations table: {}", e))
        })?;

        // Manually run migrations for better control
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let migrations_path = std::path::PathBuf::from(manifest_dir)
            .join("migrations")
            .join("libsql");

        debug!("Migrations path: {:?}", migrations_path);

        // Read and execute migration files in order
        // Only run core migrations for now
        // Advanced migrations (006-009) require SQLite extensions (vec0) that need to be loaded first
        let migration_files = vec![
            "001_initial_schema.sql",
            "002_add_indexes.sql",
        ];

        for migration_file in migration_files {
            // Check if migration already applied
            let mut rows = conn.query(
                "SELECT COUNT(*) FROM _migrations_applied WHERE migration_name = ?",
                params![migration_file],
            ).await?;

            let already_applied = if let Some(row) = rows.next().await? {
                row.get::<i64>(0).unwrap_or(0)
            } else {
                0
            };

            if already_applied > 0 {
                debug!("Skipping already applied migration: {}", migration_file);
                continue;
            }
            let file_path = migrations_path.join(migration_file);
            debug!("Executing migration: {:?}", file_path);

            let sql = std::fs::read_to_string(&file_path)
                .map_err(|e| {
                    MnemosyneError::Migration(format!(
                        "Failed to read migration file {}: {}",
                        migration_file, e
                    ))
                })?;

            // Execute the migration SQL
            // Parse SQL statements properly, handling multi-line statements like triggers
            let statements = parse_sql_statements(&sql);
            debug!("Parsed {} statements from {}", statements.len(), migration_file);
            for (i, statement) in statements.iter().enumerate() {
                let statement = statement.trim();
                if !statement.is_empty() {
                    debug!("Executing statement {}/{}", i+1, statements.len());
                    conn.execute(statement, params![]).await.map_err(|e| {
                        MnemosyneError::Migration(format!(
                            "Failed to execute statement #{} in {}: {}\nStatement: {}",
                            i+1,
                            migration_file,
                            e,
                            &statement[..statement.len().min(300)]
                        ))
                    })?;
                }
            }

            // Record migration as applied
            let now = Utc::now().timestamp();
            conn.execute(
                "INSERT INTO _migrations_applied (migration_name, applied_at) VALUES (?, ?)",
                params![migration_file, now]
            ).await.map_err(|e| {
                MnemosyneError::Migration(format!("Failed to record migration: {}", e))
            })?;

            info!("Executed migration: {}", migration_file);
        }

        info!("Database migrations completed");
        Ok(())
    }

    /// Get a connection from the database
    fn get_conn(&self) -> Result<Connection> {
        self.db
            .connect()
            .map_err(|e| MnemosyneError::Database(format!("Failed to get connection: {}", e)))
    }

    /// Check if database is healthy and operational
    ///
    /// Performs basic health checks:
    /// - Can establish connection
    /// - Can execute simple query
    /// - Database is not corrupted
    ///
    /// Returns Ok(()) if healthy, Err with diagnostic info if not
    pub async fn check_database_health(&self) -> Result<()> {
        debug!("Checking database health...");

        // Try to get a connection
        let conn = self.get_conn().map_err(|e| {
            MnemosyneError::Database(format!("Health check failed: cannot establish connection: {}", e))
        })?;

        // Try a simple query to verify database is operational
        match conn.query("SELECT 1", ()).await {
            Ok(_) => {
                debug!("Database health check passed");
                Ok(())
            }
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("readonly") || error_msg.contains("permission") {
                    Err(MnemosyneError::Database(
                        "Database is read-only or permission denied. Check file permissions.".to_string()
                    ))
                } else if error_msg.contains("corrupt") || error_msg.contains("malformed") {
                    Err(MnemosyneError::Database(
                        "Database appears to be corrupted. Consider restoring from backup.".to_string()
                    ))
                } else {
                    Err(MnemosyneError::Database(format!("Health check failed: {}", error_msg)))
                }
            }
        }
    }

    /// Attempt to recover from database errors
    ///
    /// Tries to recover from common error conditions:
    /// - Stale lock files
    /// - Permission issues
    /// - Connection pool exhaustion
    ///
    /// Returns Ok(()) if recovery successful or not needed
    pub async fn recover_from_error(&self) -> Result<()> {
        debug!("Attempting database recovery...");

        // First check if database is healthy
        match self.check_database_health().await {
            Ok(()) => {
                debug!("Database is healthy, no recovery needed");
                return Ok(());
            }
            Err(e) => {
                debug!("Database health check failed: {}, attempting recovery", e);
                // Continue with recovery attempt
            }
        }

        // Get a fresh connection for recovery operations
        let conn = self.get_conn().map_err(|e| {
            MnemosyneError::Database(format!("Cannot establish connection for recovery: {}", e))
        })?;

        // Step 1: Try to checkpoint WAL to clear pending writes
        debug!("Attempting WAL checkpoint to recover from stale state...");
        match conn.execute("PRAGMA wal_checkpoint(TRUNCATE)", ()).await {
            Ok(_) => {
                info!("WAL checkpoint successful - database recovered");
                return Ok(());
            }
            Err(e) => {
                debug!("WAL checkpoint failed: {}, trying alternative recovery", e);
            }
        }

        // Step 2: Try to reinitialize WAL mode
        debug!("Attempting to reinitialize WAL mode...");
        match conn.execute("PRAGMA journal_mode=WAL", ()).await {
            Ok(_) => {
                info!("WAL mode reinitialized - database recovered");

                // Verify recovery with a simple query
                match conn.execute("SELECT 1", ()).await {
                    Ok(_) => {
                        debug!("Database is now operational after recovery");
                        Ok(())
                    }
                    Err(e) => {
                        Err(MnemosyneError::Database(format!(
                            "Recovery partially successful but database still not operational: {}. \
                            Manual intervention may be required: delete .db-wal and .db-shm files.",
                            e
                        )))
                    }
                }
            }
            Err(e) => {
                Err(MnemosyneError::Database(format!(
                    "Recovery failed: {}. Manual intervention required: \
                    1. Check file permissions on database and WAL files (.db-wal, .db-shm) \
                    2. If permissions are correct, delete stale WAL files and retry \
                    3. As a last resort, restore from backup",
                    e
                )))
            }
        }
    }

    /// Convert a libsql row to a MemoryNote
    async fn row_to_memory(&self, row: &libsql::Row) -> Result<MemoryNote> {
        // Extract all fields from row
        let id_str: String = row.get(0)?;
        let id = MemoryId::from_string(&id_str)?;

        let namespace_json: String = row.get(1)?;
        let namespace: Namespace = serde_json::from_str(&namespace_json)?;

        let created_at: String = row.get(2)?;
        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at)
            .map_err(|e| MnemosyneError::Other(format!("Invalid timestamp: {}", e)))?
            .with_timezone(&chrono::Utc);

        let updated_at: String = row.get(3)?;
        let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at)
            .map_err(|e| MnemosyneError::Other(format!("Invalid timestamp: {}", e)))?
            .with_timezone(&chrono::Utc);

        let content: String = row.get(4)?;
        let summary: String = row.get(5)?;

        let keywords_json: String = row.get(6)?;
        let keywords: Vec<String> = serde_json::from_str(&keywords_json)?;

        let tags_json: String = row.get(7)?;
        let tags: Vec<String> = serde_json::from_str(&tags_json)?;

        let context: String = row.get(8)?;

        let memory_type_str: String = row.get(9)?;
        let memory_type = match memory_type_str.as_str() {
            "architecture_decision" => crate::types::MemoryType::ArchitectureDecision,
            "code_pattern" => crate::types::MemoryType::CodePattern,
            "bug_fix" => crate::types::MemoryType::BugFix,
            "configuration" => crate::types::MemoryType::Configuration,
            "constraint" => crate::types::MemoryType::Constraint,
            "entity" => crate::types::MemoryType::Entity,
            "insight" => crate::types::MemoryType::Insight,
            "reference" => crate::types::MemoryType::Reference,
            "preference" => crate::types::MemoryType::Preference,
            "agent_event" => crate::types::MemoryType::AgentEvent,
            _ => return Err(MnemosyneError::Other(format!("Unknown memory type: {}", memory_type_str))),
        };

        let importance: i64 = row.get(10)?;
        let confidence: f64 = row.get(11)?;

        let related_files_json: String = row.get(12)?;
        let related_files: Vec<String> = serde_json::from_str(&related_files_json)?;

        let related_entities_json: String = row.get(13)?;
        let related_entities: Vec<String> = serde_json::from_str(&related_entities_json)?;

        let access_count: i64 = row.get(14)?;

        let last_accessed_str: String = row.get(15)?;
        let last_accessed_at = chrono::DateTime::parse_from_rfc3339(&last_accessed_str)
            .map_err(|e| MnemosyneError::Other(format!("Invalid timestamp: {}", e)))?
            .with_timezone(&chrono::Utc);

        let expires_at: Option<String> = row.get(16)?;
        let expires_at = expires_at
            .map(|s| chrono::DateTime::parse_from_rfc3339(&s))
            .transpose()
            .map_err(|e| MnemosyneError::Other(format!("Invalid timestamp: {}", e)))?
            .map(|dt| dt.with_timezone(&chrono::Utc));

        let is_archived: i64 = row.get(17)?;
        let is_archived = is_archived != 0;

        let superseded_by: Option<String> = row.get(18)?;
        let superseded_by = superseded_by.and_then(|s| MemoryId::from_string(&s).ok());

        let embedding_model: String = row.get(19)?;

        // Try to get embedding from column 20 if it exists (F32_BLOB)
        // Note: We'll handle embedding parsing in Phase 3 when implementing vector search
        let embedding: Option<Vec<f32>> = None; // Placeholder for now

        Ok(MemoryNote {
            id,
            namespace,
            created_at,
            updated_at,
            content,
            summary,
            keywords,
            tags,
            context,
            memory_type,
            importance: importance as u8,
            confidence: confidence as f32,
            links: Vec::new(), // Will be populated separately via graph traversal
            related_files,
            related_entities,
            access_count: access_count as u32,
            last_accessed_at,
            expires_at,
            is_archived,
            superseded_by,
            embedding_model,
            embedding,
        })
    }

    /// Log an audit event
    async fn log_audit(
        &self,
        operation: &str,
        memory_id: Option<MemoryId>,
        metadata: serde_json::Value,
    ) -> Result<()> {
        let conn = self.get_conn()?;

        let memory_id_str = memory_id.map(|id| id.to_string());
        let metadata_json = metadata.to_string();

        conn.execute(
            "INSERT INTO audit_log (operation, memory_id, metadata) VALUES (?, ?, ?)",
            params![operation, memory_id_str, metadata_json],
        )
        .await?;

        Ok(())
    }

    /// Escape FTS5 query string to handle special characters
    ///
    /// FTS5 treats certain characters as operators:
    /// - Hyphen (-) is treated as MINUS operator
    /// - NOT, OR, AND are boolean operators
    /// - Parentheses affect query parsing
    ///
    /// To treat these literally, we wrap terms in double quotes.
    /// Internal quotes are escaped by doubling them.
    fn escape_fts5_query(term: &str) -> String {
        // Check if term contains FTS5 special characters
        let needs_escaping = term.contains('-')
            || term.contains('(')
            || term.contains(')')
            || term.contains('"')
            || term.to_lowercase().contains(" not ")
            || term.to_lowercase().contains(" and ")
            || term.to_lowercase().contains(" or ");

        if needs_escaping {
            // Escape internal quotes by doubling them
            let escaped = term.replace('"', "\"\"");
            format!("\"{}\"", escaped)
        } else {
            term.to_string()
        }
    }

    /// Set the embedding service for this storage backend
    pub fn set_embedding_service(&mut self, service: Arc<LocalEmbeddingService>) {
        self.embedding_service = Some(service);
    }

    /// Generate and store embedding for a memory
    ///
    /// This method generates an embedding for the given memory content and stores it
    /// in the memory_vectors table. If embeddings are disabled (no service), this is a no-op.
    ///
    /// # Arguments
    /// * `memory_id` - The ID of the memory to embed
    /// * `content` - The text content to embed
    ///
    /// # Returns
    /// * `Ok(())` - Embedding generated and stored successfully (or disabled)
    /// * `Err(MnemosyneError)` - If embedding generation or storage fails
    pub async fn generate_and_store_embedding(&self, memory_id: &MemoryId, content: &str) -> Result<()> {
        // Skip if no embedding service configured
        let service = match &self.embedding_service {
            Some(s) => s,
            None => {
                debug!("Embedding service not configured, skipping embedding for {}", memory_id);
                return Ok(());
            }
        };

        // Generate embedding
        debug!("Generating embedding for memory: {}", memory_id);
        let embedding = service.embed(content).await?;

        // Store in memory_vectors table
        self.store_embedding(memory_id, &embedding).await?;

        info!("Successfully generated and stored embedding for memory: {}", memory_id);
        Ok(())
    }

    /// Store an embedding vector in the memory_vectors table
    ///
    /// This is a low-level method that directly stores a pre-computed embedding.
    /// Use generate_and_store_embedding() for the high-level workflow.
    ///
    /// # Arguments
    /// * `memory_id` - The ID of the memory
    /// * `embedding` - The embedding vector (must match configured dimensions)
    pub async fn store_embedding(&self, memory_id: &MemoryId, embedding: &[f32]) -> Result<()> {
        let conn = self.get_conn()?;

        // Convert embedding to JSON array for sqlite-vec
        let embedding_json = serde_json::to_string(embedding)?;

        // Insert or replace embedding in memory_vectors table
        conn.execute(
            "INSERT OR REPLACE INTO memory_vectors (memory_id, embedding) VALUES (?, ?)",
            params![memory_id.to_string(), embedding_json],
        )
        .await
        .map_err(|e| MnemosyneError::Database(format!("Failed to store embedding: {}", e)))?;

        Ok(())
    }

    /// Retrieve embedding for a memory
    ///
    /// # Arguments
    /// * `memory_id` - The ID of the memory
    ///
    /// # Returns
    /// * `Ok(Some(Vec<f32>))` - The embedding vector if it exists
    /// * `Ok(None)` - If no embedding exists for this memory
    /// * `Err(MnemosyneError)` - If retrieval fails
    pub async fn get_embedding(&self, memory_id: &MemoryId) -> Result<Option<Vec<f32>>> {
        let conn = self.get_conn()?;

        let row = conn
            .query("SELECT embedding FROM memory_vectors WHERE memory_id = ?", params![memory_id.to_string()])
            .await
            .map_err(|e| MnemosyneError::Database(format!("Failed to retrieve embedding: {}", e)))?
            .next()
            .await
            .map_err(|e| MnemosyneError::Database(format!("Failed to get embedding row: {}", e)))?;

        match row {
            Some(row) => {
                let embedding_json: String = row.get(0)?;
                let embedding: Vec<f32> = serde_json::from_str(&embedding_json)?;
                Ok(Some(embedding))
            }
            None => Ok(None),
        }
    }

    /// Delete embedding for a memory
    ///
    /// # Arguments
    /// * `memory_id` - The ID of the memory
    pub async fn delete_embedding(&self, memory_id: &MemoryId) -> Result<()> {
        let conn = self.get_conn()?;

        conn.execute(
            "DELETE FROM memory_vectors WHERE memory_id = ?",
            params![memory_id.to_string()],
        )
        .await
        .map_err(|e| MnemosyneError::Database(format!("Failed to delete embedding: {}", e)))?;

        Ok(())
    }

    /// Set the search configuration
    pub fn set_search_config(&mut self, config: crate::config::SearchConfig) {
        self.search_config = config;
    }

    /// Perform vector similarity search
    ///
    /// Searches for memories with embeddings similar to the query embedding.
    /// Uses sqlite-vec's vec_distance_cosine for similarity.
    ///
    /// # Arguments
    /// * `query_embedding` - The query embedding vector
    /// * `limit` - Maximum number of results
    /// * `namespace` - Optional namespace filter
    ///
    /// # Returns
    /// * Vector of (MemoryId, similarity_score) tuples, sorted by similarity (desc)
    pub async fn vector_search(
        &self,
        query_embedding: &[f32],
        limit: usize,
        namespace: Option<Namespace>,
    ) -> Result<Vec<(MemoryId, f32)>> {
        // Skip if vector search is disabled
        if !self.search_config.enable_vector_search {
            debug!("Vector search disabled in config");
            return Ok(Vec::new());
        }

        // Skip if no embedding service (can't search without embeddings)
        if self.embedding_service.is_none() {
            debug!("No embedding service available for vector search");
            return Ok(Vec::new());
        }

        let conn = self.get_conn()?;

        // Convert query embedding to JSON for sqlite-vec
        let query_json = serde_json::to_string(query_embedding)?;

        // Build query with optional namespace filter
        let sql = if namespace.is_some() {
            r#"
            SELECT v.memory_id, vec_distance_cosine(v.embedding, ?) as distance
            FROM memory_vectors v
            JOIN memories m ON v.memory_id = m.id
            WHERE m.namespace = ?
            ORDER BY distance ASC
            LIMIT ?
            "#
            .to_string()
        } else {
            r#"
            SELECT memory_id, vec_distance_cosine(embedding, ?) as distance
            FROM memory_vectors
            ORDER BY distance ASC
            LIMIT ?
            "#
            .to_string()
        };

        let mut rows = if let Some(ns) = &namespace {
            let ns_json = serde_json::to_string(ns)?;
            conn.query(&sql, params![query_json, ns_json, limit as i64])
                .await?
        } else {
            conn.query(&sql, params![query_json, limit as i64]).await?
        };

        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            let memory_id_str: String = row.get(0)?;
            let distance: f64 = row.get(1)?;

            // Convert distance to similarity (0 = identical, 2 = opposite)
            // Similarity = 1 - (distance / 2), range [0, 1]
            let similarity = 1.0 - (distance as f32 / 2.0);

            let memory_id = MemoryId(uuid::Uuid::parse_str(&memory_id_str)?);
            results.push((memory_id, similarity));
        }

        debug!("Vector search found {} results", results.len());
        Ok(results)
    }

    // ========================================================================
    // Evolution System Methods
    // ========================================================================

    /// List all active (non-archived) memories for evolution jobs
    pub async fn list_all_active(&self, limit: Option<usize>) -> Result<Vec<MemoryNote>> {
        debug!("Listing all active memories for evolution");

        let conn = self.get_conn()?;
        let sql = if let Some(lim) = limit {
            format!(
                "SELECT * FROM memories WHERE is_archived = 0 AND archived_at IS NULL ORDER BY created_at DESC LIMIT {}",
                lim
            )
        } else {
            "SELECT * FROM memories WHERE is_archived = 0 AND archived_at IS NULL ORDER BY created_at DESC".to_string()
        };

        let mut rows = conn.query(&sql, params![]).await?;
        let mut memories = Vec::new();

        while let Some(row) = rows.next().await? {
            memories.push(self.row_to_memory(&row).await?);
        }

        debug!("Listed {} active memories", memories.len());
        Ok(memories)
    }

    /// Update the importance score of a memory
    pub async fn update_importance(&self, memory_id: &MemoryId, new_importance: f32) -> Result<()> {
        debug!("Updating importance for memory {} to {}", memory_id, new_importance);

        let conn = self.get_conn()?;
        conn.execute(
            r#"
            UPDATE memories
            SET importance = ?,
                updated_at = ?
            WHERE id = ?
            "#,
            params![
                new_importance as f64,
                Utc::now().to_rfc3339(),
                memory_id.to_string()
            ],
        )
        .await?;

        Ok(())
    }

    /// Find memories that are candidates for archival
    pub async fn find_archival_candidates(&self, limit: usize) -> Result<Vec<MemoryNote>> {
        debug!("Finding archival candidates (limit: {})", limit);

        let conn = self.get_conn()?;

        // Use the view from migration 007
        let sql = r#"
            SELECT m.*
            FROM memories m
            WHERE m.archived_at IS NULL AND m.is_archived = 0
              AND (
                (m.access_count = 0 AND
                 julianday('now') - julianday(m.created_at) > 180) OR
                (m.importance < 3.0 AND
                 julianday('now') - julianday(COALESCE(m.last_accessed_at, m.created_at)) > 90) OR
                (m.importance < 2.0 AND
                 julianday('now') - julianday(COALESCE(m.last_accessed_at, m.created_at)) > 30)
              )
            ORDER BY m.importance ASC, m.access_count ASC
            LIMIT ?
        "#;

        let mut rows = conn.query(sql, params![limit as i64]).await?;
        let mut candidates = Vec::new();

        while let Some(row) = rows.next().await? {
            candidates.push(self.row_to_memory(&row).await?);
        }

        debug!("Found {} archival candidates", candidates.len());
        Ok(candidates)
    }

    /// Archive a memory by setting archived_at timestamp
    pub async fn archive_memory_with_timestamp(&self, memory_id: &MemoryId) -> Result<()> {
        debug!("Archiving memory with timestamp: {}", memory_id);

        let conn = self.get_conn()?;
        let now = Utc::now();

        conn.execute(
            r#"
            UPDATE memories
            SET archived_at = ?,
                is_archived = 1,
                updated_at = ?
            WHERE id = ?
            "#,
            params![
                now.timestamp(),
                now.to_rfc3339(),
                memory_id.to_string()
            ],
        )
        .await?;

        Ok(())
    }

    /// Unarchive a memory
    pub async fn unarchive_memory(&self, memory_id: &MemoryId) -> Result<()> {
        debug!("Unarchiving memory: {}", memory_id);

        let conn = self.get_conn()?;

        conn.execute(
            r#"
            UPDATE memories
            SET archived_at = NULL,
                is_archived = 0,
                updated_at = ?
            WHERE id = ?
            "#,
            params![Utc::now().to_rfc3339(), memory_id.to_string()],
        )
        .await?;

        Ok(())
    }

    /// Record link traversal for decay tracking
    pub async fn record_link_traversal(&self, source_id: &MemoryId, target_id: &MemoryId) -> Result<()> {
        debug!("Recording link traversal: {} -> {}", source_id, target_id);

        let conn = self.get_conn()?;
        let now = Utc::now();

        conn.execute(
            r#"
            UPDATE memory_links
            SET last_traversed_at = ?
            WHERE source_id = ? AND target_id = ?
            "#,
            params![now.timestamp(), source_id.to_string(), target_id.to_string()],
        )
        .await?;

        Ok(())
    }

    /// Update link strength (for reinforcement or decay)
    pub async fn update_link_strength(
        &self,
        source_id: &MemoryId,
        target_id: &MemoryId,
        new_strength: f32,
    ) -> Result<()> {
        debug!(
            "Updating link strength: {} -> {} = {}",
            source_id, target_id, new_strength
        );

        let conn = self.get_conn()?;

        conn.execute(
            r#"
            UPDATE memory_links
            SET strength = ?
            WHERE source_id = ? AND target_id = ?
            "#,
            params![new_strength, source_id.to_string(), target_id.to_string()],
        )
        .await?;

        Ok(())
    }

    /// Find links that need decay (untraversed for long time)
    /// Returns (source_id, link) tuples
    pub async fn find_link_decay_candidates(&self, days_threshold: i64, limit: usize) -> Result<Vec<(MemoryId, MemoryLink)>> {
        debug!(
            "Finding link decay candidates (threshold: {} days, limit: {})",
            days_threshold, limit
        );

        let conn = self.get_conn()?;

        let sql = r#"
            SELECT source_id, target_id, link_type, strength, created_at, reason
            FROM memory_links
            WHERE user_created = 0
              AND strength > 0.1
              AND (
                (last_traversed_at IS NULL AND
                 julianday('now') - julianday(datetime(created_at, 'unixepoch')) > ?) OR
                (last_traversed_at IS NOT NULL AND
                 julianday('now') - julianday(datetime(last_traversed_at, 'unixepoch')) > ?)
              )
            ORDER BY strength ASC
            LIMIT ?
        "#;

        let mut rows = conn
            .query(
                sql,
                params![days_threshold as i64, days_threshold as i64, limit as i64],
            )
            .await?;

        let mut links = Vec::new();
        while let Some(row) = rows.next().await? {
            let source_id_str: String = row.get(0)?;
            let source_id = MemoryId::from_string(&source_id_str)?;

            let target_id_str: String = row.get(1)?;
            let target_id = MemoryId::from_string(&target_id_str)?;

            let link_type_str: String = row.get(2)?;
            let link_type = match link_type_str.as_str() {
                "extends" => crate::types::LinkType::Extends,
                "contradicts" => crate::types::LinkType::Contradicts,
                "implements" => crate::types::LinkType::Implements,
                "references" => crate::types::LinkType::References,
                "supersedes" => crate::types::LinkType::Supersedes,
                _ => continue,
            };

            let strength: f64 = row.get(3)?;
            let created_at_str: String = row.get(4)?;
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);

            let reason: String = row.get::<String>(5).unwrap_or_else(|_| String::from("link decay candidate"));

            links.push((
                source_id,
                MemoryLink {
                    target_id,
                    link_type,
                    strength: strength as f32,
                    reason,
                    created_at,
                },
            ));
        }

        debug!("Found {} link decay candidates", links.len());
        Ok(links)
    }

    /// Remove a weak link
    pub async fn remove_link(&self, source_id: &MemoryId, target_id: &MemoryId) -> Result<()> {
        debug!("Removing link: {} -> {}", source_id, target_id);

        let conn = self.get_conn()?;

        conn.execute(
            r#"
            DELETE FROM memory_links
            WHERE source_id = ? AND target_id = ?
            "#,
            params![source_id.to_string(), target_id.to_string()],
        )
        .await?;

        Ok(())
    }

    /// Get memory access statistics
    pub async fn get_access_stats(&self, memory_id: &MemoryId) -> Result<(u32, Option<chrono::DateTime<Utc>>)> {
        let conn = self.get_conn()?;

        let mut rows = conn
            .query(
                "SELECT access_count, last_accessed_at FROM memories WHERE id = ?",
                params![memory_id.to_string()],
            )
            .await?;

        if let Some(row) = rows.next().await? {
            let access_count: i64 = row.get(0)?;
            let last_accessed_at = if let Ok(last_accessed_str) = row.get::<String>(1) {
                chrono::DateTime::parse_from_rfc3339(&last_accessed_str)
                    .ok()
                    .map(|dt| dt.with_timezone(&Utc))
            } else {
                None
            };

            Ok((access_count as u32, last_accessed_at))
        } else {
            Err(MnemosyneError::MemoryNotFound(memory_id.to_string()))
        }
    }
}

#[async_trait]
impl StorageBackend for LibsqlStorage {
    async fn store_memory(&self, memory: &MemoryNote) -> Result<()> {
        debug!("Storing memory: {}", memory.id);

        let conn = self.get_conn().map_err(|e| {
            let error_msg = e.to_string();
            if error_msg.contains("readonly") || error_msg.contains("permission") {
                MnemosyneError::Database(
                    "Cannot write to database: read-only or permission denied. Check file permissions and ensure WAL files (.db-wal, .db-shm) are writable.".to_string()
                )
            } else {
                e
            }
        })?;
        let tx = conn.transaction().await?;

        // Insert memory metadata including embedding column
        // Note: For F32_BLOB vectors, we use vector32(?) to convert JSON array to binary
        let sql = if memory.embedding.is_some() {
            r#"
            INSERT INTO memories (
                id, namespace, created_at, updated_at,
                content, summary, keywords, tags, context,
                memory_type, importance, confidence,
                related_files, related_entities,
                access_count, last_accessed_at, expires_at,
                is_archived, superseded_by, embedding_model, embedding
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, vector32(?))
            "#
        } else {
            r#"
            INSERT INTO memories (
                id, namespace, created_at, updated_at,
                content, summary, keywords, tags, context,
                memory_type, importance, confidence,
                related_files, related_entities,
                access_count, last_accessed_at, expires_at,
                is_archived, superseded_by, embedding_model, embedding
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, NULL)
            "#
        };

        tx.execute(
            sql,
            params![
                memory.id.to_string(),
                serde_json::to_string(&memory.namespace)?,
                memory.created_at.to_rfc3339(),
                memory.updated_at.to_rfc3339(),
                memory.content.clone(),
                memory.summary.clone(),
                serde_json::to_string(&memory.keywords)?,
                serde_json::to_string(&memory.tags)?,
                memory.context.clone(),
                serde_json::to_value(&memory.memory_type)?.as_str().unwrap(),
                memory.importance as i64,
                memory.confidence as f64,
                serde_json::to_string(&memory.related_files)?,
                serde_json::to_string(&memory.related_entities)?,
                memory.access_count as i64,
                memory.last_accessed_at.to_rfc3339(),
                memory.expires_at.map(|dt| dt.to_rfc3339()),
                if memory.is_archived { 1i64 } else { 0i64 },
                memory.superseded_by.map(|id| id.to_string()),
                memory.embedding_model.clone(),
                memory.embedding.as_ref().map(|emb| {
                    serde_json::to_string(emb).expect("Failed to serialize embedding")
                })
            ],
        )
        .await?;

        // Store links
        for link in &memory.links {
            tx.execute(
                r#"
                INSERT INTO memory_links (source_id, target_id, link_type, strength, reason, created_at)
                VALUES (?, ?, ?, ?, ?, ?)
                "#,
                params![
                    memory.id.to_string(),
                    link.target_id.to_string(),
                    serde_json::to_value(&link.link_type)?.as_str().unwrap(),
                    link.strength as f64,
                    link.reason.clone(),
                    link.created_at.to_rfc3339(),
                ],
            )
            .await?;
        }

        tx.commit().await.map_err(|e| {
            let error_msg = e.to_string();
            if error_msg.contains("readonly") || error_msg.contains("permission") {
                MnemosyneError::Database(
                    "Transaction failed: database is read-only. Ensure file and WAL files have write permissions.".to_string()
                )
            } else if error_msg.contains("locked") || error_msg.contains("busy") {
                MnemosyneError::Database(
                    "Transaction failed: database is locked. Another process may be writing.".to_string()
                )
            } else {
                MnemosyneError::Database(format!("Transaction commit failed: {}", error_msg))
            }
        })?;

        self.log_audit(
            "create",
            Some(memory.id),
            serde_json::json!({
                "namespace": memory.namespace,
                "memory_type": memory.memory_type,
                "importance": memory.importance,
            }),
        )
        .await?;

        // Auto-generate embedding if embedding service is configured
        // This is a fire-and-forget operation - failures are logged but don't fail the store
        if self.embedding_service.is_some() {
            if let Err(e) = self.generate_and_store_embedding(&memory.id, &memory.content).await {
                // Log error but don't fail the store operation
                // Embeddings can be regenerated later using CLI
                tracing::warn!(
                    "Failed to generate embedding for memory {}: {}",
                    memory.id,
                    e
                );
            }
        }

        debug!("Memory stored successfully: {}", memory.id);
        Ok(())
    }

    async fn get_memory(&self, id: MemoryId) -> Result<MemoryNote> {
        debug!("Fetching memory: {}", id);

        let conn = self.get_conn()?;
        let mut rows = conn
            .query("SELECT * FROM memories WHERE id = ?", params![id.to_string()])
            .await?;

        let row = rows
            .next()
            .await?
            .ok_or_else(|| MnemosyneError::MemoryNotFound(id.to_string()))?;

        let mut memory = self.row_to_memory(&row).await?;

        // Fetch associated links
        let mut link_rows = conn
            .query(
                "SELECT target_id, link_type, strength, reason, created_at FROM memory_links WHERE source_id = ?",
                params![id.to_string()],
            )
            .await?;

        let mut links = Vec::new();
        while let Some(link_row) = link_rows.next().await? {
            let target_id_str: String = link_row.get(0)?;
            let target_id = MemoryId::from_string(&target_id_str)?;

            let link_type_str: String = link_row.get(1)?;
            let link_type = match link_type_str.as_str() {
                "extends" => crate::types::LinkType::Extends,
                "contradicts" => crate::types::LinkType::Contradicts,
                "implements" => crate::types::LinkType::Implements,
                "references" => crate::types::LinkType::References,
                "supersedes" => crate::types::LinkType::Supersedes,
                _ => continue,
            };

            let strength: f64 = link_row.get(2)?;
            let reason: String = link_row.get(3)?;
            let created_at_str: String = link_row.get(4)?;
            let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| MnemosyneError::Other(format!("Invalid timestamp: {}", e)))?
                .with_timezone(&chrono::Utc);

            links.push(crate::types::MemoryLink {
                target_id,
                link_type,
                strength: strength as f32,
                reason,
                created_at,
            });
        }

        memory.links = links;
        Ok(memory)
    }

    async fn update_memory(&self, memory: &MemoryNote) -> Result<()> {
        debug!("Updating memory: {}", memory.id);

        let conn = self.get_conn()?;
        let tx = conn.transaction().await?;

        // Build SQL and params with or without embedding
        if let Some(ref embedding) = memory.embedding {
            // Update with embedding using vector32()
            let embedding_json = serde_json::to_string(embedding)?;
            tx.execute(
                r#"
                UPDATE memories SET
                    updated_at = ?,
                    content = ?,
                    summary = ?,
                    keywords = ?,
                    tags = ?,
                    context = ?,
                    importance = ?,
                    confidence = ?,
                    related_files = ?,
                    related_entities = ?,
                    is_archived = ?,
                    superseded_by = ?,
                    embedding = vector32(?)
                WHERE id = ?
                "#,
                params![
                    Utc::now().to_rfc3339(),
                    memory.content.clone(),
                    memory.summary.clone(),
                    serde_json::to_string(&memory.keywords)?,
                    serde_json::to_string(&memory.tags)?,
                    memory.context.clone(),
                    memory.importance as i64,
                    memory.confidence as f64,
                    serde_json::to_string(&memory.related_files)?,
                    serde_json::to_string(&memory.related_entities)?,
                    if memory.is_archived { 1i64 } else { 0i64 },
                    memory.superseded_by.map(|id| id.to_string()),
                    embedding_json,
                    memory.id.to_string(),
                ],
            )
            .await?;
        } else {
            // Update without embedding
            tx.execute(
                r#"
                UPDATE memories SET
                    updated_at = ?,
                    content = ?,
                    summary = ?,
                    keywords = ?,
                    tags = ?,
                    context = ?,
                    importance = ?,
                    confidence = ?,
                    related_files = ?,
                    related_entities = ?,
                    is_archived = ?,
                    superseded_by = ?
                WHERE id = ?
                "#,
                params![
                    Utc::now().to_rfc3339(),
                    memory.content.clone(),
                    memory.summary.clone(),
                    serde_json::to_string(&memory.keywords)?,
                    serde_json::to_string(&memory.tags)?,
                    memory.context.clone(),
                    memory.importance as i64,
                    memory.confidence as f64,
                    serde_json::to_string(&memory.related_files)?,
                    serde_json::to_string(&memory.related_entities)?,
                    if memory.is_archived { 1i64 } else { 0i64 },
                    memory.superseded_by.map(|id| id.to_string()),
                    memory.id.to_string(),
                ],
            )
            .await?;
        }

        // Delete and re-insert links
        tx.execute(
            "DELETE FROM memory_links WHERE source_id = ?",
            params![memory.id.to_string()],
        )
        .await?;

        for link in &memory.links {
            tx.execute(
                r#"
                INSERT INTO memory_links (source_id, target_id, link_type, strength, reason, created_at)
                VALUES (?, ?, ?, ?, ?, ?)
                "#,
                params![
                    memory.id.to_string(),
                    link.target_id.to_string(),
                    serde_json::to_value(&link.link_type)?.as_str().unwrap(),
                    link.strength as f64,
                    link.reason.clone(),
                    link.created_at.to_rfc3339(),
                ],
            )
            .await?;
        }

        tx.commit().await.map_err(|e| {
            let error_msg = e.to_string();
            if error_msg.contains("readonly") || error_msg.contains("permission") {
                MnemosyneError::Database(
                    "Transaction failed: database is read-only. Ensure file and WAL files have write permissions.".to_string()
                )
            } else if error_msg.contains("locked") || error_msg.contains("busy") {
                MnemosyneError::Database(
                    "Transaction failed: database is locked. Another process may be writing.".to_string()
                )
            } else {
                MnemosyneError::Database(format!("Transaction commit failed: {}", error_msg))
            }
        })?;

        self.log_audit(
            "update",
            Some(memory.id),
            serde_json::json!({"importance": memory.importance}),
        )
        .await?;

        Ok(())
    }

    async fn archive_memory(&self, id: MemoryId) -> Result<()> {
        debug!("Archiving memory: {}", id);

        let conn = self.get_conn()?;

        conn.execute(
            r#"
            UPDATE memories
            SET is_archived = 1, updated_at = ?
            WHERE id = ?
            "#,
            params![Utc::now().to_rfc3339(), id.to_string()],
        )
        .await?;

        self.log_audit("archive", Some(id), serde_json::json!({}))
            .await?;

        Ok(())
    }

    async fn vector_search(
        &self,
        embedding: &[f32],
        limit: usize,
        namespace: Option<Namespace>,
    ) -> Result<Vec<SearchResult>> {
        debug!("Vector search (limit: {}, namespace: {:?})", limit, namespace);

        let conn = self.get_conn()?;
        let query_embedding = serde_json::to_string(embedding)?;

        let sql = if namespace.is_some() {
            format!(
                r#"
                SELECT
                    id, namespace, created_at, updated_at, content, summary,
                    keywords, tags, context, memory_type, importance, confidence,
                    related_files, related_entities, access_count, last_accessed_at,
                    expires_at, is_archived, superseded_by, embedding_model,
                    vector_distance_cos(embedding, vector32(?)) as distance
                FROM memories
                WHERE embedding IS NOT NULL
                  AND is_archived = 0
                  AND namespace = ?
                ORDER BY distance ASC
                LIMIT {}
                "#,
                limit
            )
        } else {
            format!(
                r#"
                SELECT
                    id, namespace, created_at, updated_at, content, summary,
                    keywords, tags, context, memory_type, importance, confidence,
                    related_files, related_entities, access_count, last_accessed_at,
                    expires_at, is_archived, superseded_by, embedding_model,
                    vector_distance_cos(embedding, vector32(?)) as distance
                FROM memories
                WHERE embedding IS NOT NULL
                  AND is_archived = 0
                ORDER BY distance ASC
                LIMIT {}
                "#,
                limit
            )
        };

        let mut rows = if let Some(ref ns) = namespace {
            let ns_json = serde_json::to_string(ns)?;
            conn.query(&sql, params![query_embedding, ns_json]).await?
        } else {
            conn.query(&sql, params![query_embedding]).await?
        };

        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            let distance: f64 = row.get(20)?;
            let memory = self.row_to_memory(&row).await?;
            let similarity = (1.0 - (distance as f32 / 2.0)).max(0.0).min(1.0);

            results.push(SearchResult {
                memory,
                score: similarity,
                match_reason: format!("Vector similarity: {:.2}", similarity),
            });
        }

        debug!("Vector search returned {} results", results.len());
        Ok(results)
    }

    async fn keyword_search(
        &self,
        query: &str,
        namespace: Option<Namespace>,
    ) -> Result<Vec<SearchResult>> {
        debug!("Keyword search: {} (namespace: {:?})", query, namespace);

        let namespace_filter = namespace.map(|ns| serde_json::to_string(&ns).unwrap());

        // Convert multi-word queries to OR logic for FTS5
        // "database architecture" -> "database OR architecture"
        // This matches user expectations: show results containing ANY of the search terms
        // Each term is escaped to handle hyphens and other FTS5 special characters
        let fts_query = if query.contains(' ') {
            query
                .split_whitespace()
                .map(|term| Self::escape_fts5_query(term))
                .collect::<Vec<String>>()
                .join(" OR ")
        } else {
            Self::escape_fts5_query(query)
        };

        // Handle empty query - return all memories in namespace (no FTS5)
        let conn = self.get_conn()?;
        let mut rows = if query.trim().is_empty() {
            // Empty query: list all memories (filtered by namespace if provided)
            let sql = if namespace_filter.is_some() {
                r#"
                SELECT * FROM memories
                WHERE namespace = ? AND is_archived = 0
                ORDER BY importance DESC, created_at DESC
                LIMIT 20
                "#
            } else {
                r#"
                SELECT * FROM memories
                WHERE is_archived = 0
                ORDER BY importance DESC, created_at DESC
                LIMIT 20
                "#
            };

            if let Some(ref ns) = namespace_filter {
                conn.query(sql, params![ns.clone()]).await?
            } else {
                conn.query(sql, params![]).await?
            }
        } else {
            // Non-empty query: use FTS5 full-text search with OR logic
            let sql = if namespace_filter.is_some() {
                r#"
                SELECT m.* FROM memories m
                WHERE m.rowid IN (
                    SELECT rowid FROM memories_fts WHERE memories_fts MATCH ?
                )
                AND m.namespace = ?
                AND m.is_archived = 0
                LIMIT 20
                "#
            } else {
                r#"
                SELECT m.* FROM memories m
                WHERE m.rowid IN (
                    SELECT rowid FROM memories_fts WHERE memories_fts MATCH ?
                )
                AND m.is_archived = 0
                LIMIT 20
                "#
            };

            if let Some(ref ns) = namespace_filter {
                conn.query(sql, params![fts_query, ns.clone()]).await?
            } else {
                conn.query(sql, params![fts_query]).await?
            }
        };

        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            let memory = self.row_to_memory(&row).await?;
            results.push(SearchResult {
                memory,
                score: 0.8,
                match_reason: "keyword_match".to_string(),
            });
        }

        debug!("Keyword search found {} results", results.len());
        Ok(results)
    }

    async fn graph_traverse(
        &self,
        seed_ids: &[MemoryId],
        max_hops: usize,
        namespace: Option<Namespace>,
    ) -> Result<Vec<MemoryNote>> {
        debug!("Graph traverse from {} seeds, max {} hops, namespace: {:?}", seed_ids.len(), max_hops, namespace);

        if seed_ids.is_empty() || max_hops == 0 {
            return Ok(vec![]);
        }

        let seed_strings: Vec<String> = seed_ids.iter().map(|id| id.to_string()).collect();
        let placeholders = seed_strings.iter().map(|_| "?").collect::<Vec<_>>().join(",");

        // Add namespace filter if provided
        let namespace_filter = if namespace.is_some() {
            "AND m.namespace = ?"
        } else {
            ""
        };

        let sql = format!(
            r#"
            WITH RECURSIVE graph_walk(memory_id, depth) AS (
                SELECT id, 0 FROM memories WHERE id IN ({placeholders})
                UNION
                SELECT
                    CASE
                        WHEN ml.source_id = gw.memory_id THEN ml.target_id
                        ELSE ml.source_id
                    END as memory_id,
                    gw.depth + 1
                FROM graph_walk gw
                JOIN memory_links ml ON (
                    ml.source_id = gw.memory_id OR ml.target_id = gw.memory_id
                )
                WHERE gw.depth < ?
            )
            SELECT DISTINCT m.*
            FROM memories m
            JOIN graph_walk gw ON m.id = gw.memory_id
            WHERE m.is_archived = 0 {namespace_filter}
            ORDER BY gw.depth, m.importance DESC
            "#,
            placeholders = placeholders,
            namespace_filter = namespace_filter
        );

        let conn = self.get_conn()?;
        let mut param_values: Vec<libsql::Value> = seed_strings
            .iter()
            .map(|s| libsql::Value::Text(s.clone()))
            .collect();
        param_values.push(libsql::Value::Integer(max_hops as i64));

        // Add namespace parameter if provided
        if let Some(ns) = namespace {
            let ns_json = serde_json::to_string(&ns)?;
            param_values.push(libsql::Value::Text(ns_json));
        }

        let mut rows = conn.query(&sql, libsql::params_from_iter(param_values)).await?;

        let mut results = Vec::new();
        while let Some(row) = rows.next().await? {
            results.push(self.row_to_memory(&row).await?);
        }

        debug!("Graph traversal found {} memories", results.len());
        Ok(results)
    }

    async fn find_consolidation_candidates(
        &self,
        namespace: Option<Namespace>,
    ) -> Result<Vec<(MemoryNote, MemoryNote)>> {
        debug!("Finding consolidation candidates (namespace: {:?})", namespace);

        let conn = self.get_conn()?;
        let sql = if namespace.is_some() {
            "SELECT * FROM memories WHERE namespace = ? AND is_archived = 0 AND embedding IS NOT NULL LIMIT 100"
        } else {
            "SELECT * FROM memories WHERE is_archived = 0 AND embedding IS NOT NULL LIMIT 100"
        };

        let mut rows = if let Some(ns) = namespace {
            let ns_json = serde_json::to_string(&ns)?;
            conn.query(sql, params![ns_json]).await?
        } else {
            conn.query(sql, params![]).await?
        };

        let mut memories = Vec::new();
        while let Some(row) = rows.next().await? {
            memories.push(self.row_to_memory(&row).await?);
        }

        debug!("Found {} memories to compare for consolidation", memories.len());

        let mut candidates = Vec::new();
        let similarity_threshold = 0.85;

        for i in 0..memories.len() {
            if let Some(ref embedding_i) = memories[i].embedding {
                let similar = self.vector_search(embedding_i, 5, None).await?;
                for (memory_id, similarity) in similar {
                    if memory_id == memories[i].id {
                        continue;
                    }
                    if similarity >= similarity_threshold {
                        let should_add = memories
                            .iter()
                            .position(|m| m.id == memory_id)
                            .map(|j| i < j)
                            .unwrap_or(false);

                        if should_add {
                            // Fetch the similar memory
                            if let Ok(similar_memory) = self.get_memory(memory_id).await {
                                debug!(
                                    "Consolidation candidate: {} <-> {} (similarity: {:.2})",
                                    memories[i].id, memory_id, similarity
                                );
                                candidates.push((memories[i].clone(), similar_memory));
                            }
                        }
                    }
                }
            }
        }

        debug!("Found {} consolidation candidate pairs", candidates.len());
        Ok(candidates)
    }

    async fn increment_access(&self, id: MemoryId) -> Result<()> {
        let conn = self.get_conn()?;
        conn.execute(
            r#"
            UPDATE memories
            SET access_count = access_count + 1,
                last_accessed_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now')
            WHERE id = ?
            "#,
            params![id.to_string()],
        )
        .await?;

        Ok(())
    }

    async fn count_memories(&self, namespace: Option<Namespace>) -> Result<usize> {
        let conn = self.get_conn()?;
        let (sql, params_vec) = if let Some(ns) = namespace {
            let ns_str = serde_json::to_string(&ns)?;
            (
                "SELECT COUNT(*) FROM memories WHERE namespace = ? AND is_archived = 0",
                vec![ns_str],
            )
        } else {
            ("SELECT COUNT(*) FROM memories WHERE is_archived = 0", vec![])
        };

        let mut rows = if params_vec.is_empty() {
            conn.query(sql, params![]).await?
        } else {
            conn.query(sql, params![params_vec[0].clone()]).await?
        };

        if let Some(row) = rows.next().await? {
            let count: i64 = row.get(0)?;
            Ok(count as usize)
        } else {
            Ok(0)
        }
    }

    async fn hybrid_search(
        &self,
        query: &str,
        namespace: Option<Namespace>,
        max_results: usize,
        expand_graph: bool,
    ) -> Result<Vec<SearchResult>> {
        debug!("Hybrid search: {} (expand_graph: {})", query, expand_graph);

        // Collect scores from different sources
        let mut memory_scores: std::collections::HashMap<MemoryId, (f32, f32, f32, f32)> =
            std::collections::HashMap::new(); // (keyword, vector, graph, depth)

        // 1. Keyword search
        let keyword_results = self.keyword_search(query, namespace.clone()).await?;
        debug!("Keyword search found {} results", keyword_results.len());

        for result in &keyword_results {
            memory_scores.insert(result.memory.id, (1.0, 0.0, 0.0, 0.0));
        }

        // 2. Vector search (if embedding service available and query non-empty)
        if !query.is_empty() && self.embedding_service.is_some() && self.search_config.enable_vector_search {
            // Generate query embedding
            if let Some(service) = &self.embedding_service {
                match service.embed(query).await {
                    Ok(query_embedding) => {
                        // Perform vector search
                        let vector_results = self.vector_search(&query_embedding, max_results * 2, namespace.clone()).await?;
                        debug!("Vector search found {} results", vector_results.len());

                        for (memory_id, similarity) in vector_results {
                            let entry = memory_scores.entry(memory_id).or_insert((0.0, 0.0, 0.0, 0.0));
                            entry.1 = similarity; // Update vector score
                        }
                    }
                    Err(e) => {
                        warn!("Failed to generate query embedding: {}", e);
                    }
                }
            }
        }

        // 3. Graph expansion (if enabled)
        let use_graph = expand_graph && self.search_config.enable_graph_expansion;
        if use_graph && !memory_scores.is_empty() {
            debug!("Expanding graph from {} seed memories", memory_scores.len());
            let seed_ids: Vec<_> = memory_scores.keys().take(5).copied().collect();
            let graph_memories = self.graph_traverse(&seed_ids, self.search_config.max_graph_depth, namespace.clone()).await?;

            for memory in graph_memories {
                let entry = memory_scores.entry(memory.id).or_insert((0.0, 0.0, 0.0, 1.0));
                entry.2 = 1.0; // Mark as graph-expanded
                entry.3 = entry.3.min(1.0); // Update depth
            }
        }

        // If no results from any source, return empty
        if memory_scores.is_empty() {
            debug!("No results from any search source");
            return Ok(vec![]);
        }

        // 4. Fetch all memories and compute final scores
        let now = Utc::now();
        let mut scored_results = Vec::new();

        for (memory_id, (keyword_score, vector_score, graph_score, depth)) in memory_scores {
            // Fetch full memory
            let memory = match self.get_memory(memory_id).await {
                Ok(m) => m,
                Err(e) => {
                    warn!("Failed to fetch memory {}: {}", memory_id, e);
                    continue;
                }
            };

            // Compute component scores
            let importance_score = memory.importance as f32 / 10.0;
            let age_days = (now - memory.created_at).num_days() as f32;
            let recency_score = (-age_days / 30.0).exp();
            let graph_depth_score = if graph_score > 0.0 { 1.0 / (1.0 + depth) } else { 0.0 };

            // Compute weighted final score using config weights
            let final_score =
                self.search_config.keyword_weight * keyword_score +
                self.search_config.vector_weight * vector_score +
                self.search_config.graph_weight * graph_depth_score +
                self.search_config.importance_weight * importance_score +
                self.search_config.recency_weight * recency_score;

            // Determine match reason
            let match_reason = if vector_score > keyword_score && vector_score > graph_depth_score {
                format!("vector_similarity ({:.2})", final_score)
            } else if keyword_score > 0.0 {
                format!("keyword_match ({:.2})", final_score)
            } else {
                format!("graph_expansion ({:.2})", final_score)
            };

            scored_results.push(SearchResult {
                memory,
                score: final_score,
                match_reason,
            });
        }

        // Sort by score and limit results
        scored_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
        scored_results.truncate(max_results);

        debug!("Hybrid search returned {} results", scored_results.len());
        Ok(scored_results)
    }

    async fn list_memories(
        &self,
        namespace: Option<Namespace>,
        limit: usize,
        sort_by: crate::storage::MemorySortOrder,
    ) -> Result<Vec<MemoryNote>> {
        use crate::storage::MemorySortOrder;

        debug!("Listing memories (namespace: {:?}, limit: {}, sort: {:?})", namespace, limit, sort_by);

        let conn = self.get_conn()?;
        let order_clause = match sort_by {
            MemorySortOrder::Recent => "created_at DESC",
            MemorySortOrder::Importance => "importance DESC, created_at DESC",
            MemorySortOrder::AccessCount => "access_count DESC, created_at DESC",
        };

        let (sql, params_vec) = if let Some(ns) = namespace {
            let ns_str = serde_json::to_string(&ns)?;
            (
                format!(
                    "SELECT * FROM memories WHERE namespace = ? AND is_archived = 0 ORDER BY {} LIMIT ?",
                    order_clause
                ),
                vec![ns_str],
            )
        } else {
            (
                format!(
                    "SELECT * FROM memories WHERE is_archived = 0 ORDER BY {} LIMIT ?",
                    order_clause
                ),
                vec![],
            )
        };

        let mut rows = if params_vec.is_empty() {
            conn.query(&sql, params![limit as i64]).await?
        } else {
            conn.query(&sql, params![params_vec[0].clone(), limit as i64])
                .await?
        };

        let mut memories = Vec::new();
        while let Some(row) = rows.next().await? {
            memories.push(self.row_to_memory(&row).await?);
        }

        debug!("Listed {} memories", memories.len());
        Ok(memories)
    }
}
