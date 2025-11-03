#!/usr/bin/env python3
"""
Import Production Logs into Training Data

This script demonstrates how to:
1. Read production logs from ProductionLogger
2. Filter and validate interactions
3. Convert to DSPy training data format
4. Merge with existing training data
5. Prepare for optimization

Usage:
    python import_production_logs.py --input logs/production.jsonl --output training_data/reviewer.json --module reviewer

    # Filter by success rate
    python import_production_logs.py --input logs/production.jsonl --output training_data/reviewer.json --module reviewer --min-success-rate 0.8

    # Filter by date range
    python import_production_logs.py --input logs/production.jsonl --output training_data/reviewer.json --module reviewer --since 2025-01-01

    # Dry run to preview
    python import_production_logs.py --input logs/production.jsonl --output training_data/reviewer.json --module reviewer --dry-run
"""

import argparse
import json
import sys
from datetime import datetime, timedelta
from pathlib import Path
from typing import Any, Dict, List, Optional

def parse_args() -> argparse.Namespace:
    """Parse command line arguments."""
    parser = argparse.ArgumentParser(
        description="Import production logs into DSPy training data",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__
    )

    parser.add_argument(
        "--input",
        type=Path,
        required=True,
        help="Path to production logs (JSON Lines format)"
    )

    parser.add_argument(
        "--output",
        type=Path,
        required=True,
        help="Path to output training data (JSON format)"
    )

    parser.add_argument(
        "--module",
        type=str,
        required=True,
        choices=["reviewer", "optimizer", "semantic"],
        help="Module name for training data"
    )

    parser.add_argument(
        "--min-success-rate",
        type=float,
        default=0.0,
        help="Minimum success rate (0.0-1.0) for included interactions"
    )

    parser.add_argument(
        "--since",
        type=str,
        help="Only include interactions after this date (YYYY-MM-DD)"
    )

    parser.add_argument(
        "--max-examples",
        type=int,
        help="Maximum number of examples to import"
    )

    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Preview without writing output"
    )

    parser.add_argument(
        "--merge",
        action="store_true",
        help="Merge with existing training data (default: overwrite)"
    )

    parser.add_argument(
        "--deduplicate",
        action="store_true",
        help="Remove duplicate examples based on input"
    )

    return parser.parse_args()


def load_production_logs(input_path: Path) -> List[Dict[str, Any]]:
    """Load production logs from JSON Lines file."""
    logs = []

    print(f"Reading production logs from: {input_path}")

    if not input_path.exists():
        print(f"Error: Input file not found: {input_path}", file=sys.stderr)
        sys.exit(1)

    with open(input_path, 'r') as f:
        for line_num, line in enumerate(f, 1):
            line = line.strip()
            if not line:
                continue

            try:
                log = json.loads(line)
                logs.append(log)
            except json.JSONDecodeError as e:
                print(f"Warning: Skipping invalid JSON at line {line_num}: {e}", file=sys.stderr)
                continue

    print(f"Loaded {len(logs)} production log entries")
    return logs


def filter_logs(
    logs: List[Dict[str, Any]],
    min_success_rate: float,
    since: Optional[str],
    module: str
) -> List[Dict[str, Any]]:
    """Filter logs based on criteria."""
    filtered = []

    # Parse since date if provided
    since_ts = None
    if since:
        try:
            since_dt = datetime.strptime(since, "%Y-%m-%d")
            since_ts = int(since_dt.timestamp() * 1000)  # Convert to milliseconds
        except ValueError:
            print(f"Error: Invalid date format for --since: {since}", file=sys.stderr)
            sys.exit(1)

    # Apply filters
    for log in logs:
        # Filter by module
        if log.get("module_name") != module:
            continue

        # Filter by success
        if not log.get("success", False):
            continue

        # Filter by date
        if since_ts and log.get("timestamp_ms", 0) < since_ts:
            continue

        filtered.append(log)

    print(f"Filtered to {len(filtered)} matching examples")

    # Check success rate
    if filtered:
        success_count = sum(1 for log in filtered if log.get("success", False))
        success_rate = success_count / len(filtered)
        print(f"Success rate: {success_rate:.1%}")

        if success_rate < min_success_rate:
            print(f"Warning: Success rate {success_rate:.1%} is below minimum {min_success_rate:.1%}", file=sys.stderr)

    return filtered


def convert_to_training_data(logs: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
    """Convert production logs to DSPy training data format."""
    training_data = []

    for log in logs:
        # Extract input and output from log
        entry = {
            "signature": log.get("signature", "unknown"),
            "input": log.get("input", {}),
            "output": log.get("output", {}),
            "metadata": {
                "source": "production",
                "timestamp": log.get("timestamp_ms"),
                "module_version": str(log.get("module_version", "unknown")),
                "latency_ms": log.get("latency_ms"),
                "tokens": log.get("tokens", {}),
                "cost_usd": log.get("cost_usd"),
                "model": log.get("model")
            }
        }

        training_data.append(entry)

    return training_data


def deduplicate_training_data(data: List[Dict[str, Any]]) -> List[Dict[str, Any]]:
    """Remove duplicate examples based on input."""
    seen_inputs = set()
    deduplicated = []

    for entry in data:
        # Create hash of input for deduplication
        input_str = json.dumps(entry["input"], sort_keys=True)

        if input_str not in seen_inputs:
            seen_inputs.add(input_str)
            deduplicated.append(entry)

    removed = len(data) - len(deduplicated)
    if removed > 0:
        print(f"Removed {removed} duplicate examples")

    return deduplicated


def load_existing_training_data(output_path: Path) -> List[Dict[str, Any]]:
    """Load existing training data if it exists."""
    if not output_path.exists():
        return []

    try:
        with open(output_path, 'r') as f:
            data = json.load(f)
            print(f"Loaded {len(data)} existing training examples")
            return data
    except (json.JSONDecodeError, IOError) as e:
        print(f"Warning: Could not load existing training data: {e}", file=sys.stderr)
        return []


def save_training_data(data: List[Dict[str, Any]], output_path: Path) -> None:
    """Save training data to JSON file."""
    # Create output directory if needed
    output_path.parent.mkdir(parents=True, exist_ok=True)

    with open(output_path, 'w') as f:
        json.dump(data, f, indent=2)

    print(f"Saved {len(data)} training examples to: {output_path}")


def print_statistics(data: List[Dict[str, Any]]) -> None:
    """Print statistics about the training data."""
    if not data:
        print("No data to analyze")
        return

    print("\n=== Training Data Statistics ===")
    print(f"Total examples: {len(data)}")

    # Count by signature
    signatures = {}
    for entry in data:
        sig = entry.get("signature", "unknown")
        signatures[sig] = signatures.get(sig, 0) + 1

    print(f"\nExamples per signature:")
    for sig, count in sorted(signatures.items(), key=lambda x: x[1], reverse=True):
        print(f"  {sig}: {count}")

    # Count by source
    sources = {}
    for entry in data:
        source = entry.get("metadata", {}).get("source", "unknown")
        sources[source] = sources.get(source, 0) + 1

    if sources:
        print(f"\nExamples by source:")
        for source, count in sorted(sources.items(), key=lambda x: x[1], reverse=True):
            print(f"  {source}: {count}")

    # Total cost
    total_cost = sum(entry.get("metadata", {}).get("cost_usd", 0) for entry in data)
    print(f"\nTotal training cost: ${total_cost:.4f}")


def main():
    """Main entry point."""
    args = parse_args()

    # Load production logs
    logs = load_production_logs(args.input)

    # Filter logs
    filtered_logs = filter_logs(
        logs,
        args.min_success_rate,
        args.since,
        args.module
    )

    if not filtered_logs:
        print("No logs matched the filter criteria", file=sys.stderr)
        sys.exit(1)

    # Convert to training data
    training_data = convert_to_training_data(filtered_logs)

    # Limit examples if requested
    if args.max_examples and len(training_data) > args.max_examples:
        print(f"Limiting to {args.max_examples} examples (from {len(training_data)})")
        training_data = training_data[:args.max_examples]

    # Merge with existing data if requested
    if args.merge:
        existing_data = load_existing_training_data(args.output)
        training_data = existing_data + training_data
        print(f"Merged with existing data: {len(existing_data)} + {len(training_data) - len(existing_data)} = {len(training_data)} total")

    # Deduplicate if requested
    if args.deduplicate:
        training_data = deduplicate_training_data(training_data)

    # Print statistics
    print_statistics(training_data)

    # Save or preview
    if args.dry_run:
        print("\n=== Dry Run: No files written ===")
        print(f"Would write {len(training_data)} examples to: {args.output}")

        # Show first example
        if training_data:
            print("\nExample entry:")
            print(json.dumps(training_data[0], indent=2))
    else:
        save_training_data(training_data, args.output)
        print("\nSuccess! Training data ready for optimization.")


if __name__ == "__main__":
    main()
