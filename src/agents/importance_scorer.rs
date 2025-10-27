//! Custom importance scoring for agent roles
//!
//! This module provides role-specific importance calculations, allowing
//! different agents to prioritize memories differently based on their needs.

use crate::agents::AgentRole;
use crate::types::MemoryNote;
use serde::{Deserialize, Serialize};

/// Importance scoring weights
///
/// Different components of importance calculation with configurable weights.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportanceWeights {
    /// Base importance score (from memory.importance)
    pub base: f32,

    /// Access frequency score
    pub access: f32,

    /// Recency score (how recently accessed)
    pub recency: f32,

    /// Relevance score (match to agent role)
    pub relevance: f32,
}

impl Default for ImportanceWeights {
    fn default() -> Self {
        Self {
            base: 0.3,
            access: 0.3,
            recency: 0.3,
            relevance: 0.1,
        }
    }
}

/// Custom importance scorer for agents
///
/// Calculates role-specific importance scores by weighting different
/// factors based on agent needs.
pub struct CustomImportanceScorer {
    /// Agent role for this scorer
    role: AgentRole,
}

impl CustomImportanceScorer {
    /// Create a new importance scorer
    pub fn new(role: AgentRole) -> Self {
        Self { role }
    }

    /// Get role-specific weights
    pub fn get_weights(&self) -> ImportanceWeights {
        match self.role {
            AgentRole::Orchestrator => ImportanceWeights {
                base: 0.3,
                access: 0.2,
                recency: 0.4,
                relevance: 0.1,
            },
            AgentRole::Optimizer => ImportanceWeights {
                base: 0.4,
                access: 0.3,
                recency: 0.1,
                relevance: 0.2,
            },
            AgentRole::Reviewer => ImportanceWeights {
                base: 0.5,
                access: 0.1,
                recency: 0.2,
                relevance: 0.2,
            },
            AgentRole::Executor => ImportanceWeights {
                base: 0.2,
                access: 0.4,
                recency: 0.3,
                relevance: 0.1,
            },
        }
    }
}
