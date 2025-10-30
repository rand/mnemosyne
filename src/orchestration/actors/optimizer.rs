//! Optimizer Actor
//!
//! Responsibilities:
//! - Context optimization and budget management
//! - Dynamic skill discovery from filesystem
//! - Memory loading for work items
//! - Context monitoring (75% threshold triggers)
//! - Context compaction and checkpointing

use crate::error::Result;
use crate::launcher::agents::AgentRole;
use crate::orchestration::events::{AgentEvent, EventPersistence};
use crate::orchestration::messages::{OptimizerMessage, OrchestratorMessage};
use crate::orchestration::state::WorkItemId;
use crate::storage::StorageBackend;
use crate::types::{MemoryId, Namespace};
use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::sync::Arc;
use std::time::Duration;

/// Context budget allocation percentages
const CRITICAL_BUDGET: f32 = 0.40;
const SKILLS_BUDGET: f32 = 0.30;
const PROJECT_BUDGET: f32 = 0.20;
const GENERAL_BUDGET: f32 = 0.10;

/// Context threshold for triggering preservation
const CONTEXT_THRESHOLD: f32 = 0.75;

/// Optimizer actor state
pub struct OptimizerState {
    /// Event persistence
    events: EventPersistence,

    /// Storage backend
    storage: Arc<dyn StorageBackend>,

    /// Reference to Orchestrator
    orchestrator: Option<ActorRef<OrchestratorMessage>>,

    /// Current context usage (0.0-1.0)
    context_usage: f32,

    /// Context budget (total tokens)
    context_budget: usize,

    /// Loaded skill count
    loaded_skills: usize,

    /// Max skills to load
    max_skills: usize,

    /// Context monitoring interval
    monitor_interval: Duration,

    // Real metrics tracking
    /// Loaded memory IDs (tracks project context)
    loaded_memories: Vec<MemoryId>,

    /// Average tokens per memory (for estimation)
    avg_memory_tokens: usize,

    /// Estimated tokens per skill
    tokens_per_skill: usize,

    /// Critical context tokens (always loaded)
    critical_tokens: usize,
}

impl OptimizerState {
    pub fn new(storage: Arc<dyn StorageBackend>, namespace: Namespace) -> Self {
        Self {
            events: EventPersistence::new(storage.clone(), namespace),
            storage,
            orchestrator: None,
            context_usage: 0.0,
            context_budget: 200_000, // 200K tokens
            loaded_skills: 0,
            max_skills: 7,
            monitor_interval: Duration::from_secs(5),
            loaded_memories: Vec::new(),
            avg_memory_tokens: 500,  // Estimated average tokens per memory
            tokens_per_skill: 3000,  // Estimated tokens per skill (~300 lines)
            critical_tokens: 80_000, // CRITICAL_BUDGET * context_budget
        }
    }

    pub fn register_orchestrator(&mut self, orchestrator: ActorRef<OrchestratorMessage>) {
        self.orchestrator = Some(orchestrator);
    }
}

/// Optimizer actor implementation
pub struct OptimizerActor {
    storage: Arc<dyn StorageBackend>,
    namespace: Namespace,
}

impl OptimizerActor {
    pub fn new(storage: Arc<dyn StorageBackend>, namespace: Namespace) -> Self {
        Self { storage, namespace }
    }

    /// Discover skills for a task
    async fn discover_skills(
        state: &mut OptimizerState,
        task_description: String,
        max_skills: usize,
    ) -> Result<Vec<String>> {
        tracing::info!("Discovering skills for: {}", task_description);

        // TODO: Implement skill discovery from filesystem
        // For now, return empty list
        let skills = Vec::new();

        state.loaded_skills = skills.len().min(max_skills);

        tracing::info!("Discovered {} skills", state.loaded_skills);
        Ok(skills)
    }

    /// Load context memories for a work item
    async fn load_context_memories(
        state: &mut OptimizerState,
        work_item_id: WorkItemId,
        query: String,
    ) -> Result<Vec<MemoryId>> {
        tracing::info!("Loading context memories for: {}", query);

        // Search for relevant memories
        let results = state
            .storage
            .hybrid_search(&query, None, 10, false)
            .await?;

        let memory_ids: Vec<MemoryId> = results
            .into_iter()
            .map(|r| r.memory.id)
            .collect();

        // Track loaded memories for real metrics
        state.loaded_memories.extend(memory_ids.clone());
        state.loaded_memories.dedup();  // Remove duplicates

        tracing::info!("Loaded {} memories (total: {})", memory_ids.len(), state.loaded_memories.len());

        // Persist event
        state
            .events
            .persist(AgentEvent::MessageSent {
                from: AgentRole::Optimizer,
                to: AgentRole::Orchestrator,
                message_type: "context_loaded".to_string(),
            })
            .await?;

        Ok(memory_ids)
    }

    /// Monitor context usage
    async fn monitor_context(state: &mut OptimizerState) -> Result<()> {
        // Calculate actual context usage based on loaded resources

        // Critical tokens (CLAUDE.md, system prompts, instructions)
        let critical_used = state.critical_tokens;

        // Skills tokens (actual loaded skills × tokens per skill)
        let skills_used = state.loaded_skills * state.tokens_per_skill;

        // Project tokens (actual loaded memories × avg tokens per memory)
        let project_used = state.loaded_memories.len() * state.avg_memory_tokens;

        // General tokens (estimated overhead: agent messages, state, etc.)
        // Use 10% of remaining budget
        let used_so_far = critical_used + skills_used + project_used;
        let general_used = (state.context_budget.saturating_sub(used_so_far) as f32 * 0.10) as usize;

        let total_used = critical_used + skills_used + project_used + general_used;
        state.context_usage = total_used as f32 / state.context_budget as f32;

        tracing::debug!(
            "Context usage: {:.1}% ({}/{})",
            state.context_usage * 100.0,
            total_used,
            state.context_budget
        );

        // Check threshold
        if state.context_usage >= CONTEXT_THRESHOLD {
            if let Some(ref orchestrator) = state.orchestrator {
                let _ = orchestrator
                    .cast(OrchestratorMessage::ContextThresholdReached {
                        current_pct: state.context_usage,
                    })
                    .map_err(|e| {
                        tracing::warn!("Failed to notify orchestrator: {:?}", e)
                    });
            }
        }

        Ok(())
    }

    /// Compact context by removing non-critical elements
    async fn compact_context(
        state: &mut OptimizerState,
        target_pct: f32,
    ) -> Result<()> {
        tracing::info!(
            "Compacting context from {:.1}% to {:.1}%",
            state.context_usage * 100.0,
            target_pct * 100.0
        );

        // Unload low-priority skills first (keep at least 3)
        if state.loaded_skills > 3 {
            let skills_to_unload = state.loaded_skills - 3;
            state.loaded_skills -= skills_to_unload;
            tracing::info!("Unloaded {} skills", skills_to_unload);
        }

        // Clear older memories if still over budget
        Self::monitor_context(state).await?;
        if state.context_usage > target_pct {
            let memories_to_remove = (state.loaded_memories.len() / 2).max(1);
            state.loaded_memories.drain(0..memories_to_remove);
            tracing::info!("Removed {} older memories", memories_to_remove);
        }

        // Recalculate usage
        Self::monitor_context(state).await?;

        tracing::info!("Context compacted to {:.1}%", state.context_usage * 100.0);

        Ok(())
    }

    /// Checkpoint context at threshold
    async fn checkpoint_context(
        state: &mut OptimizerState,
        reason: String,
    ) -> Result<()> {
        tracing::info!("Checkpointing context: {}", reason);

        // Create a checkpoint memory
        let checkpoint_content = format!(
            "Context checkpoint: usage={:.1}%, skills={}, reason={}",
            state.context_usage * 100.0,
            state.loaded_skills,
            reason
        );

        // Store checkpoint as a memory
        let memory = crate::types::MemoryNote {
            id: MemoryId::new(),
            namespace: state.events.namespace.clone(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            content: checkpoint_content.clone(),
            summary: "Context checkpoint".to_string(),
            keywords: vec!["checkpoint".to_string(), "context".to_string()],
            tags: vec!["optimization".to_string()],
            context: reason.clone(),
            memory_type: crate::types::MemoryType::AgentEvent,
            importance: 8,
            confidence: 1.0,
            links: vec![],
            related_files: vec![],
            related_entities: vec![],
            access_count: 0,
            last_accessed_at: chrono::Utc::now(),
            expires_at: None,
            is_archived: false,
            superseded_by: None,
            embedding: None,
            embedding_model: String::new(),
        };

        state.storage.store_memory(&memory).await?;

        // Persist event
        state
            .events
            .persist(AgentEvent::ContextCheckpoint {
                agent: AgentRole::Optimizer,
                usage_pct: state.context_usage,
                snapshot_id: memory.id.clone(),
                reason,
            })
            .await?;

        tracing::info!("Context checkpoint created: {}", memory.id);

        Ok(())
    }
}

#[ractor::async_trait]
impl Actor for OptimizerActor {
    type Msg = OptimizerMessage;
    type State = OptimizerState;
    type Arguments = (Arc<dyn StorageBackend>, Namespace);

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        args: Self::Arguments,
    ) -> std::result::Result<Self::State, ActorProcessingErr> {
        tracing::info!("Optimizer actor starting");
        let (storage, namespace) = args;
        Ok(OptimizerState::new(storage, namespace))
    }

    async fn post_start(
        &self,
        myself: ActorRef<Self::Msg>,
        _state: &mut Self::State,
    ) -> std::result::Result<(), ActorProcessingErr> {
        tracing::info!("Optimizer actor started: {:?}", myself.get_id());

        // Start periodic context monitoring
        let myself_clone = myself.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(5));
            loop {
                interval.tick().await;
                let _ = myself_clone.cast(OptimizerMessage::MonitorContext);
            }
        });

        Ok(())
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> std::result::Result<(), ActorProcessingErr> {
        match message {
            OptimizerMessage::Initialize => {
                tracing::info!("Optimizer initialized");
            }
            OptimizerMessage::DiscoverSkills {
                task_description,
                max_skills,
            } => {
                Self::discover_skills(state, task_description, max_skills)
                    .await
                    .map_err(|e| ActorProcessingErr::from(e.to_string()))?;
            }
            OptimizerMessage::LoadContextMemories {
                work_item_id,
                query,
            } => {
                Self::load_context_memories(state, work_item_id, query)
                    .await
                    .map_err(|e| ActorProcessingErr::from(e.to_string()))?;
            }
            OptimizerMessage::MonitorContext => {
                Self::monitor_context(state)
                    .await
                    .map_err(|e| ActorProcessingErr::from(e.to_string()))?;
            }
            OptimizerMessage::CompactContext { target_pct } => {
                Self::compact_context(state, target_pct)
                    .await
                    .map_err(|e| ActorProcessingErr::from(e.to_string()))?;
            }
            OptimizerMessage::CheckpointContext { reason } => {
                Self::checkpoint_context(state, reason)
                    .await
                    .map_err(|e| ActorProcessingErr::from(e.to_string()))?;
            }
        }

        Ok(())
    }

    async fn post_stop(
        &self,
        _myself: ActorRef<Self::Msg>,
        _state: &mut Self::State,
    ) -> std::result::Result<(), ActorProcessingErr> {
        tracing::info!("Optimizer actor stopped");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LibsqlStorage;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_optimizer_lifecycle() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let storage = Arc::new(
            LibsqlStorage::new_with_validation(
                crate::ConnectionMode::Local(db_path.to_str().unwrap().to_string()),
                true, // create_if_missing
            )
            .await
            .expect("Failed to create test storage"),
        );

        let namespace = Namespace::Session {
            project: "test".to_string(),
            session_id: "test-session".to_string(),
        };

        let (actor_ref, _handle) = Actor::spawn(
            None,
            OptimizerActor::new(storage.clone(), namespace.clone()),
            (storage, namespace),
        )
        .await
        .unwrap();

        actor_ref.cast(OptimizerMessage::Initialize).unwrap();
        actor_ref.stop(None);
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
