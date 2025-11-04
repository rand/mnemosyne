#!/usr/bin/env python3
"""
Git Mining Pipeline for DSPy Training Data Collection

Extracts training examples from git history by analyzing:
- Commit messages and descriptions
- PR descriptions and code reviews
- Code changes and file modifications

Target: 60 examples/month across all signatures
"""

import json
import re
import subprocess
import sys
from dataclasses import dataclass
from datetime import datetime, timedelta
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

import anthropic


@dataclass
class CommitData:
    """Structured commit information"""
    sha: str
    author: str
    date: datetime
    message: str
    files: List[str]
    additions: int
    deletions: int
    diff: str


@dataclass
class TrainingExample:
    """Generic training example before signature-specific formatting"""
    user_intent: str
    implementation: str
    requirements: List[str]
    is_complete: bool
    missing_requirements: List[str]
    explanation: str
    category: str
    difficulty: str
    notes: str


class GitMiner:
    """Extract training data from git history"""

    def __init__(self, repo_path: Path, api_key: str):
        self.repo_path = repo_path
        self.client = anthropic.Anthropic(api_key=api_key)

    def get_commits(self, since_days: int = 180, limit: int = 500) -> List[CommitData]:
        """Get commits from the last N days"""
        since_date = (datetime.now() - timedelta(days=since_days)).strftime('%Y-%m-%d')

        # Get commit list with metadata
        cmd = [
            'git', 'log',
            f'--since={since_date}',
            f'--max-count={limit}',
            '--format=%H|%an|%ad|%s',
            '--date=iso',
            '--no-merges',  # Skip merge commits
            '--',
        ]

        result = subprocess.run(
            cmd,
            cwd=self.repo_path,
            capture_output=True,
            text=True,
            check=True
        )

        commits = []
        for line in result.stdout.strip().split('\n'):
            if not line:
                continue

            sha, author, date_str, message = line.split('|', 3)

            # Get commit details
            files, additions, deletions = self._get_commit_stats(sha)
            diff = self._get_commit_diff(sha)

            commits.append(CommitData(
                sha=sha,
                author=author,
                date=datetime.fromisoformat(date_str.replace(' ', 'T').rsplit('+', 1)[0]),
                message=message,
                files=files,
                additions=additions,
                deletions=deletions,
                diff=diff
            ))

        return commits

    def _get_commit_stats(self, sha: str) -> Tuple[List[str], int, int]:
        """Get files changed and line counts for a commit"""
        cmd = ['git', 'show', '--stat', '--format=', sha]
        result = subprocess.run(
            cmd,
            cwd=self.repo_path,
            capture_output=True,
            text=True,
            check=True
        )

        files = []
        additions = 0
        deletions = 0

        for line in result.stdout.strip().split('\n'):
            if '|' not in line:
                continue

            parts = line.split('|')
            if len(parts) < 2:
                continue

            filename = parts[0].strip()
            files.append(filename)

            # Parse additions/deletions
            stats = parts[1].strip()
            match = re.search(r'(\d+) insertion.*?(\d+) deletion', stats)
            if match:
                additions += int(match.group(1))
                deletions += int(match.group(2))
            elif 'insertion' in stats:
                match = re.search(r'(\d+) insertion', stats)
                if match:
                    additions += int(match.group(1))
            elif 'deletion' in stats:
                match = re.search(r'(\d+) deletion', stats)
                if match:
                    deletions += int(match.group(1))

        return files, additions, deletions

    def _get_commit_diff(self, sha: str, max_lines: int = 500) -> str:
        """Get diff for a commit (limited to avoid token overflow)"""
        cmd = ['git', 'show', '--format=%B', sha]
        result = subprocess.run(
            cmd,
            cwd=self.repo_path,
            capture_output=True,
            text=True,
            check=True
        )

        diff_lines = result.stdout.split('\n')
        if len(diff_lines) > max_lines:
            # Truncate but keep structure
            diff_lines = diff_lines[:max_lines] + ['\n... (truncated)']

        return '\n'.join(diff_lines)

    def filter_quality_commits(self, commits: List[CommitData]) -> List[CommitData]:
        """Filter for commits likely to yield quality training data"""
        filtered = []

        for commit in commits:
            # Quality signals
            has_detailed_message = len(commit.message) > 50
            has_meaningful_changes = 10 < commit.additions < 1000  # Not too small, not too large
            touches_source_code = any(
                f.endswith(('.rs', '.py', '.ts', '.go'))
                for f in commit.files
            )
            not_just_formatting = commit.additions > commit.deletions * 0.5  # Avoid pure deletions

            # Multiple review rounds indicator (multiple files, thoughtful message)
            likely_reviewed = (
                has_detailed_message
                and len(commit.files) >= 2
                and len(commit.files) <= 15  # Not too many files (likely refactor)
            )

            if (has_detailed_message
                and has_meaningful_changes
                and touches_source_code
                and not_just_formatting
                and likely_reviewed):
                filtered.append(commit)

        return filtered

    def extract_training_example(self, commit: CommitData) -> Optional[TrainingExample]:
        """Use Claude to extract structured training data from a commit"""

        prompt = f"""Analyze this git commit and extract training data for a code review system.

Commit Message:
{commit.message}

Files Changed: {', '.join(commit.files[:10])}
Additions: {commit.additions} | Deletions: {commit.deletions}

Diff (excerpt):
{commit.diff[:3000]}

Extract the following:
1. **User Intent**: What was the developer trying to accomplish? (1-2 sentences)
2. **Implementation**: Summarize what was actually implemented, including files and key changes
3. **Requirements**: List 3-7 specific requirements that would have been needed to implement this
4. **Is Complete**: Does the implementation fully satisfy all requirements? (True/False)
5. **Missing Requirements**: If incomplete, what's missing?
6. **Explanation**: Why is it complete/incomplete? (2-3 sentences)
7. **Category**: authentication, caching, database, api, testing, infrastructure, etc.
8. **Difficulty**: easy, medium, or hard
9. **Notes**: Any additional context

Format your response as JSON:
{{
  "user_intent": "...",
  "implementation": "...",
  "requirements": ["...", "..."],
  "is_complete": true/false,
  "missing_requirements": ["...", "..."],
  "explanation": "...",
  "category": "...",
  "difficulty": "...",
  "notes": "..."
}}

Only extract if this appears to be a meaningful feature/fix. Return null if it's just minor tweaks."""

        try:
            response = self.client.messages.create(
                model="claude-sonnet-4-5-20250929",
                max_tokens=2000,
                messages=[{"role": "user", "content": prompt}]
            )

            content = response.content[0].text

            # Extract JSON from response
            json_match = re.search(r'\{[\s\S]*\}', content)
            if not json_match:
                return None

            data = json.loads(json_match.group(0))

            # Validate required fields
            required = ['user_intent', 'implementation', 'requirements', 'is_complete',
                       'explanation', 'category', 'difficulty']
            if not all(k in data for k in required):
                return None

            return TrainingExample(**data)

        except Exception as e:
            print(f"Error extracting from commit {commit.sha[:8]}: {e}", file=sys.stderr)
            return None

    def format_for_signatures(self, example: TrainingExample) -> Dict[str, Any]:
        """Format extracted data for multiple DSPy signatures"""

        return {
            'extract_requirements': {
                'inputs': {
                    'user_intent': example.user_intent,
                    'context': f"Implementation phase with files: {example.implementation[:100]}..."
                },
                'outputs': {
                    'requirements': example.requirements,
                    'priorities': [10] * len(example.requirements)  # Default high priority
                },
                'metadata': {
                    'source': 'git',
                    'difficulty': example.difficulty,
                    'category': example.category,
                    'notes': example.notes
                }
            },
            'validate_completeness': {
                'inputs': {
                    'implementation': example.implementation,
                    'requirements': example.requirements
                },
                'outputs': {
                    'is_complete': example.is_complete,
                    'missing_requirements': example.missing_requirements,
                    'explanation': example.explanation
                },
                'metadata': {
                    'source': 'git',
                    'difficulty': example.difficulty,
                    'category': example.category,
                    'notes': example.notes
                }
            }
        }

    def mine_and_save(
        self,
        output_dir: Path,
        target_examples: int = 60,
        since_days: int = 180
    ) -> Dict[str, int]:
        """Main mining pipeline: extract and save training data"""

        print(f"Mining git history from last {since_days} days...")
        commits = self.get_commits(since_days=since_days, limit=500)
        print(f"Found {len(commits)} commits")

        print("Filtering for quality commits...")
        quality_commits = self.filter_quality_commits(commits)
        print(f"Filtered to {len(quality_commits)} quality commits")

        # Extract training examples
        examples = []
        for i, commit in enumerate(quality_commits[:target_examples * 2], 1):  # Extract 2x target
            print(f"Extracting {i}/{min(len(quality_commits), target_examples * 2)}: {commit.sha[:8]}", end='')

            example = self.extract_training_example(commit)
            if example:
                examples.append(example)
                print(" ✓")
            else:
                print(" ✗ (skipped)")

            if len(examples) >= target_examples:
                break

        print(f"\nExtracted {len(examples)} training examples")

        # Format for signatures and save
        output_dir.mkdir(parents=True, exist_ok=True)

        signatures = {
            'extract_requirements': [],
            'validate_completeness': []
        }

        for example in examples:
            formatted = self.format_for_signatures(example)
            signatures['extract_requirements'].append(formatted['extract_requirements'])
            signatures['validate_completeness'].append(formatted['validate_completeness'])

        # Save to JSON files
        counts = {}
        for signature_name, signature_data in signatures.items():
            output_file = output_dir / f'{signature_name}_git_mined.json'
            with open(output_file, 'w') as f:
                json.dump(signature_data, f, indent=2)
            counts[signature_name] = len(signature_data)
            print(f"Saved {counts[signature_name]} examples to {output_file}")

        return counts


def main():
    import argparse

    parser = argparse.ArgumentParser(description='Mine git history for DSPy training data')
    parser.add_argument('--repo', type=Path, default=Path.cwd(),
                       help='Path to git repository')
    parser.add_argument('--output', type=Path,
                       default=Path('src/orchestration/dspy_modules/training_data_mined'),
                       help='Output directory for training data')
    parser.add_argument('--target', type=int, default=60,
                       help='Target number of examples to extract')
    parser.add_argument('--since-days', type=int, default=180,
                       help='Look back N days in git history')
    parser.add_argument('--api-key', type=str,
                       help='Anthropic API key (or set ANTHROPIC_API_KEY env var)')

    args = parser.parse_args()

    # Get API key
    import os
    api_key = args.api_key or os.environ.get('ANTHROPIC_API_KEY')
    if not api_key:
        print("Error: ANTHROPIC_API_KEY not set", file=sys.stderr)
        sys.exit(1)

    # Run mining pipeline
    miner = GitMiner(args.repo, api_key)
    counts = miner.mine_and_save(
        output_dir=args.output,
        target_examples=args.target,
        since_days=args.since_days
    )

    print(f"\n✅ Mining complete!")
    print(f"Total examples collected:")
    for sig, count in counts.items():
        print(f"  - {sig}: {count} examples")


if __name__ == '__main__':
    main()
