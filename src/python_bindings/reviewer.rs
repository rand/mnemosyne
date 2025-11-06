//! PyReviewer - LLM-based semantic validation via Python Claude SDK.
//!
//! Provides Rust → Python bridge for deep semantic validation of work artifacts.
//! Uses Claude Agent SDK for intelligent quality assessment beyond pattern matching.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::types::MemoryId;

/// Python wrapper for LLM-based reviewer validation.
///
/// Thread-safe wrapper that calls into Python ReviewerAgent's semantic validation methods.
/// Converts between Rust types and Python types for cross-language calls.
#[pyclass]
pub struct PyReviewer {
    /// Python ReviewerAgent instance
    py_agent: Arc<Mutex<PyObject>>,
    runtime: tokio::runtime::Runtime,
}

#[pymethods]
impl PyReviewer {
    /// Create new PyReviewer with Python ReviewerAgent instance.
    ///
    /// Args:
    ///     reviewer_agent: Python ReviewerAgent instance from src/orchestration/agents/reviewer.py
    #[new]
    fn new(reviewer_agent: PyObject) -> PyResult<Self> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(PyReviewer {
            py_agent: Arc::new(Mutex::new(reviewer_agent)),
            runtime,
        })
    }

    /// Semantic intent validation: Does implementation satisfy original intent?
    ///
    /// Args:
    ///     original_intent: Original work requirement/specification
    ///     implementation_content: Code/documentation that was produced
    ///     execution_memory_ids: List of memory IDs from execution (as strings)
    ///
    /// Returns:
    ///     tuple[bool, list[str]]: (passed, issues)
    fn semantic_intent_check(
        &self,
        original_intent: String,
        implementation_content: String,
        execution_memory_ids: Vec<String>,
    ) -> PyResult<(bool, Vec<String>)> {
        self.runtime.block_on(async {
            Python::with_gil(|py| {
                let agent = self.py_agent.blocking_lock();

                // Convert memory IDs to dicts for Python
                let memories = PyList::empty_bound(py);
                for mem_id in execution_memory_ids {
                    let mem_dict = PyDict::new_bound(py);
                    mem_dict.set_item("id", mem_id)?;
                    memories.append(mem_dict)?;
                }

                // Call Python method
                let result = agent.call_method1(
                    py,
                    "semantic_intent_check",
                    (original_intent, implementation_content, memories),
                )?;

                // Extract tuple result
                let tuple = result.extract::<(bool, Vec<String>)>(py)?;
                Ok(tuple)
            })
        })
    }

    /// Semantic completeness validation: Are all requirements fully implemented?
    ///
    /// Args:
    ///     requirements: List of explicit requirements to validate
    ///     implementation_content: Implementation to check
    ///     execution_memory_ids: List of memory IDs from execution
    ///
    /// Returns:
    ///     tuple[bool, list[str]]: (passed, missing_requirements)
    fn semantic_completeness_check(
        &self,
        requirements: Vec<String>,
        implementation_content: String,
        execution_memory_ids: Vec<String>,
    ) -> PyResult<(bool, Vec<String>)> {
        self.runtime.block_on(async {
            Python::with_gil(|py| {
                let agent = self.py_agent.blocking_lock();

                // Convert memory IDs to dicts
                let memories = PyList::empty_bound(py);
                for mem_id in execution_memory_ids {
                    let mem_dict = PyDict::new_bound(py);
                    mem_dict.set_item("id", mem_id)?;
                    memories.append(mem_dict)?;
                }

                // Call Python method
                let result = agent.call_method1(
                    py,
                    "semantic_completeness_check",
                    (requirements, implementation_content, memories),
                )?;

                let tuple = result.extract::<(bool, Vec<String>)>(py)?;
                Ok(tuple)
            })
        })
    }

    /// Semantic correctness validation: Is the logic sound and bug-free?
    ///
    /// Args:
    ///     implementation_content: Code/implementation to validate
    ///     test_results_json: JSON string of test results (or empty string if none)
    ///     execution_memory_ids: List of memory IDs from execution
    ///
    /// Returns:
    ///     tuple[bool, list[str]]: (passed, logic_issues)
    fn semantic_correctness_check(
        &self,
        implementation_content: String,
        test_results_json: String,
        execution_memory_ids: Vec<String>,
    ) -> PyResult<(bool, Vec<String>)> {
        self.runtime.block_on(async {
            Python::with_gil(|py| {
                let agent = self.py_agent.blocking_lock();

                // Parse test results JSON
                let test_results = if test_results_json.is_empty() {
                    PyDict::new_bound(py)
                } else {
                    // Parse JSON string to dict
                    let json_module = py.import_bound("json")?;
                    let parsed = json_module.call_method1("loads", (test_results_json,))?;
                    parsed.downcast::<PyDict>()?.clone()
                };

                // Convert memory IDs to dicts
                let memories = PyList::empty_bound(py);
                for mem_id in execution_memory_ids {
                    let mem_dict = PyDict::new_bound(py);
                    mem_dict.set_item("id", mem_id)?;
                    memories.append(mem_dict)?;
                }

                // Call Python method
                let result = agent.call_method1(
                    py,
                    "semantic_correctness_check",
                    (implementation_content, test_results, memories),
                )?;

                let tuple = result.extract::<(bool, Vec<String>)>(py)?;
                Ok(tuple)
            })
        })
    }

    /// Generate improvement guidance for review failure.
    ///
    /// Args:
    ///     failed_gates: Map of gate name to pass/fail status
    ///     issues: List of all issues identified
    ///     original_intent: Original work requirements
    ///     execution_memory_ids: List of memory IDs from previous execution
    ///
    /// Returns:
    ///     str: Consolidated improvement plan with actionable guidance
    fn generate_improvement_guidance(
        &self,
        failed_gates: HashMap<String, bool>,
        issues: Vec<String>,
        original_intent: String,
        execution_memory_ids: Vec<String>,
    ) -> PyResult<String> {
        self.runtime.block_on(async {
            Python::with_gil(|py| {
                let agent = self.py_agent.blocking_lock();

                // Convert failed_gates to Python dict
                let gates_dict = PyDict::new_bound(py);
                for (gate, passed) in failed_gates {
                    gates_dict.set_item(gate, passed)?;
                }

                // Convert memory IDs to dicts
                let memories = PyList::empty_bound(py);
                for mem_id in execution_memory_ids {
                    let mem_dict = PyDict::new_bound(py);
                    mem_dict.set_item("id", mem_id)?;
                    memories.append(mem_dict)?;
                }

                // Call Python method
                let result = agent.call_method1(
                    py,
                    "generate_improvement_guidance",
                    (gates_dict, issues, original_intent, memories),
                )?;

                let guidance = result.extract::<String>(py)?;
                Ok(guidance)
            })
        })
    }

    /// Extract explicit requirements from user intent using LLM.
    ///
    /// Args:
    ///     original_intent: User's original work request/specification
    ///     context: Optional context about the project (e.g., existing code, constraints)
    ///
    /// Returns:
    ///     Vec<String>: List of explicit requirements extracted from intent
    #[pyo3(signature = (original_intent, context=None))]
    fn extract_requirements_from_intent(
        &self,
        original_intent: String,
        context: Option<String>,
    ) -> PyResult<Vec<String>> {
        self.runtime.block_on(async {
            Python::with_gil(|py| {
                let agent = self.py_agent.blocking_lock();

                // Convert Option<String> to Python None or string
                let context_arg = match context {
                    Some(ctx) => ctx.into_py(py),
                    None => py.None(),
                };

                // Call Python method
                let result = agent.call_method1(
                    py,
                    "extract_requirements_from_intent",
                    (original_intent, context_arg),
                )?;

                // Extract list of requirements
                let requirements = result.extract::<Vec<String>>(py)?;
                Ok(requirements)
            })
        })
    }
}

/// Helper to collect implementation content from memory IDs.
///
/// This is a utility function for gathering actual implementation content
/// from storage before sending to LLM validator.
pub async fn collect_implementation_from_memories(
    storage: &Arc<dyn crate::storage::StorageBackend>,
    memory_ids: &[MemoryId],
) -> crate::error::Result<String> {
    let mut content_parts = Vec::new();

    for mem_id in memory_ids.iter().take(20) {
        // Limit to 20 memories for context
        match storage.get_memory(*mem_id).await {
            Ok(memory) => {
                content_parts.push(format!(
                    "## {}\n{}",
                    memory.summary,
                    memory.content.chars().take(500).collect::<String>() // Limit per memory
                ));
            }
            Err(e) => {
                tracing::warn!("Failed to retrieve memory {}: {:?}", mem_id, e);
            }
        }
    }

    if content_parts.is_empty() {
        Ok("No implementation content found in execution memories.".to_string())
    } else {
        Ok(content_parts.join("\n\n"))
    }
}

/// Helper to convert execution memories to Python-compatible format.
///
/// Extracts memory content and metadata for passing to Python validator.
pub async fn execution_memories_to_python_format(
    storage: &Arc<dyn crate::storage::StorageBackend>,
    memory_ids: &[MemoryId],
) -> crate::error::Result<Vec<HashMap<String, String>>> {
    let mut memories = Vec::new();

    for mem_id in memory_ids.iter().take(10) {
        // Limit to 10 for context efficiency
        match storage.get_memory(*mem_id).await {
            Ok(memory) => {
                let mut mem_map = HashMap::new();
                mem_map.insert("id".to_string(), mem_id.to_string());
                mem_map.insert("summary".to_string(), memory.summary.clone());
                mem_map.insert(
                    "content".to_string(),
                    memory.content.chars().take(200).collect::<String>(),
                );
                memories.push(mem_map);
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to retrieve memory {} for Python format: {:?}",
                    mem_id,
                    e
                );
            }
        }
    }

    Ok(memories)
}
