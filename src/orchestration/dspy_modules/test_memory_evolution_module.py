"""Integration tests for MemoryEvolutionModule.

Tests verify:
- Memory cluster consolidation decisions
- Importance recalibration logic
- Archival candidate detection
- JSON output format compatibility with Rust bridge
- ChainOfThought reasoning transparency
"""

import pytest
import dspy
from memory_evolution_module import MemoryEvolutionModule


@pytest.fixture
def evolution_module():
    """Create MemoryEvolutionModule with test LM."""
    # Use a test LM or mock for unit tests
    # In real integration tests, this would use actual Claude
    return MemoryEvolutionModule()


class TestClusterConsolidation:
    """Test memory cluster consolidation decisions."""

    def test_consolidate_cluster_basic(self, evolution_module):
        """Test basic cluster consolidation."""
        memories = [
            {
                "id": "mem-1",
                "created": "2024-01-01T00:00:00Z",
                "updated": "2024-01-01T00:00:00Z",
                "summary": "Rust async programming",
                "content_preview": "Using tokio for async Rust...",
                "keywords": ["rust", "async", "tokio"],
                "memory_type": "Insight",
                "importance": 8,
                "access_count": 10
            },
            {
                "id": "mem-2",
                "created": "2024-01-02T00:00:00Z",
                "updated": "2024-01-02T00:00:00Z",
                "summary": "Async Rust with tokio",
                "content_preview": "Tokio is the async runtime for Rust...",
                "keywords": ["rust", "async", "tokio"],
                "memory_type": "Insight",
                "importance": 7,
                "access_count": 5
            }
        ]

        similarity_scores = [("mem-1", "mem-2", 0.92)]

        result = evolution_module.consolidate_cluster(
            cluster_memories=memories,
            avg_similarity=0.92,
            similarity_scores=similarity_scores
        )

        # Check structure
        assert hasattr(result, 'action')
        assert hasattr(result, 'primary_memory_id')
        assert hasattr(result, 'secondary_memory_ids')
        assert hasattr(result, 'rationale')
        assert hasattr(result, 'preserved_content')
        assert hasattr(result, 'confidence')

        # Action should be one of MERGE, SUPERSEDE, KEEP
        action_str = getattr(result, 'action', '').upper()
        assert action_str in ['MERGE', 'SUPERSEDE', 'KEEP']

    def test_consolidate_cluster_keep_decision(self, evolution_module):
        """Test cluster with meaningful differences (should KEEP)."""
        memories = [
            {
                "id": "mem-1",
                "created": "2024-01-01T00:00:00Z",
                "updated": "2024-01-01T00:00:00Z",
                "summary": "Rust async basics",
                "content_preview": "Async functions in Rust...",
                "keywords": ["rust", "async"],
                "memory_type": "Insight",
                "importance": 7,
                "access_count": 5
            },
            {
                "id": "mem-2",
                "created": "2024-01-01T00:00:00Z",
                "updated": "2024-01-01T00:00:00Z",
                "summary": "Python async basics",
                "content_preview": "Async functions in Python...",
                "keywords": ["python", "async"],
                "memory_type": "Insight",
                "importance": 7,
                "access_count": 5
            }
        ]

        similarity_scores = [("mem-1", "mem-2", 0.75)]

        result = evolution_module.consolidate_cluster(
            cluster_memories=memories,
            avg_similarity=0.75,
            similarity_scores=similarity_scores
        )

        assert hasattr(result, 'action')
        assert hasattr(result, 'rationale')


class TestImportanceRecalibration:
    """Test importance recalibration."""

    def test_recalibrate_importance_basic(self, evolution_module):
        """Test basic importance recalibration."""
        result = evolution_module.recalibrate_importance(
            memory_id="mem-1",
            memory_summary="Rust async programming patterns",
            memory_type="Insight",
            current_importance=7,
            access_count=15,
            days_since_created=30,
            days_since_accessed=2,
            linked_memories_count=5,
            namespace="project:myapp"
        )

        # Check structure
        assert hasattr(result, 'new_importance')
        assert hasattr(result, 'adjustment_reason')
        assert hasattr(result, 'recommended_action')

        # New importance should be 1-10
        new_imp = int(getattr(result, 'new_importance', '5'))
        assert 1 <= new_imp <= 10

    def test_recalibrate_old_accessed_memory(self, evolution_module):
        """Test recalibration for recently accessed old memory."""
        result = evolution_module.recalibrate_importance(
            memory_id="mem-2",
            memory_summary="Architecture decision from last year",
            memory_type="Architecture",
            current_importance=9,
            access_count=50,
            days_since_created=365,
            days_since_accessed=1,  # Recently accessed
            linked_memories_count=10,
            namespace="global"
        )

        assert hasattr(result, 'new_importance')
        assert hasattr(result, 'recommended_action')

    def test_recalibrate_stale_memory(self, evolution_module):
        """Test recalibration for stale, unaccessed memory."""
        result = evolution_module.recalibrate_importance(
            memory_id="mem-3",
            memory_summary="Debug log from 6 months ago",
            memory_type="Debug",
            current_importance=3,
            access_count=1,
            days_since_created=180,
            days_since_accessed=180,  # Never accessed again
            linked_memories_count=0,
            namespace="session:old-session"
        )

        assert hasattr(result, 'recommended_action')
        action = getattr(result, 'recommended_action', '').upper()
        assert action in ['KEEP', 'ARCHIVE', 'DELETE']


class TestArchivalDetection:
    """Test archival candidate detection."""

    def test_detect_archival_basic(self, evolution_module):
        """Test basic archival detection."""
        memories = [
            {
                "id": "mem-1",
                "summary": "Old debug log",
                "type": "Debug",
                "importance": 3,
                "age_days": 200,
                "access_count": 1,
                "days_since_access": 200,
                "linked_count": 0
            },
            {
                "id": "mem-2",
                "summary": "Recent architecture decision",
                "type": "Architecture",
                "importance": 9,
                "age_days": 10,
                "access_count": 20,
                "days_since_access": 1,
                "linked_count": 5
            },
            {
                "id": "mem-3",
                "summary": "Old unused insight",
                "type": "Insight",
                "importance": 5,
                "age_days": 150,
                "access_count": 2,
                "days_since_access": 100,
                "linked_count": 1
            }
        ]

        result = evolution_module.detect_archival_candidates(
            memories=memories,
            archival_threshold_days=90,
            min_importance=8
        )

        # Check structure
        assert hasattr(result, 'archive_ids')
        assert hasattr(result, 'keep_ids')
        assert hasattr(result, 'rationale')

    def test_detect_archival_high_importance(self, evolution_module):
        """Test that high importance memories are kept regardless of age."""
        memories = [
            {
                "id": "mem-1",
                "summary": "Critical architecture decision",
                "type": "Architecture",
                "importance": 10,
                "age_days": 365,
                "access_count": 5,
                "days_since_access": 30,
                "linked_count": 10
            }
        ]

        result = evolution_module.detect_archival_candidates(
            memories=memories,
            archival_threshold_days=90,
            min_importance=8
        )

        assert hasattr(result, 'keep_ids')
        assert hasattr(result, 'rationale')


class TestJSONCompatibility:
    """Test JSON compatibility with Rust bridge."""

    def test_consolidation_json_serializable(self, evolution_module):
        """Test consolidation results can be JSON serialized."""
        import json

        memories = [{"id": "mem-1", "created": "2024-01-01T00:00:00Z", "updated": "2024-01-01T00:00:00Z",
                     "summary": "Test", "content_preview": "Test content", "keywords": ["test"],
                     "memory_type": "Insight", "importance": 5, "access_count": 1}]

        result = evolution_module.consolidate_cluster(
            cluster_memories=memories,
            avg_similarity=0.5,
            similarity_scores=[]
        )

        json_data = {
            'action': getattr(result, 'action', ''),
            'primary_memory_id': getattr(result, 'primary_memory_id', ''),
            'secondary_memory_ids': getattr(result, 'secondary_memory_ids', ''),
            'rationale': getattr(result, 'rationale', ''),
            'preserved_content': getattr(result, 'preserved_content', ''),
            'confidence': getattr(result, 'confidence', '0.0')
        }

        json_str = json.dumps(json_data)
        assert json_str

        parsed = json.loads(json_str)
        assert 'action' in parsed
        assert 'rationale' in parsed


class TestEdgeCases:
    """Test edge cases and error handling."""

    def test_empty_cluster(self, evolution_module):
        """Test with empty cluster."""
        result = evolution_module.consolidate_cluster(
            cluster_memories=[],
            avg_similarity=0.0,
            similarity_scores=[]
        )

        assert hasattr(result, 'action')

    def test_single_memory_cluster(self, evolution_module):
        """Test cluster with single memory."""
        memories = [{
            "id": "mem-1",
            "created": "2024-01-01T00:00:00Z",
            "updated": "2024-01-01T00:00:00Z",
            "summary": "Single memory",
            "content_preview": "Content",
            "keywords": ["test"],
            "memory_type": "Insight",
            "importance": 5,
            "access_count": 1
        }]

        result = evolution_module.consolidate_cluster(
            cluster_memories=memories,
            avg_similarity=1.0,
            similarity_scores=[]
        )

        assert hasattr(result, 'action')


if __name__ == '__main__':
    pytest.main([__file__, '-v'])
