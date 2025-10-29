//! Evaluation System Integration
//!
//! Connects the orchestration system to Mnemosyne's evaluation system for
//! adaptive context relevance learning and quality metrics.
//!
//! # Integration Points
//!
//! ## 1. Optimizer Agent Uses Evaluation Scores
//!
//! The Optimizer agent uses evaluation metrics to:
//! - Prioritize high-relevance skills for loading
//! - Identify which memories to include in context
//! - Predict useful vs. distracting information
//!
//! ## 2. Quality Gates Use Evaluation Data
//!
//! The Reviewer agent uses evaluation metrics for:
//! - Validating code quality based on historical patterns
//! - Checking if similar work succeeded or failed
//! - Assessing risk of current approach
//!
//! ## 3. Feedback Loop from Orchestration
//!
//! Work completion generates evaluation feedback:
//! - Successful completion → Positive signal for used context
//! - Failed work → Negative signal, adjust priorities
//! - Time-to-completion → Efficiency metric
//!
//! # Architecture
//!
//! ```text
//! Orchestration Engine
//!   |
//!   +-- Optimizer Agent
//!   |     |
//!   |     +-- queries: evaluation.get_relevance_scores()
//!   |     +-- updates: evaluation.record_context_usage()
//!   |
//!   +-- Reviewer Agent
//!   |     |
//!   |     +-- queries: evaluation.get_quality_metrics()
//!   |
//!   +-- Event Sourcing
//!         |
//!         +-- triggers: evaluation.record_feedback()
//! ```
//!
//! # Privacy Considerations
//!
//! All evaluation data is:
//! - Stored locally in `.mnemosyne/project.db` (gitignored)
//! - Hashed for task descriptions (SHA256, 16 chars max)
//! - Statistical features only (no raw content)
//! - Optional (can be disabled)

use crate::error::Result;
use crate::orchestration::state::WorkItemId;

/// Evaluation system integration
pub struct EvaluationIntegration;

impl EvaluationIntegration {
    /// Record that a work item completed successfully
    ///
    /// This generates positive evaluation feedback for the context
    /// that was loaded for this work item.
    pub async fn record_work_success(
        _work_id: &WorkItemId,
        _duration_ms: u64,
    ) -> Result<()> {
        // Future: Record evaluation feedback
        // - Which skills were loaded?
        // - Which memories were accessed?
        // - How long did it take?
        // This signals that the loaded context was useful.
        Ok(())
    }

    /// Record that a work item failed
    ///
    /// This generates negative evaluation feedback, indicating
    /// the loaded context may not have been optimal.
    pub async fn record_work_failure(
        _work_id: &WorkItemId,
        _error: &str,
    ) -> Result<()> {
        // Future: Record negative feedback
        // - Which context was loaded but didn't help?
        // - What was missing that could have prevented failure?
        Ok(())
    }

    /// Get relevance scores for context selection
    ///
    /// Used by Optimizer agent to prioritize which skills/memories
    /// to load based on historical effectiveness.
    pub async fn get_context_relevance(
        _task_keywords: &[String],
    ) -> Result<Vec<(String, f32)>> {
        // Future: Query evaluation system
        // Returns: [(skill_name, relevance_score)]
        Ok(vec![])
    }

    /// Check if evaluation system is available
    pub fn is_available() -> bool {
        // Evaluation system is always available (graceful degradation)
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::state::WorkItemId;

    #[tokio::test]
    async fn test_record_work_success() {
        let work_id = WorkItemId::new();
        let result = EvaluationIntegration::record_work_success(&work_id, 100).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_record_work_failure() {
        let work_id = WorkItemId::new();
        let result = EvaluationIntegration::record_work_failure(&work_id, "Test error").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_get_context_relevance() {
        let keywords = vec!["rust".to_string(), "async".to_string()];
        let result = EvaluationIntegration::get_context_relevance(&keywords).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_evaluation_available() {
        assert!(EvaluationIntegration::is_available());
    }
}
