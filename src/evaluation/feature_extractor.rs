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

use crate::error::{MnemosyneError, Result};
use crate::evaluation::feedback_collector::{ContextEvaluation, ContextType};
use crate::storage::StorageBackend;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use tracing::debug;

/// Privacy-preserving relevance features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelevanceFeatures {
    pub evaluation_id: String,

    // Statistical features (privacy-preserving)
    pub keyword_overlap_score: f32,  // Jaccard similarity [0.0, 1.0]
    pub semantic_similarity: Option<f32>,  // Cosine similarity if embeddings available
    pub recency_days: f32,  // Days since context was created
    pub access_frequency: f32,  // Accesses per day
    pub last_used_days_ago: Option<f32>,  // Days since last access

    // Contextual match features
    pub work_phase_match: bool,
    pub task_type_match: bool,
    pub agent_role_affinity: f32,  // How well this context suits this agent
    pub namespace_match: bool,
    pub file_type_match: bool,

    // Historical performance features
    pub historical_success_rate: Option<f32>,  // Past success rate for this context
    pub co_occurrence_score: Option<f32>,  // How often it appears with other useful contexts

    // Ground truth (outcome)
    pub was_useful: bool,  // Did user actually use this context?
}

/// Feature extractor
pub struct FeatureExtractor {
    storage: Arc<dyn StorageBackend>,
}

impl FeatureExtractor {
    /// Create a new feature extractor
    pub fn new(storage: Arc<dyn StorageBackend>) -> Self {
        Self { storage }
    }

    /// Extract features from an evaluation
    ///
    /// This is called after enough feedback signals have been collected
    /// to determine if the context was useful.
    pub async fn extract_features(
        &self,
        evaluation: &ContextEvaluation,
        context_keywords: &[String],  // Keywords from the actual context (skill/memory/file)
    ) -> Result<RelevanceFeatures> {
        debug!("Extracting features for evaluation {}", evaluation.id);

        // Keyword overlap (privacy-preserving: compute score, discard keywords)
        let keyword_overlap_score = self.compute_keyword_overlap(
            &evaluation.task_keywords.clone().unwrap_or_default(),
            context_keywords,
        );

        // Semantic similarity (if embeddings available)
        let semantic_similarity = None; // TODO: Implement when embeddings ready

        // Recency features
        let recency_days = self.compute_recency_days(&evaluation.context_id, &evaluation.context_type).await?;
        let access_frequency = self.compute_access_frequency(&evaluation.context_id, &evaluation.context_type).await?;
        let last_used_days_ago = self.compute_last_used_days(&evaluation.context_id, &evaluation.context_type).await?;

        // Contextual match features
        let work_phase_match = evaluation.work_phase.is_some();
        let task_type_match = evaluation.task_type.is_some();
        let namespace_match = true; // TODO: Check if context namespace matches task namespace
        let file_type_match = self.compute_file_type_match(evaluation);

        // Agent affinity (how well this context type suits this agent)
        let agent_role_affinity = self.compute_agent_affinity(
            &evaluation.agent_role,
            &evaluation.context_type,
        );

        // Historical features
        let historical_success_rate = self.compute_historical_success(
            &evaluation.context_id,
            &evaluation.context_type,
        ).await?;

        let co_occurrence_score = self.compute_co_occurrence(
            &evaluation.context_id,
            &evaluation.session_id,
        ).await?;

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
    fn compute_keyword_overlap(&self, task_keywords: &[String], context_keywords: &[String]) -> f32 {
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

    /// Compute recency (days since context was created)
    async fn compute_recency_days(&self, context_id: &str, context_type: &ContextType) -> Result<f32> {
        match context_type {
            ContextType::Memory => {
                // Fetch memory creation date
                // TODO: Implement memory lookup
                Ok(7.0) // Placeholder
            }
            ContextType::Skill | ContextType::File => {
                // For skills/files, use file modification time
                // TODO: Implement file stat lookup
                Ok(30.0) // Placeholder
            }
            _ => Ok(0.0),
        }
    }

    /// Compute access frequency (accesses per day)
    async fn compute_access_frequency(&self, context_id: &str, context_type: &ContextType) -> Result<f32> {
        match context_type {
            ContextType::Memory => {
                // TODO: Fetch memory access count and age, compute frequency
                Ok(0.5) // Placeholder
            }
            _ => Ok(0.0),
        }
    }

    /// Compute days since last use
    async fn compute_last_used_days(&self, context_id: &str, context_type: &ContextType) -> Result<Option<f32>> {
        match context_type {
            ContextType::Memory => {
                // TODO: Fetch last_accessed_at from memory
                Ok(Some(3.0)) // Placeholder
            }
            _ => Ok(None),
        }
    }

    /// Compute file type match
    fn compute_file_type_match(&self, evaluation: &ContextEvaluation) -> bool {
        // Check if context involves files matching task file types
        if let (Some(task_files), ContextType::File) = (&evaluation.file_types, &evaluation.context_type) {
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

    /// Compute historical success rate for this context
    async fn compute_historical_success(&self, context_id: &str, context_type: &ContextType) -> Result<Option<f32>> {
        // Query past evaluations for this context
        // Calculate: (times_accessed / times_provided)
        // TODO: Implement historical query
        Ok(None) // Placeholder - will implement with database queries
    }

    /// Compute co-occurrence score
    ///
    /// How often does this context appear alongside other useful contexts?
    async fn compute_co_occurrence(&self, context_id: &str, session_id: &str) -> Result<Option<f32>> {
        // Query other contexts in this session
        // Calculate: (useful_cooccurrences / total_cooccurrences)
        // TODO: Implement co-occurrence tracking
        Ok(None) // Placeholder
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
            && (evaluation.was_edited || evaluation.was_committed || evaluation.was_cited_in_response);

        // Strong signal: accessed multiple times
        let frequently_accessed = evaluation.access_count >= 2;

        // Context was useful if it was used or frequently accessed
        was_used || frequently_accessed
    }

    /// Store features in database
    pub async fn store_features(&self, features: &RelevanceFeatures) -> Result<()> {
        debug!("Storing features for evaluation {}", features.evaluation_id);

        // TODO: Implement database insert
        // INSERT INTO relevance_features (evaluation_id, keyword_overlap_score, ...)
        // VALUES (?, ?, ...)

        Ok(())
    }

    /// Get features for an evaluation
    pub async fn get_features(&self, evaluation_id: &str) -> Result<RelevanceFeatures> {
        // TODO: Implement database query
        Err(MnemosyneError::Other("Feature retrieval not yet implemented".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    // Test helpers
    fn create_test_extractor() -> FeatureExtractor {
        use crate::storage::libsql::LibsqlStorage;
        let storage = Arc::new(LibsqlStorage::new(
            crate::storage::libsql::ConnectionMode::InMemory
        ).await.unwrap());
        FeatureExtractor::new(storage)
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
