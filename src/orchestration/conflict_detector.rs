//! Intelligent Conflict Detector
//!
//! Analyzes work intents to detect conflicts between agents working on the same branch.
//! Uses heuristics biased toward principled development to decide when to BLOCK vs WARN.
//!
//! # Philosophy
//!
//! The detector encourages good development practices:
//! - BLOCK critical changes (migrations, schemas, core configs)
//! - BLOCK same-file writes (high collision risk)
//! - WARN on related modules (potential conflicts)
//! - INFO on independent work (awareness only)
//! - Allow test isolation (tests should be independent)

use crate::orchestration::branch_registry::{AgentAssignment, WorkIntent};
use crate::orchestration::identity::AgentId;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Severity of detected conflict
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ConflictSeverity {
    /// Different areas, just awareness
    Info,

    /// Overlapping directories, potential conflict
    Warning,

    /// Same file or related modules, high risk
    Error,

    /// Critical files (migrations, schemas), must block
    Block,
}

/// Recommended action for conflict
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictAction {
    /// Allow work to proceed
    Proceed,

    /// Notify agents but allow
    NotifyAndProceed,

    /// Require explicit approval from existing agents
    RequireApproval,

    /// Require coordination (suggest sequential work)
    RequireCoordination,

    /// Hard block (don't allow without admin override)
    Block,
}

/// Detailed conflict assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictAssessment {
    /// Severity level
    pub severity: ConflictSeverity,

    /// Recommended action
    pub action: ConflictAction,

    /// Conflicting agents
    pub conflicting_agents: Vec<AgentId>,

    /// Overlapping paths
    pub overlapping_paths: Vec<PathBuf>,

    /// Human-readable reason
    pub reason: String,

    /// Suggestions for resolution
    pub suggestions: Vec<String>,
}

/// Intelligent conflict detector
///
/// Analyzes work intents and applies heuristics to determine conflict severity
/// and appropriate actions.
pub struct ConflictDetector {
    /// Critical path patterns that always block
    critical_patterns: Vec<String>,

    /// Related module patterns that trigger warnings
    related_patterns: Vec<(String, String)>,
}

impl ConflictDetector {
    /// Create a new conflict detector with default heuristics
    pub fn new() -> Self {
        Self {
            critical_patterns: vec![
                "migrations/".to_string(),
                "schema.sql".to_string(),
                ".env".to_string(),
                "config/database".to_string(),
                "Cargo.toml".to_string(),
                "package.json".to_string(),
            ],
            related_patterns: vec![
                ("src/auth/".to_string(), "src/auth/".to_string()),
                ("src/api/".to_string(), "src/api/".to_string()),
                ("src/db/".to_string(), "src/db/".to_string()),
            ],
        }
    }

    /// Assess conflict between existing assignments and new intent
    ///
    /// # Arguments
    ///
    /// * `existing` - Current agent assignments on the branch
    /// * `new_intent` - Proposed work intent for new agent
    /// * `new_agent_id` - ID of agent requesting access
    ///
    /// # Returns
    ///
    /// Detailed conflict assessment with severity, action, and suggestions
    pub fn assess_conflict(
        &self,
        existing: &[AgentAssignment],
        new_intent: &WorkIntent,
        new_agent_id: &AgentId,
    ) -> Option<ConflictAssessment> {
        // No conflict if branch is empty
        if existing.is_empty() {
            return None;
        }

        // ReadOnly never conflicts
        if new_intent.is_readonly() {
            return None;
        }

        let mut max_severity = ConflictSeverity::Info;
        let mut conflicting_agents = Vec::new();
        let mut overlapping_paths = Vec::new();

        for assignment in existing {
            // Skip read-only assignments
            if assignment.intent.is_readonly() {
                continue;
            }

            let (severity, paths) = self.assess_intent_pair(&assignment.intent, new_intent);

            if severity > max_severity {
                max_severity = severity;
            }

            if severity >= ConflictSeverity::Warning {
                conflicting_agents.push(assignment.agent_id.clone());
                overlapping_paths.extend(paths);
            }
        }

        if conflicting_agents.is_empty() {
            return None;
        }

        // Determine action based on severity
        let action = match max_severity {
            ConflictSeverity::Info => ConflictAction::Proceed,
            ConflictSeverity::Warning => ConflictAction::NotifyAndProceed,
            ConflictSeverity::Error => ConflictAction::RequireCoordination,
            ConflictSeverity::Block => ConflictAction::Block,
        };

        let reason = self.generate_reason(max_severity, existing.len(), &overlapping_paths);
        let suggestions = self.generate_suggestions(max_severity, &overlapping_paths);

        Some(ConflictAssessment {
            severity: max_severity,
            action,
            conflicting_agents,
            overlapping_paths,
            reason,
            suggestions,
        })
    }

    /// Assess conflict between two work intents
    ///
    /// Returns (severity, overlapping_paths)
    fn assess_intent_pair(
        &self,
        existing: &WorkIntent,
        new: &WorkIntent,
    ) -> (ConflictSeverity, Vec<PathBuf>) {
        match (existing, new) {
            // Both FullBranch -> High conflict
            (WorkIntent::FullBranch, WorkIntent::FullBranch) => {
                (ConflictSeverity::Error, vec![PathBuf::from(".")])
            }

            // One FullBranch -> Moderate conflict
            (WorkIntent::FullBranch, WorkIntent::Write(paths))
            | (WorkIntent::Write(paths), WorkIntent::FullBranch) => {
                (ConflictSeverity::Warning, paths.clone())
            }

            // Both Write -> Check path overlaps
            (WorkIntent::Write(existing_paths), WorkIntent::Write(new_paths)) => {
                self.assess_path_overlap(existing_paths, new_paths)
            }

            // ReadOnly never conflicts (handled earlier, but defensive)
            _ => (ConflictSeverity::Info, vec![]),
        }
    }

    /// Assess overlap between path lists
    fn assess_path_overlap(
        &self,
        existing_paths: &[PathBuf],
        new_paths: &[PathBuf],
    ) -> (ConflictSeverity, Vec<PathBuf>) {
        let mut overlapping = Vec::new();
        let mut max_severity = ConflictSeverity::Info;

        for new_path in new_paths {
            for existing_path in existing_paths {
                if let Some(overlap) = self.check_path_overlap(existing_path, new_path) {
                    let severity = self.determine_path_severity(&overlap);

                    if severity > max_severity {
                        max_severity = severity;
                    }

                    overlapping.push(overlap);
                }
            }
        }

        (max_severity, overlapping)
    }

    /// Check if two paths overlap
    fn check_path_overlap(&self, path1: &Path, path2: &Path) -> Option<PathBuf> {
        if path1.starts_with(path2) {
            Some(path1.to_path_buf())
        } else if path2.starts_with(path1) {
            Some(path2.to_path_buf())
        } else if path1 == path2 {
            Some(path1.to_path_buf())
        } else {
            None
        }
    }

    /// Determine severity based on path characteristics
    fn determine_path_severity(&self, path: &Path) -> ConflictSeverity {
        let path_str = path.to_string_lossy();

        // BLOCK: Critical files/patterns
        if self.is_critical_path(&path_str) {
            return ConflictSeverity::Block;
        }

        // Allow test isolation (tests should be independent)
        if path_str.contains("/tests/") || path_str.starts_with("tests/") {
            return ConflictSeverity::Warning; // Warn but allow
        }

        // ERROR: Exact same file
        if path.extension().is_some() {
            return ConflictSeverity::Error;
        }

        // WARNING: Same directory
        ConflictSeverity::Warning
    }

    /// Check if path matches critical patterns
    fn is_critical_path(&self, path: &str) -> bool {
        self.critical_patterns
            .iter()
            .any(|pattern| path.contains(pattern))
    }

    /// Generate human-readable reason
    fn generate_reason(
        &self,
        severity: ConflictSeverity,
        num_agents: usize,
        paths: &[PathBuf],
    ) -> String {
        match severity {
            ConflictSeverity::Block => format!(
                "CRITICAL: Cannot modify {:?} - protected files require isolation",
                paths
            ),
            ConflictSeverity::Error => format!(
                "{} agent(s) writing to same file(s): {:?}",
                num_agents, paths
            ),
            ConflictSeverity::Warning => format!(
                "{} agent(s) working in overlapping directories: {:?}",
                num_agents, paths
            ),
            ConflictSeverity::Info => format!("{} agent(s) working on same branch", num_agents),
        }
    }

    /// Generate resolution suggestions
    fn generate_suggestions(&self, severity: ConflictSeverity, paths: &[PathBuf]) -> Vec<String> {
        match severity {
            ConflictSeverity::Block => vec![
                "Request isolation on this branch".to_string(),
                "Wait for other agents to complete".to_string(),
                "Work on a different branch".to_string(),
            ],
            ConflictSeverity::Error => vec![
                "Coordinate with other agent(s) via chat".to_string(),
                "Work sequentially (wait for commit)".to_string(),
                "Split work by function/module".to_string(),
                "Use file locks for critical sections".to_string(),
            ],
            ConflictSeverity::Warning => vec![
                "Monitor for conflicts during work".to_string(),
                "Communicate changes via commit messages".to_string(),
                "Consider splitting into subdirectories".to_string(),
            ],
            ConflictSeverity::Info => vec!["Awareness only - no action needed".to_string()],
        }
    }

    /// Check if paths are in related modules
    pub fn are_related_modules(&self, path1: &Path, path2: &Path) -> bool {
        let p1 = path1.to_string_lossy();
        let p2 = path2.to_string_lossy();

        self.related_patterns
            .iter()
            .any(|(pattern1, pattern2)| {
                (p1.contains(pattern1) && p2.contains(pattern2))
                    || (p1.contains(pattern2) && p2.contains(pattern1))
            })
    }

    /// Add custom critical pattern
    pub fn add_critical_pattern(&mut self, pattern: String) {
        if !self.critical_patterns.contains(&pattern) {
            self.critical_patterns.push(pattern);
        }
    }

    /// Add custom related module pattern
    pub fn add_related_pattern(&mut self, pattern1: String, pattern2: String) {
        self.related_patterns.push((pattern1, pattern2));
    }
}

impl Default for ConflictDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::branch_registry::CoordinationMode;
    use crate::orchestration::identity::AgentId;
    use chrono::Utc;

    fn make_assignment(intent: WorkIntent) -> AgentAssignment {
        AgentAssignment {
            agent_id: AgentId::new(),
            branch: "main".to_string(),
            intent,
            mode: CoordinationMode::Coordinated,
            assigned_at: Utc::now(),
            expected_duration: None,
            work_items: vec![],
            timeout: Utc::now() + chrono::Duration::hours(1),
        }
    }

    #[test]
    fn test_readonly_no_conflict() {
        let detector = ConflictDetector::new();
        let existing = vec![make_assignment(WorkIntent::FullBranch)];
        let new_intent = WorkIntent::ReadOnly;

        let result = detector.assess_conflict(&existing, &new_intent, &AgentId::new());
        assert!(result.is_none());
    }

    #[test]
    fn test_critical_path_blocks() {
        let detector = ConflictDetector::new();
        let existing = vec![make_assignment(WorkIntent::Write(vec![PathBuf::from(
            "migrations/001_init.sql",
        )]))];

        let new_intent = WorkIntent::Write(vec![PathBuf::from("migrations/002_add_users.sql")]);
        let result = detector.assess_conflict(&existing, &new_intent, &AgentId::new());

        assert!(result.is_some());
        let assessment = result.unwrap();
        assert_eq!(assessment.severity, ConflictSeverity::Block);
        assert_eq!(assessment.action, ConflictAction::Block);
    }

    #[test]
    fn test_same_file_error() {
        let detector = ConflictDetector::new();
        let existing = vec![make_assignment(WorkIntent::Write(vec![PathBuf::from(
            "src/auth/login.rs",
        )]))];

        let new_intent = WorkIntent::Write(vec![PathBuf::from("src/auth/login.rs")]);
        let result = detector.assess_conflict(&existing, &new_intent, &AgentId::new());

        assert!(result.is_some());
        let assessment = result.unwrap();
        assert_eq!(assessment.severity, ConflictSeverity::Error);
        assert_eq!(assessment.action, ConflictAction::RequireCoordination);
    }

    #[test]
    fn test_overlapping_directories_warning() {
        let detector = ConflictDetector::new();
        let existing = vec![make_assignment(WorkIntent::Write(vec![PathBuf::from(
            "src/auth/",
        )]))];

        let new_intent = WorkIntent::Write(vec![PathBuf::from("src/auth/utils.rs")]);
        let result = detector.assess_conflict(&existing, &new_intent, &AgentId::new());

        assert!(result.is_some());
        let assessment = result.unwrap();
        assert!(assessment.severity >= ConflictSeverity::Warning);
    }

    #[test]
    fn test_different_directories_no_conflict() {
        let detector = ConflictDetector::new();
        let existing = vec![make_assignment(WorkIntent::Write(vec![PathBuf::from(
            "src/auth/",
        )]))];

        let new_intent = WorkIntent::Write(vec![PathBuf::from("src/api/")]);
        let result = detector.assess_conflict(&existing, &new_intent, &AgentId::new());

        assert!(result.is_none());
    }

    #[test]
    fn test_test_files_allowed() {
        let detector = ConflictDetector::new();
        let existing = vec![make_assignment(WorkIntent::Write(vec![PathBuf::from(
            "tests/auth_tests.rs",
        )]))];

        let new_intent = WorkIntent::Write(vec![PathBuf::from("tests/api_tests.rs")]);
        let result = detector.assess_conflict(&existing, &new_intent, &AgentId::new());

        // Should not conflict (different test files)
        assert!(result.is_none());
    }

    #[test]
    fn test_full_branch_conflict() {
        let detector = ConflictDetector::new();
        let existing = vec![make_assignment(WorkIntent::FullBranch)];
        let new_intent = WorkIntent::FullBranch;

        let result = detector.assess_conflict(&existing, &new_intent, &AgentId::new());

        assert!(result.is_some());
        let assessment = result.unwrap();
        assert_eq!(assessment.severity, ConflictSeverity::Error);
    }

    #[test]
    fn test_suggestions_generated() {
        let detector = ConflictDetector::new();
        let existing = vec![make_assignment(WorkIntent::Write(vec![PathBuf::from(
            "src/main.rs",
        )]))];

        let new_intent = WorkIntent::Write(vec![PathBuf::from("src/main.rs")]);
        let result = detector.assess_conflict(&existing, &new_intent, &AgentId::new());

        assert!(result.is_some());
        let assessment = result.unwrap();
        assert!(!assessment.suggestions.is_empty());
        assert!(assessment
            .suggestions
            .iter()
            .any(|s| s.contains("coordinate") || s.contains("sequential")));
    }

    #[test]
    fn test_custom_critical_pattern() {
        let mut detector = ConflictDetector::new();
        detector.add_critical_pattern("secrets.json".to_string());

        let existing = vec![make_assignment(WorkIntent::Write(vec![PathBuf::from(
            "config/secrets.json",
        )]))];

        let new_intent = WorkIntent::Write(vec![PathBuf::from("config/secrets.json")]);
        let result = detector.assess_conflict(&existing, &new_intent, &AgentId::new());

        assert!(result.is_some());
        let assessment = result.unwrap();
        assert_eq!(assessment.severity, ConflictSeverity::Block);
    }
}
