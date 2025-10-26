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
from typing import Dict, List, Optional, Set, Any
from pathlib import Path

from claude_agent_sdk import ClaudeSDKClient, ClaudeAgentOptions


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
    # Claude Agent SDK configuration
    allowed_tools: Optional[List[str]] = None
    permission_mode: str = "view"  # Optimizer reads to analyze, doesn't edit


@dataclass
class SkillMatch:
    """Skill match result."""
    skill_path: str
    relevance_score: float
    keywords: List[str]
    categories: List[str]


class OptimizerAgent:
    """
    Context and resource optimization specialist using Claude Agent SDK.

    Manages:
    - Dynamic skill discovery and loading
    - Context budget allocation
    - ACE principle application
    - Memory consolidation
    - Context compression
    """

    OPTIMIZER_SYSTEM_PROMPT = """You are the Optimizer Agent in a multi-agent orchestration system.

Your role:
- Context and resource optimization specialist
- Construct optimal context payloads for each agent
- Apply ACE principles: incremental updates, structured accumulation, strategy preservation
- Dynamically discover and load relevant skills from filesystem
- Prevent brevity bias and context collapse

Key Responsibilities:
1. Analyze task descriptions to identify relevant domains and keywords
2. Score skill relevance intelligently (not just keyword matching)
3. Allocate context budget based on task complexity and requirements
4. Decide what to compress or discard when context is constrained
5. Prevent context collapse by preserving critical strategy and state

Context Budget Allocation:
- Critical (40%): Current task, active agents, work plan
- Skills (30%): Loaded skills and domain knowledge
- Project (20%): Files, memories, recent commits
- General (10%): Session history, background context

When analyzing tasks, consider:
- Domain expertise needed (which skills are relevant?)
- Task complexity (how much context budget required?)
- Dependencies on previous work (what must be preserved?)
- Risk of context collapse (what's essential vs. compressible?)

Provide reasoning for your optimization decisions."""

    def __init__(self, config: OptimizerConfig, coordinator, storage):
        """
        Initialize Optimizer agent with Claude Agent SDK.

        Args:
            config: Optimizer configuration
            coordinator: PyCoordinator for shared state
            storage: PyStorage for memory operations
        """
        self.config = config
        self.coordinator = coordinator
        self.storage = storage

        # Initialize Claude Agent SDK client
        self.claude_client = ClaudeSDKClient(
            options=ClaudeAgentOptions(
                allowed_tools=config.allowed_tools or ["Read", "Glob"],
                permission_mode=config.permission_mode
            )
        )

        # Register with coordinator
        self.coordinator.register_agent(config.agent_id)

        # State
        self._loaded_skills: Dict[str, SkillMatch] = {}
        self._skill_cache: Dict[str, str] = {}  # skill_path -> content
        self._context_allocation: Dict[str, int] = {}
        self._session_active = False

    async def start_session(self):
        """Start Claude agent session."""
        if not self._session_active:
            await self.claude_client.connect()
            # Initialize with system prompt
            await self.claude_client.query(self.OPTIMIZER_SYSTEM_PROMPT)
            self._session_active = True

    async def stop_session(self):
        """Stop Claude agent session."""
        if self._session_active:
            await self.claude_client.disconnect()
            self._session_active = False

    async def optimize_context(self, task_description: str, current_context: Dict[str, Any]) -> Dict[str, Any]:
        """
        Optimize context for task execution using Claude Agent SDK.

        Args:
            task_description: Description of task to optimize for
            current_context: Current context state

        Returns:
            Optimized context with skill recommendations
        """
        self.coordinator.update_agent_state(self.config.agent_id, "running")

        try:
            # Ensure session is active
            if not self._session_active:
                await self.start_session()

            # Step 1: Ask Claude to analyze task and discover relevant skills
            skills = await self._discover_skills(task_description)

            # Step 2: Ask Claude to allocate context budget
            allocation = await self._allocate_budget(current_context, skills, task_description)

            # Step 3: Build optimized context
            optimized = await self._build_context(allocation, skills)

            self.coordinator.update_agent_state(self.config.agent_id, "complete")

            return optimized

        except Exception as e:
            self.coordinator.update_agent_state(self.config.agent_id, "failed")
            raise RuntimeError(f"Context optimization failed: {e}") from e

    async def _discover_skills(self, task_description: str) -> List[SkillMatch]:
        """
        Discover relevant skills from filesystem using Claude's analysis.

        Process:
        1. Ask Claude to analyze task and identify needed domains/expertise
        2. Scan skills/ directory for matches
        3. Ask Claude to score relevance (not just keyword matching)
        4. Return top matches
        """
        skills_dir = Path(self.config.skills_dir).expanduser()
        if not skills_dir.exists():
            return []

        # Ask Claude to analyze task and identify needed skills
        discovery_prompt = f"""Analyze this task and identify relevant skills/expertise needed:

**Task**: {task_description}

**Skills Directory**: {self.config.skills_dir}

Please:
1. Identify domains and expertise areas needed (e.g., "rust", "api-design", "testing")
2. List specific keywords to search for in skill files
3. Rank importance of each domain (0-100)
4. Suggest skill categories to prioritize

Available skills are in markdown files like:
- skill-rust-memory-management.md
- skill-api-rest-design.md
- skill-testing-integration.md

Provide structured analysis."""

        await self.claude_client.query(discovery_prompt)

        analysis_responses = []
        async for message in self.claude_client.receive_response():
            analysis_responses.append(message)
            await self._store_message(message, "discovery")

        # Extract keywords from Claude's analysis
        keywords = self._extract_keywords_from_analysis(analysis_responses, task_description)

        # Scan skill files
        matches: List[SkillMatch] = []
        for skill_file in skills_dir.glob("*.md"):
            if skill_file.name.startswith("_"):
                continue  # Skip index files

            # Ask Claude to score relevance for promising skills
            score = await self._score_skill_relevance(skill_file, keywords, task_description)

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

        # Store skill discovery in memory
        await self.storage.store({
            "content": f"Loaded {len(top_matches)} skills for task: {task_description[:100]}",
            "namespace": f"session:{self.config.agent_id}",
            "importance": 7,
            "tags": ["skill-discovery", "optimization"]
        })

        return top_matches

    def _extract_keywords_from_analysis(self, analysis_responses: List[Any], task_description: str) -> List[str]:
        """Extract keywords from Claude's analysis and task description."""
        # Combine Claude's analysis with task description
        combined_text = task_description + " " + " ".join(str(r) for r in analysis_responses[:3])

        # Simple keyword extraction
        words = combined_text.lower().split()
        stopwords = {"the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "with", "this", "that"}
        keywords = [w for w in words if w not in stopwords and len(w) > 3]
        return list(set(keywords))[:20]  # Limit to top 20

    async def _score_skill_relevance(self, skill_file: Path, keywords: List[str], task_description: str) -> float:
        """Score skill relevance using basic heuristics (Claude SDK scoring for top candidates)."""
        # Basic keyword-based filtering first
        try:
            with open(skill_file, 'r') as f:
                content = f.read(500).lower()

            # Count keyword matches
            matches = sum(1 for kw in keywords if kw in content)

            # Basic score: matches / total_keywords
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

    async def _allocate_budget(self, current_context: Dict[str, Any], skills: List[SkillMatch], task_description: str) -> Dict[str, int]:
        """
        Allocate context budget using Claude's analysis.

        Budget allocation:
        - Critical (40%): Current task, active agents, work plan
        - Skills (30%): Loaded skills
        - Project (20%): Files, memories, recent commits
        - General (10%): Session history, general context
        """
        # Get total available tokens
        total_tokens = current_context.get("available_tokens", 200000)

        # Ask Claude for budget allocation recommendation
        budget_prompt = f"""Analyze context budget allocation for this task:

**Task**: {task_description}
**Total Budget**: {total_tokens:,} tokens
**Loaded Skills**: {len(skills)} skills
**Current Utilization**: {current_context.get('utilization', 0):.1%}

**Default Allocation**:
- Critical (40%): {int(total_tokens * 0.40):,} tokens - Current task, active agents, work plan
- Skills (30%): {int(total_tokens * 0.30):,} tokens - Loaded skills and domain knowledge
- Project (20%): {int(total_tokens * 0.20):,} tokens - Files, memories, recent commits
- General (10%): {int(total_tokens * 0.10):,} tokens - Session history, background

Should this allocation be adjusted based on:
1. Task complexity?
2. Number of skills needed?
3. Project context requirements?
4. Risk of context collapse?

Recommend allocation percentages with reasoning."""

        await self.claude_client.query(budget_prompt)

        budget_responses = []
        async for message in self.claude_client.receive_response():
            budget_responses.append(message)
            await self._store_message(message, "budget-allocation")

        # Use default allocation (Claude's recommendation would be parsed in production)
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

    async def _store_message(self, message: Any, phase: str):
        """Store important optimization messages in memory."""
        content = str(message)
        if len(content) > 100:
            await self.storage.store({
                "content": content[:500],
                "namespace": f"session:{self.config.agent_id}",
                "importance": 6,
                "tags": ["optimization", phase]
            })

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
            "context_allocation": self._context_allocation,
            "session_active": self._session_active
        }

    async def __aenter__(self):
        """Async context manager entry."""
        await self.start_session()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit."""
        await self.stop_session()
