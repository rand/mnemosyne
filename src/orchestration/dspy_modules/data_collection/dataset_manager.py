#!/usr/bin/env python3
"""
Dataset Manager for DSPy Training Data

Provides versioned storage, provenance tracking, and incremental updates
for training datasets across all DSPy signatures.

Features:
- Versioned datasets with timestamps (YYYYMMDD_HHMMSS)
- Data provenance tracking (git/synthetic/telemetry)
- Incremental additions without breaking existing runs
- Dataset metadata and quality metrics
- Rollback to previous versions
- Integration with data collection and validation pipeline
"""

import json
import os
import sys
from dataclasses import dataclass, asdict
from datetime import datetime
from pathlib import Path
from typing import List, Dict, Any, Optional, Set
import shutil


@dataclass
class DatasetMetadata:
    """Metadata for a versioned dataset"""
    version: str  # Timestamp: YYYYMMDD_HHMMSS
    signature_name: str
    total_examples: int
    sources: Dict[str, int]  # source -> count
    difficulty_distribution: Dict[str, int]  # easy/medium/hard -> count
    category_distribution: Dict[str, int]  # category -> count
    quality_score_avg: float
    created_at: str
    parent_version: Optional[str] = None
    notes: str = ""


@dataclass
class DatasetExample:
    """Single training example with provenance"""
    inputs: Dict[str, Any]
    outputs: Dict[str, Any]
    metadata: Dict[str, Any]


class DatasetManager:
    """
    Manage versioned training datasets for DSPy modules.

    Directory structure:
        training_data/
        ├── extract_requirements/
        │   ├── v20251104_120000/
        │   │   ├── dataset.json
        │   │   ├── metadata.json
        │   │   └── provenance.jsonl
        │   ├── v20251105_140000/
        │   └── latest -> v20251105_140000
        ├── validate_completeness/
        └── ...
    """

    SIGNATURES = [
        'extract_requirements',
        'validate_intent',
        'validate_completeness',
        'validate_correctness',
        'generate_guidance'
    ]

    def __init__(self, base_dir: str = "training_data"):
        self.base_dir = Path(base_dir)
        self.base_dir.mkdir(exist_ok=True)

        # Ensure signature directories exist
        for sig in self.SIGNATURES:
            sig_dir = self.base_dir / sig
            sig_dir.mkdir(exist_ok=True)

    def _get_version_dir(self, signature_name: str, version: str) -> Path:
        """Get path to version directory"""
        return self.base_dir / signature_name / f"v{version}"

    def _get_latest_symlink(self, signature_name: str) -> Path:
        """Get path to 'latest' symlink"""
        return self.base_dir / signature_name / "latest"

    def _get_current_timestamp(self) -> str:
        """Get current timestamp in YYYYMMDD_HHMMSS format"""
        return datetime.now().strftime("%Y%m%d_%H%M%S")

    def create_version(
        self,
        signature_name: str,
        examples: List[Dict[str, Any]],
        source: str = "unknown",
        notes: str = "",
        parent_version: Optional[str] = None
    ) -> str:
        """
        Create a new dataset version.

        Args:
            signature_name: Name of DSPy signature
            examples: List of training examples
            source: Data source (git/synthetic/telemetry)
            notes: Optional notes about this version
            parent_version: Optional parent version for incremental updates

        Returns:
            Version identifier (timestamp)
        """
        if signature_name not in self.SIGNATURES:
            raise ValueError(f"Unknown signature: {signature_name}")

        version = self._get_current_timestamp()
        version_dir = self._get_version_dir(signature_name, version)
        version_dir.mkdir(parents=True, exist_ok=True)

        # Calculate statistics
        sources = {}
        difficulties = {'easy': 0, 'medium': 0, 'hard': 0}
        categories = {}
        quality_scores = []

        for ex in examples:
            # Track source
            ex_source = ex.get('metadata', {}).get('source', source)
            sources[ex_source] = sources.get(ex_source, 0) + 1

            # Track difficulty
            difficulty = ex.get('metadata', {}).get('difficulty', 'unknown')
            if difficulty in difficulties:
                difficulties[difficulty] += 1

            # Track category
            category = ex.get('metadata', {}).get('category', 'unknown')
            categories[category] = categories.get(category, 0) + 1

            # Track quality (if available)
            quality = ex.get('metadata', {}).get('quality_score')
            if quality is not None:
                quality_scores.append(quality)

        # Create metadata
        metadata = DatasetMetadata(
            version=version,
            signature_name=signature_name,
            total_examples=len(examples),
            sources=sources,
            difficulty_distribution=difficulties,
            category_distribution=categories,
            quality_score_avg=sum(quality_scores) / len(quality_scores) if quality_scores else 0.0,
            created_at=datetime.now().isoformat(),
            parent_version=parent_version,
            notes=notes
        )

        # Write dataset
        dataset_path = version_dir / "dataset.json"
        with open(dataset_path, 'w') as f:
            json.dump(examples, f, indent=2)

        # Write metadata
        metadata_path = version_dir / "metadata.json"
        with open(metadata_path, 'w') as f:
            json.dump(asdict(metadata), f, indent=2)

        # Write provenance (one line per example for incremental processing)
        provenance_path = version_dir / "provenance.jsonl"
        with open(provenance_path, 'w') as f:
            for i, ex in enumerate(examples):
                provenance = {
                    'index': i,
                    'source': ex.get('metadata', {}).get('source', source),
                    'difficulty': ex.get('metadata', {}).get('difficulty', 'unknown'),
                    'category': ex.get('metadata', {}).get('category', 'unknown'),
                    'quality_score': ex.get('metadata', {}).get('quality_score'),
                    'added_at': datetime.now().isoformat()
                }
                f.write(json.dumps(provenance) + '\n')

        # Update 'latest' symlink
        latest_link = self._get_latest_symlink(signature_name)
        if latest_link.exists() or latest_link.is_symlink():
            latest_link.unlink()
        latest_link.symlink_to(f"v{version}")

        print(f"✓ Created dataset version: {signature_name}/v{version}")
        print(f"  Total examples: {len(examples)}")
        print(f"  Sources: {sources}")
        print(f"  Difficulty: {difficulties}")
        print(f"  Avg quality: {metadata.quality_score_avg:.1f}")

        return version

    def add_to_version(
        self,
        signature_name: str,
        new_examples: List[Dict[str, Any]],
        source: str = "incremental",
        notes: str = "Incremental addition"
    ) -> str:
        """
        Add examples to the latest version by creating a new version.

        Args:
            signature_name: Name of DSPy signature
            new_examples: New training examples to add
            source: Data source for new examples
            notes: Notes about this addition

        Returns:
            New version identifier
        """
        # Load latest version
        latest = self.get_latest_version(signature_name)
        if latest:
            current_examples = self.load_dataset(signature_name, latest)
            parent_version = latest
        else:
            current_examples = []
            parent_version = None

        # Merge examples
        all_examples = current_examples + new_examples

        # Create new version
        return self.create_version(
            signature_name=signature_name,
            examples=all_examples,
            source=source,
            notes=notes,
            parent_version=parent_version
        )

    def load_dataset(self, signature_name: str, version: Optional[str] = None) -> List[Dict[str, Any]]:
        """
        Load a dataset version.

        Args:
            signature_name: Name of DSPy signature
            version: Version to load (defaults to latest)

        Returns:
            List of training examples
        """
        if version is None:
            version = self.get_latest_version(signature_name)
            if version is None:
                return []

        dataset_path = self._get_version_dir(signature_name, version) / "dataset.json"
        if not dataset_path.exists():
            raise FileNotFoundError(f"Dataset not found: {dataset_path}")

        with open(dataset_path, 'r') as f:
            return json.load(f)

    def load_metadata(self, signature_name: str, version: Optional[str] = None) -> Optional[DatasetMetadata]:
        """
        Load metadata for a dataset version.

        Args:
            signature_name: Name of DSPy signature
            version: Version to load (defaults to latest)

        Returns:
            DatasetMetadata or None if not found
        """
        if version is None:
            version = self.get_latest_version(signature_name)
            if version is None:
                return None

        metadata_path = self._get_version_dir(signature_name, version) / "metadata.json"
        if not metadata_path.exists():
            return None

        with open(metadata_path, 'r') as f:
            data = json.load(f)
            return DatasetMetadata(**data)

    def get_latest_version(self, signature_name: str) -> Optional[str]:
        """
        Get the latest version for a signature.

        Args:
            signature_name: Name of DSPy signature

        Returns:
            Version identifier or None if no versions exist
        """
        latest_link = self._get_latest_symlink(signature_name)
        if not latest_link.exists():
            return None

        # Read symlink target
        target = latest_link.resolve()
        version = target.name.replace('v', '')
        return version

    def list_versions(self, signature_name: str) -> List[str]:
        """
        List all versions for a signature, sorted by timestamp.

        Args:
            signature_name: Name of DSPy signature

        Returns:
            List of version identifiers (newest first)
        """
        sig_dir = self.base_dir / signature_name
        if not sig_dir.exists():
            return []

        versions = []
        for path in sig_dir.iterdir():
            if path.is_dir() and path.name.startswith('v'):
                version = path.name.replace('v', '')
                versions.append(version)

        return sorted(versions, reverse=True)

    def delete_version(self, signature_name: str, version: str, force: bool = False):
        """
        Delete a specific version (not latest unless forced).

        Args:
            signature_name: Name of DSPy signature
            version: Version to delete
            force: Allow deletion of latest version
        """
        latest = self.get_latest_version(signature_name)
        if version == latest and not force:
            raise ValueError(f"Cannot delete latest version {version} without force=True")

        version_dir = self._get_version_dir(signature_name, version)
        if version_dir.exists():
            shutil.rmtree(version_dir)
            print(f"✓ Deleted version: {signature_name}/v{version}")
        else:
            raise FileNotFoundError(f"Version not found: {version_dir}")

    def rollback_to_version(self, signature_name: str, version: str):
        """
        Rollback 'latest' symlink to a specific version.

        Args:
            signature_name: Name of DSPy signature
            version: Version to rollback to
        """
        version_dir = self._get_version_dir(signature_name, version)
        if not version_dir.exists():
            raise FileNotFoundError(f"Version not found: {version_dir}")

        # Update 'latest' symlink
        latest_link = self._get_latest_symlink(signature_name)
        if latest_link.exists() or latest_link.is_symlink():
            latest_link.unlink()
        latest_link.symlink_to(f"v{version}")

        print(f"✓ Rolled back to version: {signature_name}/v{version}")

    def get_summary(self) -> Dict[str, Any]:
        """
        Get summary of all datasets.

        Returns:
            Dictionary with signature -> summary mapping
        """
        summary = {}
        for sig in self.SIGNATURES:
            versions = self.list_versions(sig)
            latest = self.get_latest_version(sig)

            if latest:
                metadata = self.load_metadata(sig, latest)
                summary[sig] = {
                    'latest_version': latest,
                    'total_versions': len(versions),
                    'total_examples': metadata.total_examples if metadata else 0,
                    'sources': metadata.sources if metadata else {},
                    'quality_avg': metadata.quality_score_avg if metadata else 0.0
                }
            else:
                summary[sig] = {
                    'latest_version': None,
                    'total_versions': 0,
                    'total_examples': 0,
                    'sources': {},
                    'quality_avg': 0.0
                }

        return summary


def main():
    """CLI for dataset management"""
    import argparse

    parser = argparse.ArgumentParser(description="DSPy Dataset Manager")
    parser.add_argument('--base-dir', default='training_data', help='Base directory for datasets')

    subparsers = parser.add_subparsers(dest='command', help='Command to run')

    # Create version
    create_parser = subparsers.add_parser('create', help='Create a new dataset version')
    create_parser.add_argument('--signature', required=True, help='Signature name')
    create_parser.add_argument('--input', required=True, help='Input JSON file with examples')
    create_parser.add_argument('--source', default='manual', help='Data source')
    create_parser.add_argument('--notes', default='', help='Notes about this version')

    # Add to version
    add_parser = subparsers.add_parser('add', help='Add examples to latest version')
    add_parser.add_argument('--signature', required=True, help='Signature name')
    add_parser.add_argument('--input', required=True, help='Input JSON file with new examples')
    add_parser.add_argument('--source', default='incremental', help='Data source')
    add_parser.add_argument('--notes', default='Incremental addition', help='Notes')

    # Load dataset
    load_parser = subparsers.add_parser('load', help='Load a dataset version')
    load_parser.add_argument('--signature', required=True, help='Signature name')
    load_parser.add_argument('--version', help='Version to load (default: latest)')
    load_parser.add_argument('--output', help='Output file (default: stdout)')

    # List versions
    list_parser = subparsers.add_parser('list', help='List versions for a signature')
    list_parser.add_argument('--signature', required=True, help='Signature name')

    # Summary
    summary_parser = subparsers.add_parser('summary', help='Show summary of all datasets')

    # Rollback
    rollback_parser = subparsers.add_parser('rollback', help='Rollback to a specific version')
    rollback_parser.add_argument('--signature', required=True, help='Signature name')
    rollback_parser.add_argument('--version', required=True, help='Version to rollback to')

    args = parser.parse_args()

    if not args.command:
        parser.print_help()
        sys.exit(1)

    manager = DatasetManager(base_dir=args.base_dir)

    if args.command == 'create':
        with open(args.input, 'r') as f:
            examples = json.load(f)
        version = manager.create_version(
            signature_name=args.signature,
            examples=examples,
            source=args.source,
            notes=args.notes
        )
        print(f"Created version: {version}")

    elif args.command == 'add':
        with open(args.input, 'r') as f:
            new_examples = json.load(f)
        version = manager.add_to_version(
            signature_name=args.signature,
            new_examples=new_examples,
            source=args.source,
            notes=args.notes
        )
        print(f"Created version: {version}")

    elif args.command == 'load':
        examples = manager.load_dataset(args.signature, args.version)
        output = json.dumps(examples, indent=2)
        if args.output:
            with open(args.output, 'w') as f:
                f.write(output)
            print(f"Loaded {len(examples)} examples to {args.output}")
        else:
            print(output)

    elif args.command == 'list':
        versions = manager.list_versions(args.signature)
        latest = manager.get_latest_version(args.signature)
        print(f"Versions for {args.signature}:")
        for v in versions:
            marker = " (latest)" if v == latest else ""
            metadata = manager.load_metadata(args.signature, v)
            if metadata:
                print(f"  v{v}{marker}: {metadata.total_examples} examples, quality={metadata.quality_score_avg:.1f}")
            else:
                print(f"  v{v}{marker}")

    elif args.command == 'summary':
        summary = manager.get_summary()
        print("Dataset Summary:")
        print(json.dumps(summary, indent=2))

    elif args.command == 'rollback':
        manager.rollback_to_version(args.signature, args.version)


if __name__ == '__main__':
    main()
