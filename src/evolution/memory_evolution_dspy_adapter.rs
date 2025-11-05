//! MemoryEvolutionDSpyAdapter - Type-safe wrapper for Memory Evolution DSPy operations
//!
//! Provides strongly-typed Rust interface to MemoryEvolutionModule Python DSPy signatures:
//! - Memory cluster consolidation (MERGE|SUPERSEDE|KEEP decisions)
//! - Importance recalibration (adjust based on access patterns and age)
//! - Archival candidate detection (identify memories for archival)
//!
//! All operations use async spawn_blocking for non-blocking Python GIL access.

use crate::error::{MnemosyneError, Result};
use crate::orchestration::dspy_bridge::DSpyBridge;
use crate::types::{MemoryId, MemoryNote};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

/// Type-safe adapter for Memory Evolution DSPy operations
pub struct MemoryEvolutionDSpyAdapter {
    bridge: Arc<DSpyBridge>,
}

impl MemoryEvolutionDSpyAdapter {
    /// Create new memory evolution adapter wrapping generic DSPy bridge
    pub fn new(bridge: Arc<DSpyBridge>) -> Self {
        Self { bridge }
    }

    /// Consolidate memory cluster using DSPy
    ///
    /// Analyzes a cluster of similar memories and decides consolidation strategy:
    /// - MERGE: Combine into single memory (truly duplicates)
    /// - SUPERSEDE: One memory obsoletes another (newer/better version)
    /// - KEEP: Keep separate (meaningful differences despite similarity)
    ///
    /// # Arguments
    /// * `cluster` - Memory cluster with similarity information
    ///
    /// # Returns
    /// ConsolidationDecision with action and metadata
    #[cfg(feature = "python")]
    pub async fn consolidate_cluster(
        &self,
        cluster: &MemoryCluster,
    ) -> Result<ConsolidationDecision> {
        // Convert cluster memories to JSON metadata
        let memories_json: Vec<Value> = cluster
            .memories
            .iter()
            .map(|m| {
                json!({
                    "id": m.id.to_string(),
                    "created": m.created_at.to_rfc3339(),
                    "updated": m.updated_at.to_rfc3339(),
                    "summary": &m.summary,
                    "content_preview": &m.content.chars().take(200).collect::<String>(),
                    "keywords": &m.keywords,
                    "memory_type": format!("{:?}", m.memory_type),
                    "importance": m.importance,
                    "access_count": m.access_count,
                })
            })
            .collect();

        // Convert similarity scores to JSON
        let scores_json: Vec<Value> = cluster
            .similarity_scores
            .iter()
            .map(|(id1, id2, score)| {
                json!([id1.to_string(), id2.to_string(), score])
            })
            .collect();

        let mut inputs = HashMap::new();
        inputs.insert("cluster_memories".to_string(), json!(memories_json));
        inputs.insert("avg_similarity".to_string(), json!(cluster.avg_similarity));
        inputs.insert("similarity_scores".to_string(), json!(scores_json));

        let result = self
            .bridge
            .call_agent_module("memory_evolution", inputs)
            .await?;

        // Parse action
        let action_str = result
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| MnemosyneError::Other("Missing action in response".to_string()))?;

        let action = match action_str.to_uppercase().as_str() {
            "MERGE" => ConsolidationAction::Merge,
            "SUPERSEDE" => ConsolidationAction::Supersede,
            "KEEP" => ConsolidationAction::Keep,
            _ => {
                return Err(MnemosyneError::Other(format!(
                    "Unknown action: {}",
                    action_str
                )))
            }
        };

        let primary_memory_id: MemoryId = result
            .get("primary_memory_id")
            .and_then(|v| v.as_str())
            .and_then(|s| MemoryId::from_string(s).ok())
            .ok_or_else(|| {
                MnemosyneError::Other("Invalid primary_memory_id in response".to_string())
            })?;

        let secondary_memory_ids: Vec<MemoryId> = result
            .get("secondary_memory_ids")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().and_then(|s| MemoryId::from_string(s).ok()))
                    .collect()
            })
            .unwrap_or_default();

        let rationale: String = result
            .get("rationale")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let preserved_content: String = result
            .get("preserved_content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let confidence: f32 = result
            .get("confidence")
            .and_then(|v| v.as_f64())
            .map(|f| f as f32)
            .unwrap_or(0.0);

        Ok(ConsolidationDecision {
            action,
            memory_ids: cluster.memories.iter().map(|m| m.id).collect(),
            primary_memory_id: Some(primary_memory_id),
            secondary_memory_ids,
            rationale,
            preserved_content,
            confidence,
        })
    }

    /// Recalibrate importance score for a memory
    ///
    /// Adjusts importance based on:
    /// - Access patterns (recent access indicates value)
    /// - Age (some memories gain value, others decay)
    /// - Network effects (highly connected = valuable)
    /// - Type-specific considerations (Architecture > Debug)
    ///
    /// # Arguments
    /// * `memory` - Memory to recalibrate
    ///
    /// # Returns
    /// ImportanceRecalibration with new score and recommended action
    #[cfg(feature = "python")]
    pub async fn recalibrate_importance(
        &self,
        memory: &MemoryNote,
    ) -> Result<ImportanceRecalibration> {
        let now = chrono::Utc::now();
        let days_since_created = (now - memory.created_at).num_days();
        let days_since_accessed = (now - memory.last_accessed_at).num_days();

        let mut inputs = HashMap::new();
        inputs.insert("memory_id".to_string(), json!(memory.id.to_string()));
        inputs.insert("memory_summary".to_string(), json!(memory.summary));
        inputs.insert("memory_type".to_string(), json!(format!("{:?}", memory.memory_type)));
        inputs.insert("current_importance".to_string(), json!(memory.importance));
        inputs.insert("access_count".to_string(), json!(memory.access_count));
        inputs.insert("days_since_created".to_string(), json!(days_since_created));
        inputs.insert("days_since_accessed".to_string(), json!(days_since_accessed));
        inputs.insert("linked_memories_count".to_string(), json!(memory.links.len()));
        inputs.insert("namespace".to_string(), json!(memory.namespace.to_string()));

        let result = self
            .bridge
            .call_agent_module("memory_evolution", inputs)
            .await?;

        let new_importance: u8 = result
            .get("new_importance")
            .and_then(|v| v.as_u64())
            .map(|n| n as u8)
            .unwrap_or(memory.importance);

        let adjustment_reason: String = result
            .get("adjustment_reason")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let recommended_action_str = result
            .get("recommended_action")
            .and_then(|v| v.as_str())
            .unwrap_or("KEEP");

        let recommended_action = match recommended_action_str.to_uppercase().as_str() {
            "KEEP" => RecommendedAction::Keep,
            "ARCHIVE" => RecommendedAction::Archive,
            "DELETE" => RecommendedAction::Delete,
            _ => RecommendedAction::Keep,
        };

        Ok(ImportanceRecalibration {
            new_importance,
            adjustment_reason,
            recommended_action,
        })
    }

    /// Detect archival candidates using DSPy
    ///
    /// Identifies memories suitable for archival (preserve but mark inactive).
    /// Good candidates: old, low access, low importance, superseded
    /// Bad candidates: highly connected, recent architecture, actively accessed
    ///
    /// # Arguments
    /// * `memories` - Memories to analyze
    /// * `config` - Archival configuration
    ///
    /// # Returns
    /// ArchivalDecisions with archive/keep lists and rationale
    #[cfg(feature = "python")]
    pub async fn detect_archival_candidates(
        &self,
        memories: &[MemoryNote],
        config: &ArchivalConfig,
    ) -> Result<ArchivalDecisions> {
        // Convert memories to JSON metadata
        let now = chrono::Utc::now();
        let memories_json: Vec<Value> = memories
            .iter()
            .map(|m| {
                let age_days = (now - m.created_at).num_days();
                let days_since_access = (now - m.last_accessed_at).num_days();

                json!({
                    "id": m.id.to_string(),
                    "summary": &m.summary,
                    "type": format!("{:?}", m.memory_type),
                    "importance": m.importance,
                    "age_days": age_days,
                    "access_count": m.access_count,
                    "days_since_access": days_since_access,
                    "linked_count": m.links.len(),
                })
            })
            .collect();

        let mut inputs = HashMap::new();
        inputs.insert("memories".to_string(), json!(memories_json));
        inputs.insert("archival_threshold_days".to_string(), json!(config.archival_threshold_days));
        inputs.insert("min_importance".to_string(), json!(config.min_importance));

        let result = self
            .bridge
            .call_agent_module("memory_evolution", inputs)
            .await?;

        let archive_ids: Vec<MemoryId> = result
            .get("archive_ids")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().and_then(|s| MemoryId::from_string(s).ok()))
                    .collect()
            })
            .unwrap_or_default();

        let keep_ids: Vec<MemoryId> = result
            .get("keep_ids")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().and_then(|s| MemoryId::from_string(s).ok()))
                    .collect()
            })
            .unwrap_or_default();

        let rationale: String = result
            .get("rationale")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        Ok(ArchivalDecisions {
            archive_ids,
            keep_ids,
            rationale,
        })
    }
}

// =============================================================================
// Type definitions
// =============================================================================

/// Memory cluster for consolidation analysis
#[derive(Debug, Clone)]
pub struct MemoryCluster {
    /// Memories in this cluster
    pub memories: Vec<MemoryNote>,
    /// Pairwise similarity scores
    pub similarity_scores: Vec<(MemoryId, MemoryId, f32)>,
    /// Average similarity within cluster
    pub avg_similarity: f32,
}

/// Consolidation action
#[derive(Debug, Clone, PartialEq)]
pub enum ConsolidationAction {
    /// Merge multiple memories into one
    Merge,
    /// One memory supersedes another
    Supersede,
    /// Keep memories separate
    Keep,
}

/// Consolidation decision from DSPy
#[derive(Debug, Clone)]
pub struct ConsolidationDecision {
    /// Action to take
    pub action: ConsolidationAction,
    /// All memory IDs in cluster
    pub memory_ids: Vec<MemoryId>,
    /// Primary memory to keep/enhance
    pub primary_memory_id: Option<MemoryId>,
    /// Secondary memories to merge/supersede
    pub secondary_memory_ids: Vec<MemoryId>,
    /// Explanation of decision
    pub rationale: String,
    /// Content to preserve from secondary memories
    pub preserved_content: String,
    /// Confidence score (0.0-1.0)
    pub confidence: f32,
}

/// Importance recalibration result
#[derive(Debug, Clone)]
pub struct ImportanceRecalibration {
    /// New importance score (1-10)
    pub new_importance: u8,
    /// Explanation of change
    pub adjustment_reason: String,
    /// Recommended action
    pub recommended_action: RecommendedAction,
}

/// Recommended action after recalibration
#[derive(Debug, Clone, PartialEq)]
pub enum RecommendedAction {
    /// Keep memory active
    Keep,
    /// Archive memory (preserve but inactive)
    Archive,
    /// Delete memory (low value)
    Delete,
}

/// Archival configuration
#[derive(Debug, Clone)]
pub struct ArchivalConfig {
    /// Age threshold in days for archival consideration
    pub archival_threshold_days: i64,
    /// Minimum importance to keep active regardless of age
    pub min_importance: u8,
}

/// Archival decisions from DSPy
#[derive(Debug, Clone)]
pub struct ArchivalDecisions {
    /// Memory IDs to archive
    pub archive_ids: Vec<MemoryId>,
    /// Memory IDs to keep active
    pub keep_ids: Vec<MemoryId>,
    /// Explanation of decisions
    pub rationale: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_is_send_sync() {
        fn assert_send<T: Send>() {}
        fn assert_sync<T: Sync>() {}

        assert_send::<MemoryEvolutionDSpyAdapter>();
        assert_sync::<MemoryEvolutionDSpyAdapter>();
    }
}
