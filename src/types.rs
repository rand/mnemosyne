//! Core data types for the Mnemosyne memory system
//!
//! This module defines the fundamental data structures used throughout mnemosyne,
//! including memories, namespaces, links, and search queries. These types form the
//! foundation of the project-aware agentic memory system.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for memories
///
/// Wraps a UUID to provide type safety and prevent mixing memory IDs
/// with other UUID-based identifiers in the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MemoryId(pub Uuid);

impl MemoryId {
    /// Create a new random memory ID
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Parse a memory ID from a string
    pub fn from_string(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl Default for MemoryId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for MemoryId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Namespace hierarchy: Global > Project > Session
///
/// Namespaces provide project-aware isolation while allowing global knowledge sharing.
/// Priority determines retrieval order (Session > Project > Global).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Namespace {
    /// Global memories accessible across all projects
    Global,

    /// Project-scoped memories tied to a specific codebase
    Project {
        /// Project name (typically from git root directory name)
        name: String,
    },

    /// Session-scoped memories for temporary context
    Session {
        /// Parent project name
        project: String,

        /// Unique session identifier
        session_id: String,
    },
}

impl Namespace {
    /// Get namespace priority for retrieval ordering
    /// Higher priority = searched first
    pub fn priority(&self) -> u8 {
        match self {
            Namespace::Session { .. } => 3,
            Namespace::Project { .. } => 2,
            Namespace::Global => 1,
        }
    }

    /// Check if this namespace is a session
    pub fn is_session(&self) -> bool {
        matches!(self, Namespace::Session { .. })
    }

    /// Check if this namespace is project-scoped or higher
    pub fn is_project_or_higher(&self) -> bool {
        !matches!(self, Namespace::Global)
    }

    /// Get the project name if applicable
    pub fn project_name(&self) -> Option<&str> {
        match self {
            Namespace::Project { name } => Some(name),
            Namespace::Session { project, .. } => Some(project),
            Namespace::Global => None,
        }
    }
}

impl std::fmt::Display for Namespace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Namespace::Global => write!(f, "global"),
            Namespace::Project { name } => write!(f, "project:{}", name),
            Namespace::Session {
                project,
                session_id,
            } => {
                write!(f, "session:{}:{}", project, session_id)
            }
        }
    }
}

/// Memory type classification for organizational and filtering purposes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryType {
    /// Architectural decisions and system design choices
    ArchitectureDecision,

    /// Code patterns and implementation approaches
    CodePattern,

    /// Bug fixes and their solutions
    BugFix,

    /// Configuration settings and preferences
    Configuration,

    /// Constraints and requirements that must be satisfied
    Constraint,

    /// Domain entities and business concepts
    Entity,

    /// Insights and learnings
    Insight,

    /// References to external resources
    Reference,

    /// User preferences and settings
    Preference,

    /// Task or action item
    Task,

    /// Agent coordination events for orchestration
    AgentEvent,
}

impl MemoryType {
    /// Get the type factor for importance calculations
    /// Different memory types have different base value
    pub fn type_factor(&self) -> f32 {
        match self {
            MemoryType::ArchitectureDecision => 1.2,
            MemoryType::Constraint => 1.1,
            MemoryType::AgentEvent => 1.0, // High value for orchestration
            MemoryType::CodePattern => 1.0,
            MemoryType::BugFix => 0.9,
            MemoryType::Insight => 0.9,
            MemoryType::Task => 0.9,
            _ => 0.8,
        }
    }
}

/// Relationship types between memories for knowledge graph construction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkType {
    /// B builds upon or extends A
    Extends,

    /// B contradicts or invalidates A
    Contradicts,

    /// B implements the concept described in A
    Implements,

    /// B references or cites A
    References,

    /// B replaces or supersedes A
    Supersedes,
}

/// Memory link with typed relationship and metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLink {
    /// Target memory ID
    pub target_id: MemoryId,

    /// Type of relationship
    pub link_type: LinkType,

    /// Link strength (0.0 - 1.0), evolves based on co-access patterns
    pub strength: f32,

    /// Human-readable explanation of the relationship
    pub reason: String,

    /// When the link was created
    pub created_at: DateTime<Utc>,

    /// When the link was last traversed (for decay tracking)
    pub last_traversed_at: Option<DateTime<Utc>>,

    /// Whether link was manually created by user (user links don't decay)
    pub user_created: bool,
}

/// Complete memory note structure with all metadata
///
/// This is the core data structure representing a single memory in the system.
/// It includes content, classification, relationships, and lifecycle information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryNote {
    // === Identity ===
    /// Unique identifier
    pub id: MemoryId,

    /// Namespace (global, project, or session)
    pub namespace: Namespace,

    /// Creation timestamp
    pub created_at: DateTime<Utc>,

    /// Last update timestamp
    pub updated_at: DateTime<Utc>,

    // === Content (human-readable) ===
    /// Full memory content
    pub content: String,

    /// Concise 1-2 sentence summary (LLM-generated)
    pub summary: String,

    /// Key terms for keyword search (LLM-extracted)
    pub keywords: Vec<String>,

    /// Categorization tags (LLM-suggested + user-added)
    pub tags: Vec<String>,

    /// Context about when/why this is relevant (LLM-generated)
    pub context: String,

    // === Classification ===
    /// Memory type
    pub memory_type: MemoryType,

    /// Importance level (1-10, higher = more important)
    pub importance: u8,

    /// Confidence in the information (0.0-1.0)
    pub confidence: f32,

    // === Relationships ===
    /// Semantic links to other memories
    pub links: Vec<MemoryLink>,

    /// Related file paths in the codebase
    pub related_files: Vec<String>,

    /// Related entities (components, services, etc.)
    pub related_entities: Vec<String>,

    // === Lifecycle ===
    /// Number of times this memory has been accessed
    pub access_count: u32,

    /// Last access timestamp
    pub last_accessed_at: DateTime<Utc>,

    /// Optional expiration timestamp
    pub expires_at: Option<DateTime<Utc>>,

    /// Whether this memory has been archived
    pub is_archived: bool,

    /// If superseded, the ID of the superseding memory
    pub superseded_by: Option<MemoryId>,

    // === Computational ===
    /// Embedding vector (not serialized to JSON, stored separately)
    #[serde(skip)]
    pub embedding: Option<Vec<f32>>,

    /// Model used to generate the embedding
    pub embedding_model: String,
}

impl MemoryNote {
    /// Calculate decayed importance based on age, access patterns, and type
    ///
    /// This implements the FEEDBACK phase of the OODA loop, adjusting memory
    /// importance over time based on usage patterns.
    pub fn decayed_importance(&self) -> f32 {
        let base = self.importance as f32;
        let recency_factor = self.recency_factor();
        let type_factor = self.memory_type.type_factor();
        let access_bonus = (self.access_count as f32).ln().max(0.0) * 0.1;

        base * recency_factor * type_factor * (1.0 + access_bonus)
    }

    /// Calculate recency factor (exponential decay with 6-month half-life)
    fn recency_factor(&self) -> f32 {
        let age_days = (Utc::now() - self.updated_at).num_days() as f32;
        (-age_days / 180.0).exp() // Half-life of 6 months
    }

    /// Check if this memory should be archived
    pub fn should_archive(&self, threshold_days: u32, min_importance: f32) -> bool {
        let age_days = (Utc::now() - self.updated_at).num_days() as u32;
        age_days > threshold_days && self.decayed_importance() < min_importance
    }
}

/// Search query with filters for memory retrieval
///
/// Supports the OBSERVE and ORIENT phases of the OODA loop by enabling
/// targeted memory recall with multiple filter dimensions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Search query string (semantic or keyword)
    pub query: String,

    /// Optional namespace filter
    pub namespace: Option<Namespace>,

    /// Filter by memory types
    pub memory_types: Vec<MemoryType>,

    /// Filter by tags
    pub tags: Vec<String>,

    /// Minimum importance threshold
    pub min_importance: Option<u8>,

    /// Maximum number of results to return
    pub max_results: usize,

    /// Whether to include archived memories
    pub include_archived: bool,
}

impl Default for SearchQuery {
    fn default() -> Self {
        Self {
            query: String::new(),
            namespace: None,
            memory_types: Vec::new(),
            tags: Vec::new(),
            min_importance: None,
            max_results: 10,
            include_archived: false,
        }
    }
}

/// Search result with relevance score and match explanation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// The memory that matched
    pub memory: MemoryNote,

    /// Relevance score (0.0 - 1.0, higher = more relevant)
    pub score: f32,

    /// Explanation of why this memory matched
    pub match_reason: String,
}

/// Updates to apply to an existing memory
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MemoryUpdates {
    /// New content (triggers re-embedding)
    pub content: Option<String>,

    /// New importance level
    pub importance: Option<u8>,

    /// New tags (replaces existing)
    pub tags: Option<Vec<String>>,

    /// Additional tags (appends to existing)
    pub add_tags: Option<Vec<String>>,
}

/// Consolidation decision from LLM analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "decision")]
pub enum ConsolidationDecision {
    /// Merge two memories into one
    Merge {
        /// Which memory ID to keep
        into: MemoryId,

        /// Merged content
        content: String,
    },

    /// One memory supersedes the other
    Supersede {
        /// Memory to keep
        kept: MemoryId,

        /// Memory to archive
        superseded: MemoryId,
    },

    /// Keep both memories (they're distinct)
    KeepBoth,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_id_creation() {
        let id1 = MemoryId::new();
        let id2 = MemoryId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_namespace_priority() {
        let global = Namespace::Global;
        let project = Namespace::Project {
            name: "test".to_string(),
        };
        let session = Namespace::Session {
            project: "test".to_string(),
            session_id: "abc123".to_string(),
        };

        assert_eq!(global.priority(), 1);
        assert_eq!(project.priority(), 2);
        assert_eq!(session.priority(), 3);
    }

    #[test]
    fn test_memory_type_factors() {
        assert!(MemoryType::ArchitectureDecision.type_factor() > 1.0);
        assert!(MemoryType::Constraint.type_factor() > 1.0);
        assert_eq!(MemoryType::CodePattern.type_factor(), 1.0);
    }

    #[test]
    fn test_decayed_importance() {
        let mut memory = MemoryNote {
            id: MemoryId::new(),
            namespace: Namespace::Global,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            content: "test".to_string(),
            summary: "test".to_string(),
            keywords: vec![],
            tags: vec![],
            context: "test".to_string(),
            memory_type: MemoryType::CodePattern,
            importance: 8,
            confidence: 0.9,
            links: vec![],
            related_files: vec![],
            related_entities: vec![],
            access_count: 0,
            last_accessed_at: Utc::now(),
            expires_at: None,
            is_archived: false,
            superseded_by: None,
            embedding: None,
            embedding_model: "test".to_string(),
        };

        // Fresh memory should have high importance
        let fresh_importance = memory.decayed_importance();
        assert!(fresh_importance >= 7.0);

        // Accessed memory should have bonus
        memory.access_count = 50;
        let accessed_importance = memory.decayed_importance();
        assert!(accessed_importance > fresh_importance);
    }
}
