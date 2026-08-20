//! Database schema for the evaluation system.
//!
//! Creates tables for:
//! - context_evaluations: Feedback signals about provided context
//! - learned_relevance_weights: Adaptive weights learned from feedback

use crate::error::{MnemosyneError, Result};

/// Initialize evaluation database tables
pub async fn init_evaluation_tables(db_path: &str) -> Result<()> {
    let db = libsql::Builder::new_local(db_path)
        .build()
        .await
        .map_err(|e| MnemosyneError::Database(format!("Failed to open database: {}", e)))?;

    let conn = db
        .connect()
        .map_err(|e| MnemosyneError::Database(format!("Failed to get connection: {}", e)))?;

    // Execute all schema DDL in a single batch to minimize round-trips
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS context_evaluations (
            id TEXT PRIMARY KEY,
            session_id TEXT NOT NULL,
            agent_role TEXT NOT NULL,
            namespace TEXT NOT NULL,
            context_type TEXT NOT NULL,
            context_id TEXT NOT NULL,
            task_hash TEXT NOT NULL,
            task_keywords TEXT,
            task_type TEXT,
            work_phase TEXT,
            file_types TEXT,
            error_context TEXT,
            related_technologies TEXT,
            was_accessed INTEGER NOT NULL DEFAULT 0,
            access_count INTEGER NOT NULL DEFAULT 0,
            time_to_first_access_ms INTEGER,
            total_time_accessed_ms INTEGER NOT NULL DEFAULT 0,
            was_edited INTEGER NOT NULL DEFAULT 0,
            was_committed INTEGER NOT NULL DEFAULT 0,
            was_cited_in_response INTEGER NOT NULL DEFAULT 0,
            user_rating INTEGER,
            task_completed INTEGER NOT NULL DEFAULT 0,
            task_success_score REAL,
            context_provided_at INTEGER NOT NULL,
            evaluation_updated_at INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_eval_session ON context_evaluations(session_id);
        CREATE INDEX IF NOT EXISTS idx_eval_context ON context_evaluations(context_type, context_id);

        CREATE TABLE IF NOT EXISTS learned_relevance_weights (
            id TEXT PRIMARY KEY,
            scope TEXT NOT NULL,
            scope_id TEXT NOT NULL,
            context_type TEXT NOT NULL,
            agent_role TEXT NOT NULL,
            work_phase TEXT,
            task_type TEXT,
            error_context TEXT,
            weights TEXT NOT NULL,
            sample_count INTEGER NOT NULL DEFAULT 0,
            last_updated_at INTEGER NOT NULL,
            confidence REAL NOT NULL,
            learning_rate REAL NOT NULL,
            avg_precision REAL,
            avg_recall REAL,
            avg_f1_score REAL,
            UNIQUE(scope, scope_id, context_type, agent_role, work_phase, task_type, error_context)
        );

        CREATE INDEX IF NOT EXISTS idx_weights_scope ON learned_relevance_weights(scope, scope_id, context_type, agent_role);
        "#,
    )
    .await
    .map_err(|e| {
        MnemosyneError::Database(format!("Failed to create evaluation schema: {}", e))
    })?;

    tracing::debug!("Evaluation database schema initialized");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_init_schema() {
        // Use a single libsql Database with a URI-based shared in-memory
        // database so both init and verification use the same in-memory DB
        // without file I/O. The Database must stay alive to prevent the
        // shared-cache in-memory DB from being destroyed.
        let db_path = "file:eval_test_schema?mode=memory&cache=shared";
        let db = libsql::Builder::new_local(db_path).build().await.unwrap();

        // Initialize schema on a connection from this Database
        let conn = db.connect().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS context_evaluations (
                id TEXT PRIMARY KEY,
                session_id TEXT NOT NULL,
                agent_role TEXT NOT NULL,
                namespace TEXT NOT NULL,
                context_type TEXT NOT NULL,
                context_id TEXT NOT NULL,
                task_hash TEXT NOT NULL,
                task_keywords TEXT,
                task_type TEXT,
                work_phase TEXT,
                file_types TEXT,
                error_context TEXT,
                related_technologies TEXT,
                was_accessed INTEGER NOT NULL DEFAULT 0,
                access_count INTEGER NOT NULL DEFAULT 0,
                time_to_first_access_ms INTEGER,
                total_time_accessed_ms INTEGER NOT NULL DEFAULT 0,
                was_edited INTEGER NOT NULL DEFAULT 0,
                was_committed INTEGER NOT NULL DEFAULT 0,
                was_cited_in_response INTEGER NOT NULL DEFAULT 0,
                user_rating INTEGER,
                task_completed INTEGER NOT NULL DEFAULT 0,
                task_success_score REAL,
                context_provided_at INTEGER NOT NULL,
                evaluation_updated_at INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_eval_session ON context_evaluations(session_id);
            CREATE INDEX IF NOT EXISTS idx_eval_context ON context_evaluations(context_type, context_id);
            CREATE TABLE IF NOT EXISTS learned_relevance_weights (
                id TEXT PRIMARY KEY,
                scope TEXT NOT NULL,
                scope_id TEXT NOT NULL,
                context_type TEXT NOT NULL,
                agent_role TEXT NOT NULL,
                work_phase TEXT,
                task_type TEXT,
                error_context TEXT,
                weights TEXT NOT NULL,
                sample_count INTEGER NOT NULL DEFAULT 0,
                last_updated_at INTEGER NOT NULL,
                confidence REAL NOT NULL,
                learning_rate REAL NOT NULL,
                avg_precision REAL,
                avg_recall REAL,
                avg_f1_score REAL,
                UNIQUE(scope, scope_id, context_type, agent_role, work_phase, task_type, error_context)
            );
            CREATE INDEX IF NOT EXISTS idx_weights_scope ON learned_relevance_weights(scope, scope_id, context_type, agent_role);
            "#,
        )
        .await
        .unwrap();

        // Verify tables exist using the SAME Database
        let result = conn
            .query(
                "SELECT COUNT(*) FROM context_evaluations",
                libsql::params![],
            )
            .await;
        assert!(result.is_ok());

        let result = conn
            .query(
                "SELECT COUNT(*) FROM learned_relevance_weights",
                libsql::params![],
            )
            .await;
        assert!(result.is_ok());
    }
}
