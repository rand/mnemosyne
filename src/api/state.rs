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
