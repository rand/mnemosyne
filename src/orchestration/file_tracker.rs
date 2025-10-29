//! File Tracking System
//!
//! Monitors file modifications by agents to detect conflicts in real-time.
//! Integrates with conflict detector to identify overlapping work.
//!
//! # Features
//!
//! - Track modified files per agent
//! - Detect overlapping modifications
//! - Generate conflict reports
//! - Integrate with git state tracker

use crate::error::{MnemosyneError, Result};
use crate::orchestration::conflict_detector::{ConflictDetector, ConflictSeverity};
use crate::orchestration::identity::AgentId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// File modification record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileModification {
    /// File path
    pub path: PathBuf,

    /// Agent that modified the file
    pub agent_id: AgentId,

    /// When modification occurred
    pub timestamp: DateTime<Utc>,

    /// Modification type
    pub modification_type: ModificationType,
}

/// Type of file modification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModificationType {
    /// File created
    Created,

    /// File modified
    Modified,

    /// File deleted
    Deleted,
}

/// Active file conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveConflict {
    /// Unique conflict ID
    pub id: String,

    /// File path
    pub path: PathBuf,

    /// Agents involved
    pub agents: Vec<AgentId>,

    /// When conflict was detected
    pub detected_at: DateTime<Utc>,

    /// Conflict severity
    pub severity: ConflictSeverity,

    /// Last notification time
    pub last_notified: Option<DateTime<Utc>>,
}

/// File tracker - monitors file changes and detects conflicts
pub struct FileTracker {
    /// Map: Agent ID -> Set of modified files
    agent_files: Arc<RwLock<HashMap<AgentId, HashSet<PathBuf>>>>,

    /// Map: File -> List of modifications
    file_modifications: Arc<RwLock<HashMap<PathBuf, Vec<FileModification>>>>,

    /// Active conflicts
    active_conflicts: Arc<RwLock<HashMap<String, ActiveConflict>>>,

    /// Conflict detector
    conflict_detector: Arc<ConflictDetector>,
}

impl FileTracker {
    /// Create a new file tracker
    pub fn new(conflict_detector: Arc<ConflictDetector>) -> Self {
        Self {
            agent_files: Arc::new(RwLock::new(HashMap::new())),
            file_modifications: Arc::new(RwLock::new(HashMap::new())),
            active_conflicts: Arc::new(RwLock::new(HashMap::new())),
            conflict_detector,
        }
    }

    /// Record a file modification
    ///
    /// # Arguments
    ///
    /// * `agent_id` - Agent that modified the file
    /// * `path` - Path to modified file
    /// * `modification_type` - Type of modification
    ///
    /// # Returns
    ///
    /// New conflicts detected (if any)
    pub fn record_modification(
        &self,
        agent_id: &AgentId,
        path: &Path,
        modification_type: ModificationType,
    ) -> Result<Vec<ActiveConflict>> {
        let modification = FileModification {
            path: path.to_path_buf(),
            agent_id: agent_id.clone(),
            timestamp: Utc::now(),
            modification_type,
        };

        // Record in agent files
        {
            let mut agent_files = self.agent_files.write().map_err(|e| {
                MnemosyneError::Other(format!("Failed to lock agent_files: {}", e))
            })?;

            agent_files
                .entry(agent_id.clone())
                .or_insert_with(HashSet::new)
                .insert(path.to_path_buf());
        }

        // Record in file modifications
        {
            let mut file_modifications = self.file_modifications.write().map_err(|e| {
                MnemosyneError::Other(format!("Failed to lock file_modifications: {}", e))
            })?;

            file_modifications
                .entry(path.to_path_buf())
                .or_insert_with(Vec::new)
                .push(modification);
        }

        // Detect conflicts
        self.detect_conflicts_for_file(path)
    }

    /// Detect conflicts for a specific file
    fn detect_conflicts_for_file(&self, path: &Path) -> Result<Vec<ActiveConflict>> {
        let file_modifications = self.file_modifications.read().map_err(|e| {
            MnemosyneError::Other(format!("Failed to lock file_modifications: {}", e))
        })?;

        let modifications = match file_modifications.get(path) {
            Some(mods) => mods,
            None => return Ok(vec![]),
        };

        // Find unique agents working on this file
        let unique_agents: HashSet<AgentId> = modifications
            .iter()
            .map(|m| m.agent_id.clone())
            .collect();

        if unique_agents.len() <= 1 {
            return Ok(vec![]); // No conflict if only one agent
        }

        // Conflict detected!
        let severity = self.determine_severity(path, &unique_agents)?;

        let conflict_id = self.generate_conflict_id(path, &unique_agents);

        // Check if this is a new conflict
        let mut active_conflicts = self.active_conflicts.write().map_err(|e| {
            MnemosyneError::Other(format!("Failed to lock active_conflicts: {}", e))
        })?;

        if active_conflicts.contains_key(&conflict_id) {
            return Ok(vec![]); // Already tracking this conflict
        }

        let conflict = ActiveConflict {
            id: conflict_id.clone(),
            path: path.to_path_buf(),
            agents: unique_agents.into_iter().collect(),
            detected_at: Utc::now(),
            severity,
            last_notified: None,
        };

        active_conflicts.insert(conflict_id, conflict.clone());

        Ok(vec![conflict])
    }

    /// Determine conflict severity
    fn determine_severity(&self, path: &Path, _agents: &HashSet<AgentId>) -> Result<ConflictSeverity> {
        // Use conflict detector's logic
        let path_str = path.to_string_lossy();

        // Critical files
        if path_str.contains("migration") || path_str.contains("schema") || path_str.ends_with(".env") {
            return Ok(ConflictSeverity::Block);
        }

        // Same file modification is always high severity
        if path.extension().is_some() {
            return Ok(ConflictSeverity::Error);
        }

        // Directory-level conflict
        Ok(ConflictSeverity::Warning)
    }

    /// Generate unique conflict ID
    fn generate_conflict_id(&self, path: &Path, agents: &HashSet<AgentId>) -> String {
        let mut agent_ids: Vec<String> = agents.iter().map(|id| id.to_string()).collect();
        agent_ids.sort();

        format!("{}:{}", path.display(), agent_ids.join(","))
    }

    /// Get modified files for agent
    pub fn get_agent_files(&self, agent_id: &AgentId) -> Result<HashSet<PathBuf>> {
        let agent_files = self.agent_files.read().map_err(|e| {
            MnemosyneError::Other(format!("Failed to lock agent_files: {}", e))
        })?;

        Ok(agent_files
            .get(agent_id)
            .cloned()
            .unwrap_or_else(HashSet::new))
    }

    /// Get all agents working on a file
    pub fn get_file_agents(&self, path: &Path) -> Result<Vec<AgentId>> {
        let file_modifications = self.file_modifications.read().map_err(|e| {
            MnemosyneError::Other(format!("Failed to lock file_modifications: {}", e))
        })?;

        let modifications = match file_modifications.get(path) {
            Some(mods) => mods,
            None => return Ok(vec![]),
        };

        let unique_agents: HashSet<AgentId> = modifications
            .iter()
            .map(|m| m.agent_id.clone())
            .collect();

        Ok(unique_agents.into_iter().collect())
    }

    /// Get all active conflicts
    pub fn get_active_conflicts(&self) -> Result<Vec<ActiveConflict>> {
        let conflicts = self.active_conflicts.read().map_err(|e| {
            MnemosyneError::Other(format!("Failed to lock active_conflicts: {}", e))
        })?;

        Ok(conflicts.values().cloned().collect())
    }

    /// Get conflicts for specific agent
    pub fn get_agent_conflicts(&self, agent_id: &AgentId) -> Result<Vec<ActiveConflict>> {
        let conflicts = self.get_active_conflicts()?;

        Ok(conflicts
            .into_iter()
            .filter(|c| c.agents.contains(agent_id))
            .collect())
    }

    /// Mark conflict as notified
    pub fn mark_conflict_notified(&self, conflict_id: &str) -> Result<()> {
        let mut conflicts = self.active_conflicts.write().map_err(|e| {
            MnemosyneError::Other(format!("Failed to lock active_conflicts: {}", e))
        })?;

        if let Some(conflict) = conflicts.get_mut(conflict_id) {
            conflict.last_notified = Some(Utc::now());
        }

        Ok(())
    }

    /// Resolve conflict (e.g., one agent finished)
    pub fn resolve_conflict(&self, conflict_id: &str) -> Result<()> {
        let mut conflicts = self.active_conflicts.write().map_err(|e| {
            MnemosyneError::Other(format!("Failed to lock active_conflicts: {}", e))
        })?;

        conflicts.remove(conflict_id);

        Ok(())
    }

    /// Clear agent's tracked files (e.g., after commit)
    pub fn clear_agent_files(&self, agent_id: &AgentId) -> Result<()> {
        let mut agent_files = self.agent_files.write().map_err(|e| {
            MnemosyneError::Other(format!("Failed to lock agent_files: {}", e))
        })?;

        agent_files.remove(agent_id);

        // Also clear file modifications for this agent
        let mut file_modifications = self.file_modifications.write().map_err(|e| {
            MnemosyneError::Other(format!("Failed to lock file_modifications: {}", e))
        })?;

        for mods in file_modifications.values_mut() {
            mods.retain(|m| &m.agent_id != agent_id);
        }

        // Clean up empty entries
        file_modifications.retain(|_, mods| !mods.is_empty());

        // Re-evaluate conflicts
        self.refresh_conflicts()?;

        Ok(())
    }

    /// Refresh conflicts after clearing agent files
    fn refresh_conflicts(&self) -> Result<()> {
        // Get all files that still have modifications
        let file_modifications = self.file_modifications.read().map_err(|e| {
            MnemosyneError::Other(format!("Failed to lock file_modifications: {}", e))
        })?;

        let mut conflicts_to_remove = Vec::new();

        let active_conflicts = self.active_conflicts.read().map_err(|e| {
            MnemosyneError::Other(format!("Failed to lock active_conflicts: {}", e))
        })?;

        for (conflict_id, conflict) in active_conflicts.iter() {
            if let Some(mods) = file_modifications.get(&conflict.path) {
                let unique_agents: HashSet<AgentId> = mods.iter().map(|m| m.agent_id.clone()).collect();

                if unique_agents.len() <= 1 {
                    conflicts_to_remove.push(conflict_id.clone());
                }
            } else {
                conflicts_to_remove.push(conflict_id.clone());
            }
        }

        drop(active_conflicts);

        // Remove resolved conflicts
        let mut active_conflicts = self.active_conflicts.write().map_err(|e| {
            MnemosyneError::Other(format!("Failed to lock active_conflicts: {}", e))
        })?;

        for conflict_id in conflicts_to_remove {
            active_conflicts.remove(&conflict_id);
        }

        Ok(())
    }

    /// Get modification history for file
    pub fn get_file_history(&self, path: &Path) -> Result<Vec<FileModification>> {
        let file_modifications = self.file_modifications.read().map_err(|e| {
            MnemosyneError::Other(format!("Failed to lock file_modifications: {}", e))
        })?;

        Ok(file_modifications
            .get(path)
            .cloned()
            .unwrap_or_else(Vec::new))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::conflict_detector::ConflictDetector;

    #[test]
    fn test_record_single_modification() {
        let detector = Arc::new(ConflictDetector::new());
        let tracker = FileTracker::new(detector);

        let agent = AgentId::new();
        let path = PathBuf::from("src/main.rs");

        let conflicts = tracker
            .record_modification(&agent, &path, ModificationType::Modified)
            .unwrap();

        assert!(conflicts.is_empty()); // No conflict with single agent

        let files = tracker.get_agent_files(&agent).unwrap();
        assert_eq!(files.len(), 1);
        assert!(files.contains(&path));
    }

    #[test]
    fn test_detect_conflict_multiple_agents() {
        let detector = Arc::new(ConflictDetector::new());
        let tracker = FileTracker::new(detector);

        let agent1 = AgentId::new();
        let agent2 = AgentId::new();
        let path = PathBuf::from("src/main.rs");

        // First agent modifies
        tracker
            .record_modification(&agent1, &path, ModificationType::Modified)
            .unwrap();

        // Second agent modifies same file -> conflict!
        let conflicts = tracker
            .record_modification(&agent2, &path, ModificationType::Modified)
            .unwrap();

        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].agents.len(), 2);
        assert_eq!(conflicts[0].severity, ConflictSeverity::Error);
    }

    #[test]
    fn test_clear_agent_files() {
        let detector = Arc::new(ConflictDetector::new());
        let tracker = FileTracker::new(detector);

        let agent = AgentId::new();
        let path = PathBuf::from("src/main.rs");

        tracker
            .record_modification(&agent, &path, ModificationType::Modified)
            .unwrap();

        assert_eq!(tracker.get_agent_files(&agent).unwrap().len(), 1);

        tracker.clear_agent_files(&agent).unwrap();

        assert_eq!(tracker.get_agent_files(&agent).unwrap().len(), 0);
    }

    #[test]
    fn test_conflict_resolution() {
        let detector = Arc::new(ConflictDetector::new());
        let tracker = FileTracker::new(detector);

        let agent1 = AgentId::new();
        let agent2 = AgentId::new();
        let path = PathBuf::from("src/main.rs");

        tracker
            .record_modification(&agent1, &path, ModificationType::Modified)
            .unwrap();

        let conflicts = tracker
            .record_modification(&agent2, &path, ModificationType::Modified)
            .unwrap();

        assert_eq!(conflicts.len(), 1);
        let conflict_id = conflicts[0].id.clone();

        // Clear one agent
        tracker.clear_agent_files(&agent1).unwrap();

        // Conflict should be resolved
        let active = tracker.get_active_conflicts().unwrap();
        assert_eq!(active.len(), 0);
    }

    #[test]
    fn test_get_file_agents() {
        let detector = Arc::new(ConflictDetector::new());
        let tracker = FileTracker::new(detector);

        let agent1 = AgentId::new();
        let agent2 = AgentId::new();
        let path = PathBuf::from("src/main.rs");

        tracker
            .record_modification(&agent1, &path, ModificationType::Modified)
            .unwrap();
        tracker
            .record_modification(&agent2, &path, ModificationType::Modified)
            .unwrap();

        let agents = tracker.get_file_agents(&path).unwrap();
        assert_eq!(agents.len(), 2);
    }

    #[test]
    fn test_critical_file_severity() {
        let detector = Arc::new(ConflictDetector::new());
        let tracker = FileTracker::new(detector);

        let agent1 = AgentId::new();
        let agent2 = AgentId::new();
        let path = PathBuf::from("migrations/001_init.sql");

        tracker
            .record_modification(&agent1, &path, ModificationType::Modified)
            .unwrap();

        let conflicts = tracker
            .record_modification(&agent2, &path, ModificationType::Modified)
            .unwrap();

        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].severity, ConflictSeverity::Block);
    }
}
