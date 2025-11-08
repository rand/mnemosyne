//! Agent Process Spawner
//!
//! Spawns and manages independent agent processes outside of Claude Code.
//! Agents run as Python processes and communicate via the API server.

use crate::api::{Event, EventBroadcaster, StateManager};
use crate::error::{MnemosyneError, Result};
use crate::launcher::agents::AgentRole;
use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Handle to a spawned agent process
#[derive(Debug)]
pub struct AgentHandle {
    /// Agent role
    pub role: AgentRole,
    /// Process ID
    pub pid: u32,
    /// Child process handle
    child: Child,
    /// Agent status
    pub status: AgentStatus,
}

/// Agent status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentStatus {
    /// Agent is starting up
    Starting,
    /// Agent is running
    Running,
    /// Agent has stopped
    Stopped,
    /// Agent has failed
    Failed,
}

/// Agent spawner configuration
#[derive(Debug, Clone)]
pub struct AgentSpawnerConfig {
    /// Python executable path
    pub python_path: String,
    /// Base directory for agent scripts
    pub agents_dir: String,
    /// API server URL for agent communication
    pub api_url: String,
    /// Database path
    pub database_path: String,
    /// Namespace for agent operations
    pub namespace: String,
}

impl Default for AgentSpawnerConfig {
    fn default() -> Self {
        Self {
            python_path: "/opt/homebrew/bin/python3".to_string(),
            agents_dir: "src/orchestration/agents".to_string(),
            api_url: "http://127.0.0.1:3000".to_string(),
            database_path: ".mnemosyne/orchestration.db".to_string(),
            namespace: "project:mnemosyne".to_string(),
        }
    }
}

/// Agent spawner - spawns and manages independent agent processes
pub struct AgentSpawner {
    /// Configuration
    config: AgentSpawnerConfig,
    /// Spawned agent handles
    agents: Arc<RwLock<HashMap<AgentRole, AgentHandle>>>,
    /// Event broadcaster for agent lifecycle events
    event_broadcaster: Option<Arc<EventBroadcaster>>,
    /// State manager for tracking agent state
    state_manager: Option<Arc<StateManager>>,
    /// Flag to track if shutdown_all() was called
    shutdown_called: Arc<AtomicBool>,
}

impl AgentSpawner {
    /// Create new agent spawner
    pub fn new(
        config: AgentSpawnerConfig,
        event_broadcaster: Option<Arc<EventBroadcaster>>,
        state_manager: Option<Arc<StateManager>>,
    ) -> Self {
        Self {
            config,
            agents: Arc::new(RwLock::new(HashMap::new())),
            event_broadcaster,
            state_manager,
            shutdown_called: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Spawn all four agents (Orchestrator, Optimizer, Reviewer, Executor)
    pub async fn spawn_all(&self) -> Result<Vec<AgentRole>> {
        info!("Spawning all agents...");

        let roles = vec![
            AgentRole::Orchestrator,
            AgentRole::Optimizer,
            AgentRole::Reviewer,
            AgentRole::Executor,
        ];

        let mut spawned = Vec::new();

        for role in &roles {
            match self.spawn_agent(*role).await {
                Ok(()) => {
                    spawned.push(*role);
                    info!("✓ Spawned {} agent", role.as_str());
                }
                Err(e) => {
                    error!("✗ Failed to spawn {} agent: {}", role.as_str(), e);
                    // Continue spawning other agents
                }
            }
        }

        // Wait a moment for agents to start
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        // Update all agent statuses to Running if they're still alive
        let mut agents_guard = self.agents.write().await;
        for handle in agents_guard.values_mut() {
            if handle.status == AgentStatus::Starting {
                match handle.child.try_wait() {
                    Ok(Some(_exit_status)) => {
                        // Process exited already
                        handle.status = AgentStatus::Failed;
                        error!("Agent {} exited immediately", handle.role.as_str());
                    }
                    Ok(None) => {
                        // Still running
                        handle.status = AgentStatus::Running;
                        debug!("Agent {} confirmed running", handle.role.as_str());
                    }
                    Err(e) => {
                        warn!("Failed to check agent {} status: {}", handle.role.as_str(), e);
                    }
                }
            }
        }

        info!("Successfully spawned {} out of {} agents", spawned.len(), roles.len());
        Ok(spawned)
    }

    /// Spawn a single agent process
    async fn spawn_agent(&self, role: AgentRole) -> Result<()> {
        let agent_script = format!("{}/{}.py", self.config.agents_dir, role.as_str());
        let agent_id = role.as_str();

        debug!("Spawning agent: {} from {}", agent_id, agent_script);

        // Check if script exists
        if !std::path::Path::new(&agent_script).exists() {
            return Err(MnemosyneError::Database(format!(
                "Agent script not found: {}",
                agent_script
            )));
        }

        // Spawn Python process
        let child = Command::new(&self.config.python_path)
            .arg(&agent_script)
            .arg("--agent-id")
            .arg(agent_id)
            .arg("--api-url")
            .arg(&self.config.api_url)
            .arg("--database")
            .arg(&self.config.database_path)
            .arg("--namespace")
            .arg(&self.config.namespace)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                MnemosyneError::Io(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Failed to spawn agent {}: {}", agent_id, e),
                ))
            })?;

        let pid = child
            .id();

        debug!("Agent {} spawned with PID: {}", agent_id, pid);

        // Create handle
        let handle = AgentHandle {
            role,
            pid,
            child,
            status: AgentStatus::Starting,
        };

        // Store handle
        let mut agents_guard = self.agents.write().await;
        agents_guard.insert(role, handle);
        drop(agents_guard);

        // Broadcast agent started event
        if let Some(broadcaster) = &self.event_broadcaster {
            let event = Event::agent_started(agent_id.to_string());
            let _ = broadcaster.broadcast(event);
        }

        // Register with state manager
        if let Some(state_manager) = &self.state_manager {
            let agent_info = crate::api::state::AgentInfo {
                id: agent_id.to_string(),
                state: crate::api::state::AgentState::Idle,
                updated_at: chrono::Utc::now(),
                metadata: [("pid".to_string(), pid.to_string())]
                    .iter()
                    .cloned()
                    .collect(),
                health: Some(crate::api::state::AgentHealth::default()),
            };
            state_manager.update_agent(agent_info).await;
        }

        info!("Agent {} (PID {}) registered with API server", agent_id, pid);

        Ok(())
    }

    /// Get PIDs of all spawned agents
    pub async fn get_pids(&self) -> HashMap<AgentRole, u32> {
        let agents = self.agents.read().await;
        agents
            .iter()
            .map(|(role, handle)| (*role, handle.pid))
            .collect()
    }

    /// Get status of all agents
    pub async fn get_statuses(&self) -> HashMap<AgentRole, AgentStatus> {
        let agents = self.agents.read().await;
        agents
            .iter()
            .map(|(role, handle)| (*role, handle.status))
            .collect()
    }

    /// Check if all agents are running
    pub async fn all_running(&self) -> bool {
        let agents = self.agents.read().await;
        agents.len() == 4
            && agents
                .values()
                .all(|handle| handle.status == AgentStatus::Running)
    }

    /// Shutdown all agent processes gracefully
    pub async fn shutdown_all(&self) -> Result<()> {
        info!("Shutting down all agents...");

        // Mark that shutdown was called explicitly
        self.shutdown_called.store(true, Ordering::SeqCst);

        let mut agents_guard = self.agents.write().await;

        for (role, handle) in agents_guard.iter_mut() {
            info!("Stopping {} agent (PID {})...", role.as_str(), handle.pid);

            // Try graceful shutdown first (SIGTERM)
            #[cfg(unix)]
            {
                use nix::sys::signal::{kill, Signal};
                use nix::unistd::Pid;

                let pid = Pid::from_raw(handle.pid as i32);
                match kill(pid, Signal::SIGTERM) {
                    Ok(_) => {
                        debug!("Sent SIGTERM to {} (PID {})", role.as_str(), handle.pid);
                    }
                    Err(nix::Error::ESRCH) => {
                        // Process already dead - this is fine
                        debug!("Agent {} (PID {}) already exited", role.as_str(), handle.pid);
                        handle.status = AgentStatus::Stopped;
                        continue; // Skip waiting, move to next agent
                    }
                    Err(e) => {
                        warn!("Failed to send SIGTERM to {}: {}", role.as_str(), e);
                    }
                }
            }

            // Wait for graceful shutdown (increased to 2s to allow Python cleanup)
            tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

            // Force kill if still running
            match handle.child.try_wait() {
                Ok(Some(status)) => {
                    info!("Agent {} exited: {:?}", role.as_str(), status);
                    handle.status = AgentStatus::Stopped;
                }
                Ok(None) => {
                    warn!("Agent {} did not stop gracefully, force killing", role.as_str());
                    if let Err(e) = handle.child.kill() {
                        error!("Failed to kill {}: {}", role.as_str(), e);
                    } else {
                        let _ = handle.child.wait();
                        handle.status = AgentStatus::Stopped;
                    }
                }
                Err(e) => {
                    error!("Error checking {} status: {}", role.as_str(), e);
                }
            }

            // Broadcast agent stopped event
            if let Some(broadcaster) = &self.event_broadcaster {
                let event = Event::agent_completed(role.as_str().to_string(), "Shutdown".to_string());
                let _ = broadcaster.broadcast(event);
            }
        }

        info!("All agents shutdown complete");
        Ok(())
    }

    /// Kill all agent processes immediately (for emergency cleanup)
    pub async fn kill_all(&self) -> Result<()> {
        warn!("Emergency kill of all agents");

        let mut agents_guard = self.agents.write().await;

        for (role, handle) in agents_guard.iter_mut() {
            if let Err(e) = handle.child.kill() {
                error!("Failed to kill {}: {}", role.as_str(), e);
            } else {
                let _ = handle.child.wait();
                handle.status = AgentStatus::Stopped;
            }
        }

        Ok(())
    }
}

impl Drop for AgentSpawner {
    fn drop(&mut self) {
        // Skip cleanup if shutdown_all() was already called
        if self.shutdown_called.load(Ordering::SeqCst) {
            debug!("AgentSpawner dropped after explicit shutdown - no cleanup needed");
            return;
        }

        // Emergency cleanup if spawner is dropped without explicit shutdown
        // Use try_write() instead of blocking_write() to avoid panics in async context
        if let Ok(mut agents) = self.agents.try_write() {
            if !agents.is_empty() {
                warn!("AgentSpawner dropped without calling shutdown_all()");
                for (role, handle) in agents.iter_mut() {
                    warn!("Emergency cleanup: killing {} (PID {})", role.as_str(), handle.pid);
                    let _ = handle.child.kill();
                    let _ = handle.child.wait();
                }
            }
        } else {
            // Lock is held, spawner is being used - skip emergency cleanup
            warn!("AgentSpawner dropped while lock is held, skipping emergency cleanup");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_spawner_creation() {
        let config = AgentSpawnerConfig::default();
        let spawner = AgentSpawner::new(config, None, None);

        let statuses = spawner.get_statuses().await;
        assert_eq!(statuses.len(), 0);
    }

    #[tokio::test]
    async fn test_config_defaults() {
        let config = AgentSpawnerConfig::default();
        assert!(config.python_path.contains("python"));
        assert!(config.agents_dir.contains("agents"));
        assert_eq!(config.api_url, "http://127.0.0.1:3000");
    }
}
