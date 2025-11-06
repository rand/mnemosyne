//! Branch Assignment Registry
//!
//! Tracks which agents are assigned to which git branches, along with their
//! work intent and coordination mode. This is NOT an exclusive locking system -
//! multiple agents can be assigned to the same branch when coordinated.
//!
//! # Key Features
//!
//! - **Assignment tracking**: Multiple agents per branch with different intents
//! - **Dynamic timeouts**: Calculate timeout based on work item complexity
//! - **Conflict detection**: Check for overlapping work intents
//! - **Persistence**: Save/load registry state for cross-process coordination
//! - **Auto-cleanup**: Remove stale assignments from crashed agents

use crate::error::{MnemosyneError, Result};
use crate::orchestration::identity::AgentId;
use crate::orchestration::state::{Phase, WorkItemId};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Work intent declaration - what does the agent plan to do?
///
/// Agents must declare their intent when requesting branch access.
/// This enables conflict detection and intelligent coordination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkIntent {
    /// Read-only access (auto-approved)
    ReadOnly,

    /// Write access to specific files/directories
    Write(Vec<PathBuf>),

    /// Unrestricted access to entire branch
    FullBranch,
}

impl WorkIntent {
    /// Check if this intent allows writes to a given path
    pub fn allows_write(&self, path: &Path) -> bool {
        match self {
            WorkIntent::ReadOnly => false,
            WorkIntent::FullBranch => true,
            WorkIntent::Write(paths) => paths.iter().any(|p| {
                // Check if path is under any of the allowed paths
                path.starts_with(p) || p.starts_with(path)
            }),
        }
    }

    /// Check if this intent is read-only
    pub fn is_readonly(&self) -> bool {
        matches!(self, WorkIntent::ReadOnly)
    }

    /// Get paths affected by this intent (empty for ReadOnly)
    pub fn affected_paths(&self) -> Vec<PathBuf> {
        match self {
            WorkIntent::ReadOnly => vec![],
            WorkIntent::Write(paths) => paths.clone(),
            WorkIntent::FullBranch => vec![PathBuf::from(".")],
        }
    }
}

/// Coordination mode - how should agents coordinate on this branch?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum CoordinationMode {
    /// Isolated: Agent is the only one on this branch (default)
    #[default]
    Isolated,

    /// Coordinated: Multiple agents allowed with conflict detection
    Coordinated,
}

/// Agent assignment to a branch
///
/// Represents one agent's assignment to a git branch, including:
/// - What they intend to do (read/write)
/// - How they coordinate with others
/// - When the assignment times out
/// - What work items they're handling
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentAssignment {
    /// Agent identity
    pub agent_id: AgentId,

    /// Git branch
    pub branch: String,

    /// Work intent (read-only, specific files, full branch)
    pub intent: WorkIntent,

    /// Coordination mode (isolated or coordinated)
    pub mode: CoordinationMode,

    /// When assignment was created
    pub assigned_at: DateTime<Utc>,

    /// Expected duration (calculated from work items)
    pub expected_duration: Option<Duration>,

    /// Active work items for this assignment
    pub work_items: Vec<WorkItemId>,

    /// Timeout for this assignment
    pub timeout: DateTime<Utc>,
}

impl AgentAssignment {
    /// Create a new assignment
    pub fn new(
        agent_id: AgentId,
        branch: String,
        intent: WorkIntent,
        mode: CoordinationMode,
    ) -> Self {
        let assigned_at = Utc::now();
        let timeout = assigned_at + Duration::hours(1); // Default 1 hour

        Self {
            agent_id,
            branch,
            intent,
            mode,
            assigned_at,
            expected_duration: None,
            work_items: Vec::new(),
            timeout,
        }
    }

    /// Check if assignment has timed out
    pub fn is_timed_out(&self) -> bool {
        Utc::now() > self.timeout
    }

    /// Get time remaining until timeout
    pub fn time_remaining(&self) -> Duration {
        let now = Utc::now();
        if now >= self.timeout {
            Duration::zero()
        } else {
            self.timeout.signed_duration_since(now)
        }
    }

    /// Update timeout based on work items
    pub fn recalculate_timeout(&mut self, phases: &HashMap<WorkItemId, Phase>) {
        let duration = calculate_dynamic_timeout(&self.work_items, phases);
        self.expected_duration = Some(duration);
        self.timeout = self.assigned_at + duration;
    }
}

/// Calculate dynamic timeout based on work item complexity
///
/// Different phases have different expected durations:
/// - PromptToSpec: 0.5x (quick clarification)
/// - SpecToFullSpec: 1.0x (standard decomposition)
/// - FullSpecToPlan: 0.5x (planning is fast)
/// - PlanToArtifacts: 2.0x (implementation takes longer)
///
/// Base timeout is 1 hour, multiplied by sum of phase factors.
pub fn calculate_dynamic_timeout(
    work_items: &[WorkItemId],
    phases: &HashMap<WorkItemId, Phase>,
) -> Duration {
    let base_hours = 1.0;

    let complexity_factor: f64 = work_items
        .iter()
        .filter_map(|item_id| phases.get(item_id))
        .map(|phase| match phase {
            Phase::PromptToSpec => 0.5,
            Phase::SpecToFullSpec => 1.0,
            Phase::FullSpecToPlan => 0.5,
            Phase::PlanToArtifacts => 2.0,
            Phase::Complete => 0.0,
        })
        .sum();

    // At least 1x factor, even if no work items
    let factor = complexity_factor.max(1.0);

    Duration::milliseconds((base_hours * factor * 3600.0 * 1000.0) as i64)
}

/// Conflict report describing overlapping work
#[derive(Debug, Clone)]
pub struct ConflictReport {
    /// Conflicting agents
    pub agents: Vec<AgentId>,

    /// Overlapping paths
    pub overlapping_paths: Vec<PathBuf>,

    /// Conflict description
    pub reason: String,
}

/// Branch assignment registry
///
/// Central registry tracking all agent-to-branch assignments.
/// Thread-safe with RwLock for concurrent access.
pub struct BranchRegistry {
    /// Map: Branch -> List of assignments
    assignments: HashMap<String, Vec<AgentAssignment>>,

    /// Work item phases for timeout calculation
    phases: HashMap<WorkItemId, Phase>,

    /// Path to persist registry state
    persistence_path: Option<PathBuf>,
}

impl BranchRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            assignments: HashMap::new(),
            phases: HashMap::new(),
            persistence_path: None,
        }
    }

    /// Create a registry with persistence
    pub fn with_persistence(path: PathBuf) -> Self {
        Self {
            assignments: HashMap::new(),
            phases: HashMap::new(),
            persistence_path: Some(path),
        }
    }

    /// Assign agent to branch
    ///
    /// # Arguments
    ///
    /// * `agent_id` - Agent to assign
    /// * `branch` - Git branch name
    /// * `intent` - What the agent intends to do
    /// * `mode` - Coordination mode (isolated or coordinated)
    ///
    /// # Returns
    ///
    /// Ok(()) if successful, Err if conflict exists and mode is Isolated
    pub fn assign_agent(
        &mut self,
        agent_id: AgentId,
        branch: String,
        intent: WorkIntent,
        mode: CoordinationMode,
    ) -> Result<()> {
        // Check for conflicts if mode is Isolated
        if mode == CoordinationMode::Isolated {
            if let Some(assignments) = self.assignments.get(&branch) {
                if !assignments.is_empty() {
                    return Err(MnemosyneError::BranchConflict(format!(
                        "Branch '{}' already has {} agent(s) assigned in Isolated mode",
                        branch,
                        assignments.len()
                    )));
                }
            }
        }

        let assignment = AgentAssignment::new(agent_id, branch.clone(), intent, mode);

        self.assignments.entry(branch).or_default().push(assignment);

        self.persist()?;

        Ok(())
    }

    /// Get all assignments for a branch
    pub fn get_assignments(&self, branch: &str) -> Vec<AgentAssignment> {
        self.assignments.get(branch).cloned().unwrap_or_default()
    }

    /// Get assignment for specific agent
    pub fn get_agent_assignment(&self, agent_id: &AgentId) -> Option<AgentAssignment> {
        self.assignments
            .values()
            .flatten()
            .find(|a| &a.agent_id == agent_id)
            .cloned()
    }

    /// Update work items for an assignment
    pub fn update_work_items(
        &mut self,
        agent_id: &AgentId,
        work_items: Vec<WorkItemId>,
    ) -> Result<()> {
        for assignments in self.assignments.values_mut() {
            if let Some(assignment) = assignments.iter_mut().find(|a| &a.agent_id == agent_id) {
                assignment.work_items = work_items;
                assignment.recalculate_timeout(&self.phases);
                self.persist()?;
                return Ok(());
            }
        }

        Err(MnemosyneError::NotFound(format!(
            "No assignment found for agent {}",
            agent_id
        )))
    }

    /// Update phase for a work item (affects timeout calculations)
    pub fn update_phase(&mut self, work_item_id: WorkItemId, phase: Phase) -> Result<()> {
        self.phases.insert(work_item_id.clone(), phase);

        // Recalculate timeouts for all assignments with this work item
        for assignments in self.assignments.values_mut() {
            for assignment in assignments.iter_mut() {
                if assignment.work_items.contains(&work_item_id) {
                    assignment.recalculate_timeout(&self.phases);
                }
            }
        }

        self.persist()?;
        Ok(())
    }

    /// Release agent's assignment
    pub fn release_assignment(&mut self, agent_id: &AgentId) -> Result<()> {
        for assignments in self.assignments.values_mut() {
            assignments.retain(|a| &a.agent_id != agent_id);
        }

        self.persist()?;
        Ok(())
    }

    /// Check for conflicts between new intent and existing assignments
    pub fn check_conflict(&self, branch: &str, new_intent: &WorkIntent) -> Option<ConflictReport> {
        let assignments = self.get_assignments(branch);

        if assignments.is_empty() {
            return None;
        }

        // Collect overlapping paths
        let mut overlapping_paths = Vec::new();
        let mut conflicting_agents = Vec::new();

        for assignment in &assignments {
            match (&assignment.intent, new_intent) {
                // ReadOnly never conflicts
                (WorkIntent::ReadOnly, _) | (_, WorkIntent::ReadOnly) => continue,

                // FullBranch conflicts with any write
                (WorkIntent::FullBranch, _) | (_, WorkIntent::FullBranch) => {
                    conflicting_agents.push(assignment.agent_id.clone());
                    overlapping_paths.push(PathBuf::from("."));
                }

                // Check path overlaps for Write intents
                (WorkIntent::Write(existing_paths), WorkIntent::Write(new_paths)) => {
                    let overlaps: Vec<_> = existing_paths
                        .iter()
                        .filter(|ep| {
                            new_paths
                                .iter()
                                .any(|np| np.starts_with(ep) || ep.starts_with(np))
                        })
                        .cloned()
                        .collect();

                    if !overlaps.is_empty() {
                        conflicting_agents.push(assignment.agent_id.clone());
                        overlapping_paths.extend(overlaps);
                    }
                }
            }
        }

        if !conflicting_agents.is_empty() {
            Some(ConflictReport {
                agents: conflicting_agents,
                overlapping_paths,
                reason: format!(
                    "{} agent(s) have overlapping write intent on {}",
                    assignments.len(),
                    branch
                ),
            })
        } else {
            None
        }
    }

    /// Clean up timed-out assignments
    pub fn cleanup_timeouts(&mut self) -> Result<Vec<AgentId>> {
        let mut removed = Vec::new();

        for assignments in self.assignments.values_mut() {
            let timed_out: Vec<_> = assignments
                .iter()
                .filter(|a| a.is_timed_out())
                .map(|a| a.agent_id.clone())
                .collect();

            assignments.retain(|a| !a.is_timed_out());
            removed.extend(timed_out);
        }

        if !removed.is_empty() {
            self.persist()?;
        }

        Ok(removed)
    }

    /// Get all branches with active assignments
    pub fn active_branches(&self) -> Vec<String> {
        self.assignments
            .iter()
            .filter(|(_, assignments)| !assignments.is_empty())
            .map(|(branch, _)| branch.clone())
            .collect()
    }

    /// Get statistics
    pub fn stats(&self) -> RegistryStats {
        let total_assignments: usize = self.assignments.values().map(|a| a.len()).sum();
        let active_branches = self.active_branches().len();
        let isolated_count = self
            .assignments
            .values()
            .flatten()
            .filter(|a| a.mode == CoordinationMode::Isolated)
            .count();
        let coordinated_count = total_assignments - isolated_count;

        RegistryStats {
            total_assignments,
            active_branches,
            isolated_assignments: isolated_count,
            coordinated_assignments: coordinated_count,
        }
    }

    /// Persist registry to disk
    fn persist(&self) -> Result<()> {
        if let Some(path) = &self.persistence_path {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    MnemosyneError::Io(std::io::Error::new(
                        e.kind(),
                        format!("Failed to create registry directory: {}", e),
                    ))
                })?;
            }

            // Zero-copy serialization: write directly to BufWriter without cloning
            use std::io::BufWriter;
            let file = std::fs::File::create(path).map_err(|e| {
                MnemosyneError::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to create registry file: {}", e),
                ))
            })?;

            let writer = BufWriter::new(file);

            let data = RegistryDataRef {
                assignments: &self.assignments,
                phases: &self.phases,
            };

            // Use compact JSON (faster than pretty) for internal persistence
            serde_json::to_writer(writer, &data).map_err(|e| {
                MnemosyneError::Other(format!("Failed to serialize registry: {}", e))
            })?;
        }

        Ok(())
    }

    /// Load registry from disk
    pub fn load(path: &Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::with_persistence(path.to_path_buf()));
        }

        // Use BufReader for efficient reading
        use std::io::BufReader;
        let file = std::fs::File::open(path).map_err(|e| {
            MnemosyneError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to open registry: {}", e),
            ))
        })?;

        let reader = BufReader::new(file);
        let data: RegistryData = serde_json::from_reader(reader)
            .map_err(|e| MnemosyneError::Other(format!("Failed to deserialize registry: {}", e)))?;

        Ok(Self {
            assignments: data.assignments,
            phases: data.phases,
            persistence_path: Some(path.to_path_buf()),
        })
    }
}

impl Default for BranchRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Registry statistics
#[derive(Debug, Clone)]
pub struct RegistryStats {
    pub total_assignments: usize,
    pub active_branches: usize,
    pub isolated_assignments: usize,
    pub coordinated_assignments: usize,
}

/// Serializable registry data (for deserialization with owned data)
#[derive(Debug, Serialize, Deserialize)]
struct RegistryData {
    assignments: HashMap<String, Vec<AgentAssignment>>,
    phases: HashMap<WorkItemId, Phase>,
}

/// Registry data with references (for zero-copy serialization)
#[derive(Debug, Serialize)]
struct RegistryDataRef<'a> {
    assignments: &'a HashMap<String, Vec<AgentAssignment>>,
    phases: &'a HashMap<WorkItemId, Phase>,
}

/// Thread-safe shared registry
pub type SharedBranchRegistry = Arc<RwLock<BranchRegistry>>;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_work_intent_allows_write() {
        let readonly = WorkIntent::ReadOnly;
        assert!(!readonly.allows_write(&PathBuf::from("any/path")));

        let full = WorkIntent::FullBranch;
        assert!(full.allows_write(&PathBuf::from("any/path")));

        let write = WorkIntent::Write(vec![PathBuf::from("src/")]);
        assert!(write.allows_write(&PathBuf::from("src/main.rs")));
        assert!(!write.allows_write(&PathBuf::from("tests/test.rs")));
    }

    #[test]
    fn test_dynamic_timeout_calculation() {
        let mut phases = HashMap::new();
        let item1 = WorkItemId::new();
        let item2 = WorkItemId::new();

        phases.insert(item1.clone(), Phase::PromptToSpec); // 0.5x
        phases.insert(item2.clone(), Phase::PlanToArtifacts); // 2.0x

        let duration = calculate_dynamic_timeout(&vec![item1, item2], &phases);

        // Expected: 1 hour * (0.5 + 2.0) = 2.5 hours
        assert_eq!(duration.num_hours(), 2);
        assert!(duration.num_minutes() >= 150); // 2.5 hours = 150 minutes
    }

    #[test]
    fn test_assign_agent_isolated() {
        let mut registry = BranchRegistry::new();
        let agent1 = AgentId::new();
        let agent2 = AgentId::new();

        // First assignment succeeds
        registry
            .assign_agent(
                agent1,
                "main".to_string(),
                WorkIntent::FullBranch,
                CoordinationMode::Isolated,
            )
            .unwrap();

        // Second assignment to same branch fails (Isolated mode)
        let result = registry.assign_agent(
            agent2,
            "main".to_string(),
            WorkIntent::ReadOnly,
            CoordinationMode::Isolated,
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_assign_agent_coordinated() {
        let mut registry = BranchRegistry::new();
        let agent1 = AgentId::new();
        let agent2 = AgentId::new();

        // Both assignments succeed in Coordinated mode
        registry
            .assign_agent(
                agent1,
                "main".to_string(),
                WorkIntent::Write(vec![PathBuf::from("src/")]),
                CoordinationMode::Coordinated,
            )
            .unwrap();

        registry
            .assign_agent(
                agent2,
                "main".to_string(),
                WorkIntent::ReadOnly,
                CoordinationMode::Coordinated,
            )
            .unwrap();

        let assignments = registry.get_assignments("main");
        assert_eq!(assignments.len(), 2);
    }

    #[test]
    fn test_conflict_detection() {
        let mut registry = BranchRegistry::new();
        let agent1 = AgentId::new();

        registry
            .assign_agent(
                agent1,
                "main".to_string(),
                WorkIntent::Write(vec![PathBuf::from("src/auth/")]),
                CoordinationMode::Coordinated,
            )
            .unwrap();

        // Overlapping write should conflict
        let conflict = registry.check_conflict(
            "main",
            &WorkIntent::Write(vec![PathBuf::from("src/auth/login.rs")]),
        );

        assert!(conflict.is_some());

        // Non-overlapping write should not conflict
        let no_conflict =
            registry.check_conflict("main", &WorkIntent::Write(vec![PathBuf::from("tests/")]));

        assert!(no_conflict.is_none());

        // ReadOnly never conflicts
        let readonly_no_conflict = registry.check_conflict("main", &WorkIntent::ReadOnly);

        assert!(readonly_no_conflict.is_none());
    }

    #[test]
    fn test_release_assignment() {
        let mut registry = BranchRegistry::new();
        let agent = AgentId::new();

        registry
            .assign_agent(
                agent.clone(),
                "main".to_string(),
                WorkIntent::FullBranch,
                CoordinationMode::Isolated,
            )
            .unwrap();

        assert_eq!(registry.get_assignments("main").len(), 1);

        registry.release_assignment(&agent).unwrap();

        assert_eq!(registry.get_assignments("main").len(), 0);
    }

    #[test]
    fn test_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let registry_path = temp_dir.path().join("registry.json");

        // Create and populate registry
        let mut registry = BranchRegistry::with_persistence(registry_path.clone());
        let agent = AgentId::new();

        registry
            .assign_agent(
                agent.clone(),
                "main".to_string(),
                WorkIntent::FullBranch,
                CoordinationMode::Isolated,
            )
            .unwrap();

        // Load from disk
        let loaded = BranchRegistry::load(&registry_path).unwrap();

        assert_eq!(loaded.get_assignments("main").len(), 1);
    }

    #[test]
    fn test_cleanup_timeouts() {
        let mut registry = BranchRegistry::new();
        let agent = AgentId::new();

        registry
            .assign_agent(
                agent.clone(),
                "main".to_string(),
                WorkIntent::FullBranch,
                CoordinationMode::Isolated,
            )
            .unwrap();

        // Manually set timeout to past
        for assignments in registry.assignments.values_mut() {
            for assignment in assignments.iter_mut() {
                assignment.timeout = Utc::now() - Duration::hours(1);
            }
        }

        let removed = registry.cleanup_timeouts().unwrap();

        assert_eq!(removed.len(), 1);
        assert_eq!(registry.get_assignments("main").len(), 0);
    }
}
