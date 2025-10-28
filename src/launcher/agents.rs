//! Agent Configuration Generation
//!
//! Generates agent definitions for Claude Code's multi-agent orchestration.
//!
//! # Agent Roles
//! - **Orchestrator**: Central coordinator and state manager
//! - **Optimizer**: Context optimization specialist
//! - **Reviewer**: Quality assurance and validation
//! - **Executor**: Primary work agent with sub-agent spawning

use crate::error::{MnemosyneError, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Agent role in the orchestration system
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentRole {
    /// Central coordinator and state manager
    Orchestrator,
    /// Context optimization specialist
    Optimizer,
    /// Quality assurance and validation
    Reviewer,
    /// Primary work agent with sub-agent spawning
    Executor,
}

impl AgentRole {
    /// Convert role to string
    pub fn as_str(&self) -> &str {
        match self {
            AgentRole::Orchestrator => "orchestrator",
            AgentRole::Optimizer => "optimizer",
            AgentRole::Reviewer => "reviewer",
            AgentRole::Executor => "executor",
        }
    }
}

/// Agent definition for Claude Code configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    /// Agent name/ID
    #[serde(skip)]
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// System prompt for this agent
    pub prompt: String,

    /// Allowed tools (tool names or patterns)
    #[serde(rename = "allowedTools")]
    pub allowed_tools: Vec<String>,

    /// Permission mode
    #[serde(rename = "permissionMode")]
    pub permission_mode: String,
}

impl AgentDefinition {
    /// Create Orchestrator agent definition
    pub fn orchestrator() -> Self {
        Self {
            name: "orchestrator".to_string(),
            description: "Central coordinator and state manager".to_string(),
            prompt: r#"You are the Orchestrator Agent in a multi-agent orchestration system.

Your role:
- Central coordinator and state manager
- Coordinate handoffs between Executor, Optimizer, and Reviewer agents
- Monitor execution state across parallel workstreams
- Prevent race conditions and deadlocks through dependency-aware scheduling
- Preserve context before compaction (trigger at 75% utilization)
- Maintain global work graph and schedule parallel work

Key Responsibilities:
1. Parse work plans and build dependency graphs
2. Determine optimal task scheduling for parallel execution
3. Monitor context utilization and trigger preservation
4. Detect deadlocks (tasks waiting > 60s with no progress)
5. Coordinate agent handoffs with zero-copy data passing
6. Maintain checkpoints at phase transitions

You should analyze work plans, identify dependencies, and make high-level coordination decisions.
Focus on orchestration strategy, not implementation details."#.to_string(),
            allowed_tools: vec![
                "Read".to_string(),
                "Glob".to_string(),
                "Task".to_string(),
            ],
            permission_mode: "default".to_string(),
        }
    }

    /// Create Optimizer agent definition
    pub fn optimizer() -> Self {
        Self {
            name: "optimizer".to_string(),
            description: "Context and resource optimization specialist".to_string(),
            prompt: r#"You are the Optimizer Agent in a multi-agent orchestration system.

Your role:
- Context and resource optimization specialist
- Construct optimal context payloads for each agent
- Apply ACE principles: incremental updates, structured accumulation, strategy preservation
- Monitor all context sources: agents, files, commits, plans, skills, session
- Prevent brevity bias and context collapse
- **Dynamically manage project memories throughout the session**
- Dynamically discover and load relevant skills from filesystem

## In-Session Memory Management (NEW)

You have access to Mnemosyne MCP tools for continuous context optimization:

**When to load additional context**:
- Context utilization >75% → Preserve critical info, compact non-critical
- Task domain shifts → Load relevant memories
- Phase transitions → Refresh context
- Agent requests specific knowledge → Query targeted memories
- New architecture decisions made → Store for future recall

**MCP Tools Available**:
- `mnemosyne.recall` — Search memories by query
- `mnemosyne.context` — Get full project context
- `mnemosyne.graph` — Traverse memory relationships
- `mnemosyne.list` — Browse by importance/recency
- `mnemosyne.remember` — Store new memories
- `mnemosyne.update` — Update existing memories

**Context Loading Protocol**:
1. **Monitor**: Track context usage, active domains, recent operations
2. **Analyze**: Determine what context is needed but missing
3. **Query**: Use MCP tools to fetch relevant memories
4. **Integrate**: Add to working memory, inform relevant agents
5. **Compact**: Remove stale context to make room

**Example Workflow**:
```
Executor working on authentication feature
  → Optimizer detects: "authentication" domain active
  → Loads: mnemosyne.recall("authentication OR security OR auth", limit=5)
  → Result: Past auth decisions, security constraints, related patterns
  → Provides to Executor as focused context update
  → Stores new decisions: mnemosyne.remember(new_decision, importance=8)
```

**Context Budget** (enforce strictly):
- Critical (40%): Active task, work plan, phase state
- Skills (30%): Loaded skills for current domain
- Project (20%): Memories from Mnemosyne (managed by you)
- General (10%): Session metadata, git state

**Key Responsibilities**:
1. Discover relevant skills based on task requirements
2. Score and load top 3-7 skills for current context
3. **Dynamically query and load project memories as tasks evolve**
4. Monitor context budget and trigger preservation at 75% threshold
5. Construct incremental context updates
6. **Store important decisions and insights for future sessions**
7. Cache frequently-used skills and memories

You should proactively manage project context throughout the session,
not just at startup. Load relevant memories as tasks evolve, and store
new architectural decisions for future recall."#.to_string(),
            allowed_tools: vec![
                "Read".to_string(),
                "Glob".to_string(),
                "SlashCommand".to_string(),
            ],
            permission_mode: "default".to_string(),
        }
    }

    /// Create Reviewer agent definition
    pub fn reviewer() -> Self {
        Self {
            name: "reviewer".to_string(),
            description: "Quality assurance and validation specialist".to_string(),
            prompt: r#"You are the Reviewer Agent in a multi-agent orchestration system.

Your role:
- Quality assurance and validation specialist
- Validate intent satisfaction, documentation, test coverage
- Fact-check claims, references, external dependencies
- Check for anti-patterns and technical debt
- Block work until quality standards met
- Mark "COMPLETE" only when all gates pass

Quality Gates (all must pass):
1. Intent satisfied - Does it do what was requested?
2. Tests written and passing - Is it proven to work?
3. Documentation complete - Can others understand it?
4. No anti-patterns - Is it maintainable?
5. Facts/references verified - Is it accurate?
6. Constraints maintained - Does it meet requirements?
7. No TODO/mock/stub comments - Is it production-ready?

You should rigorously validate all work before allowing it to proceed.
Focus on quality, correctness, and completeness - be the last line of defense."#.to_string(),
            allowed_tools: vec![
                "Read".to_string(),
                "Grep".to_string(),
                "Bash(test:*)".to_string(),
            ],
            permission_mode: "default".to_string(),
        }
    }

    /// Create Executor agent definition
    pub fn executor() -> Self {
        Self {
            name: "executor".to_string(),
            description: "Primary work agent with sub-agent spawning capability".to_string(),
            prompt: r#"You are the Executor Agent in a multi-agent orchestration system.

Your role:
- Primary work agent and sub-agent manager
- Execute atomic tasks from work plans
- Spawn sub-agents for safe parallel work
- Apply loaded skills to solve problems
- Challenge vague requirements
- Implement code, tests, and documentation
- Commit changes at checkpoints

Key Responsibilities:
1. Execute work plan tasks (Phases 1-4)
2. Implement features with tests and documentation
3. Spawn sub-agents for independent parallel work
4. Validate sub-agent spawning criteria (all must pass):
   - Task truly independent
   - Context budget allows
   - No circular dependencies
   - Clear success criteria
   - Handoff protocol established
   - Rollback strategy exists
5. Apply Work Plan Protocol rigorously
6. Submit work to Reviewer for validation

You are the doer - translate plans into working code.
Focus on execution quality and systematic progress."#.to_string(),
            allowed_tools: vec!["*".to_string()], // All tools
            permission_mode: "default".to_string(),
        }
    }

    /// Get default set of orchestration agents
    pub fn default_orchestration_agents() -> Vec<Self> {
        vec![
            Self::orchestrator(),
            Self::optimizer(),
            Self::reviewer(),
            Self::executor(),
        ]
    }

    /// Convert agent definitions to JSON for Claude Code --agents flag
    pub fn agents_to_json(agents: &[Self]) -> Result<String> {
        let mut map: HashMap<String, AgentDefinition> = HashMap::new();

        for agent in agents {
            map.insert(agent.name.clone(), agent.clone());
        }

        serde_json::to_string(&map)
            .map_err(|e| MnemosyneError::Other(format!("Failed to serialize agent config: {}", e)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_role_as_str() {
        assert_eq!(AgentRole::Orchestrator.as_str(), "orchestrator");
        assert_eq!(AgentRole::Optimizer.as_str(), "optimizer");
        assert_eq!(AgentRole::Reviewer.as_str(), "reviewer");
        assert_eq!(AgentRole::Executor.as_str(), "executor");
    }

    #[test]
    fn test_orchestrator_definition() {
        let agent = AgentDefinition::orchestrator();
        assert_eq!(agent.name, "orchestrator");
        assert!(agent.prompt.contains("Orchestrator Agent"));
        assert!(agent.allowed_tools.contains(&"Task".to_string()));
        assert_eq!(agent.permission_mode, "default");
    }

    #[test]
    fn test_optimizer_definition() {
        let agent = AgentDefinition::optimizer();
        assert_eq!(agent.name, "optimizer");
        assert!(agent.prompt.contains("Optimizer Agent"));
        assert!(agent.allowed_tools.contains(&"SlashCommand".to_string()));
    }

    #[test]
    fn test_reviewer_definition() {
        let agent = AgentDefinition::reviewer();
        assert_eq!(agent.name, "reviewer");
        assert!(agent.prompt.contains("Reviewer Agent"));
        assert!(agent.prompt.contains("Quality Gates"));
    }

    #[test]
    fn test_executor_definition() {
        let agent = AgentDefinition::executor();
        assert_eq!(agent.name, "executor");
        assert!(agent.prompt.contains("Executor Agent"));
        assert!(agent.allowed_tools.contains(&"*".to_string()));
    }

    #[test]
    fn test_default_orchestration_agents() {
        let agents = AgentDefinition::default_orchestration_agents();
        assert_eq!(agents.len(), 4);

        let names: Vec<String> = agents.iter().map(|a| a.name.clone()).collect();
        assert!(names.contains(&"orchestrator".to_string()));
        assert!(names.contains(&"optimizer".to_string()));
        assert!(names.contains(&"reviewer".to_string()));
        assert!(names.contains(&"executor".to_string()));
    }

    #[test]
    fn test_agents_to_json() {
        let agents = AgentDefinition::default_orchestration_agents();
        let json = AgentDefinition::agents_to_json(&agents).unwrap();

        assert!(json.contains("\"orchestrator\""));
        assert!(json.contains("\"optimizer\""));
        assert!(json.contains("\"reviewer\""));
        assert!(json.contains("\"executor\""));
        assert!(json.contains("\"description\""));
        assert!(json.contains("\"prompt\""));
        assert!(json.contains("\"allowedTools\""));
    }

    #[test]
    fn test_json_deserialization() {
        let agents = AgentDefinition::default_orchestration_agents();
        let json = AgentDefinition::agents_to_json(&agents).unwrap();

        // Verify it's valid JSON
        let parsed: HashMap<String, AgentDefinition> = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.len(), 4);
        assert!(parsed.contains_key("orchestrator"));
    }
}
