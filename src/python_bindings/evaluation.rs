//! Python bindings for the evaluation system.
//!
//! Provides Python access to:
//! - FeedbackCollector: Record implicit feedback signals
//! - FeatureExtractor: Extract privacy-preserving features
//! - RelevanceScorer: Score context and update weights

use crate::evaluation::feedback_collector::{
    ContextType, ErrorContext, ProvidedContext, TaskType, WorkPhase,
};
use crate::evaluation::relevance_scorer::Scope;
use crate::evaluation::{FeatureExtractor, FeedbackCollector, RelevanceScorer};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::PyDict;

/// Python wrapper for FeedbackCollector
#[pyclass(name = "FeedbackCollector")]
pub struct PyFeedbackCollector {
    collector: FeedbackCollector,
}

#[pymethods]
impl PyFeedbackCollector {
    /// Create a new feedback collector
    #[new]
    fn new(db_path: String) -> PyResult<Self> {
        let collector = FeedbackCollector::new(db_path);
        Ok(Self { collector })
    }

    /// Record context provided
    ///
    /// Args:
    ///     session_id: Session ID
    ///     agent_role: Agent role (optimizer, executor, etc.)
    ///     namespace: Namespace
    ///     context_type: Type of context (skill, memory, file, commit, plan)
    ///     context_id: ID of the context
    ///     task_hash: Privacy-preserving task hash (max 16 chars)
    ///     task_keywords: Optional list of generic keywords
    ///     task_type: Optional task type
    ///     work_phase: Optional work phase
    ///     file_types: Optional list of file type patterns
    ///     error_context: Optional error context
    ///     related_technologies: Optional list of technologies
    ///
    /// Returns:
    ///     Evaluation ID for tracking
    fn record_context_provided(
        &self,
        session_id: String,
        agent_role: String,
        namespace: String,
        context_type: String,
        context_id: String,
        task_hash: String,
        task_keywords: Option<Vec<String>>,
        task_type: Option<String>,
        work_phase: Option<String>,
        file_types: Option<Vec<String>>,
        error_context: Option<String>,
        related_technologies: Option<Vec<String>>,
    ) -> PyResult<String> {
        let context = ProvidedContext {
            session_id,
            agent_role,
            namespace,
            context_type: parse_context_type(&context_type)?,
            context_id,
            task_hash,
            task_keywords,
            task_type: task_type.map(|t| parse_task_type(&t)).transpose()?,
            work_phase: work_phase.map(|p| parse_work_phase(&p)).transpose()?,
            file_types,
            error_context: error_context.map(|e| parse_error_context(&e)).transpose()?,
            related_technologies,
        };

        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyValueError::new_err(format!("Failed to create runtime: {}", e)))?;

        runtime
            .block_on(async { self.collector.record_context_provided(context).await })
            .map_err(|e| PyValueError::new_err(format!("Failed to record context: {}", e)))
    }

    /// Record that context was accessed
    fn record_context_accessed(&self, eval_id: String) -> PyResult<()> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyValueError::new_err(format!("Failed to create runtime: {}", e)))?;

        runtime
            .block_on(async { self.collector.record_context_accessed(&eval_id).await })
            .map_err(|e| PyValueError::new_err(format!("Failed to record access: {}", e)))
    }

    /// Record that context was edited
    fn record_context_edited(&self, eval_id: String) -> PyResult<()> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyValueError::new_err(format!("Failed to create runtime: {}", e)))?;

        runtime
            .block_on(async { self.collector.record_context_edited(&eval_id).await })
            .map_err(|e| PyValueError::new_err(format!("Failed to record edit: {}", e)))
    }

    /// Record that context was committed
    fn record_context_committed(&self, eval_id: String) -> PyResult<()> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyValueError::new_err(format!("Failed to create runtime: {}", e)))?;

        runtime
            .block_on(async { self.collector.record_context_committed(&eval_id).await })
            .map_err(|e| PyValueError::new_err(format!("Failed to record commit: {}", e)))
    }

    /// Record task completion with success score
    fn record_task_completion(&self, session_id: String, success_score: f32) -> PyResult<()> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyValueError::new_err(format!("Failed to create runtime: {}", e)))?;

        runtime
            .block_on(async {
                self.collector
                    .record_task_completion(&session_id, success_score)
                    .await
            })
            .map_err(|e| PyValueError::new_err(format!("Failed to record completion: {}", e)))
    }
}

/// Python wrapper for FeatureExtractor
#[pyclass(name = "FeatureExtractor")]
pub struct PyFeatureExtractor {
    extractor: FeatureExtractor,
}

#[pymethods]
impl PyFeatureExtractor {
    /// Create a new feature extractor
    #[new]
    fn new(db_path: String) -> PyResult<Self> {
        let extractor = FeatureExtractor::new(db_path);
        Ok(Self { extractor })
    }

    /// Extract features from an evaluation
    ///
    /// Returns a dictionary of features
    fn extract_features(
        &self,
        eval_id: String,
        context_keywords: Vec<String>,
    ) -> PyResult<Py<PyDict>> {
        // Create a feedback collector to fetch the evaluation
        let collector = FeedbackCollector::new(self.extractor.db_path().to_string());

        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyValueError::new_err(format!("Failed to create runtime: {}", e)))?;

        // Fetch the evaluation
        let evaluation = runtime
            .block_on(async { collector.get_evaluation(&eval_id).await })
            .map_err(|e| PyValueError::new_err(format!("Failed to fetch evaluation: {}", e)))?;

        // Extract features
        let features = runtime
            .block_on(async {
                self.extractor
                    .extract_features(&evaluation, &context_keywords)
                    .await
            })
            .map_err(|e| PyValueError::new_err(format!("Failed to extract features: {}", e)))?;

        // Convert to Python dictionary
        Python::with_gil(|py| {
            let dict = PyDict::new_bound(py);

            dict.set_item("evaluation_id", features.evaluation_id)?;
            dict.set_item("keyword_overlap_score", features.keyword_overlap_score)?;
            dict.set_item("semantic_similarity", features.semantic_similarity)?;
            dict.set_item("recency_days", features.recency_days)?;
            dict.set_item("access_frequency", features.access_frequency)?;
            dict.set_item("last_used_days_ago", features.last_used_days_ago)?;
            dict.set_item("work_phase_match", features.work_phase_match)?;
            dict.set_item("task_type_match", features.task_type_match)?;
            dict.set_item("agent_role_affinity", features.agent_role_affinity)?;
            dict.set_item("namespace_match", features.namespace_match)?;
            dict.set_item("file_type_match", features.file_type_match)?;
            dict.set_item("historical_success_rate", features.historical_success_rate)?;
            dict.set_item("co_occurrence_score", features.co_occurrence_score)?;
            dict.set_item("was_useful", features.was_useful)?;

            Ok(dict.into())
        })
    }
}

/// Python wrapper for RelevanceScorer
#[pyclass(name = "RelevanceScorer")]
pub struct PyRelevanceScorer {
    scorer: RelevanceScorer,
}

#[pymethods]
impl PyRelevanceScorer {
    /// Create a new relevance scorer
    #[new]
    fn new(db_path: String) -> PyResult<Self> {
        let scorer = RelevanceScorer::new(db_path);
        Ok(Self { scorer })
    }

    /// Get learned weights for a context
    ///
    /// Args:
    ///     scope: session, project, or global
    ///     scope_id: session ID, namespace, or "global"
    ///     context_type: skill, memory, file, etc.
    ///     agent_role: optimizer, executor, etc.
    ///     work_phase: Optional work phase
    ///     task_type: Optional task type
    ///     error_context: Optional error context
    ///
    /// Returns:
    ///     Dictionary of feature weights
    fn get_weights(
        &self,
        scope: String,
        _scope_id: String,
        _context_type: String,
        _agent_role: String,
        _work_phase: Option<String>,
        _task_type: Option<String>,
        _error_context: Option<String>,
    ) -> PyResult<Py<PyDict>> {
        let _scope_enum = parse_scope(&scope)?;

        // TODO: Implement actual weight lookup
        // For now, return default weights
        Python::with_gil(|py| {
            let dict = PyDict::new_bound(py);
            dict.set_item("keyword_match", 0.35)?;
            dict.set_item("recency", 0.15)?;
            dict.set_item("access_patterns", 0.25)?;
            dict.set_item("historical_success", 0.15)?;
            dict.set_item("file_type_match", 0.10)?;
            dict.set_item("_confidence", 0.5)?;
            Ok(dict.into())
        })
    }

    /// Update weights based on feedback
    fn update_weights(&self, _evaluation_id: String) -> PyResult<()> {
        let _runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyValueError::new_err(format!("Failed to create runtime: {}", e)))?;

        // TODO: Implement weight update
        // runtime.block_on(async { self.scorer.update_weights(&evaluation_id, &features).await })
        //     .map_err(|e| PyValueError::new_err(format!("Failed to update weights: {}", e)))

        Ok(()) // Placeholder
    }
}

// Helper functions for parsing enums from Python strings

fn parse_context_type(s: &str) -> PyResult<ContextType> {
    match s {
        "skill" => Ok(ContextType::Skill),
        "memory" => Ok(ContextType::Memory),
        "file" => Ok(ContextType::File),
        "commit" => Ok(ContextType::Commit),
        "plan" => Ok(ContextType::Plan),
        _ => Err(PyValueError::new_err(format!(
            "Invalid context_type: {}",
            s
        ))),
    }
}

fn parse_task_type(s: &str) -> PyResult<TaskType> {
    match s {
        "feature" => Ok(TaskType::Feature),
        "bugfix" => Ok(TaskType::Bugfix),
        "refactor" => Ok(TaskType::Refactor),
        "test" => Ok(TaskType::Test),
        "documentation" => Ok(TaskType::Documentation),
        "optimization" => Ok(TaskType::Optimization),
        "exploration" => Ok(TaskType::Exploration),
        _ => Err(PyValueError::new_err(format!("Invalid task_type: {}", s))),
    }
}

fn parse_work_phase(s: &str) -> PyResult<WorkPhase> {
    match s {
        "planning" => Ok(WorkPhase::Planning),
        "implementation" => Ok(WorkPhase::Implementation),
        "debugging" => Ok(WorkPhase::Debugging),
        "review" => Ok(WorkPhase::Review),
        "testing" => Ok(WorkPhase::Testing),
        "documentation" => Ok(WorkPhase::Documentation),
        _ => Err(PyValueError::new_err(format!("Invalid work_phase: {}", s))),
    }
}

fn parse_error_context(s: &str) -> PyResult<ErrorContext> {
    match s {
        "compilation" => Ok(ErrorContext::Compilation),
        "runtime" => Ok(ErrorContext::Runtime),
        "test_failure" => Ok(ErrorContext::TestFailure),
        "lint" => Ok(ErrorContext::Lint),
        "none" => Ok(ErrorContext::None),
        _ => Err(PyValueError::new_err(format!(
            "Invalid error_context: {}",
            s
        ))),
    }
}

fn parse_scope(s: &str) -> PyResult<Scope> {
    match s {
        "session" => Ok(Scope::Session),
        "project" => Ok(Scope::Project),
        "global" => Ok(Scope::Global),
        _ => Err(PyValueError::new_err(format!("Invalid scope: {}", s))),
    }
}
