"""
Optimizer Agent - Context and Resource Optimization Specialist.

Responsibilities:
- Construct optimal context payloads for each agent
- Apply ACE principles: incremental updates, structured accumulation, strategy preservation
- Monitor all context sources: agents, files, commits, plans, beads, skills, session
- Prevent brevity bias and context collapse
- Dynamically discover and load relevant skills from filesystem
"""

from dataclasses import dataclass
from typing import Dict, List, Optional, Set
from pathlib import Path


@dataclass
class OptimizerConfig:
    """Configuration for Optimizer agent."""
    agent_id: str = "optimizer"
    skills_dir: str = "~/.claude/skills"
    context_budget_critical: float = 0.40  # 40% for critical context
    context_budget_skills: float = 0.30    # 30% for skills
    context_budget_project: float = 0.20   # 20% for project files
    context_budget_general: float = 0.10   # 10% for general
    max_skills_loaded: int = 7
    skill_relevance_threshold: float = 0.60


@dataclass
class SkillMatch:
    """Skill match result."""
    skill_path: str
    relevance_score: float
    keywords: List[str]
    categories: List[str]


class OptimizerAgent:
    """
    Context and resource optimization specialist.

    Manages:
    - Dynamic skill discovery and loading
    - Context budget allocation
    - ACE principle application
    - Memory consolidation
    - Context compression
    """

    def __init__(self, config: OptimizerConfig, coordinator, storage):
        """
        Initialize Optimizer agent.

        Args:
            config: Optimizer configuration
            coordinator: PyCoordinator for shared state
            storage: PyStorage for memory operations
        """
        self.config = config
        self.coordinator = coordinator
        self.storage = storage

        # Register with coordinator
        self.coordinator.register_agent(config.agent_id)

        # State
        self._loaded_skills: Dict[str, SkillMatch] = {}
        self._skill_cache: Dict[str, str] = {}  # skill_path -> content
        self._context_allocation: Dict[str, int] = {}

    async def optimize_context(self, task_description: str, current_context: Dict[str, Any]) -> Dict[str, Any]:
        """
        Optimize context for task execution.

        Args:
            task_description: Description of task to optimize for
            current_context: Current context state

        Returns:
            Optimized context with skill recommendations
        """
        self.coordinator.update_agent_state(self.config.agent_id, "running")

        try:
            # Step 1: Discover relevant skills
            skills = await self._discover_skills(task_description)

            # Step 2: Allocate context budget
            allocation = await self._allocate_budget(current_context, skills)

            # Step 3: Build optimized context
            optimized = await self._build_context(allocation, skills)

            self.coordinator.update_agent_state(self.config.agent_id, "complete")

            return optimized

        except Exception as e:
            self.coordinator.update_agent_state(self.config.agent_id, "failed")
            raise RuntimeError(f"Context optimization failed: {e}") from e

    async def _discover_skills(self, task_description: str) -> List[SkillMatch]:
        """
        Discover relevant skills from filesystem.

        Process:
        1. Analyze task keywords/domains
        2. Scan skills/ directory
        3. Score relevance (0-100)
        4. Return top matches
        """
        skills_dir = Path(self.config.skills_dir).expanduser()
        if not skills_dir.exists():
            return []

        matches: List[SkillMatch] = []

        # Extract keywords from task
        keywords = self._extract_keywords(task_description)

        # Scan skill files
        for skill_file in skills_dir.glob("*.md"):
            if skill_file.name.startswith("_"):
                continue  # Skip index files

            # Score relevance
            score = self._score_skill_relevance(skill_file, keywords)

            if score >= self.config.skill_relevance_threshold:
                matches.append(SkillMatch(
                    skill_path=str(skill_file),
                    relevance_score=score,
                    keywords=keywords,
                    categories=self._extract_categories(skill_file.name)
                ))

        # Sort by relevance and limit
        matches.sort(key=lambda m: m.relevance_score, reverse=True)
        top_matches = matches[:self.config.max_skills_loaded]

        # Update coordinator metrics
        self.coordinator.set_metric("skill_count", len(top_matches))

        # Cache loaded skills
        for match in top_matches:
            self._loaded_skills[match.skill_path] = match

        return top_matches

    def _extract_keywords(self, text: str) -> List[str]:
        """Extract keywords from text."""
        # Simple keyword extraction
        # Production: Use TF-IDF or similar
        words = text.lower().split()
        stopwords = {"the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for"}
        keywords = [w for w in words if w not in stopwords and len(w) > 3]
        return list(set(keywords))[:20]  # Limit to top 20

    def _score_skill_relevance(self, skill_file: Path, keywords: List[str]) -> float:
        """Score skill relevance based on keywords."""
        # Read skill file (first 500 chars for performance)
        try:
            with open(skill_file, 'r') as f:
                content = f.read(500).lower()

            # Count keyword matches
            matches = sum(1 for kw in keywords if kw in content)

            # Score: matches / total_keywords
            score = matches / len(keywords) if keywords else 0.0

            # Boost for filename match
            filename_lower = skill_file.stem.lower()
            if any(kw in filename_lower for kw in keywords):
                score += 0.2

            return min(score, 1.0)

        except Exception:
            return 0.0

    def _extract_categories(self, filename: str) -> List[str]:
        """Extract categories from skill filename."""
        # skill-category-subcategory.md -> ["category", "subcategory"]
        parts = filename.replace(".md", "").split("-")[1:]  # Skip "skill" prefix
        return parts

    async def _allocate_budget(self, current_context: Dict[str, Any], skills: List[SkillMatch]) -> Dict[str, int]:
        """
        Allocate context budget according to priority.

        Budget allocation:
        - Critical (40%): Current task, active agents, work plan
        - Skills (30%): Loaded skills
        - Project (20%): Files, memories, recent commits
        - General (10%): Session history, general context
        """
        # Get total available tokens
        total_tokens = current_context.get("available_tokens", 200000)

        allocation = {
            "critical": int(total_tokens * self.config.context_budget_critical),
            "skills": int(total_tokens * self.config.context_budget_skills),
            "project": int(total_tokens * self.config.context_budget_project),
            "general": int(total_tokens * self.config.context_budget_general)
        }

        self._context_allocation = allocation
        return allocation

    async def _build_context(self, allocation: Dict[str, int], skills: List[SkillMatch]) -> Dict[str, Any]:
        """Build optimized context within budget."""
        return {
            "allocation": allocation,
            "skills": [
                {
                    "path": s.skill_path,
                    "relevance": s.relevance_score,
                    "categories": s.categories
                }
                for s in skills
            ],
            "loaded_skill_count": len(skills),
            "total_budget": sum(allocation.values())
        }

    def get_loaded_skills(self) -> List[str]:
        """Get list of currently loaded skills."""
        return list(self._loaded_skills.keys())

    def get_context_allocation(self) -> Dict[str, int]:
        """Get current context budget allocation."""
        return dict(self._context_allocation)

    def unload_skill(self, skill_path: str):
        """Unload a skill to free context budget."""
        if skill_path in self._loaded_skills:
            del self._loaded_skills[skill_path]
            if skill_path in self._skill_cache:
                del self._skill_cache[skill_path]

    def get_status(self) -> Dict[str, Any]:
        """Get optimizer status."""
        return {
            "loaded_skills": len(self._loaded_skills),
            "cached_skills": len(self._skill_cache),
            "context_allocation": self._context_allocation
        }
