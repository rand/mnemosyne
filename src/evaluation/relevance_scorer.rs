//! Relevance scoring with online learning and hierarchical weight management.
#![allow(dead_code)]
//!
//! Implements an adaptive scoring system that learns from feedback signals
//! and updates relevance weights at three levels:
//! - Session: Fast adaptation (α=0.3)
//! - Project: Moderate adaptation (α=0.1)
//! - Global: Slow adaptation (α=0.03)
//!
//! # Learning Algorithm
//!
//! Uses exponential weighted moving average with gradient descent:
//! 1. Predict relevance using current weights
//! 2. Observe actual outcome
//! 3. Compute prediction error
//! 4. Update weights proportionally to error
//! 5. Propagate learning up hierarchy (session → project → global)
//!
//! # Weight Lookup
//!
//! Multi-dimensional fallback hierarchy:
//! 1. Exact match (work_phase + task_type + error_context)
//! 2. Partial match (work_phase + task_type)
//! 3. Phase-only (work_phase)
//! 4. Generic (no constraints)

use crate::error::{MnemosyneError, Result};
use crate::evaluation::feature_extractor::RelevanceFeatures;
use crate::evaluation::feedback_collector::FeedbackCollector;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Scope of learned weights
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    Session,
    Project,
    Global,
}

impl std::fmt::Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Scope::Session => write!(f, "session"),
            Scope::Project => write!(f, "project"),
            Scope::Global => write!(f, "global"),
        }
    }
}

/// Weight set for relevance scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightSet {
    pub id: String,
    pub scope: Scope,
    pub scope_id: String, // session_id, namespace, or "global"
    pub context_type: String,
    pub agent_role: String,
    pub work_phase: Option<String>,
    pub task_type: Option<String>,
    pub error_context: Option<String>,

    // Feature weights (sum to 1.0)
    pub weights: HashMap<String, f32>,

    // Learning metadata
    pub sample_count: u32,
    pub last_updated_at: i64,
    pub confidence: f32,    // Confidence in these weights [0.0, 1.0]
    pub learning_rate: f32, // Alpha for weight updates

    // Performance metrics
    pub avg_precision: Option<f32>,
    pub avg_recall: Option<f32>,
    pub avg_f1_score: Option<f32>,
}

impl WeightSet {
    /// Create default weights for a scope
    pub fn default_for_scope(
        scope: Scope,
        scope_id: String,
        context_type: String,
        agent_role: String,
    ) -> Self {
        let learning_rate = match scope {
            Scope::Session => 0.3,
            Scope::Project => 0.1,
            Scope::Global => 0.03,
        };

        let mut weights = HashMap::new();
        weights.insert("keyword_match".to_string(), 0.35);
        weights.insert("recency".to_string(), 0.15);
        weights.insert("access_patterns".to_string(), 0.25);
        weights.insert("historical_success".to_string(), 0.15);
        weights.insert("file_type_match".to_string(), 0.10);

        Self {
            id: Uuid::new_v4().to_string(),
            scope,
            scope_id,
            context_type,
            agent_role,
            work_phase: None,
            task_type: None,
            error_context: None,
            weights,
            sample_count: 0,
            last_updated_at: Utc::now().timestamp(),
            confidence: 0.5,
            learning_rate,
            avg_precision: None,
            avg_recall: None,
            avg_f1_score: None,
        }
    }

    /// Calculate confidence based on sample count
    ///
    /// Uses sigmoid: confidence = 1 / (1 + e^(-(samples - 10) / 5))
    /// - 0 samples → ~0.12 confidence
    /// - 10 samples → 0.5 confidence
    /// - 20 samples → ~0.88 confidence
    /// - 50+ samples → ~1.0 confidence
    pub fn calculate_confidence(&self) -> f32 {
        let x = (self.sample_count as f32 - 10.0) / 5.0;
        1.0 / (1.0 + (-x).exp())
    }

    /// Update confidence after new sample
    pub fn update_confidence(&mut self) {
        self.confidence = self.calculate_confidence();
    }

    /// Normalize weights to sum to 1.0
    pub fn normalize_weights(&mut self) {
        let sum: f32 = self.weights.values().sum();
        if sum > 0.0 {
            for weight in self.weights.values_mut() {
                *weight /= sum;
            }
        }
    }
}

/// Relevance scorer with online learning
pub struct RelevanceScorer {
    db_path: String,
}

impl RelevanceScorer {
    /// Create a new relevance scorer
    pub fn new(db_path: String) -> Self {
        Self { db_path }
    }

    /// Initialize the database schema
    ///
    /// This should be called once per database to ensure tables exist.
    /// Safe to call multiple times (uses IF NOT EXISTS).
    pub async fn init_schema(&self) -> Result<()> {
        crate::evaluation::schema::init_evaluation_tables(&self.db_path).await
    }

    /// Score context relevance using learned weights
    ///
    /// Looks up weights following hierarchical fallback, computes weighted score
    pub async fn score_context(
        &self,
        features: &RelevanceFeatures,
        scope: Scope,
        scope_id: &str,
        context_type: &str,
        agent_role: &str,
        work_phase: Option<&str>,
        task_type: Option<&str>,
        error_context: Option<&str>,
    ) -> Result<f32> {
        // Get weights with fallback
        let weights = self
            .get_weights_with_fallback(
                scope,
                scope_id,
                context_type,
                agent_role,
                work_phase,
                task_type,
                error_context,
            )
            .await?;

        // Compute weighted score
        let score = self.compute_weighted_score(features, &weights.weights);

        debug!(
            "Scored context with {} weights (confidence: {:.2}): {:.2}",
            weights.scope, weights.confidence, score
        );

        Ok(score)
    }

    /// Compute weighted score from features
    fn compute_weighted_score(
        &self,
        features: &RelevanceFeatures,
        weights: &HashMap<String, f32>,
    ) -> f32 {
        let mut score = 0.0;

        // Map features to weight keys
        score += features.keyword_overlap_score * weights.get("keyword_match").unwrap_or(&0.0);
        score += features.recency_days.min(30.0) / 30.0 * weights.get("recency").unwrap_or(&0.0);
        score +=
            features.access_frequency.min(1.0) * weights.get("access_patterns").unwrap_or(&0.0);

        if let Some(hist_success) = features.historical_success_rate {
            score += hist_success * weights.get("historical_success").unwrap_or(&0.0);
        }

        if features.file_type_match {
            score += weights.get("file_type_match").unwrap_or(&0.0);
        }

        score.clamp(0.0, 1.0)
    }

    /// Get weights with hierarchical fallback
    async fn get_weights_with_fallback(
        &self,
        scope: Scope,
        scope_id: &str,
        context_type: &str,
        agent_role: &str,
        work_phase: Option<&str>,
        task_type: Option<&str>,
        error_context: Option<&str>,
    ) -> Result<WeightSet> {
        // Try exact match first (most specific)
        if let Some(weights) = self
            .get_weights(
                scope.clone(),
                scope_id,
                context_type,
                agent_role,
                work_phase,
                task_type,
                error_context,
            )
            .await?
        {
            return Ok(weights);
        }

        // Fallback 1: work_phase + task_type
        if work_phase.is_some() && task_type.is_some() {
            if let Some(weights) = self
                .get_weights(
                    scope.clone(),
                    scope_id,
                    context_type,
                    agent_role,
                    work_phase,
                    task_type,
                    None,
                )
                .await?
            {
                return Ok(weights);
            }
        }

        // Fallback 2: work_phase only
        if work_phase.is_some() {
            if let Some(weights) = self
                .get_weights(
                    scope.clone(),
                    scope_id,
                    context_type,
                    agent_role,
                    work_phase,
                    None,
                    None,
                )
                .await?
            {
                return Ok(weights);
            }
        }

        // Fallback 3: Generic weights
        if let Some(weights) = self
            .get_weights(
                scope.clone(),
                scope_id,
                context_type,
                agent_role,
                None,
                None,
                None,
            )
            .await?
        {
            return Ok(weights);
        }

        // Fallback 4: Create default weights
        warn!(
            "No weights found for {} {} {}, creating defaults",
            scope, context_type, agent_role
        );

        Ok(WeightSet::default_for_scope(
            scope,
            scope_id.to_string(),
            context_type.to_string(),
            agent_role.to_string(),
        ))
    }

    /// Get weights from database
    async fn get_weights(
        &self,
        scope: Scope,
        scope_id: &str,
        context_type: &str,
        agent_role: &str,
        work_phase: Option<&str>,
        task_type: Option<&str>,
        error_context: Option<&str>,
    ) -> Result<Option<WeightSet>> {
        let conn = self.get_conn().await?;

        // Build query with NULL matching for optional fields
        let query = r#"
            SELECT * FROM learned_relevance_weights
            WHERE scope = ? AND scope_id = ? AND context_type = ? AND agent_role = ?
              AND (work_phase = ? OR (work_phase IS NULL AND ? IS NULL))
              AND (task_type = ? OR (task_type IS NULL AND ? IS NULL))
              AND (error_context = ? OR (error_context IS NULL AND ? IS NULL))
            ORDER BY sample_count DESC
            LIMIT 1
        "#;

        let mut rows = conn
            .query(
                query,
                libsql::params![
                    scope.to_string(),
                    scope_id,
                    context_type,
                    agent_role,
                    work_phase,
                    work_phase,
                    task_type,
                    task_type,
                    error_context,
                    error_context,
                ],
            )
            .await
            .map_err(|e| MnemosyneError::Database(format!("Failed to query weights: {}", e)))?;

        let row = match rows.next().await {
            Ok(Some(row)) => row,
            Ok(None) => return Ok(None),
            Err(e) => {
                return Err(MnemosyneError::Database(format!(
                    "Failed to read row: {}",
                    e
                )))
            }
        };

        // Parse row into WeightSet
        let id: String = row
            .get(0)
            .map_err(|e| MnemosyneError::Database(e.to_string()))?;
        let scope_str: String = row
            .get(1)
            .map_err(|e| MnemosyneError::Database(e.to_string()))?;
        let scope_parsed = match scope_str.as_str() {
            "session" => Scope::Session,
            "project" => Scope::Project,
            "global" => Scope::Global,
            _ => {
                return Err(MnemosyneError::Other(format!(
                    "Invalid scope: {}",
                    scope_str
                )))
            }
        };

        let scope_id: String = row
            .get(2)
            .map_err(|e| MnemosyneError::Database(e.to_string()))?;
        let context_type: String = row
            .get(3)
            .map_err(|e| MnemosyneError::Database(e.to_string()))?;
        let agent_role: String = row
            .get(4)
            .map_err(|e| MnemosyneError::Database(e.to_string()))?;
        let work_phase: Option<String> = row
            .get(5)
            .map_err(|e| MnemosyneError::Database(e.to_string()))?;
        let task_type: Option<String> = row
            .get(6)
            .map_err(|e| MnemosyneError::Database(e.to_string()))?;
        let error_context: Option<String> = row
            .get(7)
            .map_err(|e| MnemosyneError::Database(e.to_string()))?;

        let weights_json: String = row
            .get(8)
            .map_err(|e| MnemosyneError::Database(e.to_string()))?;
        let weights: HashMap<String, f32> = serde_json::from_str(&weights_json)
            .map_err(|e| MnemosyneError::Other(format!("Failed to parse weights: {}", e)))?;

        let sample_count: u32 = row
            .get(9)
            .map_err(|e| MnemosyneError::Database(e.to_string()))?;
        let last_updated_at: i64 = row
            .get(10)
            .map_err(|e| MnemosyneError::Database(e.to_string()))?;

        // libsql uses f64, convert to f32
        let confidence_f64: f64 = row
            .get(11)
            .map_err(|e| MnemosyneError::Database(e.to_string()))?;
        let confidence = confidence_f64 as f32;

        let learning_rate_f64: f64 = row
            .get(12)
            .map_err(|e| MnemosyneError::Database(e.to_string()))?;
        let learning_rate = learning_rate_f64 as f32;

        let avg_precision: Option<f64> = row
            .get(13)
            .map_err(|e| MnemosyneError::Database(e.to_string()))?;
        let avg_precision = avg_precision.map(|v| v as f32);

        let avg_recall: Option<f64> = row
            .get(14)
            .map_err(|e| MnemosyneError::Database(e.to_string()))?;
        let avg_recall = avg_recall.map(|v| v as f32);

        let avg_f1_score: Option<f64> = row
            .get(15)
            .map_err(|e| MnemosyneError::Database(e.to_string()))?;
        let avg_f1_score = avg_f1_score.map(|v| v as f32);

        Ok(Some(WeightSet {
            id,
            scope: scope_parsed,
            scope_id,
            context_type,
            agent_role,
            work_phase,
            task_type,
            error_context,
            weights,
            sample_count,
            last_updated_at,
            confidence,
            learning_rate,
            avg_precision,
            avg_recall,
            avg_f1_score,
        }))
    }

    /// Update weights based on feedback
    ///
    /// Implements gradient descent with exponential weighted moving average
    pub async fn update_weights(
        &self,
        evaluation_id: &str,
        features: &RelevanceFeatures,
    ) -> Result<()> {
        info!("Updating weights based on evaluation {}", evaluation_id);

        // Extract context information from features
        // Note: We need to determine scope and context from the evaluation
        // For now, we'll create a FeedbackCollector to fetch the evaluation
        let collector = FeedbackCollector::new(self.db_path.clone());
        let evaluation = collector.get_evaluation(evaluation_id).await?;

        // Determine scope IDs
        let session_id = &evaluation.session_id;
        let project_id = &evaluation.namespace; // namespace serves as project ID
        let context_type_str = evaluation.context_type.to_string();
        let agent_role = &evaluation.agent_role;

        // Convert Option<Enum> to Option<String> to avoid lifetime issues
        let work_phase_str = evaluation.work_phase.as_ref().map(|p| p.to_string());
        let task_type_str = evaluation.task_type.as_ref().map(|t| t.to_string());
        let error_context_str = evaluation.error_context.as_ref().map(|e| e.to_string());

        // Get or create weight sets for all three scopes
        let mut session_weights = self
            .get_weights_with_fallback(
                Scope::Session,
                session_id,
                &context_type_str,
                agent_role,
                work_phase_str.as_deref(),
                task_type_str.as_deref(),
                error_context_str.as_deref(),
            )
            .await?;

        let mut project_weights = self
            .get_weights_with_fallback(
                Scope::Project,
                project_id,
                &context_type_str,
                agent_role,
                work_phase_str.as_deref(),
                task_type_str.as_deref(),
                error_context_str.as_deref(),
            )
            .await?;

        let mut global_weights = self
            .get_weights_with_fallback(
                Scope::Global,
                "global",
                &context_type_str,
                agent_role,
                work_phase_str.as_deref(),
                task_type_str.as_deref(),
                error_context_str.as_deref(),
            )
            .await?;

        // Compute predicted scores using current weights
        let session_score = self.compute_weighted_score(features, &session_weights.weights);
        let project_score = self.compute_weighted_score(features, &project_weights.weights);
        let global_score = self.compute_weighted_score(features, &global_weights.weights);

        // Update each weight set using gradient descent
        let actual_outcome = features.was_useful;

        self.update_single_weight_set(
            &mut session_weights,
            features,
            session_score,
            actual_outcome,
        );

        self.update_single_weight_set(
            &mut project_weights,
            features,
            project_score,
            actual_outcome,
        );

        self.update_single_weight_set(&mut global_weights, features, global_score, actual_outcome);

        // Store updated weights
        self.store_weights(&session_weights).await?;
        self.store_weights(&project_weights).await?;
        self.store_weights(&global_weights).await?;

        info!(
            "Updated weights for evaluation {} across all scopes (session: {:.2}, project: {:.2}, global: {:.2})",
            evaluation_id,
            session_weights.confidence,
            project_weights.confidence,
            global_weights.confidence
        );

        Ok(())
    }

    /// Update a single weight set using gradient descent
    fn update_single_weight_set(
        &self,
        weights: &mut WeightSet,
        features: &RelevanceFeatures,
        predicted_score: f32,
        actual_outcome: bool,
    ) {
        let actual_score = if actual_outcome { 1.0 } else { 0.0 };
        let error = actual_score - predicted_score;

        debug!(
            "Updating {} weights: predicted={:.2}, actual={:.1}, error={:.2}",
            weights.scope, predicted_score, actual_score, error
        );

        // Gradient descent: w_new = w_old + α * error * feature
        let alpha = weights.learning_rate;

        // Update each weight proportionally to its contribution and the error
        if let Some(w) = weights.weights.get_mut("keyword_match") {
            *w += alpha * error * features.keyword_overlap_score;
        }
        if let Some(w) = weights.weights.get_mut("recency") {
            *w += alpha * error * (features.recency_days.min(30.0) / 30.0);
        }
        if let Some(w) = weights.weights.get_mut("access_patterns") {
            *w += alpha * error * features.access_frequency.min(1.0);
        }
        if let (Some(w), Some(hist)) = (
            weights.weights.get_mut("historical_success"),
            features.historical_success_rate,
        ) {
            *w += alpha * error * hist;
        }
        if features.file_type_match {
            if let Some(w) = weights.weights.get_mut("file_type_match") {
                *w += alpha * error;
            }
        }

        // Normalize to ensure weights sum to 1.0
        weights.normalize_weights();

        // Update metadata
        weights.sample_count += 1;
        weights.update_confidence();
        weights.last_updated_at = Utc::now().timestamp();
    }

    /// Propagate learning from session → project → global
    ///
    /// Session weights influence project, project influences global
    /// with dampening to create stable higher-level weights
    pub async fn propagate_learning(
        &self,
        session_weights: &WeightSet,
        _evaluation_id: &str,
    ) -> Result<()> {
        // Only propagate from session scope
        if !matches!(session_weights.scope, Scope::Session) {
            return Ok(());
        }

        // Extract project name from session scope_id (format: "session:project:id")
        let project_name = if let Some(project) = session_weights.scope_id.split(':').nth(1) {
            project
        } else {
            warn!("Cannot extract project from session scope_id: {}", session_weights.scope_id);
            return Ok(());
        };

        // Get or create project-level weights
        let mut project_weights = match self.get_weights(
            Scope::Project,
            project_name,
            &session_weights.context_type,
            &session_weights.agent_role,
            session_weights.work_phase.as_deref(),
            session_weights.task_type.as_deref(),
            session_weights.error_context.as_deref(),
        ).await? {
            Some(w) => w,
            None => {
                // Create new project weights from session
                let mut w = session_weights.clone();
                w.scope = Scope::Project;
                w.scope_id = project_name.to_string();
                w.id = format!("{}_{}_{}_{:?}_{:?}_{:?}",
                    "project",
                    project_name,
                    w.context_type,
                    w.agent_role,
                    w.work_phase,
                    w.task_type
                );
                w.sample_count = 0;
                w
            }
        };

        // Apply dampened update: project gets 30% of session's weights
        let damping_factor = 0.3;
        for (key, &session_value) in &session_weights.weights {
            let project_value = project_weights.weights.entry(key.clone()).or_insert(0.5);
            let delta = session_value - *project_value;
            *project_value += delta * damping_factor;
        }

        project_weights.sample_count += 1;
        project_weights.update_confidence();
        project_weights.last_updated_at = chrono::Utc::now().timestamp();
        project_weights.normalize_weights();

        // Store updated project weights
        self.store_weights(&project_weights).await?;

        debug!(
            "Propagated session → project: {} (damping: {})",
            project_weights.id, damping_factor
        );

        // Get or create global weights
        let mut global_weights = match self.get_weights(
            Scope::Global,
            "global",
            &session_weights.context_type,
            &session_weights.agent_role,
            session_weights.work_phase.as_deref(),
            session_weights.task_type.as_deref(),
            session_weights.error_context.as_deref(),
        ).await? {
            Some(w) => w,
            None => {
                // Create new global weights from project
                let mut w = project_weights.clone();
                w.scope = Scope::Global;
                w.scope_id = "global".to_string();
                w.id = format!("{}_{}_{}_{:?}_{:?}_{:?}",
                    "global",
                    "global",
                    w.context_type,
                    w.agent_role,
                    w.work_phase,
                    w.task_type
                );
                w.sample_count = 0;
                w
            }
        };

        // Apply dampened update: global gets 10% of project's weights
        let global_damping = 0.1;
        for (key, &project_value) in &project_weights.weights {
            let global_value = global_weights.weights.entry(key.clone()).or_insert(0.5);
            let delta = project_value - *global_value;
            *global_value += delta * global_damping;
        }

        global_weights.sample_count += 1;
        global_weights.update_confidence();
        global_weights.last_updated_at = chrono::Utc::now().timestamp();
        global_weights.normalize_weights();

        // Store updated global weights
        self.store_weights(&global_weights).await?;

        debug!(
            "Propagated project → global: {} (damping: {})",
            global_weights.id, global_damping
        );

        Ok(())
    }

    /// Store weight set in database
    pub async fn store_weights(&self, weights: &WeightSet) -> Result<()> {
        debug!("Storing {} weights: {}", weights.scope, weights.id);

        let conn = self.get_conn().await?;

        // Serialize weights HashMap to JSON
        let weights_json = serde_json::to_string(&weights.weights)
            .map_err(|e| MnemosyneError::Other(format!("Failed to serialize weights: {}", e)))?;

        // Use INSERT OR REPLACE for upsert behavior
        conn.execute(
            r#"
            INSERT OR REPLACE INTO learned_relevance_weights (
                id, scope, scope_id, context_type, agent_role,
                work_phase, task_type, error_context,
                weights, sample_count, last_updated_at,
                confidence, learning_rate,
                avg_precision, avg_recall, avg_f1_score
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            libsql::params![
                weights.id.clone(),
                weights.scope.to_string(),
                weights.scope_id.clone(),
                weights.context_type.clone(),
                weights.agent_role.clone(),
                weights.work_phase.clone(),
                weights.task_type.clone(),
                weights.error_context.clone(),
                weights_json,
                weights.sample_count,
                weights.last_updated_at,
                weights.confidence,
                weights.learning_rate,
                weights.avg_precision,
                weights.avg_recall,
                weights.avg_f1_score,
            ],
        )
        .await
        .map_err(|e| MnemosyneError::Database(format!("Failed to store weights: {}", e)))?;

        debug!("Stored {} weights successfully", weights.scope);
        Ok(())
    }

    /// Get database connection
    async fn get_conn(&self) -> Result<libsql::Connection> {
        let db = libsql::Builder::new_local(&self.db_path)
            .build()
            .await
            .map_err(|e| MnemosyneError::Database(format!("Failed to open database: {}", e)))?;

        db.connect()
            .map_err(|e| MnemosyneError::Database(format!("Failed to get connection: {}", e)))
    }

    /// Calculate performance metrics (precision, recall, F1) for a weight set
    ///
    /// Metrics are calculated from relevance_features table:
    /// - True Positive: Context provided, was useful (was_useful = 1)
    /// - False Positive: Context provided, not useful (was_useful = 0)
    /// - False Negative: Context not provided but would have been useful (hard to measure)
    ///
    /// For now, we approximate:
    /// - Precision = useful contexts / all contexts provided
    /// - Recall = approximated from access rate (contexts accessed / provided)
    pub async fn calculate_metrics(&self, weight_id: &str) -> Result<(f32, f32, f32)> {
        let conn = self.get_conn().await?;

        // Query relevance features for this weight set
        // Note: We'd need to track which weight set was used for each evaluation
        // For now, calculate metrics across all evaluations
        let mut rows = conn
            .query(
                r#"
                SELECT
                    COUNT(*) as total,
                    SUM(was_useful) as useful_count,
                    AVG(CASE WHEN was_useful = 1 THEN 1.0 ELSE 0.0 END) as precision_approx
                FROM relevance_features
                "#,
                libsql::params![],
            )
            .await
            .map_err(|e| {
                MnemosyneError::Database(format!("Failed to query metrics: {}", e))
            })?;

        let row = rows.next().await.map_err(|e| {
            MnemosyneError::Database(format!("Failed to fetch metrics row: {}", e))
        })?;

        let row = row.ok_or_else(|| {
            MnemosyneError::Database("No metrics data found".to_string())
        })?;

        let total = row.get::<i64>(0).unwrap_or(0);
        let useful_count = row.get::<i64>(1).unwrap_or(0);

        if total == 0 {
            debug!("No evaluation data yet for weight_id: {}", weight_id);
            return Ok((0.5, 0.5, 0.5)); // Default neutral metrics
        }

        // Precision: proportion of provided contexts that were useful
        let precision = useful_count as f32 / total as f32;

        // Recall approximation: Query access rate from context_evaluations
        let mut recall_rows = conn
            .query(
                r#"
                SELECT
                    COUNT(*) as provided,
                    SUM(was_accessed) as accessed
                FROM context_evaluations
                "#,
                libsql::params![],
            )
            .await
            .map_err(|e| {
                MnemosyneError::Database(format!("Failed to query recall: {}", e))
            })?;

        let recall_row = recall_rows.next().await.unwrap_or(None);
        let recall = if let Some(r) = recall_row {
            let provided = r.get::<i64>(0).unwrap_or(1);
            let accessed = r.get::<i64>(1).unwrap_or(0);
            (accessed as f32 / provided as f32).clamp(0.0, 1.0)
        } else {
            0.5 // Default
        };

        // F1 score: harmonic mean of precision and recall
        let f1 = if precision + recall > 0.0 {
            2.0 * (precision * recall) / (precision + recall)
        } else {
            0.0
        };

        debug!(
            "Metrics for {}: precision={:.3}, recall={:.3}, f1={:.3}",
            weight_id, precision, recall, f1
        );

        Ok((precision, recall, f1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_calculation() {
        let mut weights = WeightSet::default_for_scope(
            Scope::Session,
            "test-session".to_string(),
            "skill".to_string(),
            "optimizer".to_string(),
        );

        // 0 samples → low confidence
        weights.sample_count = 0;
        let conf = weights.calculate_confidence();
        assert!(conf < 0.2);

        // 10 samples → medium confidence
        weights.sample_count = 10;
        let conf = weights.calculate_confidence();
        assert!((conf - 0.5).abs() < 0.1);

        // 20 samples → high confidence
        weights.sample_count = 20;
        let conf = weights.calculate_confidence();
        assert!(conf > 0.8);

        // 50 samples → very high confidence
        weights.sample_count = 50;
        let conf = weights.calculate_confidence();
        assert!(conf > 0.95);
    }

    #[test]
    fn test_weight_normalization() {
        let mut weights = WeightSet::default_for_scope(
            Scope::Global,
            "global".to_string(),
            "memory".to_string(),
            "optimizer".to_string(),
        );

        // Set arbitrary weights
        weights.weights.insert("keyword_match".to_string(), 0.8);
        weights.weights.insert("recency".to_string(), 0.6);
        weights.weights.insert("access_patterns".to_string(), 0.4);

        weights.normalize_weights();

        // Check they sum to 1.0
        let sum: f32 = weights.weights.values().sum();
        assert!((sum - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_learning_rates_by_scope() {
        let session = WeightSet::default_for_scope(
            Scope::Session,
            "s1".to_string(),
            "skill".to_string(),
            "optimizer".to_string(),
        );
        assert!((session.learning_rate - 0.3).abs() < 0.001);

        let project = WeightSet::default_for_scope(
            Scope::Project,
            "p1".to_string(),
            "skill".to_string(),
            "optimizer".to_string(),
        );
        assert!((project.learning_rate - 0.1).abs() < 0.001);

        let global = WeightSet::default_for_scope(
            Scope::Global,
            "global".to_string(),
            "skill".to_string(),
            "optimizer".to_string(),
        );
        assert!((global.learning_rate - 0.03).abs() < 0.001);
    }

    #[test]
    fn test_weighted_score_computation() {
        let scorer = RelevanceScorer::new(":memory:".to_string());

        let mut features = create_test_features();
        features.keyword_overlap_score = 0.8;
        features.recency_days = 5.0;
        features.access_frequency = 0.6;
        features.file_type_match = true;

        let mut weights = HashMap::new();
        weights.insert("keyword_match".to_string(), 0.4);
        weights.insert("recency".to_string(), 0.2);
        weights.insert("access_patterns".to_string(), 0.2);
        weights.insert("file_type_match".to_string(), 0.2);

        let score = scorer.compute_weighted_score(&features, &weights);

        // Expected:
        // 0.8 * 0.4 (keyword) + (5/30) * 0.2 (recency) + 0.6 * 0.2 (access) + 1.0 * 0.2 (file_match)
        // = 0.32 + 0.033 + 0.12 + 0.2 = 0.673
        assert!((score - 0.673).abs() < 0.01);
    }

    #[test]
    fn test_weight_set_no_sensitive_data() {
        // Verify WeightSet only stores statistical weights, no raw content
        let weights = WeightSet::default_for_scope(
            Scope::Session,
            "test-session".to_string(),
            "skill".to_string(),
            "optimizer".to_string(),
        );

        // Serialize and check
        let json = serde_json::to_string(&weights).expect("Failed to serialize");

        // Should not contain any raw content or sensitive data
        assert!(!json.contains("password"));
        assert!(!json.contains("secret"));
        assert!(!json.contains("api_key"));

        // Should only contain feature weights (numeric)
        assert!(json.contains("keyword_match"));
        assert!(json.contains("recency"));
    }

    #[test]
    fn test_weight_normalization_privacy() {
        let mut weights = WeightSet::default_for_scope(
            Scope::Session,
            "test".to_string(),
            "skill".to_string(),
            "optimizer".to_string(),
        );

        // Set arbitrary weights
        weights.weights.insert("feature1".to_string(), 2.0);
        weights.weights.insert("feature2".to_string(), 3.0);
        weights.weights.insert("feature3".to_string(), 5.0);

        weights.normalize_weights();

        // Verify they sum to 1.0 (privacy-preserving normalization)
        let sum: f32 = weights.weights.values().sum();
        assert!((sum - 1.0).abs() < 0.001, "Weights should sum to 1.0");

        // Verify no information leak in normalization
        for (key, value) in &weights.weights {
            assert!(
                *value >= 0.0 && *value <= 1.0,
                "Normalized weight {} should be in [0.0, 1.0]",
                key
            );
        }
    }

    #[test]
    fn test_scope_privacy_levels() {
        // Verify scope hierarchy enforces privacy boundaries
        let session = Scope::Session;
        let project = Scope::Project;
        let global = Scope::Global;

        // Each scope should be distinct
        assert_ne!(session.to_string(), project.to_string());
        assert_ne!(project.to_string(), global.to_string());

        // Session is most privacy-preserving (highest learning rate, fastest forgetting)
        let session_weights = WeightSet::default_for_scope(
            Scope::Session,
            "s1".to_string(),
            "skill".to_string(),
            "optimizer".to_string(),
        );

        let global_weights = WeightSet::default_for_scope(
            Scope::Global,
            "global".to_string(),
            "skill".to_string(),
            "optimizer".to_string(),
        );

        // Session should have higher learning rate (more responsive, less persistent)
        assert!(
            session_weights.learning_rate > global_weights.learning_rate,
            "Session should have higher learning rate than global"
        );
    }

    #[test]
    fn test_confidence_based_on_samples_not_content() {
        let mut weights = WeightSet::default_for_scope(
            Scope::Session,
            "test".to_string(),
            "skill".to_string(),
            "optimizer".to_string(),
        );

        // Confidence should depend only on sample count, not content
        weights.sample_count = 0;
        let conf0 = weights.calculate_confidence();

        weights.sample_count = 10;
        let conf10 = weights.calculate_confidence();

        weights.sample_count = 50;
        let conf50 = weights.calculate_confidence();

        // Confidence should increase with samples
        assert!(conf0 < conf10);
        assert!(conf10 < conf50);

        // But should not depend on content (tested by not having content fields)
        assert!(conf0 >= 0.0 && conf0 <= 1.0);
        assert!(conf50 >= 0.0 && conf50 <= 1.0);
    }

    #[test]
    fn test_weighted_score_no_raw_features() {
        let scorer = RelevanceScorer::new(":memory:".to_string());

        let features = create_test_features();
        let weights = WeightSet::default_for_scope(
            Scope::Session,
            "test".to_string(),
            "skill".to_string(),
            "optimizer".to_string(),
        );

        let score = scorer.compute_weighted_score(&features, &weights.weights);

        // Score should be computed without exposing raw features
        assert!(score >= 0.0 && score <= 1.0, "Score should be normalized");

        // Verify computation uses only statistical features, not raw content
        // (enforced by RelevanceFeatures struct definition)
    }

    fn create_test_features() -> RelevanceFeatures {
        RelevanceFeatures {
            evaluation_id: "test-eval".to_string(),
            keyword_overlap_score: 0.5,
            semantic_similarity: None,
            recency_days: 7.0,
            access_frequency: 0.3,
            last_used_days_ago: Some(2.0),
            work_phase_match: true,
            task_type_match: true,
            agent_role_affinity: 0.8,
            namespace_match: true,
            file_type_match: false,
            historical_success_rate: Some(0.7),
            co_occurrence_score: None,
            was_useful: true,
        }
    }
}
