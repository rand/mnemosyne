#!/usr/bin/env python3
"""
Telemetry Data Aggregator for DSPy Production Logs

Reads sampled production interactions from JSON Lines logs,
converts them to DSPy-compatible training data, applies quality
filtering and deduplication, and outputs to versioned datasets.

Features:
- Reads InteractionLog entries from production logs
- Converts to DSPy TrainingDataEntry format
- Deduplicates based on input similarity
- Filters by success rate and quality thresholds
- Integrates with DatasetManager for versioned storage
- Tracks provenance (telemetry source)
"""

import argparse
import json
import hashlib
from collections import defaultdict
from dataclasses import dataclass
from datetime import datetime
from pathlib import Path
from typing import List, Dict, Any, Set, Optional

# Import existing data collection infrastructure
from dataset_manager import DatasetManager
from data_validator import DataValidator
from quality_gates import QualityGates


@dataclass
class InteractionLog:
    """Production interaction log entry (matches Rust InteractionLog format)"""
    module_name: str
    module_version: str  # "Baseline" or "Optimized(v1)"
    signature: str
    input: Dict[str, Any]
    output: Dict[str, Any]
    timestamp_ms: int
    latency_ms: int
    tokens: Dict[str, int]  # {prompt_tokens, completion_tokens, total_tokens}
    cost_usd: float
    model: str
    success: bool
    error: Optional[str] = None


class TelemetryAggregator:
    """
    Aggregate production telemetry into DSPy training datasets.

    Workflow:
    1. Read production logs (JSON Lines format)
    2. Parse InteractionLog entries
    3. Filter by success and quality
    4. Deduplicate by input similarity
    5. Convert to DSPy training format
    6. Save to versioned datasets via DatasetManager
    """

    SIGNATURES = [
        'extract_requirements',
        'validate_intent',
        'validate_completeness',
        'validate_correctness',
        'generate_guidance'
    ]

    def __init__(
        self,
        log_file: Path,
        output_dir: Path = Path("training_data"),
        min_success_rate: float = 0.90,
        min_quality_score: float = 0.70,
        dedup_threshold: float = 0.85
    ):
        self.log_file = Path(log_file)
        self.dataset_manager = DatasetManager(base_dir=str(output_dir))
        self.validator = DataValidator()
        self.quality_gates = QualityGates()

        self.min_success_rate = min_success_rate
        self.min_quality_score = min_quality_score
        self.dedup_threshold = dedup_threshold

        # Stats
        self.stats = {
            "total_read": 0,
            "parse_errors": 0,
            "failed_interactions": 0,
            "low_quality": 0,
            "duplicates": 0,
            "added": defaultdict(int)
        }

    def read_logs(self) -> List[InteractionLog]:
        """Read and parse production log file (JSON Lines format)"""
        interactions = []

        if not self.log_file.exists():
            print(f"Warning: Log file not found: {self.log_file}")
            return interactions

        print(f"Reading logs from: {self.log_file}")

        with open(self.log_file, 'r') as f:
            for line_num, line in enumerate(f, 1):
                line = line.strip()
                if not line:
                    continue

                try:
                    data = json.loads(line)
                    interaction = InteractionLog(
                        module_name=data["module_name"],
                        module_version=data["module_version"],
                        signature=data["signature"],
                        input=data["input"],
                        output=data["output"],
                        timestamp_ms=data["timestamp_ms"],
                        latency_ms=data["latency_ms"],
                        tokens=data["tokens"],
                        cost_usd=data["cost_usd"],
                        model=data["model"],
                        success=data["success"],
                        error=data.get("error")
                    )
                    interactions.append(interaction)
                    self.stats["total_read"] += 1
                except (json.JSONDecodeError, KeyError) as e:
                    print(f"Warning: Failed to parse line {line_num}: {e}")
                    self.stats["parse_errors"] += 1
                    continue

        print(f"Read {self.stats['total_read']} interactions from log")
        return interactions

    def filter_interactions(self, interactions: List[InteractionLog]) -> List[InteractionLog]:
        """Filter interactions by success and quality"""
        filtered = []

        for interaction in interactions:
            # Skip failed interactions
            if not interaction.success:
                self.stats["failed_interactions"] += 1
                continue

            # Estimate quality score based on output structure
            quality_score = self._estimate_quality(interaction)

            if quality_score < self.min_quality_score:
                self.stats["low_quality"] += 1
                continue

            filtered.append(interaction)

        print(f"Filtered to {len(filtered)} high-quality interactions")
        print(f"  - Failed: {self.stats['failed_interactions']}")
        print(f"  - Low quality: {self.stats['low_quality']}")

        return filtered

    def _estimate_quality(self, interaction: InteractionLog) -> float:
        """Estimate quality score for an interaction"""
        score = 1.0

        # Penalize if output is empty or trivial
        output_str = json.dumps(interaction.output)
        if len(output_str) < 20:
            score *= 0.5

        # Penalize very short or very long latencies
        if interaction.latency_ms < 100:  # Too fast, likely cached/trivial
            score *= 0.8
        elif interaction.latency_ms > 30000:  # Too slow, might have issues
            score *= 0.9

        # Penalize if tokens are suspiciously low
        total_tokens = interaction.tokens.get("total_tokens", 0)
        if total_tokens < 50:
            score *= 0.7

        return score

    def deduplicate(self, interactions: List[InteractionLog]) -> List[InteractionLog]:
        """Deduplicate interactions by input similarity"""
        seen_hashes: Set[str] = set()
        deduplicated = []

        for interaction in interactions:
            # Hash input for deduplication
            input_str = json.dumps(interaction.input, sort_keys=True)
            input_hash = hashlib.sha256(input_str.encode()).hexdigest()[:16]

            if input_hash in seen_hashes:
                self.stats["duplicates"] += 1
                continue

            seen_hashes.add(input_hash)
            deduplicated.append(interaction)

        print(f"Deduplicated to {len(deduplicated)} unique interactions")
        print(f"  - Duplicates removed: {self.stats['duplicates']}")

        return deduplicated

    def convert_to_training_data(self, interactions: List[InteractionLog]) -> Dict[str, List[Dict[str, Any]]]:
        """Convert interactions to DSPy training data format, grouped by signature"""
        training_data = defaultdict(list)

        for interaction in interactions:
            signature = interaction.signature

            # Skip if signature not recognized
            if signature not in self.SIGNATURES:
                print(f"Warning: Unknown signature: {signature}")
                continue

            # Convert to DSPy training format
            example = {
                "inputs": interaction.input,
                "outputs": interaction.output,
                "metadata": {
                    "source": "telemetry",
                    "module_version": interaction.module_version,
                    "timestamp_ms": interaction.timestamp_ms,
                    "latency_ms": interaction.latency_ms,
                    "tokens": interaction.tokens,
                    "cost_usd": interaction.cost_usd,
                    "model": interaction.model,
                    "quality_score": self._estimate_quality(interaction)
                }
            }

            training_data[signature].append(example)

        # Print summary by signature
        print("\nTraining data by signature:")
        for sig, examples in training_data.items():
            print(f"  - {sig}: {len(examples)} examples")

        return dict(training_data)

    def save_to_datasets(self, training_data: Dict[str, List[Dict[str, Any]]]):
        """Save training data to versioned datasets"""
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")

        for signature, examples in training_data.items():
            if not examples:
                continue

            print(f"\nSaving {len(examples)} examples for signature: {signature}")

            # Create new dataset version
            version = self.dataset_manager.create_version(
                signature_name=signature,
                examples=examples,
                source="telemetry",
                notes=f"Production telemetry aggregation at {timestamp}"
            )

            print(f"  - Created dataset version: {version}")
            self.stats["added"][signature] = len(examples)

    def aggregate(self) -> Dict[str, Any]:
        """Main aggregation workflow"""
        print("=== Telemetry Data Aggregation ===\n")

        # 1. Read logs
        interactions = self.read_logs()

        if not interactions:
            print("No interactions to process")
            return self.stats

        # 2. Filter by quality
        interactions = self.filter_interactions(interactions)

        # 3. Deduplicate
        interactions = self.deduplicate(interactions)

        # 4. Convert to training data
        training_data = self.convert_to_training_data(interactions)

        # 5. Save to datasets
        self.save_to_datasets(training_data)

        # Print final stats
        print("\n=== Aggregation Complete ===")
        print(f"Total read: {self.stats['total_read']}")
        print(f"Parse errors: {self.stats['parse_errors']}")
        print(f"Failed interactions: {self.stats['failed_interactions']}")
        print(f"Low quality: {self.stats['low_quality']}")
        print(f"Duplicates: {self.stats['duplicates']}")
        print(f"\nAdded to datasets:")
        for sig, count in self.stats["added"].items():
            print(f"  - {sig}: {count}")

        return self.stats


def main():
    parser = argparse.ArgumentParser(
        description="Aggregate production telemetry into DSPy training datasets"
    )
    parser.add_argument(
        "--log-file",
        type=str,
        required=True,
        help="Path to production log file (JSON Lines format)"
    )
    parser.add_argument(
        "--output-dir",
        type=str,
        default="training_data",
        help="Output directory for versioned datasets (default: training_data)"
    )
    parser.add_argument(
        "--min-success-rate",
        type=float,
        default=0.90,
        help="Minimum success rate threshold (default: 0.90)"
    )
    parser.add_argument(
        "--min-quality-score",
        type=float,
        default=0.70,
        help="Minimum quality score threshold (default: 0.70)"
    )
    parser.add_argument(
        "--dedup-threshold",
        type=float,
        default=0.85,
        help="Deduplication similarity threshold (default: 0.85)"
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Process without saving (for testing)"
    )

    args = parser.parse_args()

    aggregator = TelemetryAggregator(
        log_file=Path(args.log_file),
        output_dir=Path(args.output_dir),
        min_success_rate=args.min_success_rate,
        min_quality_score=args.min_quality_score,
        dedup_threshold=args.dedup_threshold
    )

    stats = aggregator.aggregate()

    # Return exit code based on success
    if stats["added"]:
        print("\nSuccess: Training data aggregated")
        return 0
    else:
        print("\nWarning: No training data added")
        return 1


if __name__ == "__main__":
    import sys
    sys.exit(main())
