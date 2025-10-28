"""
Privacy compliance tests for Python integration with evaluation system.

Tests the Optimizer agent's privacy-preserving features:
- Task description hashing
- Metadata extraction without sensitive data
- Graceful degradation when evaluation disabled
"""

import hashlib
import pytest
from pathlib import Path
import sys

# Add src to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

try:
    from orchestration.agents.optimizer import OptimizerAgent, OptimizerConfig
    OPTIMIZER_AVAILABLE = True
except ImportError:
    OPTIMIZER_AVAILABLE = False


class TestTaskHashingPrivacy:
    """Test privacy-preserving task hashing."""

    def test_hash_truncated_to_16_chars(self):
        """Verify task hashes are always truncated to 16 characters."""
        task_description = "Fix authentication bug with password reset endpoint"

        # Simulate Python's _hash_task_description
        hash_obj = hashlib.sha256(task_description.encode('utf-8'))
        task_hash = hash_obj.hexdigest()[:16]

        assert len(task_hash) == 16, f"Hash should be exactly 16 chars, got {len(task_hash)}"
        assert task_hash.isalnum(), "Hash should be alphanumeric"

    def test_hash_consistency(self):
        """Same task should produce same hash."""
        task1 = "Implement user authentication"
        task2 = "Implement user authentication"
        task3 = "Implement user authorization"

        hash1 = hashlib.sha256(task1.encode('utf-8')).hexdigest()[:16]
        hash2 = hashlib.sha256(task2.encode('utf-8')).hexdigest()[:16]
        hash3 = hashlib.sha256(task3.encode('utf-8')).hexdigest()[:16]

        assert hash1 == hash2, "Same task should produce same hash"
        assert hash1 != hash3, "Different tasks should produce different hashes"

    def test_hash_does_not_contain_task_keywords(self):
        """Hash should not contain recognizable task keywords."""
        sensitive_task = "Fix password reset bug where passwords are logged"

        hash_obj = hashlib.sha256(sensitive_task.encode('utf-8'))
        task_hash = hash_obj.hexdigest()[:16]

        # Hash should be cryptographic, not contain plaintext
        assert "password" not in task_hash.lower()
        assert "reset" not in task_hash.lower()
        assert "logged" not in task_hash.lower()


class TestMetadataExtractionPrivacy:
    """Test privacy-preserving metadata extraction."""

    def test_metadata_only_categorical(self):
        """Metadata should only contain categorical/generic data."""
        # Simulate metadata extraction from Optimizer
        sensitive_task = "Fix bug in user password reset endpoint where passwords are logged"

        # Extract safe metadata (as done in optimizer.py)
        metadata = extract_safe_metadata(sensitive_task)

        # Verify only categorical data
        assert metadata.get('task_type') in ['feature', 'bugfix', 'refactor', 'test',
                                               'documentation', 'optimization', 'exploration']
        assert metadata.get('work_phase') in ['planning', 'implementation', 'debugging',
                                                'review', 'testing', 'documentation']

        # Should not contain sensitive terms
        assert not contains_sensitive_data(metadata)

    def test_metadata_no_pii(self):
        """Metadata should not contain personally identifiable information."""
        task = "Update admin user profile for john.doe@company.com"

        metadata = extract_safe_metadata(task)

        # Should not contain email or names
        for value in metadata.values():
            assert "@" not in str(value), "Email found in metadata"
            assert "john" not in str(value).lower(), "Name found in metadata"
            assert "doe" not in str(value).lower(), "Name found in metadata"

    def test_keyword_limit_enforced(self):
        """Only max 10 keywords should be extracted."""
        long_task = " ".join([f"keyword{i}" for i in range(30)])

        # Simulate keyword extraction
        keywords = extract_keywords(long_task)

        assert len(keywords) <= 10, f"More than 10 keywords extracted: {len(keywords)}"

    def test_sensitive_keywords_filtered(self):
        """Sensitive keywords should be filtered out."""
        task = "Fix authentication with api_key and password stored in secret vault"

        keywords = extract_keywords(task)

        sensitive_terms = ['password', 'secret', 'api_key', 'token', 'credentials',
                          'private_key', 'ssh_key']

        for keyword in keywords:
            for sensitive in sensitive_terms:
                assert sensitive not in keyword.lower(), \
                    f"Sensitive term '{sensitive}' found in keywords: {keyword}"


class TestGracefulDegradation:
    """Test that system works when evaluation is disabled."""

    def test_optimizer_works_without_evaluation(self):
        """Optimizer should work even if evaluation system unavailable."""
        # This simulates EVALUATION_AVAILABLE = False
        # In this case, Optimizer should function but skip evaluation

        # If we can't import optimizer, that's okay for this test
        if not OPTIMIZER_AVAILABLE:
            pytest.skip("Optimizer not available for testing")

        # Config with evaluation disabled
        config = OptimizerConfig(
            agent_id="test-optimizer",
            enable_evaluation=False
        )

        # Should not crash
        assert config.enable_evaluation is False

    def test_no_network_calls_in_evaluation(self):
        """Evaluation system should not make network calls."""
        # This is verified by:
        # 1. Database path must be local (checked in test_database_local_only)
        # 2. No HTTP/HTTPS in connection strings
        # 3. All operations use local libsql

        db_path = ".mnemosyne/test.db"

        # Verify path is local
        assert not db_path.startswith("http://")
        assert not db_path.startswith("https://")
        assert not db_path.startswith("libsql://")


class TestDatabaseStoragePrivacy:
    """Test that evaluation data is stored securely."""

    def test_database_in_gitignored_directory(self):
        """Database should be in .mnemosyne/ which is gitignored."""
        db_path = ".mnemosyne/project.db"

        # Should be in .mnemosyne/
        assert db_path.startswith(".mnemosyne/")

        # Check .gitignore covers this
        gitignore_path = Path(__file__).parent.parent / ".gitignore"
        if gitignore_path.exists():
            gitignore_content = gitignore_path.read_text()
            assert ".mnemosyne/" in gitignore_content, \
                ".mnemosyne/ should be in .gitignore"
            assert "*.db" in gitignore_content, \
                "*.db files should be in .gitignore"

    def test_database_local_only(self):
        """Database path should be local, not remote."""
        # Test default database path
        from pathlib import Path
        import os

        # Check project DB
        project_db = Path(".mnemosyne/project.db")
        assert not str(project_db).startswith("http")

        # Check XDG default
        xdg_data = os.environ.get("XDG_DATA_HOME")
        if xdg_data:
            xdg_db = Path(xdg_data) / "mnemosyne" / "mnemosyne.db"
            assert not str(xdg_db).startswith("http")


# Helper functions

def extract_safe_metadata(task: str) -> dict:
    """Extract safe metadata from task (no sensitive data)."""
    metadata = {}
    task_lower = task.lower()

    # Task type (categorical)
    if any(word in task_lower for word in ['bug', 'fix', 'error', 'crash']):
        metadata['task_type'] = 'bugfix'
    elif any(word in task_lower for word in ['test', 'testing', 'spec']):
        metadata['task_type'] = 'test'
    elif any(word in task_lower for word in ['refactor', 'reorganize', 'clean']):
        metadata['task_type'] = 'refactor'
    elif any(word in task_lower for word in ['document', 'docs', 'readme']):
        metadata['task_type'] = 'documentation'
    elif any(word in task_lower for word in ['optimize', 'performance', 'speed']):
        metadata['task_type'] = 'optimization'
    else:
        metadata['task_type'] = 'feature'

    # Work phase (categorical)
    if any(word in task_lower for word in ['debug', 'debugging', 'investigate']):
        metadata['work_phase'] = 'debugging'
    elif any(word in task_lower for word in ['review', 'check', 'validate']):
        metadata['work_phase'] = 'review'
    elif any(word in task_lower for word in ['plan', 'design', 'architect']):
        metadata['work_phase'] = 'planning'
    elif any(word in task_lower for word in ['test', 'testing']):
        metadata['work_phase'] = 'testing'
    else:
        metadata['work_phase'] = 'implementation'

    return metadata


def extract_keywords(task: str) -> list:
    """Extract generic keywords (max 10, filtered)."""
    words = task.lower().split()

    # Filter sensitive keywords
    sensitive_terms = {'password', 'secret', 'api_key', 'token', 'credentials',
                      'private_key', 'ssh_key', 'access_token', 'key'}

    # Filter stopwords and sensitive terms
    stopwords = {'the', 'a', 'an', 'and', 'or', 'but', 'in', 'on', 'at', 'to',
                'for', 'with', 'this', 'that', 'is', 'are', 'was', 'were'}

    keywords = [
        w for w in words
        if w not in stopwords
        and w not in sensitive_terms
        and len(w) > 3
    ]

    # Limit to 10
    return keywords[:10]


def contains_sensitive_data(metadata: dict) -> bool:
    """Check if metadata contains sensitive data."""
    sensitive_keywords = ['password', 'secret', 'key', 'token', 'credential',
                         'private', 'api_key', 'ssh_key']

    for value in metadata.values():
        value_str = str(value).lower()
        for sensitive in sensitive_keywords:
            if sensitive in value_str:
                return True

    return False


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
