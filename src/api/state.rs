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

/// Agent health information (for Python bridges)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHealth {
    /// Error count
    pub error_count: usize,
    /// Last error timestamp
    pub last_error: Option<DateTime<Utc>>,
    /// Is healthy (below error threshold)
    pub is_healthy: bool,
    /// Last restart timestamp
    pub last_restart: Option<DateTime<Utc>>,
}

impl Default for AgentHealth {
    fn default() -> Self {
        Self {
            error_count: 0,
            last_error: None,
            is_healthy: true,
            last_restart: None,
        }
    }
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
    /// Health information (for Python bridges)
    #[serde(default)]
    pub health: Option<AgentHealth>,
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
    /// Time-series metrics collector
    metrics: Arc<RwLock<crate::api::MetricsCollector>>,
}

impl StateManager {
    /// Create new state manager
    pub fn new() -> Self {
        Self {
            agents: Arc::new(RwLock::new(HashMap::new())),
            context_files: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(crate::api::MetricsCollector::new())),
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
        let mut completed_count = 0;
        let mut failed_count = 0;

        for agent in agents.values() {
            match agent.state {
                AgentState::Active { .. } => active_count += 1,
                AgentState::Idle => idle_count += 1,
                AgentState::Waiting { .. } => waiting_count += 1,
                AgentState::Completed { .. } => completed_count += 1,
                AgentState::Failed { .. } => failed_count += 1,
            }
        }

        // Update metrics with current agent state counts
        let agent_counts = crate::api::AgentStateCounts {
            active: active_count,
            idle: idle_count,
            waiting: waiting_count,
            completed: completed_count,
            failed: failed_count,
            total: agents.len(),
        };

        let mut metrics = self.metrics.write().await;
        metrics.update_agent_states(agent_counts);

        StateStats {
            total_agents: agents.len(),
            active_agents: active_count,
            idle_agents: idle_count,
            waiting_agents: waiting_count,
            context_files: files.len(),
        }
    }

    /// Get current metrics snapshot
    pub async fn metrics_snapshot(&self) -> crate::api::MetricsSnapshot {
        let metrics = self.metrics.read().await;
        metrics.snapshot()
    }

    /// Get metrics collector (for time-series data)
    pub async fn metrics(&self) -> crate::api::MetricsCollector {
        let metrics = self.metrics.read().await;
        metrics.clone()
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
        let metrics = self.metrics.clone();

        tokio::spawn(async move {
            tracing::info!("StateManager subscribed to event stream");

            while let Ok(event) = event_rx.recv().await {
                if let Err(e) = Self::apply_event_static(event, &agents, &context_files, &metrics).await {
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
        metrics: &Arc<RwLock<crate::api::MetricsCollector>>,
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
                        health: Some(AgentHealth::default()),  // Initialize healthy
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
            EventType::AgentErrorRecorded {
                agent_id,
                error_count,
                ..
            } => {
                let mut agents_map = agents.write().await;
                if let Some(agent) = agents_map.get_mut(&agent_id) {
                    // Update or create health info
                    let mut health = agent.health.take().unwrap_or_default();
                    health.error_count = error_count;
                    health.last_error = Some(Utc::now());
                    health.is_healthy = error_count < 5;
                    agent.health = Some(health);
                    agent.updated_at = Utc::now();
                    tracing::debug!("State updated: agent error recorded (count: {})", error_count);
                }
            }
            EventType::AgentHealthDegraded {
                agent_id,
                error_count,
                is_healthy,
                ..
            } => {
                let mut agents_map = agents.write().await;
                if let Some(agent) = agents_map.get_mut(&agent_id) {
                    let mut health = agent.health.take().unwrap_or_default();
                    health.error_count = error_count;
                    health.is_healthy = is_healthy;
                    agent.health = Some(health);
                    agent.updated_at = Utc::now();
                    tracing::warn!(
                        "State updated: agent health degraded (errors: {}, healthy: {})",
                        error_count,
                        is_healthy
                    );
                }
            }
            EventType::AgentRestarted { agent_id, .. } => {
                let mut agents_map = agents.write().await;
                if let Some(agent) = agents_map.get_mut(&agent_id) {
                    // Reset health on restart
                    let mut health = agent.health.take().unwrap_or_default();
                    health.error_count = 0;
                    health.last_error = None;
                    health.is_healthy = true;
                    health.last_restart = Some(Utc::now());
                    agent.health = Some(health);
                    agent.state = AgentState::Idle;
                    agent.updated_at = Utc::now();
                    tracing::info!("State updated: agent restarted and reset to healthy");
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
                            health: Some(AgentHealth::default()),  // Initialize healthy
                        },
                    );
                    tracing::debug!(
                        "State initialized: agent {} auto-created from heartbeat",
                        instance_id
                    );
                }
            }
            EventType::MemoryStored { .. } => {
                // Record in metrics
                let mut metrics_guard = metrics.write().await;
                metrics_guard.record_memory_store();
                tracing::trace!("Memory stored event received");
            }
            EventType::MemoryRecalled { .. } => {
                // Record in metrics
                let mut metrics_guard = metrics.write().await;
                metrics_guard.record_memory_recall();
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
                let mut metrics_guard = metrics.write().await;
                let mut current_work = metrics_guard.work_series().latest().cloned().unwrap_or_default();
                current_work.current_phase = to.clone();
                metrics_guard.update_work(current_work);
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
            EventType::WorkItemAssigned {
                agent_id,
                item_id,
                task,
                ..
            } => {
                let mut agents_map = agents.write().await;
                if let Some(agent) = agents_map.get_mut(&agent_id) {
                    agent.state = AgentState::Active { task: task.clone() };
                    agent.updated_at = Utc::now();
                    tracing::debug!(
                        "State updated: agent {} assigned work item {} ({})",
                        agent_id,
                        item_id,
                        task
                    );
                }
            }
            EventType::WorkItemCompleted {
                agent_id, item_id, ..
            } => {
                let mut agents_map = agents.write().await;
                if let Some(agent) = agents_map.get_mut(&agent_id) {
                    agent.state = AgentState::Idle;
                    agent.updated_at = Utc::now();
                    tracing::debug!(
                        "State updated: agent {} completed work item {}",
                        agent_id,
                        item_id
                    );
                }
                // Update work progress metrics
                let mut metrics_guard = metrics.write().await;
                let mut current_work = metrics_guard.work_series().latest().cloned().unwrap_or_default();
                current_work.completed_tasks += 1;
                metrics_guard.update_work(current_work);
            }
            // Skill events
            EventType::SkillLoaded { skill_name, agent_id, .. } => {
                tracing::debug!("Skill loaded: {} by {:?}", skill_name, agent_id);
                let mut metrics_guard = metrics.write().await;
                let mut current_skills = metrics_guard.skills_series().latest().cloned().unwrap_or_default();
                if !current_skills.loaded_skills.contains(&skill_name) {
                    current_skills.loaded_skills.push(skill_name.clone());
                    current_skills.loaded_skills.sort();
                }
                *current_skills.usage_counts.entry(skill_name.clone()).or_insert(0) += 1;
                metrics_guard.update_skills(current_skills);
            }
            EventType::SkillUnloaded { skill_name, reason, .. } => {
                tracing::debug!("Skill unloaded: {} ({})", skill_name, reason);
                let mut metrics_guard = metrics.write().await;
                let mut current_skills = metrics_guard.skills_series().latest().cloned().unwrap_or_default();
                current_skills.loaded_skills.retain(|s| s != &skill_name);
                metrics_guard.update_skills(current_skills);
            }
            EventType::SkillUsed { skill_name, agent_id, .. } => {
                tracing::trace!("Skill used: {} by {}", skill_name, agent_id);
                let mut metrics_guard = metrics.write().await;
                let mut current_skills = metrics_guard.skills_series().latest().cloned().unwrap_or_default();
                *current_skills.usage_counts.entry(skill_name.clone()).or_insert(0) += 1;
                current_skills.recently_used.push((skill_name.clone(), Utc::now()));
                // Keep only last 20 recent uses
                if current_skills.recently_used.len() > 20 {
                    current_skills.recently_used.remove(0);
                }
                metrics_guard.update_skills(current_skills);
            }
            EventType::SkillCompositionDetected { skills, task_description, .. } => {
                tracing::info!("Skill composition: {:?} for '{}'", skills, task_description);
            }

            // Memory evolution events
            EventType::MemoryEvolutionStarted { reason, .. } => {
                tracing::info!("Memory evolution started: {}", reason);
                let mut metrics_guard = metrics.write().await;
                let mut current_memory = metrics_guard.memory_ops_series().latest().cloned().unwrap_or_default();
                current_memory.evolutions_total += 1;
                metrics_guard.update_memory_rates(
                    current_memory.evolutions_total,
                    current_memory.consolidations_total,
                    current_memory.graph_nodes,
                );
            }
            EventType::MemoryConsolidated { source_ids, target_id, .. } => {
                tracing::debug!("Memory consolidated: {:?} → {}", source_ids, target_id);
                let mut metrics_guard = metrics.write().await;
                let mut current_memory = metrics_guard.memory_ops_series().latest().cloned().unwrap_or_default();
                current_memory.consolidations_total += 1;
                metrics_guard.update_memory_rates(
                    current_memory.evolutions_total,
                    current_memory.consolidations_total,
                    current_memory.graph_nodes,
                );
            }
            EventType::MemoryDecayed { memory_id, old_importance, new_importance, .. } => {
                tracing::trace!(
                    "Memory decayed: {} ({} → {})",
                    memory_id,
                    old_importance,
                    new_importance
                );
            }
            EventType::MemoryArchived { memory_id, reason, .. } => {
                tracing::debug!("Memory archived: {} ({})", memory_id, reason);
            }

            // Agent interaction events
            EventType::AgentHandoff { from_agent, to_agent, task_description, .. } => {
                tracing::info!("Agent handoff: {} → {} ({})", from_agent, to_agent, task_description);
                // Update both agents' states
                let mut agents_map = agents.write().await;
                if let Some(from) = agents_map.get_mut(&from_agent) {
                    from.state = AgentState::Idle;
                    from.updated_at = Utc::now();
                }
                if let Some(to) = agents_map.get_mut(&to_agent) {
                    to.state = AgentState::Active { task: task_description };
                    to.updated_at = Utc::now();
                }
            }
            EventType::AgentBlocked { agent_id, blocked_on, reason, .. } => {
                let mut agents_map = agents.write().await;
                if let Some(agent) = agents_map.get_mut(&agent_id) {
                    agent.state = AgentState::Waiting { reason: format!("Blocked on {}: {}", blocked_on, reason) };
                    agent.updated_at = Utc::now();
                    tracing::debug!("Agent {} blocked on {}", agent_id, blocked_on);
                }
            }
            EventType::AgentUnblocked { agent_id, unblocked_by, .. } => {
                let mut agents_map = agents.write().await;
                if let Some(agent) = agents_map.get_mut(&agent_id) {
                    agent.state = AgentState::Idle;
                    agent.updated_at = Utc::now();
                    tracing::debug!("Agent {} unblocked by {}", agent_id, unblocked_by);
                }
            }
            EventType::SubAgentSpawned { parent_agent, sub_agent, task_description, .. } => {
                let mut agents_map = agents.write().await;
                // Create sub-agent
                agents_map.insert(
                    sub_agent.clone(),
                    AgentInfo {
                        id: sub_agent.clone(),
                        state: AgentState::Active { task: task_description.clone() },
                        updated_at: Utc::now(),
                        metadata: HashMap::from([("parent".to_string(), parent_agent.clone())]),
                        health: Some(AgentHealth::default()),
                    },
                );
                tracing::info!("Sub-agent {} spawned by {} for '{}'", sub_agent, parent_agent, task_description);
            }

            // Work orchestration events
            EventType::ParallelStreamStarted { stream_id, task_count, .. } => {
                tracing::info!("Parallel stream {} started with {} tasks", stream_id, task_count);
                let mut metrics_guard = metrics.write().await;
                let mut current_work = metrics_guard.work_series().latest().cloned().unwrap_or_default();
                current_work.parallel_streams.push(format!("{} ({} tasks)", stream_id, task_count));
                current_work.total_tasks += task_count;
                metrics_guard.update_work(current_work);
            }
            EventType::CriticalPathUpdated { path_items, estimated_completion, .. } => {
                tracing::info!(
                    "Critical path updated: {} items, ETA: {}",
                    path_items.len(),
                    estimated_completion
                );
                let mut metrics_guard = metrics.write().await;
                let mut current_work = metrics_guard.work_series().latest().cloned().unwrap_or_default();
                // Calculate progress based on path items completion
                let completed = path_items.iter().filter(|item| item.contains("✓")).count();
                current_work.critical_path_progress = if !path_items.is_empty() {
                    (completed as f32 / path_items.len() as f32) * 100.0
                } else {
                    0.0
                };
                metrics_guard.update_work(current_work);
            }
            EventType::TypedHoleFilled { hole_name, component_a, component_b, .. } => {
                tracing::info!(
                    "Typed hole filled: {} ({} ↔ {})",
                    hole_name,
                    component_a,
                    component_b
                );
            }

            EventType::HealthUpdate { .. }
            | EventType::SessionStarted { .. }
            | EventType::CliCommandStarted { .. }
            | EventType::CliCommandCompleted { .. }
            | EventType::CliCommandFailed { .. }
            | EventType::SearchPerformed { .. }
            | EventType::DatabaseOperation { .. } => {
                // System-level and CLI operation events, no state update needed
                // These are displayed in the Operations panel, not in agent state
                tracing::trace!("System/CLI event received (no state update)");
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
            health: None,
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
        let metrics = manager.metrics.clone();

        StateManager::apply_event_static(event, &agents, &context_files, &metrics)
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

        StateManager::apply_event_static(event2, &agents, &context_files, &metrics)
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
