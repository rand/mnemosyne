//! LibSQL storage backend implementation
//!
//! Provides persistent storage using Turso/libSQL with native vector search,
//! FTS5 for keyword search, and efficient indexing for graph traversal.

use crate::error::{MnemosyneError, Result};
use crate::storage::StorageBackend;
use crate::types::{MemoryId, MemoryNote, Namespace, SearchResult};
use async_trait::async_trait;
use chrono::Utc;
use libsql::{params, Builder, Connection, Database};
use tracing::{debug, info};

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

        let storage = Self { db };

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
    /// Default behavior: creates database if parent directory exists, fails if parent doesn't exist.
    /// This provides a balance between convenience and security.
    ///
    /// # Arguments
    /// * `mode` - Connection mode (local, in-memory, remote, or replica)
    ///
    /// # Example
    /// ```ignore
    /// // Normal use (creates database if parent directory exists)
    /// let storage = LibsqlStorage::new(ConnectionMode::Local("mnemosyne.db".into())).await?;
    /// ```
    pub async fn new(mode: ConnectionMode) -> Result<Self> {
        // Default behavior: create if parent directory exists (convenient but safe)
        // Use new_with_validation(..., true) for unconditional creation (init mode)
        // Use new_with_validation(..., false) for strict validation (must exist)
        Self::new_with_validation(mode, true).await
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
}

#[async_trait]
impl StorageBackend for LibsqlStorage {
    async fn store_memory(&self, memory: &MemoryNote) -> Result<()> {
        debug!("Storing memory: {}", memory.id);

        let conn = self.get_conn()?;
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

        tx.commit().await?;

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

        tx.commit().await?;

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
            // Non-empty query: use FTS5 full-text search
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
                conn.query(sql, params![query, ns.clone()]).await?
            } else {
                conn.query(sql, params![query]).await?
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
                for result in similar {
                    if result.memory.id == memories[i].id {
                        continue;
                    }
                    if result.score >= similarity_threshold {
                        let should_add = memories
                            .iter()
                            .position(|m| m.id == result.memory.id)
                            .map(|j| i < j)
                            .unwrap_or(false);

                        if should_add {
                            debug!(
                                "Consolidation candidate: {} <-> {} (similarity: {:.2})",
                                memories[i].id, result.memory.id, result.score
                            );
                            candidates.push((memories[i].clone(), result.memory));
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
                last_accessed_at = CURRENT_TIMESTAMP
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

        let keyword_results = self.keyword_search(query, namespace.clone()).await?;

        if keyword_results.is_empty() {
            debug!("No keyword matches found");
            return Ok(vec![]);
        }

        let mut all_memories = std::collections::HashMap::new();

        for result in keyword_results {
            all_memories.insert(result.memory.id, (result.memory.clone(), 1.0, 0));
        }

        if expand_graph {
            debug!("Expanding graph from {} seed memories", all_memories.len());
            let seed_ids: Vec<_> = all_memories.keys().take(5).copied().collect();
            let graph_memories = self.graph_traverse(&seed_ids, 2, namespace.clone()).await?;

            for memory in graph_memories {
                if !all_memories.contains_key(&memory.id) {
                    all_memories.insert(memory.id, (memory, 0.3, 1));
                }
            }
        }

        let now = Utc::now();
        let mut scored_results: Vec<_> = all_memories
            .into_values()
            .map(|(memory, keyword_score, depth)| {
                let importance_score = memory.importance as f32 / 10.0;
                let age_days = (now - memory.created_at).num_days() as f32;
                let recency_score = (-age_days / 30.0).exp();
                let graph_score = 1.0 / (1.0 + depth as f32);

                let score = 0.5 * keyword_score
                    + 0.2 * graph_score
                    + 0.2 * importance_score
                    + 0.1 * recency_score;

                let match_reason = if keyword_score > 0.5 {
                    format!("keyword_match (score: {:.2})", score)
                } else {
                    format!("graph_expansion (score: {:.2})", score)
                };

                SearchResult {
                    memory,
                    score,
                    match_reason,
                }
            })
            .collect();

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
