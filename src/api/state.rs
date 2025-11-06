//! State coordination for agents and context

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Agent state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    /// Agent is idle
    Idle,
    /// Agent is active/running
    Active { task: String },
    /// Agent is waiting (blocked)
    Waiting { reason: String },
    /// Agent completed
    Completed { result: String },
    /// Agent failed
    Failed { error: String },
}

/// Agent information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    /// Agent ID
    pub id: String,
    /// Agent state
    pub state: AgentState,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Agent metadata
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Context file state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextFile {
    /// File path
    pub path: String,
    /// Last modified timestamp
    pub modified_at: DateTime<Utc>,
    /// Validation errors (if any)
    #[serde(default)]
    pub errors: Vec<String>,
}

/// State manager for coordinating agents and context
pub struct StateManager {
    /// Active agents registry
    agents: Arc<RwLock<HashMap<String, AgentInfo>>>,
    /// Context files being tracked
    context_files: Arc<RwLock<HashMap<String, ContextFile>>>,
}

impl StateManager {
    /// Create new state manager
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            context_files: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register or update an agent
    pub async fn update_agent(&self, agent: AgentInfo) {
        let mut agents = self.agents.write().await;
        agents.insert(agent.id.clone(), agent);
    }

    /// Get agent by ID
    pub async fn get_agent(&self, id: &str) -> Option<AgentInfo> {
        let agents = self.agents.read().await;
        agents.get(id).cloned()
    }

    /// List all agents
    pub async fn list_agents(&self) -> Vec<AgentInfo> {
        let agents = self.agents.read().await;
        agents.values().cloned().collect()
    }

    /// Remove agent
    pub async fn remove_agent(&self, id: &str) -> Option<AgentInfo> {
        let mut agents = self.agents.write().await;
        agents.remove(id)
    }

    /// Register or update a context file
    pub async fn update_context_file(&self, file: ContextFile) {
        let mut files = self.context_files.write().await;
        files.insert(file.path.clone(), file);
    }

    /// Get context file by path
    pub async fn get_context_file(&self, path: &str) -> Option<ContextFile> {
        let files = self.context_files.read().await;
        files.get(path).cloned()
    }

    /// List all context files
    pub async fn list_context_files(&self) -> Vec<ContextFile> {
        let files = self.context_files.read().await;
        files.values().cloned().collect()
    }

    /// Remove context file
    pub async fn remove_context_file(&self, path: &str) -> Option<ContextFile> {
        let mut files = self.context_files.write().await;
        files.remove(path)
    }

    /// Get statistics
    pub async fn stats(&self) -> StateStats {
        let agents = self.agents.read().await;
        let files = self.context_files.read().await;

        let mut active_count = 0;
        let mut idle_count = 0;
        let mut waiting_count = 0;

        for agent in agents.values() {
            match agent.state {
                AgentState::Active { .. } => active_count += 1,
                AgentState::Idle => idle_count += 1,
                AgentState::Waiting { .. } => waiting_count += 1,
                _ => {}
            }
        }

        StateStats {
            total_agents: agents.len(),
            active_agents: active_count,
            idle_agents: idle_count,
            waiting_agents: waiting_count,
            context_files: files.len(),
        }
    }

    /// Subscribe to event stream and automatically update state
    ///
    /// This creates a single source of truth: events drive state updates.
    /// Spawns a background task that receives events and projects them to state.
    pub fn subscribe_to_events(
        &self,
        mut event_rx: tokio::sync::broadcast::Receiver<crate::api::Event>,
    ) {
        let agents = self.agents.clone();
        let context_files = self.context_files.clone();

        tokio::spawn(async move {
            tracing::info!("StateManager subscribed to event stream");

            while let Ok(event) = event_rx.recv().await {
                if let Err(e) = Self::apply_event_static(event, &agents, &context_files).await {
                    tracing::warn!("Failed to apply event to state: {}", e);
                }
            }

            tracing::warn!("StateManager event subscription ended");
        });
    }

    /// Apply event to state (static version for spawned task)
    async fn apply_event_static(
        event: crate::api::Event,
        agents: &Arc<RwLock<HashMap<String, AgentInfo>>>,
        context_files: &Arc<RwLock<HashMap<String, ContextFile>>>,
    ) -> Result<(), String> {
        use crate::api::EventType;

        match event.event_type {
            EventType::AgentStarted { agent_id, task, .. } => {
                let mut agents_map = agents.write().await;
                agents_map.insert(
                    agent_id.clone(),
                    AgentInfo {
                        id: agent_id.clone(),
                        state: if let Some(task_desc) = task {
                            AgentState::Active { task: task_desc }
                        } else {
                            AgentState::Idle
                        },
                        updated_at: Utc::now(),
                        metadata: HashMap::new(),
                    },
                );
                tracing::debug!("State updated: agent {} started", agent_id);
            }
            EventType::AgentCompleted {
                agent_id, result, ..
            } => {
                let mut agents_map = agents.write().await;
                if let Some(agent) = agents_map.get_mut(&agent_id) {
                    agent.state = AgentState::Completed { result };
                    agent.updated_at = Utc::now();
                    tracing::debug!("State updated: agent completed");
                }
            }
            EventType::AgentFailed {
                agent_id, error, ..
            } => {
                let mut agents_map = agents.write().await;
                if let Some(agent) = agents_map.get_mut(&agent_id) {
                    agent.state = AgentState::Failed { error };
                    agent.updated_at = Utc::now();
                    tracing::debug!("State updated: agent failed");
                }
            }
            EventType::Heartbeat { instance_id, .. } => {
                let mut agents_map = agents.write().await;
                if let Some(agent) = agents_map.get_mut(&instance_id) {
                    agent.updated_at = Utc::now();
                    tracing::trace!("State updated: heartbeat from {}", instance_id);
                } else {
                    // Auto-create agent on first heartbeat (handles startup race conditions)
                    agents_map.insert(
                        instance_id.clone(),
                        AgentInfo {
                            id: instance_id.clone(),
                            state: AgentState::Idle,
                            updated_at: Utc::now(),
                            metadata: HashMap::new(),
                        },
                    );
                    tracing::debug!(
                        "State initialized: agent {} auto-created from heartbeat",
                        instance_id
                    );
                }
            }
            EventType::MemoryStored { .. } => {
                // Context activity - could track this as metadata in the future
                tracing::trace!("Memory stored event received");
            }
            EventType::MemoryRecalled { .. } => {
                // Context activity
                tracing::trace!("Memory recalled event received");
            }
            EventType::ContextModified { file, .. } => {
                let mut files_map = context_files.write().await;
                files_map.insert(
                    file.clone(),
                    ContextFile {
                        path: file,
                        modified_at: Utc::now(),
                        errors: vec![],
                    },
                );
                tracing::debug!("State updated: context file modified");
            }
            EventType::ContextValidated { file, errors, .. } => {
                let mut files_map = context_files.write().await;
                if let Some(context_file) = files_map.get_mut(&file) {
                    context_file.errors = errors;
                    context_file.modified_at = Utc::now();
                } else {
                    files_map.insert(
                        file.clone(),
                        ContextFile {
                            path: file,
                            modified_at: Utc::now(),
                            errors,
                        },
                    );
                }
                tracing::debug!("State updated: context file validated");
            }
            EventType::PhaseChanged { from, to, .. } => {
                tracing::info!("Phase transition: {} → {}", from, to);
                // Could add phase to metadata if needed
            }
            EventType::DeadlockDetected { blocked_items, .. } => {
                tracing::warn!("Deadlock detected: {} items blocked", blocked_items.len());
                // Could track deadlocks in state for dashboard display
            }
            EventType::ContextCheckpointed {
                agent_id,
                usage_percent,
                snapshot_id,
                ..
            } => {
                tracing::info!(
                    "Context checkpoint by {}: {}% usage, snapshot: {}",
                    agent_id,
                    usage_percent,
                    snapshot_id
                );
                // Could track checkpoints as agent metadata
            }
            EventType::ReviewFailed {
                item_id,
                issues,
                attempt,
                ..
            } => {
                tracing::warn!(
                    "Review failed for {}: {} issues (attempt {})",
                    item_id,
                    issues.len(),
                    attempt
                );
                // Could track review failures for dashboard metrics
            }
            EventType::WorkItemRetried {
                item_id,
                reason,
                attempt,
                ..
            } => {
                tracing::info!(
                    "Work item {} retried (attempt {}): {}",
                    item_id,
                    attempt,
                    reason
                );
                // Could track retries for dashboard metrics
            }
            EventType::HealthUpdate { .. } | EventType::SessionStarted { .. } => {
                // System-level events, no state update needed
                tracing::trace!("System event received (no state update)");
            }
        }

        Ok(())
    }
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
    }
}

/// State statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateStats {
    pub total_agents: usize,
    pub active_agents: usize,
    pub idle_agents: usize,
    pub waiting_agents: usize,
    pub context_files: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_agent_management() {
        let manager = StateManager::new();

        let agent = AgentInfo {
            id: "executor".to_string(),
            state: AgentState::Active {
                task: "test task".to_string(),
            },
            updated_at: Utc::now(),
            metadata: HashMap::new(),
        };

        manager.update_agent(agent.clone()).await;

        let retrieved = manager.get_agent("executor").await.unwrap();
        assert_eq!(retrieved.id, "executor");

        let stats = manager.stats().await;
        assert_eq!(stats.total_agents, 1);
        assert_eq!(stats.active_agents, 1);
    }

    #[tokio::test]
    async fn test_heartbeat_auto_creates_agent() {
        let manager = StateManager::new();

        // Verify agent doesn't exist initially
        assert!(manager.get_agent("test-agent").await.is_none());

        // Simulate heartbeat event (auto-create agent)
        let event = crate::api::Event::heartbeat("test-agent".to_string());
        let agents = manager.agents.clone();
        let context_files = manager.context_files.clone();

        StateManager::apply_event_static(event, &agents, &context_files)
            .await
            .unwrap();

        // Verify agent was auto-created
        let agent = manager.get_agent("test-agent").await.unwrap();
        assert_eq!(agent.id, "test-agent");
        assert!(matches!(agent.state, AgentState::Idle));

        // Verify second heartbeat updates existing agent
        let event2 = crate::api::Event::heartbeat("test-agent".to_string());
        let before_update = agent.updated_at;
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        StateManager::apply_event_static(event2, &agents, &context_files)
            .await
            .unwrap();

        let agent_after = manager.get_agent("test-agent").await.unwrap();
        assert!(agent_after.updated_at > before_update);

        // Verify stats reflect auto-created agent
        let stats = manager.stats().await;
        assert_eq!(stats.total_agents, 1);
        assert_eq!(stats.idle_agents, 1);
    }

    #[tokio::test]
    async fn test_context_file_tracking() {
        let manager = StateManager::new();

        let file = ContextFile {
            path: "context.md".to_string(),
            modified_at: Utc::now(),
            errors: vec![],
        };

        manager.update_context_file(file.clone()).await;

        let retrieved = manager.get_context_file("context.md").await.unwrap();
        assert_eq!(retrieved.path, "context.md");

        let stats = manager.stats().await;
        assert_eq!(stats.context_files, 1);
    }
}
