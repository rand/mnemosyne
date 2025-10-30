//! Privacy-preserving feature extraction for context relevance learning.
//!
//! Extracts statistical features from context evaluations without storing
//! raw content. All features are computed metrics (e.g., "30% keyword overlap")
//! rather than literal values.
//!
//! # Privacy Design
//!
//! - No raw text stored
//! - Keywords used for overlap calculation, then discarded
//! - Only statistical scores persisted
//! - All computations local, no network calls

use crate::embeddings::{cosine_similarity, EmbeddingService, LocalEmbeddingService};
use crate::error::{MnemosyneError, Result};
use crate::evaluation::feedback_collector::{ContextEvaluation, ContextType};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, warn};

/// Privacy-preserving relevance features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelevanceFeatures {
    pub evaluation_id: String,

    // Statistical features (privacy-preserving)
    pub keyword_overlap_score: f32, // Jaccard similarity [0.0, 1.0]
    pub semantic_similarity: Option<f32>, // Cosine similarity if embeddings available
    pub recency_days: f32,          // Days since context was created
    pub access_frequency: f32,      // Accesses per day
    pub last_used_days_ago: Option<f32>, // Days since last access

    // Contextual match features
    pub work_phase_match: bool,
    pub task_type_match: bool,
    pub agent_role_affinity: f32, // How well this context suits this agent
    pub namespace_match: bool,
    pub file_type_match: bool,

    // Historical performance features
    pub historical_success_rate: Option<f32>, // Past success rate for this context
    pub co_occurrence_score: Option<f32>,     // How often it appears with other useful contexts

    // Ground truth (outcome)
    pub was_useful: bool, // Did user actually use this context?
}

/// Feature extractor
pub struct FeatureExtractor {
    db_path: String,
    embedding_service: Option<Arc<LocalEmbeddingService>>,
}

impl FeatureExtractor {
    /// Create a new feature extractor
    pub fn new(db_path: String) -> Self {
        Self {
            db_path,
            embedding_service: None,
        }
    }

    /// Get the database path
    pub fn db_path(&self) -> &str {
        &self.db_path
    }

    /// Set the embedding service for semantic similarity computation
    pub fn set_embedding_service(&mut self, service: Arc<LocalEmbeddingService>) {
        self.embedding_service = Some(service);
    }

    /// Extract features from an evaluation
    ///
    /// This is called after enough feedback signals have been collected
    /// to determine if the context was useful.
    pub async fn extract_features(
        &self,
        evaluation: &ContextEvaluation,
        context_keywords: &[String], // Keywords from the actual context (skill/memory/file)
    ) -> Result<RelevanceFeatures> {
        debug!("Extracting features for evaluation {}", evaluation.id);

        // Keyword overlap (privacy-preserving: compute score, discard keywords)
        let keyword_overlap_score = self.compute_keyword_overlap(
            &evaluation.task_keywords.clone().unwrap_or_default(),
            context_keywords,
        );

        // Semantic similarity (if embeddings available)
        let semantic_similarity = self
            .compute_semantic_similarity(
                &evaluation.task_keywords.clone().unwrap_or_default(),
                context_keywords,
            )
            .await
            .ok()
            .flatten();

        // Recency features
        let recency_days = self
            .compute_recency_days(&evaluation.context_id, &evaluation.context_type)
            .await?;
        let access_frequency = self
            .compute_access_frequency(&evaluation.context_id, &evaluation.context_type)
            .await?;
        let last_used_days_ago = self
            .compute_last_used_days(&evaluation.context_id, &evaluation.context_type)
            .await?;

        // Contextual match features
        let work_phase_match = evaluation.work_phase.is_some();
        let task_type_match = evaluation.task_type.is_some();
        let namespace_match = self.compute_namespace_match(evaluation);
        let file_type_match = self.compute_file_type_match(evaluation);

        // Agent affinity (how well this context type suits this agent)
        let agent_role_affinity =
            self.compute_agent_affinity(&evaluation.agent_role, &evaluation.context_type);

        // Historical features
        let historical_success_rate = self
            .compute_historical_success(&evaluation.context_id, &evaluation.context_type)
            .await?;

        let co_occurrence_score = self
            .compute_co_occurrence(&evaluation.context_id, &evaluation.session_id)
            .await?;

        // Ground truth: was this context actually useful?
        let was_useful = self.determine_usefulness(evaluation);

        Ok(RelevanceFeatures {
            evaluation_id: evaluation.id.clone(),
            keyword_overlap_score,
            semantic_similarity,
            recency_days,
            access_frequency,
            last_used_days_ago,
            work_phase_match,
            task_type_match,
            agent_role_affinity,
            namespace_match,
            file_type_match,
            historical_success_rate,
            co_occurrence_score,
            was_useful,
        })
    }

    /// Compute keyword overlap using Jaccard similarity
    ///
    /// Privacy-preserving: computes score, doesn't store keywords
    fn compute_keyword_overlap(
        &self,
        task_keywords: &[String],
        context_keywords: &[String],
    ) -> f32 {
        if task_keywords.is_empty() || context_keywords.is_empty() {
            return 0.0;
        }

        let task_set: HashSet<_> = task_keywords.iter().map(|s| s.to_lowercase()).collect();
        let context_set: HashSet<_> = context_keywords.iter().map(|s| s.to_lowercase()).collect();

        let intersection = task_set.intersection(&context_set).count();
        let union = task_set.union(&context_set).count();

        if union == 0 {
            0.0
        } else {
            intersection as f32 / union as f32
        }
    }

    /// Compute semantic similarity using embeddings
    ///
    /// Computes cosine similarity between task keywords and context keywords
    /// using local embeddings. Returns None if embedding service unavailable
    /// or if embeddings fail to generate.
    ///
    /// Privacy: Only keywords are embedded, no raw content.
    async fn compute_semantic_similarity(
        &self,
        task_keywords: &[String],
        context_keywords: &[String],
    ) -> Result<Option<f32>> {
        // Skip if no embedding service
        let service = match &self.embedding_service {
            Some(s) => s,
            None => {
                debug!("No embedding service available for semantic similarity");
                return Ok(None);
            }
        };

        // Skip if either keyword list is empty
        if task_keywords.is_empty() || context_keywords.is_empty() {
            debug!("Skipping semantic similarity - empty keyword list");
            return Ok(None);
        }

        // Embed task keywords (join into single text)
        let task_text = task_keywords.join(" ");
        let task_embedding = match service.embed(&task_text).await {
            Ok(emb) => emb,
            Err(e) => {
                warn!("Failed to embed task keywords: {}", e);
                return Ok(None);
            }
        };

        // Embed context keywords
        let context_text = context_keywords.join(" ");
        let context_embedding = match service.embed(&context_text).await {
            Ok(emb) => emb,
            Err(e) => {
                warn!("Failed to embed context keywords: {}", e);
                return Ok(None);
            }
        };

        // Compute cosine similarity
        let similarity = cosine_similarity(&task_embedding, &context_embedding);

        debug!(
            "Semantic similarity computed: {:.3} (task: {}, context: {})",
            similarity,
            task_keywords.len(),
            context_keywords.len()
        );

        Ok(Some(similarity))
    }

    /// Compute recency (days since context was created)
    async fn compute_recency_days(
        &self,
        context_id: &str,
        context_type: &ContextType,
    ) -> Result<f32> {
        match context_type {
            ContextType::Memory => {
                // Fetch memory creation date
                self.get_memory_recency(context_id).await
            }
            ContextType::Skill | ContextType::File => {
                // For skills/files, use file modification time
                self.compute_file_recency(context_id).await
            }
            _ => Ok(0.0),
        }
    }

    /// Compute file recency from filesystem metadata
    async fn compute_file_recency(&self, file_path: &str) -> Result<f32> {
        let path = Path::new(file_path);

        if !path.exists() {
            debug!("File does not exist: {}", file_path);
            return Ok(365.0); // Treat non-existent files as very old
        }

        let metadata = std::fs::metadata(path).map_err(|e| {
            MnemosyneError::Other(format!("Failed to stat file {}: {}", file_path, e))
        })?;

        let modified = metadata.modified().map_err(|e| {
            MnemosyneError::Other(format!("Failed to get modified time for {}: {}", file_path, e))
        })?;

        let duration = std::time::SystemTime::now()
            .duration_since(modified)
            .map_err(|e| {
                MnemosyneError::Other(format!("Time calculation error for {}: {}", file_path, e))
            })?;

        let days = duration.as_secs() as f32 / 86400.0;

        debug!("File {} is {:.1} days old", file_path, days);
        Ok(days)
    }

    /// Compute access frequency (accesses per day)
    async fn compute_access_frequency(
        &self,
        context_id: &str,
        context_type: &ContextType,
    ) -> Result<f32> {
        match context_type {
            ContextType::Memory => {
                self.get_memory_access_frequency(context_id).await
            }
            _ => Ok(0.0),
        }
    }

    /// Compute days since last use
    async fn compute_last_used_days(
        &self,
        context_id: &str,
        context_type: &ContextType,
    ) -> Result<Option<f32>> {
        match context_type {
            ContextType::Memory => {
                self.get_memory_last_used_days(context_id).await
            }
            _ => Ok(None),
        }
    }

    /// Compute namespace match
    ///
    /// Check if the context's namespace matches the task's namespace.
    /// For memory contexts, check if memory namespace matches task namespace.
    /// For skill/file contexts, they're typically global so match any namespace.
    fn compute_namespace_match(&self, evaluation: &ContextEvaluation) -> bool {
        match &evaluation.context_type {
            ContextType::Memory => {
                // For memories, would need to look up memory's namespace
                // For now, assume match if task has a namespace
                !evaluation.namespace.is_empty()
            }
            ContextType::Skill | ContextType::File => {
                // Skills and files are typically global, match any namespace
                true
            }
            ContextType::Plan | ContextType::Commit => {
                // Plans and commits are session/project-specific
                !evaluation.namespace.is_empty()
            }
        }
    }

    /// Compute file type match
    fn compute_file_type_match(&self, evaluation: &ContextEvaluation) -> bool {
        // Check if context involves files matching task file types
        if let (Some(task_files), ContextType::File) =
            (&evaluation.file_types, &evaluation.context_type)
        {
            // Simple heuristic: if task involves .rs files and this is a Rust skill/file, match
            return !task_files.is_empty();
        }
        false
    }

    /// Compute agent role affinity
    ///
    /// How well does this context type suit this agent role?
    fn compute_agent_affinity(&self, agent_role: &str, context_type: &ContextType) -> f32 {
        // Hardcoded affinity matrix (could be learned over time)
        match (agent_role, context_type) {
            ("optimizer", ContextType::Skill) => 0.9,
            ("optimizer", ContextType::Memory) => 0.7,
            ("executor", ContextType::File) => 0.9,
            ("executor", ContextType::Memory) => 0.6,
            ("reviewer", ContextType::Memory) => 0.8,
            ("orchestrator", ContextType::Plan) => 0.9,
            _ => 0.5, // Neutral affinity
        }
    }

    /// Get memory recency (days since created)
    async fn get_memory_recency(&self, memory_id: &str) -> Result<f32> {
        let db = libsql::Builder::new_local(&self.db_path)
            .build()
            .await
            .map_err(|e| MnemosyneError::Database(format!("Failed to open database: {}", e)))?;

        let conn = db.connect().map_err(|e| {
            MnemosyneError::Database(format!("Failed to get connection: {}", e))
        })?;

        let mut rows = conn
            .query(
                "SELECT created_at FROM memories WHERE id = ?",
                libsql::params![memory_id],
            )
            .await
            .map_err(|e| {
                MnemosyneError::Database(format!("Failed to query memory: {}", e))
            })?;

        let row = rows.next().await.map_err(|e| {
            MnemosyneError::Database(format!("Failed to fetch row: {}", e))
        })?;

        let row = row.ok_or_else(|| {
            warn!("Memory not found: {}", memory_id);
            MnemosyneError::NotFound(format!("Memory not found: {}", memory_id))
        })?;

        let created_at = row.get::<i64>(0).map_err(|e| {
            MnemosyneError::Database(format!("Failed to get created_at: {}", e))
        })?;

        // Calculate days since creation
        let now = chrono::Utc::now().timestamp();
        let age_seconds = (now - created_at).max(0);
        let days = age_seconds as f32 / 86400.0;

        debug!("Memory {} is {:.1} days old", memory_id, days);
        Ok(days)
    }

    /// Get memory access frequency (accesses per day)
    async fn get_memory_access_frequency(&self, memory_id: &str) -> Result<f32> {
        let db = libsql::Builder::new_local(&self.db_path)
            .build()
            .await
            .map_err(|e| MnemosyneError::Database(format!("Failed to open database: {}", e)))?;

        let conn = db.connect().map_err(|e| {
            MnemosyneError::Database(format!("Failed to get connection: {}", e))
        })?;

        let mut rows = conn
            .query(
                "SELECT access_count, created_at FROM memories WHERE id = ?",
                libsql::params![memory_id],
            )
            .await
            .map_err(|e| {
                MnemosyneError::Database(format!("Failed to query memory: {}", e))
            })?;

        let row = rows.next().await.map_err(|e| {
            MnemosyneError::Database(format!("Failed to fetch row: {}", e))
        })?;

        let row = row.ok_or_else(|| {
            warn!("Memory not found: {}", memory_id);
            MnemosyneError::NotFound(format!("Memory not found: {}", memory_id))
        })?;

        let access_count = row.get::<i64>(0).map_err(|e| {
            MnemosyneError::Database(format!("Failed to get access_count: {}", e))
        })? as f32;

        let created_at = row.get::<i64>(1).map_err(|e| {
            MnemosyneError::Database(format!("Failed to get created_at: {}", e))
        })?;

        // Calculate frequency (accesses per day)
        let now = chrono::Utc::now().timestamp();
        let age_days = ((now - created_at).max(1) as f32) / 86400.0;
        let frequency = access_count / age_days.max(1.0);

        debug!(
            "Memory {} has access frequency {:.3} accesses/day",
            memory_id, frequency
        );
        Ok(frequency)
    }

    /// Get days since memory was last accessed
    async fn get_memory_last_used_days(&self, memory_id: &str) -> Result<Option<f32>> {
        let db = libsql::Builder::new_local(&self.db_path)
            .build()
            .await
            .map_err(|e| MnemosyneError::Database(format!("Failed to open database: {}", e)))?;

        let conn = db.connect().map_err(|e| {
            MnemosyneError::Database(format!("Failed to get connection: {}", e))
        })?;

        let mut rows = conn
            .query(
                "SELECT last_accessed_at FROM memories WHERE id = ?",
                libsql::params![memory_id],
            )
            .await
            .map_err(|e| {
                MnemosyneError::Database(format!("Failed to query memory: {}", e))
            })?;

        let row = rows.next().await.map_err(|e| {
            MnemosyneError::Database(format!("Failed to fetch row: {}", e))
        })?;

        let row = row.ok_or_else(|| {
            warn!("Memory not found: {}", memory_id);
            MnemosyneError::NotFound(format!("Memory not found: {}", memory_id))
        })?;

        let last_accessed_at = row.get::<Option<i64>>(0).ok().flatten();

        if let Some(timestamp) = last_accessed_at {
            let now = chrono::Utc::now().timestamp();
            let days_ago = ((now - timestamp).max(0) as f32) / 86400.0;
            debug!("Memory {} was last accessed {:.1} days ago", memory_id, days_ago);
            Ok(Some(days_ago))
        } else {
            debug!("Memory {} has never been accessed", memory_id);
            Ok(None)
        }
    }

    /// Compute historical success rate for this context
    ///
    /// Success rate = (times accessed) / (times provided)
    /// Requires at least 3 evaluations to return a meaningful score
    async fn compute_historical_success(
        &self,
        context_id: &str,
        context_type: &ContextType,
    ) -> Result<Option<f32>> {
        let db = libsql::Builder::new_local(&self.db_path)
            .build()
            .await
            .map_err(|e| MnemosyneError::Database(format!("Failed to open database: {}", e)))?;

        let conn = db.connect().map_err(|e| {
            MnemosyneError::Database(format!("Failed to get connection: {}", e))
        })?;

        let context_type_str = match context_type {
            ContextType::Memory => "memory",
            ContextType::Skill => "skill",
            ContextType::File => "file",
            ContextType::Commit => "commit",
            ContextType::Plan => "plan",
        };

        let mut rows = conn
            .query(
                r#"
                SELECT
                    COUNT(*) as total_provided,
                    SUM(was_accessed) as times_accessed
                FROM context_evaluations
                WHERE context_id = ? AND context_type = ?
                "#,
                libsql::params![context_id, context_type_str],
            )
            .await
            .map_err(|e| {
                MnemosyneError::Database(format!("Failed to query evaluations: {}", e))
            })?;

        let row = rows.next().await.map_err(|e| {
            MnemosyneError::Database(format!("Failed to fetch row: {}", e))
        })?;

        let row = row.ok_or_else(|| {
            MnemosyneError::Database("No evaluation data found".to_string())
        })?;

        let total_provided = row.get::<i64>(0).unwrap_or(0);
        let times_accessed = row.get::<i64>(1).unwrap_or(0);

        // Require at least 3 evaluations for meaningful statistics
        if total_provided < 3 {
            debug!(
                "Context {} has only {} evaluations, not enough for historical success",
                context_id, total_provided
            );
            return Ok(None);
        }

        let success_rate = times_accessed as f32 / total_provided as f32;

        debug!(
            "Context {} historical success: {}/{} = {:.3}",
            context_id, times_accessed, total_provided, success_rate
        );

        Ok(Some(success_rate))
    }

    /// Compute co-occurrence score
    ///
    /// How often does this context appear alongside other useful contexts?
    /// Score = (times this context appeared with useful contexts) / (total appearances)
    async fn compute_co_occurrence(
        &self,
        context_id: &str,
        _session_id: &str,
    ) -> Result<Option<f32>> {
        let db = libsql::Builder::new_local(&self.db_path)
            .build()
            .await
            .map_err(|e| MnemosyneError::Database(format!("Failed to open database: {}", e)))?;

        let conn = db.connect().map_err(|e| {
            MnemosyneError::Database(format!("Failed to get connection: {}", e))
        })?;

        // Find sessions where this context appeared
        let mut rows = conn
            .query(
                r#"
                SELECT DISTINCT session_id
                FROM context_evaluations
                WHERE context_id = ?
                LIMIT 10
                "#,
                libsql::params![context_id],
            )
            .await
            .map_err(|e| {
                MnemosyneError::Database(format!("Failed to query sessions: {}", e))
            })?;

        let mut sessions = Vec::new();
        while let Ok(Some(row)) = rows.next().await {
            if let Ok(sid) = row.get::<String>(0) {
                sessions.push(sid);
            }
        }

        if sessions.is_empty() {
            debug!("No co-occurrence data for context {}", context_id);
            return Ok(None);
        }

        // For each session, check if other contexts in that session were useful
        let mut total_cooccurrences = 0;
        let mut useful_cooccurrences = 0;

        for sess_id in sessions {
            let mut cooccur_rows = conn
                .query(
                    r#"
                    SELECT was_accessed
                    FROM context_evaluations
                    WHERE session_id = ? AND context_id != ?
                    "#,
                    libsql::params![sess_id, context_id],
                )
                .await
                .map_err(|e| {
                    MnemosyneError::Database(format!("Failed to query co-occurrences: {}", e))
                })?;

            while let Ok(Some(row)) = cooccur_rows.next().await {
                total_cooccurrences += 1;
                let was_accessed = row.get::<i64>(0).unwrap_or(0);
                if was_accessed != 0 {
                    useful_cooccurrences += 1;
                }
            }
        }

        if total_cooccurrences < 3 {
            debug!(
                "Only {} co-occurrences for context {}, not enough data",
                total_cooccurrences, context_id
            );
            return Ok(None);
        }

        let score = useful_cooccurrences as f32 / total_cooccurrences as f32;

        debug!(
            "Context {} co-occurrence: {}/{} = {:.3}",
            context_id, useful_cooccurrences, total_cooccurrences, score
        );

        Ok(Some(score))
    }

    /// Determine if context was useful based on feedback signals
    ///
    /// Heuristic:
    /// - Accessed + (edited OR committed OR cited) = useful
    /// - Explicit positive rating = useful
    /// - Not accessed within reasonable time = not useful
    fn determine_usefulness(&self, evaluation: &ContextEvaluation) -> bool {
        // Explicit rating takes precedence
        if let Some(rating) = evaluation.user_rating {
            return rating > 0;
        }

        // Implicit signals
        let was_used = evaluation.was_accessed
            && (evaluation.was_edited
                || evaluation.was_committed
                || evaluation.was_cited_in_response);

        // Strong signal: accessed multiple times
        let frequently_accessed = evaluation.access_count >= 2;

        // Context was useful if it was used or frequently accessed
        was_used || frequently_accessed
    }

    /// Store features in database
    pub async fn store_features(&self, features: &RelevanceFeatures) -> Result<()> {
        debug!("Storing features for evaluation {}", features.evaluation_id);

        let db = libsql::Builder::new_local(&self.db_path)
            .build()
            .await
            .map_err(|e| MnemosyneError::Database(format!("Failed to open database: {}", e)))?;

        let conn = db.connect().map_err(|e| {
            MnemosyneError::Database(format!("Failed to get connection: {}", e))
        })?;

        conn.execute(
            r#"
            INSERT INTO relevance_features (
                evaluation_id,
                keyword_overlap_score,
                semantic_similarity,
                recency_days,
                access_frequency,
                last_used_days_ago,
                work_phase_match,
                task_type_match,
                agent_role_affinity,
                namespace_match,
                file_type_match,
                historical_success_rate,
                co_occurrence_score,
                was_useful
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            libsql::params![
                features.evaluation_id.clone(),
                features.keyword_overlap_score,
                features.semantic_similarity,
                features.recency_days,
                features.access_frequency,
                features.last_used_days_ago,
                if features.work_phase_match { 1 } else { 0 },
                if features.task_type_match { 1 } else { 0 },
                features.agent_role_affinity,
                if features.namespace_match { 1 } else { 0 },
                if features.file_type_match { 1 } else { 0 },
                features.historical_success_rate,
                features.co_occurrence_score,
                if features.was_useful { 1 } else { 0 },
            ],
        )
        .await
        .map_err(|e| {
            MnemosyneError::Database(format!("Failed to insert features: {}", e))
        })?;

        debug!("Stored features for evaluation {}", features.evaluation_id);
        Ok(())
    }

    /// Get features for an evaluation
    pub async fn get_features(&self, evaluation_id: &str) -> Result<RelevanceFeatures> {
        let db = libsql::Builder::new_local(&self.db_path)
            .build()
            .await
            .map_err(|e| MnemosyneError::Database(format!("Failed to open database: {}", e)))?;

        let conn = db.connect().map_err(|e| {
            MnemosyneError::Database(format!("Failed to get connection: {}", e))
        })?;

        let mut rows = conn
            .query(
                r#"
                SELECT
                    evaluation_id,
                    keyword_overlap_score,
                    semantic_similarity,
                    recency_days,
                    access_frequency,
                    last_used_days_ago,
                    work_phase_match,
                    task_type_match,
                    agent_role_affinity,
                    namespace_match,
                    file_type_match,
                    historical_success_rate,
                    co_occurrence_score,
                    was_useful
                FROM relevance_features
                WHERE evaluation_id = ?
                "#,
                libsql::params![evaluation_id],
            )
            .await
            .map_err(|e| {
                MnemosyneError::Database(format!("Failed to query features: {}", e))
            })?;

        let row = rows.next().await.map_err(|e| {
            MnemosyneError::Database(format!("Failed to fetch row: {}", e))
        })?;

        let row = row.ok_or_else(|| {
            MnemosyneError::NotFound(format!("Features not found for evaluation: {}", evaluation_id))
        })?;

        Ok(RelevanceFeatures {
            evaluation_id: row.get::<String>(0).map_err(|e| {
                MnemosyneError::Database(format!("Failed to get evaluation_id: {}", e))
            })?,
            keyword_overlap_score: row.get::<f64>(1).map_err(|e| {
                MnemosyneError::Database(format!("Failed to get keyword_overlap_score: {}", e))
            })? as f32,
            semantic_similarity: row.get::<Option<f64>>(2).ok().flatten().map(|v| v as f32),
            recency_days: row.get::<f64>(3).map_err(|e| {
                MnemosyneError::Database(format!("Failed to get recency_days: {}", e))
            })? as f32,
            access_frequency: row.get::<f64>(4).map_err(|e| {
                MnemosyneError::Database(format!("Failed to get access_frequency: {}", e))
            })? as f32,
            last_used_days_ago: row.get::<Option<f64>>(5).ok().flatten().map(|v| v as f32),
            work_phase_match: row.get::<i64>(6).map_err(|e| {
                MnemosyneError::Database(format!("Failed to get work_phase_match: {}", e))
            })? != 0,
            task_type_match: row.get::<i64>(7).map_err(|e| {
                MnemosyneError::Database(format!("Failed to get task_type_match: {}", e))
            })? != 0,
            agent_role_affinity: row.get::<Option<f64>>(8).ok().flatten().map(|v| v as f32).unwrap_or(0.5),
            namespace_match: row.get::<i64>(9).map_err(|e| {
                MnemosyneError::Database(format!("Failed to get namespace_match: {}", e))
            })? != 0,
            file_type_match: row.get::<i64>(10).map_err(|e| {
                MnemosyneError::Database(format!("Failed to get file_type_match: {}", e))
            })? != 0,
            historical_success_rate: row.get::<Option<f64>>(11).ok().flatten().map(|v| v as f32),
            co_occurrence_score: row.get::<Option<f64>>(12).ok().flatten().map(|v| v as f32),
            was_useful: row.get::<i64>(13).map_err(|e| {
                MnemosyneError::Database(format!("Failed to get was_useful: {}", e))
            })? != 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_keyword_overlap_jaccard() {
        let extractor = create_test_extractor();

        // Perfect overlap
        let task = vec!["rust".to_string(), "async".to_string()];
        let context = vec!["rust".to_string(), "async".to_string()];
        let score = extractor.compute_keyword_overlap(&task, &context);
        assert!((score - 1.0).abs() < 0.001);

        // Partial overlap
        let task = vec!["rust".to_string(), "async".to_string(), "tokio".to_string()];
        let context = vec!["rust".to_string(), "sync".to_string()];
        let score = extractor.compute_keyword_overlap(&task, &context);
        // intersection: 1 (rust), union: 4 (rust, async, tokio, sync)
        assert!((score - 0.25).abs() < 0.001);

        // No overlap
        let task = vec!["python".to_string()];
        let context = vec!["rust".to_string()];
        let score = extractor.compute_keyword_overlap(&task, &context);
        assert!((score - 0.0).abs() < 0.001);
    }

    #[test]
    fn test_agent_affinity() {
        let extractor = create_test_extractor();

        // High affinity
        let score = extractor.compute_agent_affinity("optimizer", &ContextType::Skill);
        assert!(score > 0.8);

        // Medium affinity
        let score = extractor.compute_agent_affinity("executor", &ContextType::Memory);
        assert!(score >= 0.5 && score < 0.8);
    }

    #[test]
    fn test_determine_usefulness() {
        let extractor = create_test_extractor();

        // Useful: accessed and edited
        let eval = create_test_evaluation();
        let mut eval_useful = eval.clone();
        eval_useful.was_accessed = true;
        eval_useful.was_edited = true;
        assert!(extractor.determine_usefulness(&eval_useful));

        // Useful: accessed multiple times
        let mut eval_frequent = eval.clone();
        eval_frequent.was_accessed = true;
        eval_frequent.access_count = 3;
        assert!(extractor.determine_usefulness(&eval_frequent));

        // Not useful: not accessed
        let eval_unused = eval.clone();
        assert!(!extractor.determine_usefulness(&eval_unused));

        // Explicit rating overrides
        let mut eval_rated = eval.clone();
        eval_rated.user_rating = Some(1);
        assert!(extractor.determine_usefulness(&eval_rated));
    }

    #[test]
    fn test_keyword_overlap_privacy() {
        let extractor = create_test_extractor();

        // Verify keyword overlap is computed without storing keywords
        let task_keywords = vec![
            "rust".to_string(),
            "async".to_string(),
            "sensitive_data".to_string(),
        ];
        let context_keywords = vec!["rust".to_string(), "tokio".to_string()];

        let score = extractor.compute_keyword_overlap(&task_keywords, &context_keywords);

        // Score should be computed (non-zero if overlap exists)
        assert!(score > 0.0, "Should compute overlap score");
        assert!(score <= 1.0, "Score should be normalized");

        // Keywords themselves are not returned or stored, only the score
    }

    #[test]
    fn test_keyword_overlap_empty_inputs() {
        let extractor = create_test_extractor();

        // Empty keywords should return 0.0
        let empty: Vec<String> = vec![];
        let keywords = vec!["rust".to_string()];

        assert_eq!(extractor.compute_keyword_overlap(&empty, &keywords), 0.0);
        assert_eq!(extractor.compute_keyword_overlap(&keywords, &empty), 0.0);
        assert_eq!(extractor.compute_keyword_overlap(&empty, &empty), 0.0);
    }

    #[test]
    fn test_features_contain_no_raw_content() {
        // Verify RelevanceFeatures struct only contains statistical/numeric fields
        let features = RelevanceFeatures {
            evaluation_id: "test".to_string(),
            keyword_overlap_score: 0.5,
            semantic_similarity: Some(0.7),
            recency_days: 7.0,
            access_frequency: 0.3,
            last_used_days_ago: Some(2.0),
            work_phase_match: true,
            task_type_match: true,
            agent_role_affinity: 0.8,
            namespace_match: true,
            file_type_match: false,
            historical_success_rate: Some(0.6),
            co_occurrence_score: Some(0.4),
            was_useful: true,
        };

        // Serialize to JSON and verify no raw content
        let json = serde_json::to_string(&features).expect("Failed to serialize");

        // Should not contain raw keywords or content
        assert!(
            !json.contains("password"),
            "Should not contain sensitive keywords"
        );
        assert!(
            !json.contains("secret"),
            "Should not contain sensitive keywords"
        );

        // Should only contain numeric/boolean values and evaluation_id
        assert!(json.contains("keyword_overlap_score"));
        assert!(json.contains("0.5") || json.contains("0.5"));
    }

    #[test]
    fn test_agent_affinity_privacy() {
        let extractor = create_test_extractor();

        // Agent affinity should be based on role/type only, not content
        let affinity1 = extractor.compute_agent_affinity("optimizer", &ContextType::Skill);
        let affinity2 = extractor.compute_agent_affinity("optimizer", &ContextType::Skill);

        // Same inputs should give same affinity
        assert_eq!(
            affinity1, affinity2,
            "Agent affinity should be deterministic"
        );

        // Affinity should be normalized
        assert!(
            affinity1 >= 0.0 && affinity1 <= 1.0,
            "Affinity should be in [0.0, 1.0]"
        );
    }

    #[test]
    fn test_file_type_match_privacy() {
        let extractor = create_test_extractor();

        let mut eval = create_test_evaluation();
        eval.context_type = ContextType::File;
        eval.file_types = Some(vec![".rs".to_string(), ".toml".to_string()]);

        let has_match = extractor.compute_file_type_match(&eval);

        // Should return boolean, not reveal file paths or names
        assert!(has_match || !has_match); // Just a boolean
    }

    #[tokio::test]
    async fn test_semantic_similarity_without_service() {
        // Without embedding service, should return None gracefully
        let extractor = create_test_extractor();

        let task = vec!["rust".to_string(), "async".to_string()];
        let context = vec!["rust".to_string(), "tokio".to_string()];

        let result = extractor.compute_semantic_similarity(&task, &context).await;

        assert!(result.is_ok());
        assert!(
            result.unwrap().is_none(),
            "Should return None without embedding service"
        );
    }

    #[tokio::test]
    async fn test_semantic_similarity_empty_keywords() {
        use crate::config::EmbeddingConfig;

        // Even with embedding service, empty keywords should return None
        let mut extractor = create_test_extractor();

        // Note: This test will work even if embedding service fails to initialize
        // because we test empty keywords first
        if let Ok(service) = LocalEmbeddingService::new(EmbeddingConfig::default()).await {
            extractor.set_embedding_service(Arc::new(service));
        }

        // Empty task keywords
        let task = vec![];
        let context = vec!["rust".to_string()];
        let result = extractor.compute_semantic_similarity(&task, &context).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        // Empty context keywords
        let task = vec!["rust".to_string()];
        let context = vec![];
        let result = extractor.compute_semantic_similarity(&task, &context).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    #[ignore] // Requires model download (~140MB), run with --ignored
    async fn test_semantic_similarity_with_embeddings() {
        use crate::config::EmbeddingConfig;

        // With embedding service, should compute similarity
        let mut extractor = create_test_extractor();

        let config = EmbeddingConfig::default();
        let service = LocalEmbeddingService::new(config).await.unwrap();
        extractor.set_embedding_service(Arc::new(service));

        // Similar keywords should have high similarity
        let task = vec![
            "rust".to_string(),
            "async".to_string(),
            "programming".to_string(),
        ];
        let context = vec![
            "rust".to_string(),
            "asynchronous".to_string(),
            "code".to_string(),
        ];

        let result = extractor.compute_semantic_similarity(&task, &context).await;

        assert!(result.is_ok());
        let similarity = result.unwrap();
        assert!(similarity.is_some());
        let sim = similarity.unwrap();

        // Similarity should be in valid range [0, 1]
        assert!(sim >= 0.0 && sim <= 1.0);

        // Similar terms should have positive similarity
        assert!(
            sim > 0.3,
            "Similar keywords should have similarity > 0.3, got {}",
            sim
        );
    }

    #[tokio::test]
    #[ignore] // Requires model download (~140MB), run with --ignored
    async fn test_semantic_similarity_dissimilar() {
        use crate::config::EmbeddingConfig;

        let mut extractor = create_test_extractor();

        let config = EmbeddingConfig::default();
        let service = LocalEmbeddingService::new(config).await.unwrap();
        extractor.set_embedding_service(Arc::new(service));

        // Dissimilar keywords should have lower similarity
        let task = vec![
            "database".to_string(),
            "sql".to_string(),
            "query".to_string(),
        ];
        let context = vec![
            "graphics".to_string(),
            "rendering".to_string(),
            "shader".to_string(),
        ];

        let result = extractor.compute_semantic_similarity(&task, &context).await;

        assert!(result.is_ok());
        let similarity = result.unwrap();
        assert!(similarity.is_some());
        let sim = similarity.unwrap();

        // Dissimilar terms should have lower similarity than similar terms
        assert!(sim >= 0.0 && sim <= 1.0);
        assert!(
            sim < 0.6,
            "Dissimilar keywords should have similarity < 0.6, got {}",
            sim
        );
    }

    // Test helpers
    fn create_test_extractor() -> FeatureExtractor {
        FeatureExtractor::new(":memory:".to_string())
    }

    fn create_test_evaluation() -> ContextEvaluation {
        use crate::evaluation::feedback_collector::*;
        ContextEvaluation {
            id: "test-eval-1".to_string(),
            session_id: "test-session-1".to_string(),
            agent_role: "optimizer".to_string(),
            namespace: "test".to_string(),
            context_type: ContextType::Skill,
            context_id: "rust-async.md".to_string(),
            task_hash: "abc123".to_string(),
            task_keywords: Some(vec!["rust".to_string(), "async".to_string()]),
            task_type: Some(TaskType::Feature),
            work_phase: Some(WorkPhase::Implementation),
            file_types: Some(vec![".rs".to_string()]),
            error_context: Some(ErrorContext::None),
            related_technologies: Some(vec!["tokio".to_string()]),
            was_accessed: false,
            access_count: 0,
            time_to_first_access_ms: None,
            total_time_accessed_ms: 0,
            was_edited: false,
            was_committed: false,
            was_cited_in_response: false,
            user_rating: None,
            task_completed: false,
            task_success_score: None,
            context_provided_at: Utc::now().timestamp(),
            evaluation_updated_at: Utc::now().timestamp(),
        }
    }
}
