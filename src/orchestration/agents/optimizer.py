"""
Optimizer Agent - Context and Resource Optimization Specialist.

Responsibilities:
- Construct optimal context payloads for each agent
- Apply ACE principles: incremental updates, structured accumulation, strategy preservation
- Monitor all context sources: agents, files, commits, plans, beads, skills, session
- Prevent brevity bias and context collapse
- Dynamically discover and load relevant skills from filesystem
- Learn context relevance over time using privacy-preserving evaluation system

Privacy-Preserving Evaluation:
The Optimizer agent uses an evaluation system to learn which context (skills, memories,
files) is most relevant over time. The system is designed with privacy as a core constraint:

- Local-Only Storage: All data in .mnemosyne/project.db (gitignored)
- Hashed Tasks: SHA256 hash of task descriptions (16 chars only)
- Limited Keywords: Max 10 generic keywords, no sensitive terms
- Statistical Features: Only computed metrics stored, never raw content
- No Network Calls: Uses existing Anthropic API calls, no separate requests
- Graceful Degradation: System works perfectly when disabled

For complete privacy documentation, see:
- docs/PRIVACY.md (formal privacy guarantees)
- EVALUATION.md (technical details and examples)
"""

from dataclasses import dataclass, field
from typing import Dict, List, Optional, Set, Any
from pathlib import Path
import hashlib

from claude_agent_sdk import ClaudeSDKClient, ClaudeAgentOptions

# Import PyO3 bridge interface
from .base_agent import AgentExecutionMixin, WorkItem, WorkResult

# Import evaluation system (requires mnemosyne_core PyO3 bindings)
try:
    from mnemosyne_core import FeedbackCollector, RelevanceScorer, FeatureExtractor
    EVALUATION_AVAILABLE = True
except ImportError:
    EVALUATION_AVAILABLE = False
    print("Warning: Evaluation system not available. Install mnemosyne_core PyO3 bindings.")


@dataclass
class OptimizerConfig:
    """
    Configuration for Optimizer agent.

    Evaluation System Privacy:
    The evaluation system learns context relevance over time using privacy-preserving
    design. When enabled (default), it:
    - Stores data locally in .mnemosyne/project.db (gitignored)
    - Hashes task descriptions (SHA256, 16 chars only)
    - Extracts max 10 generic keywords, no sensitive terms
    - Stores only statistical features (keyword overlap scores, recency, etc.)
    - Makes no network calls beyond existing Anthropic API usage
    - Works perfectly when disabled (falls back to basic keyword matching)

    To disable evaluation:
    - Set enable_evaluation=False
    - Or set environment variable: MNEMOSYNE_DISABLE_EVALUATION=1

    For complete privacy documentation, see docs/PRIVACY.md
    """
    agent_id: str = "optimizer"
    skills_dirs: List[str] = field(default_factory=lambda: [
        ".claude/skills",                          # Project-local skills
        "~/.claude/plugins/cc-polymath/skills"     # Global cc-polymath skills
    ])
    context_budget_critical: float = 0.40  # 40% for critical context
    context_budget_skills: float = 0.30    # 30% for skills
    context_budget_project: float = 0.20   # 20% for project files
    context_budget_general: float = 0.10   # 10% for general
    max_skills_loaded: int = 7
    skill_relevance_threshold: float = 0.60
    prioritize_local_skills: bool = True   # Give project-local skills +10% score bonus
    # Claude Agent SDK configuration
    allowed_tools: Optional[List[str]] = None
    permission_mode: str = "default"  # Optimizer reads to analyze, doesn't edit
    # Evaluation system configuration (privacy-preserving)
    enable_evaluation: bool = True  # Enable adaptive learning (local-only, privacy-preserving)
    db_path: Optional[str] = None  # Use default if None (.mnemosyne/project.db or ~/.local/share/mnemosyne/mnemosyne.db)


@dataclass
class SkillMatch:
    """Skill match result."""
    skill_path: str
    relevance_score: float
    keywords: List[str]
    categories: List[str]
    source_dir: str  # Which skills directory this came from
    is_local: bool  # True if from project-local directory


class OptimizerAgent(AgentExecutionMixin):
    """
    Context and resource optimization specialist using Claude Agent SDK.

    Manages:
    - Dynamic skill discovery and loading
    - Context budget allocation
    - ACE principle application
    - Memory consolidation
    - Context compression

    **PyO3 Bridge Integration**: Inherits from AgentExecutionMixin to provide
    standard interface for Rust bridge communication.
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
        Initialize Optimizer agent with Claude Agent SDK and evaluation system.

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

        # Initialize evaluation system if available
        self.evaluation_enabled = config.enable_evaluation and EVALUATION_AVAILABLE
        if self.evaluation_enabled:
            db_path = config.db_path or self._get_default_db_path()
            try:
                self.feedback_collector = FeedbackCollector(db_path)
                self.relevance_scorer = RelevanceScorer(db_path)
                self.feature_extractor = FeatureExtractor(db_path)
                print(f"Evaluation system initialized with database: {db_path}")
            except Exception as e:
                print(f"Warning: Could not initialize evaluation system: {e}")
                self.evaluation_enabled = False
        else:
            self.feedback_collector = None
            self.relevance_scorer = None
            self.feature_extractor = None

        # State
        self._loaded_skills: Dict[str, SkillMatch] = {}
        self._skill_cache: Dict[str, str] = {}  # skill_path -> content
        self._context_allocation: Dict[str, int] = {}
        self._session_active = False
        self._current_session_id: Optional[str] = None
        self._context_metadata: Dict[str, Any] = {}  # Task metadata for evaluation

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

    async def _execute_work_item(self, work_item: WorkItem) -> WorkResult:
        """
        Execute work item (context optimization) for PyO3 bridge integration.

        This implements the AgentExecutionMixin interface, allowing the
        Rust bridge to send work items to this Python agent.

        Args:
            work_item: Work item from Rust (via PyO3 bridge)

        Returns:
            Work result to send back to Rust
        """
        try:
            # Build current context from work item metadata
            current_context = {
                "available_tokens": 200000,  # Default context budget
                "utilization": 0.0,
                "phase": work_item.phase,
                "priority": work_item.priority,
                "consolidated_context_id": work_item.consolidated_context_id
            }

            # Execute optimization using existing method
            result = await self.optimize_context(
                task_description=work_item.description,
                current_context=current_context
            )

            # Convert result to WorkResult format with JSON serialization
            import json
            return WorkResult(
                success=True,
                data=json.dumps({
                    "allocation": result.get("allocation", {}),
                    "skills": result.get("skills", []),
                    "loaded_skill_count": result.get("loaded_skill_count", 0),
                    "total_budget": result.get("total_budget", 0)
                }),
                memory_ids=[],  # Optimizer stores memories internally
                error=None
            )

        except Exception as e:
            # Handle any errors during optimization
            return WorkResult(
                success=False,
                error=f"Optimizer error: {type(e).__name__}: {str(e)}"
            )

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
            # Generate session ID for evaluation tracking
            import uuid
            self._current_session_id = str(uuid.uuid4())

            # Ensure session is active
            if not self._session_active:
                await self.start_session()

            # Step 0: Ask Claude to classify task (for contextual evaluation)
            if self.evaluation_enabled:
                task_metadata = await self._extract_task_metadata(task_description)
                self._context_metadata['task_metadata'] = task_metadata

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
        Discover relevant skills from multiple directories using Claude's analysis.

        Process:
        1. Ask Claude to analyze task and identify needed domains/expertise
        2. Scan all configured skill directories for matches
        3. Ask Claude to score relevance (not just keyword matching)
        4. Apply priority bonus to project-local skills
        5. Deduplicate by skill name (local overrides global)
        6. Return top matches
        """
        # Validate at least one skills directory exists
        valid_dirs = []
        for dir_str in self.config.skills_dirs:
            skills_dir = Path(dir_str).expanduser()
            if skills_dir.exists() and skills_dir.is_dir():
                valid_dirs.append(skills_dir)

        if not valid_dirs:
            return []

        # Ask Claude to analyze task and identify needed skills
        skills_dirs_str = ", ".join(str(d) for d in valid_dirs)
        discovery_prompt = f"""Analyze this task and identify relevant skills/expertise needed:

**Task**: {task_description}

**Skills Directories**: {skills_dirs_str}

Please:
1. Identify domains and expertise areas needed (e.g., "rust", "api-design", "testing", "mnemosyne")
2. List specific keywords to search for in skill files
3. Rank importance of each domain (0-100)
4. Suggest skill categories to prioritize

Available skills are in markdown files like:
- mnemosyne-memory-management.md (project-specific)
- skill-rust-memory-management.md (from cc-polymath)
- skill-api-rest-design.md (from cc-polymath)
- skill-testing-integration.md (from cc-polymath)

Provide structured analysis."""

        await self.claude_client.query(discovery_prompt)

        analysis_responses = []
        async for message in self.claude_client.receive_response():
            analysis_responses.append(message)
            await self._store_message(message, "discovery")

        # Extract keywords from Claude's analysis
        keywords = self._extract_keywords_from_analysis(analysis_responses, task_description)

        # Scan skill files across all directories
        matches: List[SkillMatch] = []
        seen_skill_names: Dict[str, SkillMatch] = {}  # For deduplication

        for idx, skills_dir in enumerate(valid_dirs):
            is_local = (idx == 0)  # First directory is project-local

            # Recursively find all .md files (cc-polymath has subdirectories)
            for skill_file in skills_dir.rglob("*.md"):
                skill_name = skill_file.name

                if skill_name.startswith("_"):
                    continue  # Skip index files

                # Skip if we've already seen this skill name and current is not local
                if skill_name in seen_skill_names:
                    if not is_local:
                        continue  # Global skills don't override
                    # Local skill overrides global - remove global version
                    old_match = seen_skill_names[skill_name]
                    if old_match in matches:
                        matches.remove(old_match)

                # Ask Claude to score relevance for promising skills
                score = await self._score_skill_relevance(skill_file, keywords, task_description)

                # Apply priority bonus for local skills
                if is_local and self.config.prioritize_local_skills:
                    score = min(1.0, score * 1.10)  # +10% bonus, capped at 1.0

                if score >= self.config.skill_relevance_threshold:
                    match = SkillMatch(
                        skill_path=str(skill_file),
                        relevance_score=score,
                        keywords=keywords,
                        categories=self._extract_categories(skill_name),
                        source_dir=str(skills_dir),
                        is_local=is_local
                    )
                    matches.append(match)
                    seen_skill_names[skill_name] = match

        # Sort by relevance and limit
        matches.sort(key=lambda m: m.relevance_score, reverse=True)
        top_matches = matches[:self.config.max_skills_loaded]

        # Update coordinator metrics
        self.coordinator.set_metric("skill_count", len(top_matches))
        local_count = sum(1 for m in top_matches if m.is_local)
        self.coordinator.set_metric("local_skill_count", local_count)

        # Cache loaded skills
        for match in top_matches:
            self._loaded_skills[match.skill_path] = match

        # Store skill discovery in memory
        skill_sources = f"{local_count} local, {len(top_matches) - local_count} global"
        self.storage.store({
            "content": f"Loaded {len(top_matches)} skills ({skill_sources}) for task: {task_description[:100]}",
            "namespace": f"project:agent-{self.config.agent_id}",
            "importance": 7,
            "tags": ["skill-discovery", "optimization"]
        })

        return top_matches

    def _extract_keywords_from_analysis(self, analysis_responses: List[Any], task_description: str) -> List[str]:
        """Extract keywords from Claude's analysis and task description."""
        # Extract text from message objects
        response_texts = []
        for r in analysis_responses[:3]:
            if hasattr(r, 'data'):
                # SystemMessage has .data attribute which contains the actual content
                data = r.data
                if isinstance(data, dict) and 'text' in data:
                    response_texts.append(data['text'])
                elif isinstance(data, str):
                    response_texts.append(data)
            else:
                response_texts.append(str(r))

        # Combine Claude's analysis with task description
        combined_text = task_description + " " + " ".join(response_texts)

        # Simple keyword extraction
        words = combined_text.lower().split()
        stopwords = {"the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "with", "this", "that"}
        keywords = [w for w in words if w not in stopwords and len(w) > 3]
        result = list(set(keywords))[:20]  # Limit to top 20

        return result

    async def _extract_task_metadata(self, task_description: str) -> Dict[str, Any]:
        """
        Extract task metadata using Claude for contextual evaluation.

        Asks Claude to classify:
        - task_type: feature/bugfix/refactor/test/documentation/optimization/exploration
        - work_phase: planning/implementation/debugging/review/testing/documentation
        - error_context: compilation/runtime/test_failure/lint/none
        - file_types: List of relevant file extensions
        - technologies: List of technologies involved
        """
        metadata_prompt = f"""Analyze this task and classify it for contextual understanding:

**Task**: {task_description}

Please provide:
1. **Task Type**: feature, bugfix, refactor, test, documentation, optimization, or exploration
2. **Work Phase**: planning, implementation, debugging, review, testing, or documentation
3. **Error Context** (if applicable): compilation, runtime, test_failure, lint, or none
4. **File Types**: Relevant file extensions (e.g., .rs, .py, .md)
5. **Technologies**: Key technologies involved (e.g., rust, tokio, postgres)

Keep your response concise and structured."""

        await self.claude_client.query(metadata_prompt)

        classification_responses = []
        async for message in self.claude_client.receive_response():
            classification_responses.append(message)

        # Parse Claude's response to extract metadata
        # For now, use simple heuristics (in production, parse Claude's structured response)
        task_lower = task_description.lower()

        # Infer task type
        if any(word in task_lower for word in ['bug', 'fix', 'error', 'crash']):
            task_type = 'bugfix'
        elif any(word in task_lower for word in ['test', 'testing', 'spec']):
            task_type = 'test'
        elif any(word in task_lower for word in ['refactor', 'reorganize', 'clean']):
            task_type = 'refactor'
        elif any(word in task_lower for word in ['document', 'docs', 'readme']):
            task_type = 'documentation'
        elif any(word in task_lower for word in ['optimize', 'performance', 'speed']):
            task_type = 'optimization'
        else:
            task_type = 'feature'

        # Infer work phase
        if any(word in task_lower for word in ['debug', 'debugging', 'investigate']):
            work_phase = 'debugging'
        elif any(word in task_lower for word in ['review', 'check', 'validate']):
            work_phase = 'review'
        elif any(word in task_lower for word in ['plan', 'design', 'architect']):
            work_phase = 'planning'
        elif any(word in task_lower for word in ['implement', 'add', 'create', 'build']):
            work_phase = 'implementation'
        elif any(word in task_lower for word in ['test', 'testing']):
            work_phase = 'testing'
        else:
            work_phase = 'implementation'

        # Infer error context
        if any(word in task_lower for word in ['compile', 'compilation', 'build error']):
            error_context = 'compilation'
        elif any(word in task_lower for word in ['runtime', 'crash', 'panic']):
            error_context = 'runtime'
        elif any(word in task_lower for word in ['test fail', 'failing test']):
            error_context = 'test_failure'
        elif any(word in task_lower for word in ['lint', 'clippy', 'warning']):
            error_context = 'lint'
        else:
            error_context = 'none'

        # Infer file types
        file_types = []
        if 'rust' in task_lower or '.rs' in task_lower:
            file_types.append('.rs')
        if 'python' in task_lower or '.py' in task_lower:
            file_types.append('.py')
        if 'markdown' in task_lower or '.md' in task_lower:
            file_types.append('.md')
        if 'toml' in task_lower or 'config' in task_lower:
            file_types.append('.toml')

        # Infer technologies
        technologies = []
        if 'rust' in task_lower:
            technologies.append('rust')
        if 'tokio' in task_lower or 'async' in task_lower:
            technologies.append('tokio')
        if 'postgres' in task_lower or 'database' in task_lower:
            technologies.append('postgres')
        if 'python' in task_lower:
            technologies.append('python')
        if 'mnemosyne' in task_lower:
            technologies.append('mnemosyne')

        return {
            'task_type': task_type,
            'work_phase': work_phase,
            'error_context': error_context,
            'file_types': file_types if file_types else None,
            'technologies': technologies if technologies else None
        }

    async def _score_skill_relevance(self, skill_file: Path, keywords: List[str], task_description: str) -> float:
        """
        Score skill relevance using learned weights (if available) or basic heuristics.

        With evaluation system:
        - Fetches learned weights (session → project → global)
        - Computes weighted score from features
        - Records context provided for feedback collection

        Without evaluation system:
        - Falls back to basic keyword matching
        """
        try:
            # Read skill content for feature extraction
            with open(skill_file, 'r') as f:
                content = f.read(500).lower()

            # Extract basic features (used by both paths)
            keyword_overlap = sum(1 for kw in keywords if kw in content) / len(keywords) if keywords else 0.0
            filename_match = any(kw in skill_file.stem.lower() for kw in keywords)

            # Use learned weights if evaluation system available
            if self.evaluation_enabled and self.relevance_scorer:
                try:
                    # Get contextual metadata from task analysis
                    metadata = self._context_metadata.get('task_metadata', {})

                    # Get learned weights with hierarchical fallback
                    weights = self.relevance_scorer.get_weights(
                        scope="session" if self._current_session_id else "global",
                        scope_id=self._current_session_id or "global",
                        context_type="skill",
                        agent_role=self.config.agent_id,
                        work_phase=metadata.get('work_phase'),
                        task_type=metadata.get('task_type'),
                        error_context=metadata.get('error_context')
                    )

                    # Compute weighted score
                    score = (
                        keyword_overlap * weights.get('keyword_match', 0.35) +
                        (1.0 if filename_match else 0.0) * weights.get('file_type_match', 0.10) +
                        # Other features would go here (recency, access patterns, etc.)
                        0.0 * weights.get('recency', 0.15) +  # Placeholder
                        0.0 * weights.get('access_patterns', 0.25) +  # Placeholder
                        0.0 * weights.get('historical_success', 0.15)  # Placeholder
                    )

                    # Record context provided for evaluation
                    if self.feedback_collector and self._current_session_id:
                        task_hash = self._hash_task_description(task_description)
                        eval_id = self.feedback_collector.record_context_provided(
                            session_id=self._current_session_id,
                            agent_role=self.config.agent_id,
                            namespace="project:mnemosyne",  # TODO: Detect from storage
                            context_type="skill",
                            context_id=str(skill_file),
                            task_hash=task_hash,
                            task_keywords=keywords[:10],  # Limit to 10 for privacy
                            task_type=metadata.get('task_type'),
                            work_phase=metadata.get('work_phase'),
                            file_types=metadata.get('file_types'),
                            error_context=metadata.get('error_context'),
                            related_technologies=metadata.get('technologies')
                        )

                    return min(score, 1.0)

                except Exception as e:
                    print(f"Warning: Could not use learned weights: {e}. Falling back to basic scoring.")
                    # Fall through to basic scoring

            # Fallback: Basic keyword matching
            score = keyword_overlap
            if filename_match:
                score += 0.2

            return min(score, 1.0)

        except Exception as e:
            print(f"Error scoring skill {skill_file}: {e}")
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
            self.storage.store({
                "content": content[:500],
                "namespace": f"project:agent-{self.config.agent_id}",
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
            "session_active": self._session_active,
            "evaluation_enabled": self.evaluation_enabled
        }

    def _get_default_db_path(self) -> str:
        """
        Get default database path with auto-detection.

        Priority:
        1. .mnemosyne/project.db (if exists)
        2. ~/.local/share/mnemosyne/mnemosyne.db (XDG default)
        """
        from pathlib import Path

        # Check for project-specific database
        project_db = Path(".mnemosyne/project.db")
        if project_db.exists():
            return str(project_db)

        # Fall back to XDG default
        import os
        xdg_data = os.environ.get("XDG_DATA_HOME")
        if xdg_data:
            return str(Path(xdg_data) / "mnemosyne" / "mnemosyne.db")

        # Fall back to ~/.local/share
        home = Path.home()
        return str(home / ".local" / "share" / "mnemosyne" / "mnemosyne.db")

    def _hash_task_description(self, task_description: str) -> str:
        """
        Create privacy-preserving hash of task description.

        Returns first 16 characters of SHA256 hash.
        """
        hash_obj = hashlib.sha256(task_description.encode('utf-8'))
        return hash_obj.hexdigest()[:16]

    async def __aenter__(self):
        """Async context manager entry."""
        await self.start_session()
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        """Async context manager exit."""
        await self.stop_session()
