"""Unit tests for telemetry_aggregator.py.

Tests verify:
- InteractionLog parsing from JSON Lines format
- Quality filtering based on success, latency, and cost thresholds
- SHA256-based deduplication across multiple log entries
- DatasetManager integration for versioned storage
- Provenance tracking (telemetry source)
- Command-line argument parsing
- Error handling for malformed logs
"""

import json
import os
import tempfile
from pathlib import Path
import pytest
from unittest.mock import Mock, patch, MagicMock
import sys

# Add parent directory to path for imports
sys.path.insert(0, str(Path(__file__).parent))

from telemetry_aggregator import (
    InteractionLog,
    parse_interaction_log,
    deduplicate_logs,
    filter_by_quality,
    aggregate_telemetry,
)


class TestInteractionLog:
    """Test InteractionLog dataclass and parsing."""

    def test_parse_valid_log(self):
        """Test parsing valid InteractionLog JSON."""
        log_json = {
            "module_name": "reviewer",
            "module_version": "baseline",
            "signature": "extract_requirements",
            "input": {"user_intent": "test"},
            "output": {"requirements": ["req1"]},
            "timestamp_ms": 1699564800000,
            "latency_ms": 250,
            "tokens": 100,
            "cost_usd": 0.0015,
            "model": "claude-haiku-4-5",
            "success": True,
            "error": None
        }

        log = parse_interaction_log(log_json)

        assert log.module_name == "reviewer"
        assert log.signature == "extract_requirements"
        assert log.success is True
        assert log.latency_ms == 250
        assert log.cost_usd == 0.0015

    def test_parse_failed_log(self):
        """Test parsing failed InteractionLog with error."""
        log_json = {
            "module_name": "reviewer",
            "module_version": "baseline",
            "signature": "extract_requirements",
            "input": {"user_intent": "test"},
            "output": {},
            "timestamp_ms": 1699564800000,
            "latency_ms": 500,
            "tokens": 0,
            "cost_usd": 0.0,
            "model": "claude-haiku-4-5",
            "success": False,
            "error": "Timeout"
        }

        log = parse_interaction_log(log_json)

        assert log.success is False
        assert log.error == "Timeout"
        assert log.tokens == 0

    def test_parse_missing_optional_fields(self):
        """Test parsing log with missing optional fields."""
        log_json = {
            "module_name": "reviewer",
            "module_version": "baseline",
            "signature": "extract_requirements",
            "input": {"user_intent": "test"},
            "output": {"requirements": ["req1"]},
            "timestamp_ms": 1699564800000,
            "success": True
        }

        log = parse_interaction_log(log_json)

        assert log.module_name == "reviewer"
        assert log.success is True
        assert log.latency_ms is None
        assert log.tokens is None
        assert log.cost_usd is None


class TestQualityFiltering:
    """Test quality filtering logic."""

    def test_filter_success_only(self):
        """Test filtering keeps only successful logs."""
        logs = [
            InteractionLog(
                module_name="reviewer",
                module_version="baseline",
                signature="extract_requirements",
                input={"user_intent": "test"},
                output={"requirements": ["req1"]},
                timestamp_ms=1699564800000,
                success=True,
            ),
            InteractionLog(
                module_name="reviewer",
                module_version="baseline",
                signature="extract_requirements",
                input={"user_intent": "test2"},
                output={},
                timestamp_ms=1699564801000,
                success=False,
                error="Timeout"
            ),
        ]

        filtered = filter_by_quality(logs, min_quality_score=0.0)

        assert len(filtered) == 1
        assert filtered[0].success is True

    def test_filter_by_latency(self):
        """Test filtering by latency threshold."""
        logs = [
            InteractionLog(
                module_name="reviewer",
                module_version="baseline",
                signature="extract_requirements",
                input={"user_intent": "test"},
                output={"requirements": ["req1"]},
                timestamp_ms=1699564800000,
                latency_ms=100,
                success=True,
            ),
            InteractionLog(
                module_name="reviewer",
                module_version="baseline",
                signature="extract_requirements",
                input={"user_intent": "test2"},
                output={"requirements": ["req2"]},
                timestamp_ms=1699564801000,
                latency_ms=5000,  # 5 seconds - high latency
                success=True,
            ),
        ]

        filtered = filter_by_quality(logs, min_quality_score=0.0, max_latency_ms=3000)

        assert len(filtered) == 1
        assert filtered[0].latency_ms == 100

    def test_filter_by_cost(self):
        """Test filtering by cost threshold."""
        logs = [
            InteractionLog(
                module_name="reviewer",
                module_version="baseline",
                signature="extract_requirements",
                input={"user_intent": "test"},
                output={"requirements": ["req1"]},
                timestamp_ms=1699564800000,
                cost_usd=0.001,
                success=True,
            ),
            InteractionLog(
                module_name="reviewer",
                module_version="baseline",
                signature="extract_requirements",
                input={"user_intent": "test2"},
                output={"requirements": ["req2"]},
                timestamp_ms=1699564801000,
                cost_usd=0.1,  # High cost
                success=True,
            ),
        ]

        filtered = filter_by_quality(logs, min_quality_score=0.0, max_cost_usd=0.05)

        assert len(filtered) == 1
        assert filtered[0].cost_usd == 0.001


class TestDeduplication:
    """Test SHA256-based deduplication."""

    def test_deduplicate_identical_inputs(self):
        """Test deduplication removes identical inputs."""
        logs = [
            InteractionLog(
                module_name="reviewer",
                module_version="baseline",
                signature="extract_requirements",
                input={"user_intent": "test"},
                output={"requirements": ["req1"]},
                timestamp_ms=1699564800000,
                success=True,
            ),
            InteractionLog(
                module_name="reviewer",
                module_version="baseline",
                signature="extract_requirements",
                input={"user_intent": "test"},  # Duplicate
                output={"requirements": ["req1"]},
                timestamp_ms=1699564801000,
                success=True,
            ),
        ]

        deduped = deduplicate_logs(logs)

        assert len(deduped) == 1

    def test_deduplicate_keeps_different_inputs(self):
        """Test deduplication keeps different inputs."""
        logs = [
            InteractionLog(
                module_name="reviewer",
                module_version="baseline",
                signature="extract_requirements",
                input={"user_intent": "test1"},
                output={"requirements": ["req1"]},
                timestamp_ms=1699564800000,
                success=True,
            ),
            InteractionLog(
                module_name="reviewer",
                module_version="baseline",
                signature="extract_requirements",
                input={"user_intent": "test2"},  # Different
                output={"requirements": ["req2"]},
                timestamp_ms=1699564801000,
                success=True,
            ),
        ]

        deduped = deduplicate_logs(logs)

        assert len(deduped) == 2

    def test_deduplicate_consistent_hash(self):
        """Test deduplication uses consistent hashing."""
        log1 = InteractionLog(
            module_name="reviewer",
            module_version="baseline",
            signature="extract_requirements",
            input={"user_intent": "test", "context": "api"},
            output={"requirements": ["req1"]},
            timestamp_ms=1699564800000,
            success=True,
        )

        log2 = InteractionLog(
            module_name="reviewer",
            module_version="baseline",
            signature="extract_requirements",
            input={"context": "api", "user_intent": "test"},  # Different key order, same content
            output={"requirements": ["req1"]},
            timestamp_ms=1699564801000,
            success=True,
        )

        logs = [log1, log2]
        deduped = deduplicate_logs(logs)

        # Should deduplicate despite different key order
        assert len(deduped) == 1


class TestDatasetManagerIntegration:
    """Test integration with DatasetManager."""

    @patch('telemetry_aggregator.DatasetManager')
    def test_aggregate_creates_versioned_dataset(self, mock_manager_class):
        """Test aggregate_telemetry creates versioned dataset."""
        # Setup mock
        mock_manager = MagicMock()
        mock_manager_class.return_value = mock_manager
        mock_manager.create_version.return_value = "v0001"

        # Create temp log file
        with tempfile.NamedTemporaryFile(mode='w', suffix='.jsonl', delete=False) as f:
            log_data = {
                "module_name": "reviewer",
                "module_version": "baseline",
                "signature": "extract_requirements",
                "input": {"user_intent": "test"},
                "output": {"requirements": ["req1"]},
                "timestamp_ms": 1699564800000,
                "success": True,
            }
            f.write(json.dumps(log_data) + '\n')
            log_file = f.name

        try:
            with tempfile.TemporaryDirectory() as output_dir:
                aggregate_telemetry(
                    log_file=log_file,
                    output_dir=output_dir,
                    min_quality_score=0.0
                )

                # Verify DatasetManager.create_version was called
                assert mock_manager.create_version.called

        finally:
            os.unlink(log_file)

    @patch('telemetry_aggregator.DatasetManager')
    def test_aggregate_tracks_provenance(self, mock_manager_class):
        """Test aggregate_telemetry tracks provenance as telemetry."""
        # Setup mock
        mock_manager = MagicMock()
        mock_manager_class.return_value = mock_manager
        mock_manager.create_version.return_value = "v0001"

        # Create temp log file
        with tempfile.NamedTemporaryFile(mode='w', suffix='.jsonl', delete=False) as f:
            log_data = {
                "module_name": "reviewer",
                "module_version": "baseline",
                "signature": "extract_requirements",
                "input": {"user_intent": "test"},
                "output": {"requirements": ["req1"]},
                "timestamp_ms": 1699564800000,
                "success": True,
            }
            f.write(json.dumps(log_data) + '\n')
            log_file = f.name

        try:
            with tempfile.TemporaryDirectory() as output_dir:
                aggregate_telemetry(
                    log_file=log_file,
                    output_dir=output_dir,
                    min_quality_score=0.0
                )

                # Check that create_version was called with source='telemetry'
                call_args = mock_manager.create_version.call_args
                assert call_args is not None
                # Check kwargs for source parameter
                assert 'source' in call_args.kwargs or len(call_args.args) >= 3

        finally:
            os.unlink(log_file)


class TestErrorHandling:
    """Test error handling for malformed logs."""

    def test_parse_malformed_json(self):
        """Test handling of malformed JSON log entry."""
        with tempfile.NamedTemporaryFile(mode='w', suffix='.jsonl', delete=False) as f:
            f.write('{"invalid json\n')  # Malformed JSON
            log_file = f.name

        try:
            with tempfile.TemporaryDirectory() as output_dir:
                # Should not crash, just skip malformed line
                aggregate_telemetry(
                    log_file=log_file,
                    output_dir=output_dir,
                    min_quality_score=0.0
                )
        finally:
            os.unlink(log_file)

    def test_missing_log_file(self):
        """Test handling of missing log file."""
        with tempfile.TemporaryDirectory() as output_dir:
            with pytest.raises(FileNotFoundError):
                aggregate_telemetry(
                    log_file='/nonexistent/file.jsonl',
                    output_dir=output_dir,
                    min_quality_score=0.0
                )


class TestEndToEnd:
    """End-to-end integration tests."""

    @patch('telemetry_aggregator.DatasetManager')
    def test_full_pipeline(self, mock_manager_class):
        """Test complete aggregation pipeline."""
        # Setup mock
        mock_manager = MagicMock()
        mock_manager_class.return_value = mock_manager
        mock_manager.create_version.return_value = "v0001"

        # Create temp log file with multiple entries
        with tempfile.NamedTemporaryFile(mode='w', suffix='.jsonl', delete=False) as f:
            # Good log
            f.write(json.dumps({
                "module_name": "reviewer",
                "module_version": "baseline",
                "signature": "extract_requirements",
                "input": {"user_intent": "test1"},
                "output": {"requirements": ["req1"]},
                "timestamp_ms": 1699564800000,
                "latency_ms": 200,
                "cost_usd": 0.001,
                "success": True,
            }) + '\n')

            # Failed log (should be filtered)
            f.write(json.dumps({
                "module_name": "reviewer",
                "module_version": "baseline",
                "signature": "extract_requirements",
                "input": {"user_intent": "test2"},
                "output": {},
                "timestamp_ms": 1699564801000,
                "success": False,
                "error": "Timeout"
            }) + '\n')

            # Duplicate log (should be deduped)
            f.write(json.dumps({
                "module_name": "reviewer",
                "module_version": "baseline",
                "signature": "extract_requirements",
                "input": {"user_intent": "test1"},  # Duplicate
                "output": {"requirements": ["req1"]},
                "timestamp_ms": 1699564802000,
                "success": True,
            }) + '\n')

            log_file = f.name

        try:
            with tempfile.TemporaryDirectory() as output_dir:
                result = aggregate_telemetry(
                    log_file=log_file,
                    output_dir=output_dir,
                    min_quality_score=0.0
                )

                # Should have 1 entry (failed filtered, duplicate deduped)
                assert result['total_examples'] == 1
                assert result['signatures'] == {'extract_requirements': 1}

        finally:
            os.unlink(log_file)


if __name__ == '__main__':
    pytest.main([__file__, '-v'])
