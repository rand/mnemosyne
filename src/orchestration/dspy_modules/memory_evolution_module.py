"""MemoryEvolutionModule - DSPy signatures for memory evolution operations.

This module provides DSPy-based implementations for:
- Memory cluster consolidation (MERGE|SUPERSEDE|KEEP decisions)
- Importance recalibration (adjust importance based on access patterns and age)
- Archival candidate detection (identify memories for archival)

All operations use ChainOfThought for transparency and optimizability.
"""

import dspy
from typing import List, Dict, Any


# =============================================================================
# Signatures
# =============================================================================


class ConsolidateMemoryCluster(dspy.Signature):
    """Analyze memory cluster and decide consolidation strategy.

    Given a cluster of similar memories (detected via vector similarity),
    decide whether to:
    - MERGE: Combine into single memory (truly duplicates)
    - SUPERSEDE: One memory obsoletes another (newer/better version)
    - KEEP: Keep separate (meaningful differences despite similarity)

    Decision should consider:
    - Semantic similarity vs superficial similarity
    - Temporal context (older vs newer information)
    - Information preservation (what would be lost in merge/supersede)
    - Usage patterns (which memories are accessed)
    """

    cluster_memories = dspy.InputField(
        desc="List of memory metadata: [{id, created, updated, summary, content_preview, keywords, memory_type, importance, access_count}]"
    )
    avg_similarity = dspy.InputField(
        desc="Average vector similarity score in cluster (0.0-1.0)"
    )
    similarity_scores = dspy.InputField(
        desc="Pairwise similarity scores: [(mem_id_1, mem_id_2, score)]"
    )

    action = dspy.OutputField(
        desc="Consolidation action: MERGE|SUPERSEDE|KEEP"
    )
    primary_memory_id = dspy.OutputField(
        desc="ID of memory to keep or enhance (if MERGE/SUPERSEDE)"
    )
    secondary_memory_ids = dspy.OutputField(
        desc="List of memory IDs to merge into primary or mark as superseded"
    )
    rationale = dspy.OutputField(
        desc="Detailed explanation of decision: why this action, what distinguishes these memories, what happens to information"
    )
    preserved_content = dspy.OutputField(
        desc="Key facts, insights, or context to preserve from secondary memories if merging/superseding"
    )
    confidence = dspy.OutputField(
        desc="Confidence score for this decision (0.0-1.0)"
    )


class RecalibrateImportance(dspy.Signature):
    """Recalibrate memory importance based on access patterns and age.

    Importance should reflect:
    - Current relevance (recent access indicates value)
    - Age-adjusted value (some memories gain value over time, others decay)
    - Network effects (highly connected memories are valuable)
    - Type-specific considerations (Architecture > Debug, Insight > Log)

    Output new importance score and recommended action.
    """

    memory_id = dspy.InputField(desc="Memory ID")
    memory_summary = dspy.InputField(desc="Memory summary")
    memory_type = dspy.InputField(
        desc="Type: Insight|Decision|Architecture|Task|Reference|Event|Debug"
    )
    current_importance = dspy.InputField(desc="Current importance score (1-10)")
    access_count = dspy.InputField(desc="Number of times accessed")
    days_since_created = dspy.InputField(desc="Age in days")
    days_since_accessed = dspy.InputField(desc="Days since last access")
    linked_memories_count = dspy.InputField(desc="Number of linked memories")
    namespace = dspy.InputField(
        desc="Memory namespace (global, project:X, session:Y)"
    )

    new_importance = dspy.OutputField(
        desc="Recalibrated importance score (1-10)"
    )
    adjustment_reason = dspy.OutputField(
        desc="Explanation of why importance changed (increased/decreased/unchanged)"
    )
    recommended_action = dspy.OutputField(
        desc="Recommended action: KEEP (active)|ARCHIVE (preserve but inactive)|DELETE (low value)"
    )


class DetectArchivalCandidates(dspy.Signature):
    """Identify memories suitable for archival.

    Archival preserves memories while removing them from active recall.
    Good archival candidates:
    - Old memories with low recent access
    - Low importance, not critical to current work
    - Superseded by other memories
    - Event logs and debug info past retention period

    Bad archival candidates:
    - Highly connected memories (central to knowledge graph)
    - Recent architecture decisions
    - Actively accessed memories
    - High-importance insights regardless of age
    """

    memories = dspy.InputField(
        desc="List of memory metadata: [{id, summary, type, importance, age_days, access_count, days_since_access, linked_count}]"
    )
    archival_threshold_days = dspy.InputField(
        desc="Age threshold in days for archival consideration"
    )
    min_importance = dspy.InputField(
        desc="Minimum importance score to keep active regardless of age"
    )

    archive_ids = dspy.OutputField(
        desc="List of memory IDs to archive (preserve but mark inactive)"
    )
    keep_ids = dspy.OutputField(
        desc="List of memory IDs to keep active"
    )
    rationale = dspy.OutputField(
        desc="Explanation of archival decisions: why archive these, why keep others, what patterns were identified"
    )


# =============================================================================
# Module
# =============================================================================


class MemoryEvolutionModule(dspy.Module):
    """DSPy module for memory evolution operations.

    Provides three core capabilities:
    1. consolidate_cluster: Decide MERGE/SUPERSEDE/KEEP for similar memories
    2. recalibrate_importance: Adjust importance based on access patterns
    3. detect_archival_candidates: Identify memories for archival

    All methods use ChainOfThought for reasoning transparency.
    """

    def __init__(self):
        """Initialize MemoryEvolutionModule with ChainOfThought for all signatures."""
        super().__init__()
        self.consolidate = dspy.ChainOfThought(ConsolidateMemoryCluster)
        self.recalibrate = dspy.ChainOfThought(RecalibrateImportance)
        self.detect_archival = dspy.ChainOfThought(DetectArchivalCandidates)

    def consolidate_cluster(
        self,
        cluster_memories: List[Dict[str, Any]],
        avg_similarity: float,
        similarity_scores: List[tuple],
    ) -> dspy.Prediction:
        """Decide consolidation strategy for memory cluster.

        Args:
            cluster_memories: List of memory metadata dicts
            avg_similarity: Average similarity in cluster (0.0-1.0)
            similarity_scores: List of (mem_id_1, mem_id_2, score) tuples

        Returns:
            Prediction with:
            - action: "MERGE"|"SUPERSEDE"|"KEEP"
            - primary_memory_id: Memory to keep/enhance
            - secondary_memory_ids: Memories to merge/supersede
            - rationale: Explanation of decision
            - preserved_content: Content to preserve from secondary memories
            - confidence: Confidence score (0.0-1.0)
        """
        # Format inputs for prompt
        memories_formatted = str(cluster_memories)
        scores_formatted = str(similarity_scores)

        result = self.consolidate(
            cluster_memories=memories_formatted,
            avg_similarity=str(avg_similarity),
            similarity_scores=scores_formatted,
        )
        return result

    def recalibrate_importance(
        self,
        memory_id: str,
        memory_summary: str,
        memory_type: str,
        current_importance: int,
        access_count: int,
        days_since_created: int,
        days_since_accessed: int,
        linked_memories_count: int,
        namespace: str,
    ) -> dspy.Prediction:
        """Recalibrate importance score for a memory.

        Args:
            memory_id: Memory ID
            memory_summary: Memory summary
            memory_type: Type (Insight|Decision|Architecture|Task|Reference|Event|Debug)
            current_importance: Current importance (1-10)
            access_count: Number of accesses
            days_since_created: Age in days
            days_since_accessed: Days since last access
            linked_memories_count: Number of linked memories
            namespace: Memory namespace

        Returns:
            Prediction with:
            - new_importance: Recalibrated score (1-10)
            - adjustment_reason: Explanation of change
            - recommended_action: "KEEP"|"ARCHIVE"|"DELETE"
        """
        result = self.recalibrate(
            memory_id=memory_id,
            memory_summary=memory_summary,
            memory_type=memory_type,
            current_importance=str(current_importance),
            access_count=str(access_count),
            days_since_created=str(days_since_created),
            days_since_accessed=str(days_since_accessed),
            linked_memories_count=str(linked_memories_count),
            namespace=namespace,
        )
        return result

    def detect_archival_candidates(
        self,
        memories: List[Dict[str, Any]],
        archival_threshold_days: int,
        min_importance: int,
    ) -> dspy.Prediction:
        """Identify memories suitable for archival.

        Args:
            memories: List of memory metadata dicts
            archival_threshold_days: Age threshold for archival consideration
            min_importance: Min importance to keep active regardless of age

        Returns:
            Prediction with:
            - archive_ids: Memory IDs to archive
            - keep_ids: Memory IDs to keep active
            - rationale: Explanation of decisions
        """
        # Format inputs for prompt
        memories_formatted = str(memories)

        result = self.detect_archival(
            memories=memories_formatted,
            archival_threshold_days=str(archival_threshold_days),
            min_importance=str(min_importance),
        )
        return result


# =============================================================================
# Standalone testing (if run directly)
# =============================================================================

if __name__ == "__main__":
    # This would require actual DSPy setup with LM
    print("MemoryEvolutionModule loaded successfully")
    print("Signatures:")
    print("  - ConsolidateMemoryCluster")
    print("  - RecalibrateImportance")
    print("  - DetectArchivalCandidates")
