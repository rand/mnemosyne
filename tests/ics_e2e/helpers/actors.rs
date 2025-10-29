//! Mock agent actors for E2E testing
//!
//! Simulates agent behavior including:
//! - Analyzing documents
//! - Creating proposals
//! - Updating status
//! - Collaborative editing
//! - Error scenarios

use mnemosyne_core::ics::*;
use std::sync::Arc;
use std::time::SystemTime;

/// Mock agent for testing
#[derive(Clone)]
pub struct MockAgent {
    /// Agent information
    pub info: AgentInfo,
    /// Whether agent is enabled
    enabled: bool,
    /// Proposal generator function
    proposal_fn: Option<Arc<dyn Fn(&str) -> Vec<ChangeProposal> + Send + Sync>>,
}

impl std::fmt::Debug for MockAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockAgent")
            .field("info", &self.info)
            .field("enabled", &self.enabled)
            .field("proposal_fn", &self.proposal_fn.is_some())
            .finish()
    }
}

impl MockAgent {
    /// Create new mock agent
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            info: AgentInfo {
                id: id.into(),
                name: name.into(),
                activity: AgentActivity::Idle,
                last_active: SystemTime::now(),
                message: None,
            },
            enabled: true,
            proposal_fn: None,
        }
    }

    /// Create Orchestrator mock
    pub fn orchestrator() -> Self {
        let mut agent = Self::new("orchestrator", "Orchestrator");
        agent.proposal_fn = Some(Arc::new(|text: &str| {
            // Orchestrator looks for high-level structure issues
            if !text.contains("# ") {
                vec![ChangeProposal {
                    id: "orch-1".to_string(),
                    agent: "agent:orchestrator".to_string(),
                    description: "Add document title".to_string(),
                    original: text[..20.min(text.len())].to_string(),
                    proposed: format!("# Document Title\n\n{}", &text[..20.min(text.len())]),
                    line_range: (0, 0),
                    created_at: SystemTime::now(),
                    status: ProposalStatus::Pending,
                    rationale: "Document should have a title for structure".to_string(),
                }]
            } else {
                Vec::new()
            }
        }));
        agent
    }

    /// Create Optimizer mock
    pub fn optimizer() -> Self {
        let mut agent = Self::new("optimizer", "Optimizer");
        agent.proposal_fn = Some(Arc::new(|text: &str| {
            // Optimizer looks for optimization opportunities
            let mut proposals = Vec::new();
            if text.contains("However") || text.contains("but") {
                proposals.push(ChangeProposal {
                    id: "opt-1".to_string(),
                    agent: "agent:optimizer".to_string(),
                    description: "Resolve contradiction".to_string(),
                    original: "However, ...".to_string(),
                    proposed: "Clarified statement without contradiction".to_string(),
                    line_range: (0, 0),
                    created_at: SystemTime::now(),
                    status: ProposalStatus::Pending,
                    rationale: "Detected potential contradiction".to_string(),
                });
            }
            proposals
        }));
        agent
    }

    /// Create Reviewer mock
    pub fn reviewer() -> Self {
        let mut agent = Self::new("reviewer", "Reviewer");
        agent.proposal_fn = Some(Arc::new(|text: &str| {
            // Reviewer looks for quality issues
            let mut proposals = Vec::new();
            if text.contains("TODO") || text.contains("FIXME") {
                proposals.push(ChangeProposal {
                    id: "rev-1".to_string(),
                    agent: "agent:reviewer".to_string(),
                    description: "Complete TODO item".to_string(),
                    original: "TODO: ...".to_string(),
                    proposed: "Completed implementation".to_string(),
                    line_range: (0, 0),
                    created_at: SystemTime::now(),
                    status: ProposalStatus::Pending,
                    rationale: "TODO markers should be resolved".to_string(),
                });
            }
            proposals
        }));
        agent
    }

    /// Create Executor mock
    pub fn executor() -> Self {
        let mut agent = Self::new("executor", "Executor");
        agent.proposal_fn = Some(Arc::new(|text: &str| {
            // Executor implements concrete changes
            let mut proposals = Vec::new();
            if text.contains("@undefined") || text.contains("#missing") {
                proposals.push(ChangeProposal {
                    id: "exec-1".to_string(),
                    agent: "agent:executor".to_string(),
                    description: "Define undefined reference".to_string(),
                    original: "@undefined".to_string(),
                    proposed: "@defined: implementation added".to_string(),
                    line_range: (0, 0),
                    created_at: SystemTime::now(),
                    status: ProposalStatus::Pending,
                    rationale: "Undefined reference needs definition".to_string(),
                });
            }
            proposals
        }));
        agent
    }

    /// Create SubAgent mock
    pub fn sub_agent(id: impl Into<String>) -> Self {
        Self::new(id, "SubAgent")
    }

    /// Set agent activity
    pub fn set_activity(&mut self, activity: AgentActivity, message: Option<String>) {
        self.info.activity = activity;
        self.info.message = message;
        self.info.last_active = SystemTime::now();
    }

    /// Generate proposals for given text
    pub fn propose(&self, text: &str) -> Vec<ChangeProposal> {
        if !self.enabled {
            return Vec::new();
        }

        if let Some(ref proposal_fn) = self.proposal_fn {
            self.set_activity_mut(AgentActivity::Proposing, Some("Analyzing text".to_string()));
            let proposals = proposal_fn(text);
            self.set_activity_mut(AgentActivity::Waiting, Some("Waiting for review".to_string()));
            proposals
        } else {
            Vec::new()
        }
    }

    /// Set activity (mutable version for internal use)
    fn set_activity_mut(&self, _activity: AgentActivity, _message: Option<String>) {
        // In real implementation, would use interior mutability (RefCell, Mutex, etc.)
        // For testing, we'll accept the limitation
    }

    /// Enable agent
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable agent
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Simulate agent error
    pub fn trigger_error(&mut self, error_msg: impl Into<String>) {
        self.set_activity(AgentActivity::Error(error_msg.into()), None);
    }
}

/// Create standard set of mock agents for testing
pub fn create_mock_agents() -> Vec<MockAgent> {
    vec![
        MockAgent::orchestrator(),
        MockAgent::optimizer(),
        MockAgent::reviewer(),
        MockAgent::executor(),
    ]
}

/// Create agent with custom proposal function
pub fn create_custom_agent(
    id: impl Into<String>,
    name: impl Into<String>,
    proposal_fn: impl Fn(&str) -> Vec<ChangeProposal> + Send + Sync + 'static,
) -> MockAgent {
    let mut agent = MockAgent::new(id, name);
    agent.proposal_fn = Some(Arc::new(proposal_fn));
    agent
}

/// Simulate concurrent agent activity
pub async fn simulate_concurrent_agents(
    agents: &mut [MockAgent],
    text: &str,
) -> Vec<Vec<ChangeProposal>> {
    let mut all_proposals = Vec::new();

    for agent in agents.iter() {
        // Simulate concurrent processing
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        let proposals = agent.propose(text);
        all_proposals.push(proposals);
    }

    all_proposals
}

/// Simulate agent coordination pattern
pub async fn simulate_coordination(
    orchestrator: &MockAgent,
    sub_agents: &[MockAgent],
    text: &str,
) -> (Vec<ChangeProposal>, Vec<Vec<ChangeProposal>>) {
    // Orchestrator analyzes first
    let orch_proposals = orchestrator.propose(text);

    // Sub-agents work in parallel
    let mut sub_proposals = Vec::new();
    for agent in sub_agents {
        tokio::time::sleep(tokio::time::Duration::from_millis(5)).await;
        sub_proposals.push(agent.propose(text));
    }

    (orch_proposals, sub_proposals)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_agent_creation() {
        let agent = MockAgent::new("test", "TestAgent");
        assert_eq!(agent.info.id, "test");
        assert_eq!(agent.info.name, "TestAgent");
        assert!(matches!(agent.info.activity, AgentActivity::Idle));
    }

    #[test]
    fn test_standard_agents() {
        let agents = create_mock_agents();
        assert_eq!(agents.len(), 4);
        assert_eq!(agents[0].info.name, "Orchestrator");
        assert_eq!(agents[1].info.name, "Optimizer");
        assert_eq!(agents[2].info.name, "Reviewer");
        assert_eq!(agents[3].info.name, "Executor");
    }

    #[test]
    fn test_orchestrator_proposals() {
        let agent = MockAgent::orchestrator();
        let text = "Some text without title";
        let proposals = agent.propose(text);
        assert!(!proposals.is_empty());
        assert_eq!(proposals[0].description, "Add document title");
    }

    #[test]
    fn test_optimizer_proposals() {
        let agent = MockAgent::optimizer();
        let text = "The system is fast. However, it's slow.";
        let proposals = agent.propose(text);
        assert!(!proposals.is_empty());
        assert!(proposals[0].description.contains("contradiction"));
    }

    #[test]
    fn test_reviewer_proposals() {
        let agent = MockAgent::reviewer();
        let text = "TODO: implement feature";
        let proposals = agent.propose(text);
        assert!(!proposals.is_empty());
        assert!(proposals[0].description.contains("TODO"));
    }
}
