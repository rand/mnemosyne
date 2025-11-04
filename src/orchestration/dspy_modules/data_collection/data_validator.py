#!/usr/bin/env python3
"""
Data Validation Pipeline for DSPy Training Data

Validates, scores, and deduplicates training examples across all collection methods.
Ensures high-quality, diverse training data for optimization.

Key features:
- Schema validation (required fields, types, structure)
- Quality scoring (length, specificity, realism)
- Semantic deduplication (embedding-based similarity)
- Distribution validation (difficulty, completeness)
"""

import hashlib
import json
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Dict, List, Optional, Set, Tuple

import numpy as np

# Conditional import for embeddings (optional dependency)
try:
    from sentence_transformers import SentenceTransformer
    SENTENCE_TRANSFORMERS_AVAILABLE = True
except ImportError:
    SENTENCE_TRANSFORMERS_AVAILABLE = False
    SentenceTransformer = None


@dataclass
class ValidationResult:
    """Result of validating a single example"""
    valid: bool
    score: float  # 0-100, overall quality score
    errors: List[str]
    warnings: List[str]
    content_hash: str  # For exact deduplication
    embedding: Optional[np.ndarray] = None  # For semantic similarity


@dataclass
class DatasetMetrics:
    """Metrics for a complete dataset"""
    total_examples: int
    valid_examples: int
    invalid_examples: int
    duplicates_removed: int
    avg_quality_score: float
    difficulty_distribution: Dict[str, int]
    completeness_distribution: Dict[str, int]
    category_distribution: Dict[str, int]


class DataValidator:
    """Validate and deduplicate DSPy training data"""

    # Expected schema for each signature
    SCHEMAS = {
        'extract_requirements': {
            'inputs': ['user_intent', 'context'],
            'outputs': ['requirements', 'priorities'],
            'metadata': ['source', 'difficulty', 'category']
        },
        'validate_intent': {
            'inputs': ['user_intent', 'requirements'],
            'outputs': ['intent_satisfied', 'gaps', 'explanation'],
            'metadata': ['source', 'difficulty', 'category']
        },
        'validate_completeness': {
            'inputs': ['implementation', 'requirements'],
            'outputs': ['is_complete', 'missing_requirements', 'explanation'],
            'metadata': ['source', 'difficulty', 'category']
        },
        'validate_correctness': {
            'inputs': ['implementation', 'requirements'],
            'outputs': ['is_correct', 'errors', 'explanation'],
            'metadata': ['source', 'difficulty', 'category']
        },
        'generate_guidance': {
            'inputs': ['implementation', 'requirements', 'gaps'],
            'outputs': ['guidance', 'priority', 'rationale'],
            'metadata': ['source', 'difficulty', 'category']
        }
    }

    def __init__(self, use_embeddings: bool = True):
        """
        Initialize validator

        Args:
            use_embeddings: If True, use embeddings for semantic deduplication
                           If False, only use exact hash-based deduplication
        """
        self.use_embeddings = use_embeddings
        if use_embeddings:
            if not SENTENCE_TRANSFORMERS_AVAILABLE:
                print("Warning: sentence-transformers not available", file=sys.stderr)
                print("Falling back to hash-based deduplication only", file=sys.stderr)
                self.use_embeddings = False
                self.encoder = None
            else:
                try:
                    self.encoder = SentenceTransformer('all-MiniLM-L6-v2')
                except Exception as e:
                    print(f"Warning: Failed to load embedding model: {e}", file=sys.stderr)
                    print("Falling back to hash-based deduplication only", file=sys.stderr)
                    self.use_embeddings = False
                    self.encoder = None
        else:
            self.encoder = None

    def validate_schema(
        self,
        example: Dict[str, Any],
        signature_name: str
    ) -> Tuple[bool, List[str]]:
        """Validate example matches expected schema"""
        errors = []

        # Check signature is known
        if signature_name not in self.SCHEMAS:
            errors.append(f"Unknown signature: {signature_name}")
            return False, errors

        schema = self.SCHEMAS[signature_name]

        # Check top-level structure
        for section in ['inputs', 'outputs', 'metadata']:
            if section not in example:
                errors.append(f"Missing section: {section}")

        if errors:
            return False, errors

        # Check required fields in each section
        for section, required_fields in schema.items():
            if section not in example:
                continue
            section_data = example[section]
            for field in required_fields:
                if field not in section_data:
                    errors.append(f"Missing {section}.{field}")
                elif section_data[field] is None:
                    errors.append(f"Null value for {section}.{field}")
                elif isinstance(section_data[field], str) and not section_data[field].strip():
                    errors.append(f"Empty string for {section}.{field}")
                elif isinstance(section_data[field], list) and len(section_data[field]) == 0:
                    errors.append(f"Empty list for {section}.{field}")

        return len(errors) == 0, errors

    def score_quality(
        self,
        example: Dict[str, Any],
        signature_name: str
    ) -> Tuple[float, List[str]]:
        """
        Score example quality (0-100)

        Criteria:
        - Length: Sufficient detail in text fields
        - Specificity: Concrete details, not vague language
        - Realism: Realistic scenarios and implementations
        - Diversity: Variety in categories and scenarios
        """
        warnings = []
        score = 100.0  # Start perfect, deduct points

        inputs = example.get('inputs', {})
        outputs = example.get('outputs', {})

        # Length checks (deduct up to 20 points)
        text_fields = []
        if signature_name == 'extract_requirements':
            text_fields = [
                ('user_intent', inputs.get('user_intent', ''), 20, 10),
            ]
            if 'requirements' in outputs:
                for i, req in enumerate(outputs['requirements']):
                    text_fields.append((f'requirement[{i}]', req, 15, 5))
        elif signature_name in ['validate_completeness', 'validate_correctness']:
            text_fields = [
                ('implementation', inputs.get('implementation', ''), 50, 10),
                ('explanation', outputs.get('explanation', ''), 30, 10),
            ]
        elif signature_name == 'generate_guidance':
            text_fields = [
                ('guidance', outputs.get('guidance', ''), 50, 10),
                ('rationale', outputs.get('rationale', ''), 30, 10),
            ]

        for field_name, text, min_len, deduction in text_fields:
            if len(text) < min_len:
                warnings.append(f"{field_name} too short ({len(text)} < {min_len})")
                score -= deduction

        # Specificity checks (deduct up to 30 points)
        vague_terms = ['proper', 'correctly', 'appropriately', 'adequate',
                      'sufficient', 'good', 'bad', 'nice', 'better']

        all_text = ' '.join([
            str(v) for v in inputs.values() if isinstance(v, str)
        ] + [
            str(v) for v in outputs.values() if isinstance(v, str)
        ]).lower()

        vague_count = sum(1 for term in vague_terms if term in all_text)
        if vague_count > 3:
            warnings.append(f"Too many vague terms ({vague_count}): {', '.join(vague_terms)}")
            score -= min(vague_count * 5, 30)

        # Realism checks (deduct up to 20 points)
        if signature_name == 'validate_completeness':
            impl = inputs.get('implementation', '').lower()
            has_files = any(word in impl for word in ['file', '.rs', '.py', '.ts', '.go', 'src/', 'test'])
            has_metrics = any(word in impl for word in ['lines', 'tests', 'functions', 'methods', 'coverage'])
            if not (has_files or has_metrics):
                warnings.append("Implementation lacks concrete details (no files or metrics)")
                score -= 20

        # Consistency checks (deduct up to 30 points)
        if signature_name == 'extract_requirements':
            reqs = outputs.get('requirements', [])
            priorities = outputs.get('priorities', [])
            if len(reqs) != len(priorities):
                warnings.append(f"Requirements count ({len(reqs)}) != priorities count ({len(priorities)})")
                score -= 30
        elif signature_name == 'validate_completeness':
            is_complete = outputs.get('is_complete', False)
            missing = outputs.get('missing_requirements', [])
            if is_complete and len(missing) > 0:
                warnings.append(f"Marked complete but has {len(missing)} missing requirements")
                score -= 30
            elif not is_complete and len(missing) == 0:
                warnings.append("Marked incomplete but no missing requirements listed")
                score -= 30

        return max(0.0, score), warnings

    def compute_content_hash(self, example: Dict[str, Any]) -> str:
        """Compute hash of example content for exact deduplication"""
        # Use inputs and outputs only (ignore metadata)
        content = {
            'inputs': example.get('inputs', {}),
            'outputs': example.get('outputs', {})
        }
        content_str = json.dumps(content, sort_keys=True)
        return hashlib.sha256(content_str.encode()).hexdigest()

    def compute_embedding(self, example: Dict[str, Any]) -> Optional[np.ndarray]:
        """Compute embedding for semantic similarity"""
        if not self.use_embeddings or self.encoder is None:
            return None

        # Concatenate all text fields
        inputs = example.get('inputs', {})
        outputs = example.get('outputs', {})

        text_parts = []
        for value in inputs.values():
            if isinstance(value, str):
                text_parts.append(value)
            elif isinstance(value, list):
                text_parts.extend([str(v) for v in value if isinstance(v, str)])

        for value in outputs.values():
            if isinstance(value, str):
                text_parts.append(value)
            elif isinstance(value, list):
                text_parts.extend([str(v) for v in value if isinstance(v, str)])

        text = ' '.join(text_parts)

        try:
            embedding = self.encoder.encode(text, convert_to_numpy=True)
            return embedding
        except Exception as e:
            print(f"Warning: Failed to compute embedding: {e}", file=sys.stderr)
            return None

    def validate_example(
        self,
        example: Dict[str, Any],
        signature_name: str
    ) -> ValidationResult:
        """Validate a single example"""
        errors = []
        warnings = []

        # Schema validation
        schema_valid, schema_errors = self.validate_schema(example, signature_name)
        errors.extend(schema_errors)

        # Quality scoring
        quality_score = 0.0
        if schema_valid:
            quality_score, quality_warnings = self.score_quality(example, signature_name)
            warnings.extend(quality_warnings)

        # Content hashing
        content_hash = self.compute_content_hash(example)

        # Embedding
        embedding = self.compute_embedding(example) if schema_valid else None

        return ValidationResult(
            valid=schema_valid,
            score=quality_score,
            errors=errors,
            warnings=warnings,
            content_hash=content_hash,
            embedding=embedding
        )

    def deduplicate(
        self,
        examples: List[Dict[str, Any]],
        results: List[ValidationResult],
        similarity_threshold: float = 0.9
    ) -> List[int]:
        """
        Find duplicate examples

        Args:
            examples: List of examples
            results: Validation results for each example
            similarity_threshold: Cosine similarity threshold (0-1)

        Returns:
            List of indices to remove (duplicates)
        """
        to_remove = set()
        seen_hashes = set()

        # Exact deduplication by hash
        for i, result in enumerate(results):
            if result.content_hash in seen_hashes:
                to_remove.add(i)
            else:
                seen_hashes.add(result.content_hash)

        # Semantic deduplication by embedding similarity
        if self.use_embeddings:
            embeddings = [r.embedding for r in results if r.embedding is not None]
            valid_indices = [i for i, r in enumerate(results) if r.embedding is not None and i not in to_remove]

            if len(valid_indices) > 1:
                # Compute pairwise similarities
                embeddings_array = np.array([results[i].embedding for i in valid_indices])
                # Normalize for cosine similarity
                norms = np.linalg.norm(embeddings_array, axis=1, keepdims=True)
                embeddings_normalized = embeddings_array / (norms + 1e-8)
                similarities = np.dot(embeddings_normalized, embeddings_normalized.T)

                # Find duplicates (upper triangle only to avoid double-counting)
                for i in range(len(valid_indices)):
                    if valid_indices[i] in to_remove:
                        continue
                    for j in range(i + 1, len(valid_indices)):
                        if similarities[i, j] >= similarity_threshold:
                            # Keep higher quality example
                            if results[valid_indices[i]].score >= results[valid_indices[j]].score:
                                to_remove.add(valid_indices[j])
                            else:
                                to_remove.add(valid_indices[i])
                                break  # i is removed, no need to check further

        return sorted(list(to_remove))

    def validate_dataset(
        self,
        dataset_path: Path,
        signature_name: str,
        min_quality_score: float = 50.0,
        similarity_threshold: float = 0.9
    ) -> Tuple[List[Dict[str, Any]], DatasetMetrics]:
        """
        Validate and clean a complete dataset

        Args:
            dataset_path: Path to JSON file
            signature_name: DSPy signature name
            min_quality_score: Minimum quality score (0-100)
            similarity_threshold: Deduplication threshold (0-1)

        Returns:
            (cleaned_examples, metrics)
        """
        # Load dataset
        with open(dataset_path) as f:
            examples = json.load(f)

        print(f"Validating {len(examples)} examples from {dataset_path}")

        # Validate each example
        results = []
        for i, example in enumerate(examples):
            result = self.validate_example(example, signature_name)
            results.append(result)
            if not result.valid:
                print(f"  Example {i}: INVALID - {', '.join(result.errors)}")
            elif result.score < min_quality_score:
                print(f"  Example {i}: LOW QUALITY ({result.score:.1f}) - {', '.join(result.warnings)}")

        # Count valid and high-quality examples
        valid_examples = [i for i, r in enumerate(results) if r.valid]
        high_quality = [i for i in valid_examples if results[i].score >= min_quality_score]

        print(f"Valid: {len(valid_examples)}/{len(examples)}")
        print(f"High quality (≥{min_quality_score}): {len(high_quality)}/{len(valid_examples)}")

        # Deduplicate
        to_remove = self.deduplicate(examples, results, similarity_threshold)
        print(f"Duplicates found: {len(to_remove)}")

        # Filter: keep valid, high-quality, non-duplicate examples
        keep_indices = [
            i for i in high_quality
            if i not in to_remove
        ]

        cleaned = [examples[i] for i in keep_indices]

        # Compute metrics
        difficulty_dist = {}
        completeness_dist = {}
        category_dist = {}
        total_score = 0.0

        for i in keep_indices:
            result = results[i]
            example = examples[i]
            total_score += result.score

            # Distribution metrics
            difficulty = example.get('metadata', {}).get('difficulty', 'unknown')
            difficulty_dist[difficulty] = difficulty_dist.get(difficulty, 0) + 1

            category = example.get('metadata', {}).get('category', 'unknown')
            category_dist[category] = category_dist.get(category, 0) + 1

            # Completeness (signature-specific)
            if signature_name == 'validate_completeness':
                is_complete = example.get('outputs', {}).get('is_complete', False)
                key = 'complete' if is_complete else 'incomplete'
                completeness_dist[key] = completeness_dist.get(key, 0) + 1

        metrics = DatasetMetrics(
            total_examples=len(examples),
            valid_examples=len(valid_examples),
            invalid_examples=len(examples) - len(valid_examples),
            duplicates_removed=len(to_remove),
            avg_quality_score=total_score / len(keep_indices) if keep_indices else 0.0,
            difficulty_distribution=difficulty_dist,
            completeness_distribution=completeness_dist,
            category_distribution=category_dist
        )

        print(f"\nMetrics:")
        print(f"  Total: {metrics.total_examples}")
        print(f"  Valid: {metrics.valid_examples}")
        print(f"  Invalid: {metrics.invalid_examples}")
        print(f"  Duplicates removed: {metrics.duplicates_removed}")
        print(f"  Final dataset size: {len(cleaned)}")
        print(f"  Avg quality score: {metrics.avg_quality_score:.1f}")
        print(f"  Difficulty: {metrics.difficulty_distribution}")
        print(f"  Completeness: {metrics.completeness_distribution}")
        print(f"  Categories: {metrics.category_distribution}")

        return cleaned, metrics


def main():
    import argparse

    parser = argparse.ArgumentParser(description='Validate and clean DSPy training data')
    parser.add_argument('input', type=Path, help='Input JSON file')
    parser.add_argument('--signature', type=str, required=True,
                       choices=list(DataValidator.SCHEMAS.keys()),
                       help='DSPy signature name')
    parser.add_argument('--output', type=Path, help='Output cleaned JSON file')
    parser.add_argument('--min-quality', type=float, default=50.0,
                       help='Minimum quality score (0-100)')
    parser.add_argument('--similarity-threshold', type=float, default=0.9,
                       help='Deduplication similarity threshold (0-1)')
    parser.add_argument('--no-embeddings', action='store_true',
                       help='Disable semantic deduplication (hash-only)')

    args = parser.parse_args()

    # Validate dataset
    validator = DataValidator(use_embeddings=not args.no_embeddings)
    cleaned, metrics = validator.validate_dataset(
        dataset_path=args.input,
        signature_name=args.signature,
        min_quality_score=args.min_quality,
        similarity_threshold=args.similarity_threshold
    )

    # Save cleaned dataset
    if args.output:
        args.output.parent.mkdir(parents=True, exist_ok=True)
        with open(args.output, 'w') as f:
            json.dump(cleaned, f, indent=2)
        print(f"\n✅ Saved {len(cleaned)} cleaned examples to {args.output}")

    # Exit with status
    if len(cleaned) < metrics.total_examples * 0.5:
        print(f"\n⚠️  Warning: Removed >50% of examples. Check quality thresholds.", file=sys.stderr)
        sys.exit(1)


if __name__ == '__main__':
    main()
