//! PyO3 bridge for Claude Agent SDK integration
//!
//! This module provides a Rust interface to Python Claude SDK agents for
//! intelligent multi-agent orchestration. It enables:
//!
//! - Spawning Python agents with Claude SDK intelligence
//! - Bidirectional message passing (Rust ↔ Python)
//! - State synchronization with dashboard
//! - Async-friendly GIL management
//!
//! # Architecture
//!
//! ```text
//! Rust (Ractor Actors) → ClaudeAgentBridge → Python (Claude SDK) → LLM
//!                       ↑
//!                       └── Callbacks (state updates, events)
//! ```
//!
//! # Usage
//!
//! ```rust,no_run
//! use mnemosyne_core::orchestration::claude_agent_bridge::ClaudeAgentBridge;
//! use mnemosyne_core::launcher::agents::AgentRole;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create event broadcaster for dashboard integration
//! let broadcaster = EventBroadcaster::new(1000);
//!
//! // Spawn Python agent with Claude SDK
//! let bridge = ClaudeAgentBridge::spawn(
//!     AgentRole::Executor,
//!     broadcaster.subscribe(),
//! ).await?;
//!
//! // Send work to Python agent
//! let work_item = WorkItem::new("Implement feature", AgentRole::Executor, Phase::PlanToArtifacts, 5);
//! let result = bridge.send_work(work_item).await?;
//! # Ok(())
//! # }
//! ```

// Guard entire module behind python feature flag
#[cfg(feature = "python")]
use crate::api::Event;
#[cfg(feature = "python")]
use crate::error::{MnemosyneError, Result};
#[cfg(feature = "python")]
use crate::launcher::agents::AgentRole;
#[cfg(feature = "python")]
use crate::orchestration::messages::WorkResult;
#[cfg(feature = "python")]
use crate::orchestration::state::{AgentState, Phase, WorkItem};
#[cfg(feature = "python")]
use crate::secrets::SecretsManager;
#[cfg(feature = "python")]
use pyo3::prelude::*;
#[cfg(feature = "python")]
use secrecy::ExposeSecret;
#[cfg(feature = "python")]
use pyo3::types::{PyDict, PyList};
#[cfg(feature = "python")]
use serde_json::Value;
#[cfg(feature = "python")]
use std::collections::HashMap;
#[cfg(feature = "python")]
use std::sync::Arc;
#[cfg(feature = "python")]
use tokio::sync::{broadcast, Mutex, RwLock};
#[cfg(feature = "python")]
use tracing::{debug, error, info, warn};

/// PyO3 bridge to Python Claude SDK agents
///
/// Manages Python agent lifecycle and provides async-friendly interface
/// to Claude SDK for intelligent agent behavior. Thread-safe via Arc<Mutex<>>
/// for GIL management.
///
/// **Error Handling**: Tracks errors and supports automatic restart on failure.
#[cfg(feature = "python")]
#[derive(Clone)]
pub struct ClaudeAgentBridge {
    /// Python agent instance (holds GIL when accessed)
    agent: Arc<Mutex<Py<PyAny>>>,
    /// Agent role (Orchestrator, Optimizer, Reviewer, Executor)
    role: AgentRole,
    /// Event broadcaster for dashboard updates
    event_tx: broadcast::Sender<Event>,
    /// Current agent state
    state: Arc<RwLock<AgentState>>,
    /// Agent ID for event tracking
    agent_id: String,
    /// Error count for restart logic
    error_count: Arc<RwLock<usize>>,
    /// Last error timestamp for rate limiting restarts
    last_error: Arc<RwLock<Option<std::time::Instant>>>,
}

#[cfg(feature = "python")]
impl ClaudeAgentBridge {
    /// Spawn a new Python Claude SDK agent
    ///
    /// Initializes Python interpreter, imports agent module, and creates
    /// agent instance with Claude SDK client.
    ///
    /// # Arguments
    ///
    /// * `role` - Agent role (Orchestrator, Optimizer, Reviewer, Executor)
    /// * `event_tx` - Event broadcaster for dashboard updates
    ///
    /// # Returns
    ///
    /// Bridge instance ready to send work to Python agent
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Python interpreter initialization fails
    /// - Agent module import fails
    /// - Agent instantiation fails
    /// - Session startup fails
    pub async fn spawn(role: AgentRole, event_tx: broadcast::Sender<Event>) -> Result<Self> {
        let role_clone = role.clone();
        let event_tx_clone = event_tx.clone();
        let agent_id = format!("{:?}-agent", role).to_lowercase();

        info!("Spawning Python Claude SDK agent for role: {:?}", role);

        // Ensure ANTHROPIC_API_KEY is available from secrets if not in environment
        if std::env::var("ANTHROPIC_API_KEY").is_err() {
            match SecretsManager::new() {
                Ok(secrets) => match secrets.get_secret("ANTHROPIC_API_KEY") {
                    Ok(api_key) => {
                        // Set environment variable for Python to access
                        std::env::set_var("ANTHROPIC_API_KEY", api_key.expose_secret());
                        info!("API key loaded from secrets for Python agent");
                    }
                    Err(e) => {
                        warn!("Failed to load API key from secrets: {}", e);
                    }
                },
                Err(e) => {
                    warn!("Failed to initialize secrets manager: {}", e);
                }
            }
        } else {
            debug!("API key already present in environment");
        }

        // Spawn in blocking task to avoid blocking tokio runtime
        let agent = tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                // Import agent factory module
                let agent_factory = py
                    .import_bound("mnemosyne.orchestration.agents.agent_factory")
                    .map_err(|e| {
                        error!("Failed to import agent_factory module: {}", e);
                        MnemosyneError::Other(format!("Agent factory import failed: {}", e))
                    })?;

                // Get create_agent function
                let create_fn = agent_factory.getattr("create_agent").map_err(|e| {
                    error!("Failed to get create_agent function: {}", e);
                    MnemosyneError::Other(format!("create_agent not found: {}", e))
                })?;

                // Convert role to Python string
                let role_str = match role_clone {
                    AgentRole::Orchestrator => "orchestrator",
                    AgentRole::Optimizer => "optimizer",
                    AgentRole::Reviewer => "reviewer",
                    AgentRole::Executor => "executor",
                };

                // Create agent instance
                let agent = create_fn.call1((role_str,)).map_err(|e| {
                    error!("Failed to create agent for role {:?}: {}", role_clone, e);
                    MnemosyneError::Other(format!("Agent creation failed: {}", e))
                })?;

                info!("Python agent created for role: {:?}", role_clone);

                Ok::<Py<PyAny>, MnemosyneError>(agent.unbind().into())
            })
        })
        .await
        .map_err(|e| {
            error!("Tokio spawn_blocking failed: {}", e);
            MnemosyneError::Other(format!("Async execution failed: {}", e))
        })??;

        let bridge = Self {
            agent: Arc::new(Mutex::new(agent)),
            role,
            event_tx: event_tx_clone,
            state: Arc::new(RwLock::new(AgentState::Idle)),
            agent_id: agent_id.clone(),
            error_count: Arc::new(RwLock::new(0)),
            last_error: Arc::new(RwLock::new(None)),
        };

        // Start agent session
        bridge.start_session().await?;

        // Send initial heartbeat/agent started event
        let event = Event::agent_started(agent_id);
        if let Err(e) = bridge.event_tx.send(event) {
            warn!("Failed to broadcast agent started event: {}", e);
        }

        Ok(bridge)
    }

    /// Start Python agent session
    ///
    /// Calls agent.start_session() to initialize Claude SDK connection
    async fn start_session(&self) -> Result<()> {
        let agent = self.agent.clone();

        tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                let agent_guard = agent.blocking_lock();
                let agent_ref = agent_guard.bind(py);

                // Call start_session() method
                agent_ref.call_method0("start_session").map_err(|e| {
                    error!("Failed to start agent session: {}", e);
                    MnemosyneError::Other(format!("Session start failed: {}", e))
                })?;

                info!("Agent session started successfully");
                Ok::<(), MnemosyneError>(())
            })
        })
        .await
        .map_err(|e| {
            error!("Tokio spawn_blocking failed: {}", e);
            MnemosyneError::Other(format!("Async execution failed: {}", e))
        })??;

        Ok(())
    }

    /// Send work item to Python agent for execution
    ///
    /// Converts work item to Python dict, calls agent.execute_work(),
    /// and returns result. Updates agent state during execution.
    ///
    /// # Arguments
    ///
    /// * `item` - Work item to execute
    ///
    /// # Returns
    ///
    /// Work result with success status, artifacts, and memory IDs
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Work item conversion fails
    /// - Python agent execution fails
    /// - Result extraction fails
    pub async fn send_work(&self, item: WorkItem) -> Result<WorkResult> {
        debug!(
            "Sending work to Python agent {:?}: {}",
            self.role, item.description
        );

        // Update state to Active
        {
            let mut state = self.state.write().await;
            *state = AgentState::Active;
        }

        // Broadcast agent started event with task
        let event = Event::agent_started_with_task(self.agent_id.clone(), item.description.clone());
        if let Err(e) = self.event_tx.send(event) {
            warn!("Failed to broadcast agent started event: {}", e);
        }

        let agent = self.agent.clone();
        let agent_id = self.agent_id.clone();
        let event_tx = self.event_tx.clone();
        let item_id = item.id.clone();

        // Execute work in blocking task
        let result = tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                let agent_guard = agent.blocking_lock();
                let agent_ref = agent_guard.bind(py);

                // Convert work item to Python dict
                let py_work = work_item_to_python(py, &item)?;

                // Call execute_work method
                let py_result = agent_ref.call_method1("execute_work", (py_work,)).map_err(|e| {
                    error!("Python agent execution failed: {}", e);

                    // Broadcast agent failed event
                    let error_msg = format!("{}", e);
                    let event = Event::agent_failed(agent_id.clone(), error_msg.clone());
                    let _ = event_tx.send(event);

                    MnemosyneError::Other(format!("Agent execution failed: {}", e))
                })?;

                // Extract result
                let result = extract_work_result(py, &py_result, item_id)?;

                // Broadcast agent completed event
                let summary = format!("Completed: {}", item.description);
                let event = Event::agent_completed(agent_id, summary);
                let _ = event_tx.send(event);

                Ok::<WorkResult, MnemosyneError>(result)
            })
        })
        .await
        .map_err(|e| {
            error!("Tokio spawn_blocking failed: {}", e);
            MnemosyneError::Other(format!("Async execution failed: {}", e))
        });

        // Handle result and track errors
        let result = match result {
            Ok(Ok(work_result)) => {
                // Check if work result itself indicates failure
                if !work_result.success {
                    // Work failed at Python level - record error
                    self.record_error().await;
                }
                Ok(work_result)
            }
            Ok(Err(e)) => {
                // Python execution error - record and propagate
                self.record_error().await;
                Err(e)
            }
            Err(e) => {
                // Tokio error - record and propagate
                self.record_error().await;
                Err(e)
            }
        };

        // Update state back to Idle
        {
            let mut state = self.state.write().await;
            *state = AgentState::Idle;
        }

        if result.is_ok() {
            debug!("Python agent completed work: {:?}", self.role);
        } else {
            warn!("Python agent failed work: {:?}", self.role);
        }

        result
    }

    /// Get current agent state
    pub async fn get_state(&self) -> AgentState {
        *self.state.read().await
    }

    /// Shutdown Python agent session
    ///
    /// Calls agent.stop_session() to cleanup Claude SDK connection
    pub async fn shutdown(&self) -> Result<()> {
        let agent = self.agent.clone();

        tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                let agent_guard = agent.blocking_lock();
                let agent_ref = agent_guard.bind(py);

                // Call stop_session() method
                agent_ref.call_method0("stop_session").map_err(|e| {
                    error!("Failed to stop agent session: {}", e);
                    MnemosyneError::Other(format!("Session stop failed: {}", e))
                })?;

                info!("Agent session stopped successfully");
                Ok::<(), MnemosyneError>(())
            })
        })
        .await
        .map_err(|e| {
            error!("Tokio spawn_blocking failed: {}", e);
            MnemosyneError::Other(format!("Async execution failed: {}", e))
        })??;

        Ok(())
    }

    /// Get agent role
    pub fn role(&self) -> AgentRole {
        self.role.clone()
    }

    /// Get agent ID
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }
}

/// Debug implementation for ClaudeAgentBridge
#[cfg(feature = "python")]
impl std::fmt::Debug for ClaudeAgentBridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClaudeAgentBridge")
            .field("role", &self.role)
            .field("agent_id", &self.agent_id)
            .field("state", &"<RwLock>")
            .field("agent", &"<Python Object>")
            .finish()
    }
}

/// Convert WorkItem to Python dict
#[cfg(feature = "python")]
fn work_item_to_python(py: Python, item: &WorkItem) -> Result<PyObject> {
    let py_dict = PyDict::new_bound(py);

    py_dict
        .set_item("id", item.id.to_string())
        .map_err(|e| MnemosyneError::Other(format!("Failed to set id: {}", e)))?;

    py_dict
        .set_item("description", &item.description)
        .map_err(|e| MnemosyneError::Other(format!("Failed to set description: {}", e)))?;

    let phase_str = match item.phase {
        Phase::PromptToSpec => "prompt_to_spec",
        Phase::SpecToFullSpec => "spec_to_full_spec",
        Phase::FullSpecToPlan => "full_spec_to_plan",
        Phase::PlanToArtifacts => "plan_to_artifacts",
        Phase::Complete => "complete",
    };
    py_dict
        .set_item("phase", phase_str)
        .map_err(|e| MnemosyneError::Other(format!("Failed to set phase: {}", e)))?;

    py_dict
        .set_item("priority", item.priority)
        .map_err(|e| MnemosyneError::Other(format!("Failed to set priority: {}", e)))?;

    // Add optional fields
    if let Some(ref consolidated_id) = item.consolidated_context_id {
        py_dict
            .set_item("consolidated_context_id", consolidated_id.to_string())
            .map_err(|e| {
                MnemosyneError::Other(format!("Failed to set consolidated_context_id: {}", e))
            })?;
    }

    if let Some(ref review_feedback) = item.review_feedback {
        let feedback_list = PyList::new_bound(py, review_feedback.iter().map(|s| s.as_str()));
        py_dict
            .set_item("review_feedback", feedback_list)
            .map_err(|e| MnemosyneError::Other(format!("Failed to set review_feedback: {}", e)))?;
    }

    if item.review_attempt > 0 {
        py_dict
            .set_item("review_attempt", item.review_attempt)
            .map_err(|e| MnemosyneError::Other(format!("Failed to set review_attempt: {}", e)))?;
    }

    Ok(py_dict.to_object(py))
}

/// Extract WorkResult from Python result dict
#[cfg(feature = "python")]
fn extract_work_result(_py: Python, result: &Bound<PyAny>, item_id: crate::orchestration::state::WorkItemId) -> Result<WorkResult> {
    // Extract success status (required)
    let success = result
        .getattr("success")
        .map_err(|e| MnemosyneError::Other(format!("Failed to get success: {}", e)))?
        .extract::<bool>()
        .map_err(|e| MnemosyneError::Other(format!("Failed to extract success: {}", e)))?;

    // Extract data (optional serialized result)
    let data = result
        .getattr("data")
        .ok()
        .and_then(|d| d.extract::<String>().ok());

    // Extract memory IDs (optional list)
    let memory_ids = if let Ok(memory_ids_attr) = result.getattr("memory_ids") {
        if let Ok(memory_ids_list) = memory_ids_attr.downcast::<PyList>() {
            let mut ids = Vec::new();
            for item in memory_ids_list.iter() {
                if let Ok(memory_id_str) = item.extract::<String>() {
                    if let Ok(memory_id) = crate::types::MemoryId::from_string(&memory_id_str) {
                        ids.push(memory_id);
                    }
                }
            }
            ids
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    // Extract error if present (optional)
    let error = result
        .getattr("error")
        .ok()
        .and_then(|e| e.extract::<String>().ok());

    Ok(WorkResult {
        item_id,
        success,
        data,
        error,
        duration: std::time::Duration::from_secs(0), // Will be tracked by Rust actor
        memory_ids,
    })
}

// Note: JSON conversion helpers available in dspy_bridge if needed
// but not re-exported here since they're private

#[cfg(feature = "python")]
impl ClaudeAgentBridge {
    /// Record an error from the Python agent
    ///
    /// Increments error count and updates last error timestamp.
    /// Used for restart logic and health monitoring.
    /// Broadcasts error event to dashboard.
    pub async fn record_error(&self) {
        let mut error_count = self.error_count.write().await;
        *error_count += 1;

        let mut last_error = self.last_error.write().await;
        *last_error = Some(std::time::Instant::now());

        let count = *error_count;

        warn!(
            "Python agent {} error count: {}",
            self.agent_id,
            count
        );

        // Broadcast error recorded event
        let event = Event::agent_error_recorded(
            self.agent_id.clone(),
            count,
            format!("Python agent error"),
        );
        if let Err(e) = self.event_tx.send(event) {
            warn!("Failed to broadcast agent error event: {}", e);
        }

        // Check if health has degraded
        if count >= 3 {  // Warning threshold at 3 errors
            let event = Event::agent_health_degraded(
                self.agent_id.clone(),
                count,
                count < 5,  // Unhealthy at 5+ errors
            );
            if let Err(e) = self.event_tx.send(event) {
                warn!("Failed to broadcast health degraded event: {}", e);
            }
        }
    }

    /// Get error count for monitoring
    pub async fn error_count(&self) -> usize {
        *self.error_count.read().await
    }

    /// Reset error count (after successful restart)
    pub async fn reset_errors(&self) {
        let mut error_count = self.error_count.write().await;
        *error_count = 0;

        let mut last_error = self.last_error.write().await;
        *last_error = None;

        info!("Python agent {} errors reset", self.agent_id);
    }

    /// Check if bridge is healthy and should be restarted
    ///
    /// Returns true if:
    /// - Error count exceeds threshold (5 errors)
    /// - Last error was recent (within 60 seconds)
    pub async fn should_restart(&self) -> bool {
        let error_count = self.error_count.read().await;
        let last_error = self.last_error.read().await;

        // Check error threshold
        if *error_count >= 5 {
            // Check if errors are recent
            if let Some(last_err_time) = *last_error {
                let elapsed = last_err_time.elapsed();
                if elapsed.as_secs() < 60 {
                    warn!(
                        "Python agent {} should restart: {} errors in {}s",
                        self.agent_id,
                        *error_count,
                        elapsed.as_secs()
                    );
                    return true;
                }
            }
        }

        false
    }

    /// Attempt to restart the Python agent
    ///
    /// Respawns the Python agent instance and restarts the session.
    /// Returns Ok if restart succeeds, Err otherwise.
    pub async fn restart(&mut self) -> Result<()> {
        warn!("Restarting Python agent: {}", self.agent_id);

        // Respawn agent
        let role_clone = self.role.clone();
        let event_tx_clone = self.event_tx.clone();

        let new_agent = tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                // Import agent factory module
                let agent_factory = py
                    .import_bound("mnemosyne.orchestration.agents.agent_factory")
                    .map_err(|e| {
                        error!("Failed to import agent_factory module: {}", e);
                        MnemosyneError::Other(format!("Agent factory import failed: {}", e))
                    })?;

                // Get create_agent function
                let create_fn = agent_factory.getattr("create_agent").map_err(|e| {
                    error!("Failed to get create_agent function: {}", e);
                    MnemosyneError::Other(format!("create_agent not found: {}", e))
                })?;

                // Convert role to Python string
                let role_str = match role_clone {
                    AgentRole::Orchestrator => "orchestrator",
                    AgentRole::Optimizer => "optimizer",
                    AgentRole::Reviewer => "reviewer",
                    AgentRole::Executor => "executor",
                };

                // Create agent instance
                let agent = create_fn.call1((role_str,)).map_err(|e| {
                    error!("Failed to create agent for role {:?}: {}", role_clone, e);
                    MnemosyneError::Other(format!("Agent creation failed: {}", e))
                })?;

                info!("Python agent recreated for role: {:?}", role_clone);

                Ok::<Py<PyAny>, MnemosyneError>(agent.unbind().into())
            })
        })
        .await
        .map_err(|e| {
            error!("Tokio spawn_blocking failed: {}", e);
            MnemosyneError::Other(format!("Async execution failed: {}", e))
        })??;

        // Replace agent instance
        {
            let mut agent = self.agent.lock().await;
            *agent = new_agent;
        }

        // Restart session
        self.start_session().await?;

        // Reset error count
        self.reset_errors().await;

        // Broadcast restart event (specific restart event, not just "started")
        let event = Event::agent_restarted(
            self.agent_id.clone(),
            "Automatic restart after error threshold exceeded".to_string(),
        );
        if let Err(e) = self.event_tx.send(event) {
            warn!("Failed to broadcast agent restart event: {}", e);
        }

        // Also broadcast agent started to update state
        let event = Event::agent_started(self.agent_id.clone());
        if let Err(e) = self.event_tx.send(event) {
            warn!("Failed to broadcast agent started event: {}", e);
        }

        info!("Python agent {} restarted successfully", self.agent_id);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::EventBroadcaster;

    #[test]
    fn test_work_item_to_python_conversion() {
        Python::with_gil(|py| {
            let work_item = WorkItem::new(
                "Test work".to_string(),
                AgentRole::Executor,
                Phase::PlanToArtifacts,
                5,
            );

            let py_dict = work_item_to_python(py, &work_item).unwrap();
            let py_dict_bound: &Bound<PyDict> = py_dict.downcast_bound(py).unwrap();

            // Verify fields
            assert!(py_dict_bound.contains("id").unwrap());
            assert!(py_dict_bound.contains("description").unwrap());
            assert!(py_dict_bound.contains("phase").unwrap());
            assert!(py_dict_bound.contains("priority").unwrap());

            let description: String = py_dict_bound
                .get_item("description")
                .unwrap()
                .unwrap()
                .extract()
                .unwrap();
            assert_eq!(description, "Test work");

            let priority: u8 = py_dict_bound
                .get_item("priority")
                .unwrap()
                .unwrap()
                .extract()
                .unwrap();
            assert_eq!(priority, 5);
        });
    }

    #[test]
    fn test_extract_work_result() {
        Python::with_gil(|py| {
            // Create mock Python result dict
            let py_dict = PyDict::new_bound(py);
            py_dict.set_item("success", true).unwrap();
            py_dict
                .set_item("data", "some result data")
                .unwrap();
            py_dict
                .set_item("memory_ids", PyList::new_bound(py, &["mem-1", "mem-2"]))
                .unwrap();

            let item_id = crate::orchestration::state::WorkItemId::new();
            let result = extract_work_result(py, &py_dict, item_id.clone()).unwrap();

            assert!(result.success);
            assert_eq!(result.data, Some("some result data".to_string()));
            assert_eq!(result.memory_ids.len(), 2);
            assert_eq!(result.item_id, item_id);
        });
    }

    #[tokio::test]
    async fn test_bridge_lifecycle() {
        // Note: This test requires Python environment with agent_factory module
        // Skip if not available
        if std::env::var("SKIP_PYTHON_TESTS").is_ok() {
            return;
        }

        let broadcaster = EventBroadcaster::new(10);

        // Test spawning bridge
        let bridge_result =
            ClaudeAgentBridge::spawn(AgentRole::Executor, broadcaster.sender()).await;

        if let Ok(bridge) = bridge_result {
            // Test state
            let state = bridge.get_state().await;
            assert_eq!(state, AgentState::Idle);

            // Test shutdown
            bridge.shutdown().await.unwrap();
        }
    }
}
