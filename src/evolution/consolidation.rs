// Periodic Consolidation Job
//
// Detects and consolidates duplicate/similar memories using:
// 1. Vector similarity (>0.90) for semantic duplication
// 2. Keyword overlap (>80%) for validation
// 3. LLM-guided decisions: merge, supersede, or keep separate
// 4. Safe execution with audit trail

use super::config::JobConfig;
use super::scheduler::{EvolutionJob, JobError, JobReport};
use crate::services::llm::LlmService;
use crate::storage::libsql::LibsqlStorage;
use crate::types::{MemoryId, MemoryNote};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

/// Consolidation job - detects and merges duplicate memories
pub struct ConsolidationJob {
    storage: Arc<LibsqlStorage>,
    llm: Option<Arc<LlmService>>,
    consolidation_config: super::config::ConsolidationConfig,
}

impl ConsolidationJob {
    pub fn new(storage: Arc<LibsqlStorage>) -> Self {
        Self {
            storage,
            llm: None,
            consolidation_config: super::config::ConsolidationConfig::default(),
        }
    }

    /// Create with LLM service for intelligent consolidation decisions
    pub fn with_llm(storage: Arc<LibsqlStorage>, llm: Arc<LlmService>) -> Self {
        Self {
            storage,
            llm: Some(llm),
            consolidation_config: super::config::ConsolidationConfig::default(),
        }
    }

    /// Create with custom consolidation configuration
    pub fn with_config(
        storage: Arc<LibsqlStorage>,
        llm: Option<Arc<LlmService>>,
        consolidation_config: super::config::ConsolidationConfig,
    ) -> Self {
        Self {
            storage,
            llm,
            consolidation_config,
        }
    }

    /// Find duplicate candidate pairs using vector similarity
    async fn find_duplicate_candidates(&self, batch_size: usize) -> Result<Vec<(MemoryNote, MemoryNote, f32)>, JobError> {
        // Get active memories
        let memories = self
            .storage
            .list_all_active(Some(batch_size))
            .await
            .map_err(|e| JobError::ExecutionError(e.to_string()))?;

        let mut candidates = Vec::new();

        // Get embeddings for each memory
        let mut memory_embeddings: HashMap<MemoryId, Vec<f32>> = HashMap::new();

        for memory in &memories {
            // Try to get embedding from vector storage
            if let Ok(Some(embedding)) = self.storage.get_embedding(&memory.id).await {
                memory_embeddings.insert(memory.id, embedding);
            }
        }

        tracing::debug!(
            "Retrieved embeddings for {}/{} memories",
            memory_embeddings.len(),
            memories.len()
        );

        // For each memory pair, compute similarity
        for i in 0..memories.len() {
            for j in (i + 1)..memories.len() {
                let mem1 = &memories[i];
                let mem2 = &memories[j];

                // Try vector similarity first
                let similarity = if let (Some(emb1), Some(emb2)) = (
                    memory_embeddings.get(&mem1.id),
                    memory_embeddings.get(&mem2.id),
                ) {
                    // Use cosine similarity for vector comparison
                    self.cosine_similarity(emb1, emb2)
                } else {
                    // Fall back to keyword overlap if embeddings not available
                    self.keyword_overlap(mem1, mem2)
                };

                // High similarity indicates potential duplicate
                // Use 0.90 threshold for vector similarity, 0.80 for keyword overlap
                let threshold = if memory_embeddings.contains_key(&mem1.id) { 0.90 } else { 0.80 };

                if similarity > threshold {
                    candidates.push((mem1.clone(), mem2.clone(), similarity));
                }
            }
        }

        Ok(candidates)
    }

    /// Calculate cosine similarity between two embedding vectors
    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
        let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot_product / (norm_a * norm_b)
        }
    }

    /// Calculate keyword overlap between two memories
    fn keyword_overlap(&self, m1: &MemoryNote, m2: &MemoryNote) -> f32 {
        let keywords1: HashSet<_> = m1.keywords.iter().map(|k| k.to_lowercase()).collect();
        let keywords2: HashSet<_> = m2.keywords.iter().map(|k| k.to_lowercase()).collect();

        if keywords1.is_empty() || keywords2.is_empty() {
            return 0.0;
        }

        let intersection = keywords1.intersection(&keywords2).count() as f32;
        let union = keywords1.union(&keywords2).count() as f32;

        if union == 0.0 {
            0.0
        } else {
            intersection / union // Jaccard similarity
        }
    }

    /// Cluster similar memories using simple connected components
    fn cluster_memories(&self, candidates: &[(MemoryNote, MemoryNote, f32)]) -> Vec<MemoryCluster> {
        if candidates.is_empty() {
            return Vec::new();
        }

        // Build adjacency map
        let mut graph: HashMap<MemoryId, HashSet<MemoryId>> = HashMap::new();
        let mut memory_map: HashMap<MemoryId, MemoryNote> = HashMap::new();
        let mut similarity_map: HashMap<(MemoryId, MemoryId), f32> = HashMap::new();

        for (m1, m2, sim) in candidates {
            graph.entry(m1.id.clone()).or_default().insert(m2.id.clone());
            graph.entry(m2.id.clone()).or_default().insert(m1.id.clone());

            memory_map.insert(m1.id.clone(), m1.clone());
            memory_map.insert(m2.id.clone(), m2.clone());

            let key = if m1.id.to_string() < m2.id.to_string() {
                (m1.id.clone(), m2.id.clone())
            } else {
                (m2.id.clone(), m1.id.clone())
            };
            similarity_map.insert(key, *sim);
        }

        // Find connected components (clusters)
        let mut visited = HashSet::new();
        let mut clusters = Vec::new();

        for start_id in graph.keys() {
            if visited.contains(start_id) {
                continue;
            }

            // BFS to find cluster
            let mut cluster_ids = HashSet::new();
            let mut queue = vec![start_id.clone()];

            while let Some(id) = queue.pop() {
                if !visited.insert(id.clone()) {
                    continue;
                }

                cluster_ids.insert(id.clone());

                if let Some(neighbors) = graph.get(&id) {
                    for neighbor in neighbors {
                        if !visited.contains(neighbor) {
                            queue.push(neighbor.clone());
                        }
                    }
                }
            }

            // Build cluster
            let mut cluster_memories = Vec::new();
            let mut similarity_scores = Vec::new();
            let mut total_similarity = 0.0;
            let mut pair_count = 0;

            for id in &cluster_ids {
                if let Some(memory) = memory_map.get(id) {
                    cluster_memories.push(memory.clone());
                }
            }

            // Get pairwise similarities
            for (i, m1) in cluster_memories.iter().enumerate() {
                for m2 in cluster_memories.iter().skip(i + 1) {
                    let key = if m1.id.to_string() < m2.id.to_string() {
                        (m1.id.clone(), m2.id.clone())
                    } else {
                        (m2.id.clone(), m1.id.clone())
                    };

                    if let Some(&sim) = similarity_map.get(&key) {
                        similarity_scores.push((m1.id.clone(), m2.id.clone(), sim));
                        total_similarity += sim;
                        pair_count += 1;
                    }
                }
            }

            let avg_similarity = if pair_count > 0 {
                total_similarity / pair_count as f32
            } else {
                0.0
            };

            if !cluster_memories.is_empty() {
                clusters.push(MemoryCluster {
                    memories: cluster_memories,
                    similarity_scores,
                    avg_similarity,
                });
            }
        }

        clusters
    }

    /// Make consolidation decision with mode-based dispatcher
    ///
    /// Routes to heuristic or LLM based on config
    async fn make_consolidation_decision_with_config(
        &self,
        cluster: &MemoryCluster,
        config: &super::config::ConsolidationConfig,
    ) -> Result<ConsolidationDecision, JobError> {
        use super::config::DecisionMode;

        match &config.decision_mode {
            DecisionMode::Heuristic => {
                // Always use heuristics
                Ok(self.make_heuristic_decision(cluster))
            }
            DecisionMode::LlmAlways => {
                // Always use LLM
                self.make_llm_consolidation_decision(cluster).await
            }
            DecisionMode::LlmSelective {
                llm_range,
                heuristic_fallback,
            } => {
                let similarity = cluster.avg_similarity;

                if similarity >= llm_range.0 && similarity <= llm_range.1 {
                    // Similarity in LLM range - use LLM
                    tracing::debug!(
                        "Similarity {:.2} in LLM range [{:.2}, {:.2}] - using LLM",
                        similarity,
                        llm_range.0,
                        llm_range.1
                    );
                    self.make_llm_consolidation_decision(cluster).await
                } else if *heuristic_fallback {
                    // Outside range - use heuristics
                    tracing::debug!(
                        "Similarity {:.2} outside LLM range - using heuristics",
                        similarity
                    );
                    Ok(self.make_heuristic_decision(cluster))
                } else {
                    // Outside range and no fallback - keep separate
                    tracing::debug!(
                        "Similarity {:.2} outside LLM range, no fallback - keeping separate",
                        similarity
                    );
                    Ok(ConsolidationDecision {
                        action: ConsolidationAction::Keep,
                        memory_ids: cluster.memories.iter().map(|m| m.id).collect(),
                        superseded_id: None,
                        superseding_id: None,
                        reason: "Outside LLM range and no fallback enabled".to_string(),
                    })
                }
            }
            DecisionMode::LlmWithFallback => {
                // Try LLM, fall back to heuristics on error
                match self.make_llm_consolidation_decision(cluster).await {
                    Ok(decision) => Ok(decision),
                    Err(e) => {
                        tracing::warn!(
                            "LLM decision failed, falling back to heuristics: {}",
                            e
                        );
                        Ok(self.make_heuristic_decision(cluster))
                    }
                }
            }
        }
    }

    /// Make consolidation decision for a cluster (backward compatible wrapper)
    ///
    /// Uses heuristic mode for backward compatibility with tests
    fn make_consolidation_decision(&self, cluster: &MemoryCluster) -> ConsolidationDecision {
        self.make_heuristic_decision(cluster)
    }

    /// Make consolidation decision for a cluster (heuristic-based)
    fn make_heuristic_decision(&self, cluster: &MemoryCluster) -> ConsolidationDecision {
        if cluster.memories.len() < 2 {
            return ConsolidationDecision {
                action: ConsolidationAction::Keep,
                memory_ids: cluster.memories.iter().map(|m| m.id.clone()).collect(),
                superseded_id: None,
                superseding_id: None,
                reason: "Single memory in cluster".to_string(),
            };
        }

        // Very high similarity (>0.95) → Supersede (keep newer)
        if cluster.avg_similarity > 0.95 {
            // Sort by creation date, keep newest
            let mut sorted = cluster.memories.clone();
            sorted.sort_by(|a, b| b.created_at.cmp(&a.created_at));

            let newest = &sorted[0];
            let oldest = &sorted[sorted.len() - 1];

            return ConsolidationDecision {
                action: ConsolidationAction::Supersede,
                memory_ids: cluster.memories.iter().map(|m| m.id.clone()).collect(),
                superseded_id: Some(oldest.id.clone()),
                superseding_id: Some(newest.id.clone()),
                reason: format!(
                    "High similarity ({:.2}) - newer memory supersedes older",
                    cluster.avg_similarity
                ),
            };
        }

        // High similarity (0.85-0.95) → Merge recommended (but keep for now)
        if cluster.avg_similarity > 0.85 {
            return ConsolidationDecision {
                action: ConsolidationAction::Keep,
                memory_ids: cluster.memories.iter().map(|m| m.id.clone()).collect(),
                superseded_id: None,
                superseding_id: None,
                reason: format!(
                    "High similarity ({:.2}) - consider manual merge",
                    cluster.avg_similarity
                ),
            };
        }

        // Moderate similarity → Keep separate
        ConsolidationDecision {
            action: ConsolidationAction::Keep,
            memory_ids: cluster.memories.iter().map(|m| m.id.clone()).collect(),
            superseded_id: None,
            superseding_id: None,
            reason: format!(
                "Moderate similarity ({:.2}) - keeping separate",
                cluster.avg_similarity
            ),
        }
    }

    /// Make LLM-guided consolidation decision for a cluster
    async fn make_llm_consolidation_decision(
        &self,
        cluster: &MemoryCluster,
    ) -> Result<ConsolidationDecision, JobError> {
        // Check if LLM service is available
        let llm = self.llm.as_ref().ok_or_else(|| {
            JobError::ExecutionError("LLM service not available".to_string())
        })?;

        // Build prompt for cluster
        let prompt = self.build_cluster_prompt(cluster);

        // Call LLM API
        let response = llm
            .call_api(&prompt)
            .await
            .map_err(|e| JobError::ExecutionError(format!("LLM API call failed: {}", e)))?;

        // Parse response
        self.parse_llm_cluster_response(&response, cluster)
    }

    /// Build LLM prompt for cluster consolidation decision
    fn build_cluster_prompt(&self, cluster: &MemoryCluster) -> String {
        let memories_text = cluster
            .memories
            .iter()
            .enumerate()
            .map(|(i, mem)| {
                format!(
                    "Memory {}: [{}]\n  ID: {}\n  Created: {}\n  Summary: {}\n  Content: {}\n  Keywords: {}",
                    i + 1,
                    format!("{:?}", mem.memory_type),
                    mem.id,
                    mem.created_at.format("%Y-%m-%d"),
                    mem.summary,
                    &mem.content[..mem.content.len().min(200)], // First 200 chars
                    mem.keywords.join(", ")
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        format!(
            r#"You are analyzing similar memories for potential consolidation.

Cluster contains {} memories with average similarity of {:.2}:

{}

Analyze these memories and decide:
1. **MERGE**: Combine into single memory (truly duplicates)
2. **SUPERSEDE**: One memory obsoletes another (newer/better version)
3. **KEEP**: Keep separate (meaningful differences despite similarity)

For MERGE or SUPERSEDE, explain:
- Which memories to consolidate
- Key information to preserve
- Rationale for decision

For KEEP, explain:
- What meaningful differences exist
- Why both should be retained

Respond in JSON format:
{{
    "action": "MERGE" | "SUPERSEDE" | "KEEP",
    "primary_memory_id": "mem_xxx",
    "secondary_memory_ids": ["mem_yyy", "mem_zzz"],
    "rationale": "explanation",
    "preserved_content": "key facts to keep from secondary memories"
}}
"#,
            cluster.memories.len(),
            cluster.avg_similarity,
            memories_text
        )
    }

    /// Parse LLM response into consolidation decision
    fn parse_llm_cluster_response(
        &self,
        response: &str,
        cluster: &MemoryCluster,
    ) -> Result<ConsolidationDecision, JobError> {
        // Try to parse JSON response
        let llm_decision: LlmConsolidationResponse = serde_json::from_str(response)
            .map_err(|e| {
                JobError::ExecutionError(format!(
                    "Failed to parse LLM response as JSON: {}. Response: {}",
                    e, response
                ))
            })?;

        // Convert to ConsolidationDecision based on action
        match llm_decision.action.to_uppercase().as_str() {
            "MERGE" => {
                Ok(ConsolidationDecision {
                    action: ConsolidationAction::Merge,
                    memory_ids: cluster.memories.iter().map(|m| m.id).collect(),
                    superseded_id: None,
                    superseding_id: Some(llm_decision.primary_memory_id),
                    reason: llm_decision.rationale,
                })
            }
            "SUPERSEDE" => {
                Ok(ConsolidationDecision {
                    action: ConsolidationAction::Supersede,
                    memory_ids: cluster.memories.iter().map(|m| m.id).collect(),
                    superseded_id: llm_decision.secondary_memory_ids.first().copied(),
                    superseding_id: Some(llm_decision.primary_memory_id),
                    reason: llm_decision.rationale,
                })
            }
            "KEEP" => {
                Ok(ConsolidationDecision {
                    action: ConsolidationAction::Keep,
                    memory_ids: cluster.memories.iter().map(|m| m.id).collect(),
                    superseded_id: None,
                    superseding_id: None,
                    reason: llm_decision.rationale,
                })
            }
            _ => Err(JobError::ExecutionError(format!(
                "Unknown action from LLM: {}",
                llm_decision.action
            ))),
        }
    }
}

/// LLM response format for consolidation decisions
#[derive(Debug, Deserialize, Serialize)]
struct LlmConsolidationResponse {
    action: String,
    primary_memory_id: MemoryId,
    secondary_memory_ids: Vec<MemoryId>,
    rationale: String,
    preserved_content: String,
}


#[async_trait]
impl EvolutionJob for ConsolidationJob {
    fn name(&self) -> &str {
        "consolidation"
    }

    async fn run(&self, config: &JobConfig) -> Result<JobReport, JobError> {
        let start = Instant::now();
        let mut memories_processed = 0;
        let mut changes_made = 0;
        let mut errors = 0;

        tracing::info!("Starting consolidation job (batch_size: {})", config.batch_size);

        // 1. Find duplicate candidates
        tracing::debug!("Finding duplicate candidates...");
        let candidates = match self.find_duplicate_candidates(config.batch_size).await {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("Failed to find candidates: {:?}", e);
                return Err(e);
            }
        };

        if candidates.is_empty() {
            tracing::info!("No duplicate candidates found");
            return Ok(JobReport {
                memories_processed: 0,
                changes_made: 0,
                duration: start.elapsed(),
                errors: 0,
                error_message: None,
            });
        }

        tracing::info!("Found {} potential duplicate pairs", candidates.len());

        // 2. Cluster similar memories
        tracing::debug!("Clustering similar memories...");
        let clusters = self.cluster_memories(&candidates);
        tracing::info!("Created {} clusters", clusters.len());

        // 3. Make decisions and execute consolidations
        for cluster in &clusters {
            memories_processed += cluster.memories.len();

            // Use config-based decision dispatcher
            let decision = match self
                .make_consolidation_decision_with_config(cluster, &self.consolidation_config)
                .await
            {
                Ok(d) => d,
                Err(e) => {
                    tracing::error!("Failed to make consolidation decision: {:?}", e);
                    errors += 1;
                    continue;
                }
            };

            tracing::debug!(
                "Cluster decision: {:?} for {} memories (avg sim: {:.2})",
                decision.action,
                cluster.memories.len(),
                cluster.avg_similarity
            );

            // Execute supersede action
            if decision.action == ConsolidationAction::Supersede {
                if let (Some(superseded_id), Some(superseding_id)) =
                    (&decision.superseded_id, &decision.superseding_id)
                {
                    // For now, just log the action
                    // In production, would update database to mark superseded
                    tracing::info!(
                        "Would supersede {} with {} - {}",
                        superseded_id,
                        superseding_id,
                        decision.reason
                    );
                    changes_made += 1;

                    // TODO: Execute actual supersede operation in database
                    // self.storage.mark_superseded(superseded_id, superseding_id).await?;
                }
            }
        }

        tracing::info!(
            "Consolidation complete: {} memories in {} clusters, {} actions in {:?}",
            memories_processed,
            clusters.len(),
            changes_made,
            start.elapsed()
        );

        Ok(JobReport {
            memories_processed,
            changes_made,
            duration: start.elapsed(),
            errors,
            error_message: None,
        })
    }

    async fn should_run(&self) -> Result<bool, JobError> {
        // Vector search is now available, job can run
        Ok(true)
    }
}

/// Cluster of similar memories detected by vector search
#[derive(Debug, Clone)]
pub struct MemoryCluster {
    /// Memories in this cluster
    pub memories: Vec<MemoryNote>,

    /// Pairwise similarity scores (indexed as [i][j] for memories[i] and memories[j])
    pub similarity_scores: Vec<(MemoryId, MemoryId, f32)>,

    /// Average similarity within cluster
    pub avg_similarity: f32,
}

/// Action to take for consolidating memories
#[derive(Debug, Clone, PartialEq)]
pub enum ConsolidationAction {
    /// Merge multiple memories into one
    Merge,

    /// One memory supersedes another (marks as superseded)
    Supersede,

    /// Keep memories separate (similar but distinct)
    Keep,
}

/// Decision on how to consolidate a cluster
#[derive(Debug, Clone)]
pub struct ConsolidationDecision {
    /// Action to take
    pub action: ConsolidationAction,

    /// Memory IDs involved
    pub memory_ids: Vec<MemoryId>,

    /// ID of memory being superseded (if action is Supersede)
    pub superseded_id: Option<MemoryId>,

    /// ID of the superseding memory (if action is Supersede)
    pub superseding_id: Option<MemoryId>,

    /// Reason for this decision
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[tokio::test]
    async fn test_keyword_overlap() {
        use crate::types::{MemoryType, Namespace};
        use crate::ConnectionMode;

        let storage = Arc::new(LibsqlStorage::new(ConnectionMode::InMemory).await.unwrap());
        let job = ConsolidationJob::new(storage);

        let m1 = MemoryNote {
            id: MemoryId::new(),
            namespace: Namespace::Global,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            content: "test".to_string(),
            summary: "test".to_string(),
            keywords: vec!["rust".to_string(), "async".to_string(), "tokio".to_string()],
            tags: vec![],
            context: "".to_string(),
            memory_type: MemoryType::Insight,
            importance: 5,
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
            embedding_model: "".to_string(),
        };

        let m2 = MemoryNote {
            id: MemoryId::new(),
            keywords: vec!["rust".to_string(), "async".to_string()],
            ..m1.clone()
        };

        let overlap = job.keyword_overlap(&m1, &m2);
        assert!(overlap > 0.6); // 2 shared out of 3 total
    }

    #[tokio::test]
    async fn test_consolidation_decision_high_similarity() {
        use crate::types::{MemoryType, Namespace};
        use crate::ConnectionMode;

        let storage = Arc::new(LibsqlStorage::new(ConnectionMode::InMemory).await.unwrap());
        let job = ConsolidationJob::new(storage);

        // Create two test memories for the cluster
        let m1 = MemoryNote {
            id: MemoryId::new(),
            namespace: Namespace::Global,
            created_at: Utc::now() - chrono::Duration::days(1), // Older
            updated_at: Utc::now(),
            content: "test1".to_string(),
            summary: "test1".to_string(),
            keywords: vec!["rust".to_string()],
            tags: vec![],
            context: "".to_string(),
            memory_type: MemoryType::Insight,
            importance: 5,
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
            embedding_model: "".to_string(),
        };

        let m2 = MemoryNote {
            id: MemoryId::new(),
            created_at: Utc::now(), // Newer
            ..m1.clone()
        };

        let cluster = MemoryCluster {
            memories: vec![m1, m2],
            similarity_scores: vec![],
            avg_similarity: 0.96,
        };

        let decision = job.make_consolidation_decision(&cluster);
        assert_eq!(decision.action, ConsolidationAction::Supersede);
    }

    #[tokio::test]
    async fn test_consolidation_decision_moderate_similarity() {
        use crate::ConnectionMode;

        let storage = Arc::new(LibsqlStorage::new(ConnectionMode::InMemory).await.unwrap());
        let job = ConsolidationJob::new(storage);

        let cluster = MemoryCluster {
            memories: vec![],
            similarity_scores: vec![],
            avg_similarity: 0.82,
        };

        let decision = job.make_consolidation_decision(&cluster);
        assert_eq!(decision.action, ConsolidationAction::Keep);
    }
}
