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
use crate::types::{MemoryId, MemoryNote, MemoryType};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;

#[cfg(feature = "python")]
use pyo3::Python;

/// Type-safe adapter for Memory Evolution DSPy operations
pub struct MemoryEvolutionDSpyAdapter {
    bridge: Arc<DSpyBridge>,
}

impl MemoryEvolutionDSpyAdapter {
    /// Create new memory evolution adapter wrapping generic DSPy bridge
    pub fn new(bridge: Arc<DSpyBridge>) -> Self {
        Self { bridge }
    }

    /// Get MemoryEvolutionModule from DSPy service
    #[cfg(feature = "python")]
    async fn get_evolution_module(
        &self,
    ) -> Result<pyo3::Py<pyo3::PyAny>> {
        let service = self.bridge.service().clone();

        tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| -> Result<pyo3::Py<pyo3::PyAny>> {
                let service_guard = service.blocking_lock();
                let service_ref = service_guard.bind(py);

                let module = service_ref
                    .call_method0("get_memory_evolution_module")
                    .map_err(|e| {
                        MnemosyneError::Other(format!(
                            "Failed to get memory evolution module: {}",
                            e
                        ))
                    })?;

                Ok(module.into())
            })
        })
        .await
        .map_err(|e| MnemosyneError::Other(format!("Tokio spawn error: {}", e)))?
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

        let module = self.get_evolution_module().await?;
        let memories_str = serde_json::to_string(&memories_json)?;
        let scores_str = serde_json::to_string(&scores_json)?;
        let avg_sim = cluster.avg_similarity;

        let result = tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| -> Result<HashMap<String, Value>> {
                let module_ref = module.bind(py);

                // Call consolidate_cluster method
                let prediction = module_ref
                    .call_method1(
                        "consolidate_cluster",
                        (memories_str, avg_sim, scores_str),
                    )
                    .map_err(|e| {
                        MnemosyneError::Other(format!("consolidate_cluster call failed: {}", e))
                    })?;

                // Extract fields from prediction
                let mut outputs = HashMap::new();

                let action = prediction.getattr("action")
                    .and_then(|v| v.extract::<String>())
                    .map_err(|e| MnemosyneError::Other(format!("Failed to extract action: {}", e)))?;
                outputs.insert("action".to_string(), json!(action));

                let primary_id = prediction.getattr("primary_memory_id")
                    .and_then(|v| v.extract::<String>())
                    .map_err(|e| MnemosyneError::Other(format!("Failed to extract primary_memory_id: {}", e)))?;
                outputs.insert("primary_memory_id".to_string(), json!(primary_id));

                // secondary_memory_ids - may be string or list
                if let Ok(secondary_ids_str) = prediction.getattr("secondary_memory_ids")
                    .and_then(|v| v.extract::<String>()) {
                    // Parse as JSON array
                    if let Ok(ids) = serde_json::from_str::<Vec<String>>(&secondary_ids_str) {
                        outputs.insert("secondary_memory_ids".to_string(), json!(ids));
                    } else {
                        outputs.insert("secondary_memory_ids".to_string(), json!(Vec::<String>::new()));
                    }
                } else {
                    outputs.insert("secondary_memory_ids".to_string(), json!(Vec::<String>::new()));
                }

                let rationale = prediction.getattr("rationale")
                    .and_then(|v| v.extract::<String>())
                    .unwrap_or_default();
                outputs.insert("rationale".to_string(), json!(rationale));

                let preserved_content = prediction.getattr("preserved_content")
                    .and_then(|v| v.extract::<String>())
                    .unwrap_or_default();
                outputs.insert("preserved_content".to_string(), json!(preserved_content));

                let confidence = prediction.getattr("confidence")
                    .and_then(|v| v.extract::<String>())
                    .and_then(|s| s.parse::<f32>().map_err(|e| pyo3::PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e))))
                    .unwrap_or(0.0);
                outputs.insert("confidence".to_string(), json!(confidence));

                Ok(outputs)
            })
        })
        .await
        .map_err(|e| MnemosyneError::Other(format!("Tokio spawn error: {}", e)))??;

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
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| {
                MnemosyneError::Other("Invalid primary_memory_id in response".to_string())
            })?;

        let secondary_memory_ids: Vec<MemoryId> = result
            .get("secondary_memory_ids")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().and_then(|s| s.parse().ok()))
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
        let module = self.get_evolution_module().await?;

        let memory_id = memory.id.to_string();
        let memory_summary = memory.summary.clone();
        let memory_type = format!("{:?}", memory.memory_type);
        let current_importance = memory.importance;
        let access_count = memory.access_count;

        let now = chrono::Utc::now();
        let days_since_created = (now - memory.created_at).num_days() as i64;
        let days_since_accessed = (now - memory.last_accessed_at).num_days() as i64;
        let linked_memories_count = memory.links.len();
        let namespace = memory.namespace.to_string();

        let result = tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| -> Result<HashMap<String, Value>> {
                let module_ref = module.bind(py);

                // Call recalibrate_importance method
                let prediction = module_ref
                    .call_method1(
                        "recalibrate_importance",
                        (
                            memory_id,
                            memory_summary,
                            memory_type,
                            current_importance,
                            access_count,
                            days_since_created,
                            days_since_accessed,
                            linked_memories_count,
                            namespace,
                        ),
                    )
                    .map_err(|e| {
                        MnemosyneError::Other(format!("recalibrate_importance call failed: {}", e))
                    })?;

                // Extract fields
                let mut outputs = HashMap::new();

                let new_importance = prediction.getattr("new_importance")
                    .and_then(|v| v.extract::<String>())
                    .and_then(|s| s.parse::<u8>().map_err(|e| pyo3::PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("{}", e))))
                    .map_err(|e| MnemosyneError::Other(format!("Failed to extract new_importance: {}", e)))?;
                outputs.insert("new_importance".to_string(), json!(new_importance));

                let adjustment_reason = prediction.getattr("adjustment_reason")
                    .and_then(|v| v.extract::<String>())
                    .unwrap_or_default();
                outputs.insert("adjustment_reason".to_string(), json!(adjustment_reason));

                let recommended_action = prediction.getattr("recommended_action")
                    .and_then(|v| v.extract::<String>())
                    .unwrap_or_else(|_| "KEEP".to_string());
                outputs.insert("recommended_action".to_string(), json!(recommended_action));

                Ok(outputs)
            })
        })
        .await
        .map_err(|e| MnemosyneError::Other(format!("Tokio spawn error: {}", e)))??;

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

        let module = self.get_evolution_module().await?;
        let memories_str = serde_json::to_string(&memories_json)?;
        let threshold_days = config.archival_threshold_days;
        let min_importance = config.min_importance;

        let result = tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| -> Result<HashMap<String, Value>> {
                let module_ref = module.bind(py);

                // Call detect_archival_candidates method
                let prediction = module_ref
                    .call_method1(
                        "detect_archival_candidates",
                        (memories_str, threshold_days, min_importance),
                    )
                    .map_err(|e| {
                        MnemosyneError::Other(format!(
                            "detect_archival_candidates call failed: {}",
                            e
                        ))
                    })?;

                // Extract fields
                let mut outputs = HashMap::new();

                // archive_ids - may be string or list
                if let Ok(archive_ids_str) = prediction.getattr("archive_ids")
                    .and_then(|v| v.extract::<String>()) {
                    if let Ok(ids) = serde_json::from_str::<Vec<String>>(&archive_ids_str) {
                        outputs.insert("archive_ids".to_string(), json!(ids));
                    } else {
                        outputs.insert("archive_ids".to_string(), json!(Vec::<String>::new()));
                    }
                } else {
                    outputs.insert("archive_ids".to_string(), json!(Vec::<String>::new()));
                }

                // keep_ids - may be string or list
                if let Ok(keep_ids_str) = prediction.getattr("keep_ids")
                    .and_then(|v| v.extract::<String>()) {
                    if let Ok(ids) = serde_json::from_str::<Vec<String>>(&keep_ids_str) {
                        outputs.insert("keep_ids".to_string(), json!(ids));
                    } else {
                        outputs.insert("keep_ids".to_string(), json!(Vec::<String>::new()));
                    }
                } else {
                    outputs.insert("keep_ids".to_string(), json!(Vec::<String>::new()));
                }

                let rationale = prediction.getattr("rationale")
                    .and_then(|v| v.extract::<String>())
                    .unwrap_or_default();
                outputs.insert("rationale".to_string(), json!(rationale));

                Ok(outputs)
            })
        })
        .await
        .map_err(|e| MnemosyneError::Other(format!("Tokio spawn error: {}", e)))??;

        let archive_ids: Vec<MemoryId> = result
            .get("archive_ids")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().and_then(|s| s.parse().ok()))
                    .collect()
            })
            .unwrap_or_default();

        let keep_ids: Vec<MemoryId> = result
            .get("keep_ids")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().and_then(|s| s.parse().ok()))
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
