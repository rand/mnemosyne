"""
Python client for Mnemosyne, wrapping the Rust CLI.

Provides async interface for storing and retrieving memories from Python code.
"""
import subprocess
import json
import os
from typing import List, Optional, Dict, Any


class MnemosyneClient:
    """
    Async client for Mnemosyne memory operations.

    Wraps the Rust CLI binary to provide Python-friendly interface.
    """

    def __init__(self, db_path: Optional[str] = None, binary_path: str = "mnemosyne"):
        """
        Initialize Mnemosyne client.

        Args:
            db_path: Optional custom database path
            binary_path: Path to mnemosyne binary (default: "mnemosyne" in PATH)
        """
        self.db_path = db_path or os.getenv("DATABASE_URL")
        self.binary_path = binary_path

    async def remember(
        self,
        content: str,
        namespace: str,
        importance: int,
        context: Optional[str] = None
    ) -> Dict[str, Any]:
        """
        Store a memory in Mnemosyne.

        Args:
            content: Memory content
            namespace: Namespace (e.g., "session:orchestration")
            importance: Importance score 1-10
            context: Optional context information

        Returns:
            dict: Memory metadata (id, summary, keywords)
        """
        cmd = [
            self.binary_path, "remember",
            content,
            "--namespace", namespace,
            "--importance", str(importance),
            "--format", "json",
        ]

        if context:
            cmd.extend(["--context", context])

        if self.db_path:
            cmd.extend(["--db", self.db_path])

        result = subprocess.run(cmd, capture_output=True, text=True)

        if result.returncode != 0:
            raise RuntimeError(f"mnemosyne remember failed: {result.stderr}")

        try:
            return json.loads(result.stdout)
        except json.JSONDecodeError:
            # Fallback if JSON parsing fails
            return {"output": result.stdout, "success": True}

    async def recall(
        self,
        query: str,
        namespace: Optional[str] = None,
        max_results: int = 10,
        min_importance: Optional[int] = None
    ) -> List[Dict[str, Any]]:
        """
        Search Mnemosyne memories.

        Args:
            query: Search query
            namespace: Optional namespace filter
            max_results: Maximum number of results
            min_importance: Minimum importance filter

        Returns:
            List[dict]: Matching memories
        """
        cmd = [self.binary_path, "recall", query]

        if namespace:
            cmd.extend(["--namespace", namespace])

        cmd.extend(["--limit", str(max_results)])
        cmd.extend(["--format", "json"])

        if min_importance:
            cmd.extend(["--min-importance", str(min_importance)])

        if self.db_path:
            cmd.extend(["--db", self.db_path])

        result = subprocess.run(cmd, capture_output=True, text=True)

        if result.returncode != 0:
            # Return empty list if search fails (e.g., no results)
            return []

        try:
            output = json.loads(result.stdout)
            # Handle both list and dict response formats
            if isinstance(output, list):
                return output
            elif isinstance(output, dict) and "memories" in output:
                return output["memories"]
            # Fallback
            return [{"content": result.stdout, "raw": output}]
        except json.JSONDecodeError:
            return [{"content": result.stdout}]

    async def list_memories(
        self,
        namespace: Optional[str] = None,
        limit: int = 20,
        sort_by: str = "recent"
    ) -> List[Dict[str, Any]]:
        """
        List memories.

        Args:
            namespace: Optional namespace filter
            limit: Maximum number of results
            sort_by: Sort order (recent, importance, access)

        Returns:
            List[dict]: Memories
        """
        cmd = [self.binary_path, "list"]

        if namespace:
            cmd.extend(["--namespace", namespace])

        cmd.extend(["--limit", str(limit)])
        cmd.extend(["--sort", sort_by])

        if self.db_path:
            cmd.extend(["--db", self.db_path])

        result = subprocess.run(cmd, capture_output=True, text=True)

        if result.returncode != 0:
            return []

        return [{"content": result.stdout}]

    async def consolidate(
        self,
        namespace: Optional[str] = None,
        auto_apply: bool = False
    ) -> Dict[str, Any]:
        """
        Consolidate similar memories.

        Args:
            namespace: Optional namespace filter
            auto_apply: Automatically apply consolidation recommendations

        Returns:
            dict: Consolidation results
        """
        cmd = [self.binary_path, "consolidate"]

        if namespace:
            cmd.extend(["--namespace", namespace])

        if auto_apply:
            cmd.append("--auto")

        if self.db_path:
            cmd.extend(["--db", self.db_path])

        result = subprocess.run(cmd, capture_output=True, text=True)

        return {
            "output": result.stdout,
            "success": result.returncode == 0
        }

    async def graph(
        self,
        query: Optional[str] = None,
        namespace: Optional[str] = None,
        depth: int = 1,
    ) -> Dict[str, Any]:
        """
        Get memory graph.

        Args:
            query: Optional search query to center graph
            namespace: Optional namespace filter
            depth: Graph traversal depth

        Returns:
            dict: Graph structure (nodes, edges)
        """
        cmd = [
            self.binary_path, "graph",
            "--format", "json",
            "--depth", str(depth)
        ]

        if query:
            cmd.extend(["--query", query])

        if namespace:
            cmd.extend(["--namespace", namespace])

        if self.db_path:
            cmd.extend(["--db", self.db_path])

        result = subprocess.run(cmd, capture_output=True, text=True)

        if result.returncode != 0:
            raise RuntimeError(f"mnemosyne graph failed: {result.stderr}")

        try:
            return json.loads(result.stdout)
        except json.JSONDecodeError:
            return {"error": "Failed to parse JSON output", "raw_output": result.stdout}
