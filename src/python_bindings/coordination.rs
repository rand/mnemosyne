//! PyCoordinator - Thread-safe coordination for multi-agent system.
//!
//! Provides shared state management and synchronization primitives
//! for coordinating multiple agents accessing Mnemosyne storage.

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Agent execution state.
#[derive(Clone, Debug)]
pub enum AgentState {
    Idle,
    Running,
    Blocked,
    Complete,
    Failed,
}

/// Shared coordination state for multi-agent orchestration.
///
/// Thread-safe via Arc<RwLock<T>>. Tracks:
/// - Agent states and progress
/// - Context utilization metrics
/// - Task dependencies and readiness
/// - Work queue and completion status
#[pyclass]
pub struct PyCoordinator {
    /// Agent states (agent_id -> state)
    agent_states: Arc<RwLock<HashMap<String, AgentState>>>,

    /// Context utilization (0.0 - 1.0)
    context_utilization: Arc<RwLock<f64>>,

    /// Task readiness (task_id -> ready)
    task_ready: Arc<RwLock<HashMap<String, bool>>>,

    /// Shared metrics
    metrics: Arc<RwLock<HashMap<String, f64>>>,

    /// Tokio runtime for async operations
    runtime: tokio::runtime::Runtime,
}

#[pymethods]
impl PyCoordinator {
    /// Create new coordinator.
    #[new]
    fn new() -> PyResult<Self> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(PyCoordinator {
            agent_states: Arc::new(RwLock::new(HashMap::new())),
            context_utilization: Arc::new(RwLock::new(0.0)),
            task_ready: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(HashMap::new())),
            runtime,
        })
    }

    /// Register an agent.
    ///
    /// Args:
    ///     agent_id: Unique agent identifier
    fn register_agent(&self, agent_id: String) -> PyResult<()> {
        self.runtime.block_on(async {
            let mut states = self.agent_states.write().await;
            states.insert(agent_id, AgentState::Idle);
        });
        Ok(())
    }

    /// Update agent state.
    ///
    /// Args:
    ///     agent_id: Agent identifier
    ///     state: New state ("idle", "running", "blocked", "complete", "failed")
    fn update_agent_state(&self, agent_id: String, state: &str) -> PyResult<()> {
        let new_state = match state {
            "idle" => AgentState::Idle,
            "running" => AgentState::Running,
            "blocked" => AgentState::Blocked,
            "complete" => AgentState::Complete,
            "failed" => AgentState::Failed,
            _ => {
                return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                    "Invalid state: {}",
                    state
                )))
            }
        };

        self.runtime.block_on(async {
            let mut states = self.agent_states.write().await;
            states.insert(agent_id, new_state);
        });

        Ok(())
    }

    /// Get agent state.
    ///
    /// Args:
    ///     agent_id: Agent identifier
    ///
    /// Returns:
    ///     str: Current state or None if not registered
    fn get_agent_state(&self, agent_id: String) -> PyResult<Option<String>> {
        let state = self.runtime.block_on(async {
            let states = self.agent_states.read().await;
            states.get(&agent_id).cloned()
        });

        Ok(state.map(|s| match s {
            AgentState::Idle => "idle".to_string(),
            AgentState::Running => "running".to_string(),
            AgentState::Blocked => "blocked".to_string(),
            AgentState::Complete => "complete".to_string(),
            AgentState::Failed => "failed".to_string(),
        }))
    }

    /// Get all agent states.
    ///
    /// Returns:
    ///     dict: Mapping of agent_id -> state
    fn get_all_agent_states(&self) -> PyResult<PyObject> {
        let states = self.runtime.block_on(async {
            let states = self.agent_states.read().await;
            states.clone()
        });

        Python::with_gil(|py| {
            let dict = PyDict::new_bound(py);
            for (agent_id, state) in states {
                let state_str = match state {
                    AgentState::Idle => "idle",
                    AgentState::Running => "running",
                    AgentState::Blocked => "blocked",
                    AgentState::Complete => "complete",
                    AgentState::Failed => "failed",
                };
                dict.set_item(agent_id, state_str)?;
            }
            Ok(dict.into())
        })
    }

    /// Update context utilization.
    ///
    /// Args:
    ///     utilization: Context utilization (0.0 - 1.0)
    fn update_context_utilization(&self, utilization: f64) -> PyResult<()> {
        if utilization < 0.0 || utilization > 1.0 {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                "Utilization must be between 0.0 and 1.0",
            ));
        }

        self.runtime.block_on(async {
            let mut util = self.context_utilization.write().await;
            *util = utilization;
        });

        Ok(())
    }

    /// Get context utilization.
    ///
    /// Returns:
    ///     float: Current utilization (0.0 - 1.0)
    fn get_context_utilization(&self) -> PyResult<f64> {
        let util = self.runtime.block_on(async {
            let util = self.context_utilization.read().await;
            *util
        });
        Ok(util)
    }

    /// Check if context threshold exceeded.
    ///
    /// Args:
    ///     threshold: Threshold value (e.g., 0.75 for 75%)
    ///
    /// Returns:
    ///     bool: True if utilization >= threshold
    fn is_context_threshold_exceeded(&self, threshold: f64) -> PyResult<bool> {
        let util = self.get_context_utilization()?;
        Ok(util >= threshold)
    }

    /// Mark task as ready.
    ///
    /// Args:
    ///     task_id: Task identifier
    fn mark_task_ready(&self, task_id: String) -> PyResult<()> {
        self.runtime.block_on(async {
            let mut ready = self.task_ready.write().await;
            ready.insert(task_id, true);
        });
        Ok(())
    }

    /// Mark task as blocked.
    ///
    /// Args:
    ///     task_id: Task identifier
    fn mark_task_blocked(&self, task_id: String) -> PyResult<()> {
        self.runtime.block_on(async {
            let mut ready = self.task_ready.write().await;
            ready.insert(task_id, false);
        });
        Ok(())
    }

    /// Check if task is ready.
    ///
    /// Args:
    ///     task_id: Task identifier
    ///
    /// Returns:
    ///     bool: True if ready, False if blocked or not registered
    fn is_task_ready(&self, task_id: String) -> PyResult<bool> {
        let ready = self.runtime.block_on(async {
            let ready_map = self.task_ready.read().await;
            ready_map.get(&task_id).copied().unwrap_or(false)
        });
        Ok(ready)
    }

    /// Set a metric.
    ///
    /// Args:
    ///     key: Metric name
    ///     value: Metric value
    fn set_metric(&self, key: String, value: f64) -> PyResult<()> {
        self.runtime.block_on(async {
            let mut metrics = self.metrics.write().await;
            metrics.insert(key, value);
        });
        Ok(())
    }

    /// Get a metric.
    ///
    /// Args:
    ///     key: Metric name
    ///
    /// Returns:
    ///     float: Metric value or None if not set
    fn get_metric(&self, key: String) -> PyResult<Option<f64>> {
        let value = self.runtime.block_on(async {
            let metrics = self.metrics.read().await;
            metrics.get(&key).copied()
        });
        Ok(value)
    }

    /// Get all metrics.
    ///
    /// Returns:
    ///     dict: All metrics
    fn get_all_metrics(&self) -> PyResult<PyObject> {
        let metrics = self.runtime.block_on(async {
            let metrics = self.metrics.read().await;
            metrics.clone()
        });

        Python::with_gil(|py| {
            let dict = PyDict::new_bound(py);
            for (key, value) in metrics {
                dict.set_item(key, value)?;
            }
            Ok(dict.into())
        })
    }

    /// Reset coordinator state.
    fn reset(&self) -> PyResult<()> {
        self.runtime.block_on(async {
            let mut states = self.agent_states.write().await;
            states.clear();

            let mut util = self.context_utilization.write().await;
            *util = 0.0;

            let mut ready = self.task_ready.write().await;
            ready.clear();

            let mut metrics = self.metrics.write().await;
            metrics.clear();
        });
        Ok(())
    }
}
