//! End-to-End Tests for Multi-Agent Coordination
//!
//! Comprehensive integration tests covering:
//! - Multiple agents coordinating on same branch
//! - Isolation mode enforcement
//! - Read-only auto-approval
//! - Orchestrator bypass
//! - Conflict detection and notification
//! - Cross-process coordination
//! - Timeout handling

#[cfg(test)]
mod e2e_tests {
    use crate::launcher::agents::AgentRole;
    use crate::orchestration::branch_coordinator::{
        BranchCoordinator, BranchCoordinatorConfig, JoinRequest, JoinResponse,
    };
    use crate::orchestration::branch_guard::BranchGuard;
    use crate::orchestration::branch_registry::{BranchRegistry, CoordinationMode, WorkIntent};
    use crate::orchestration::conflict_detector::ConflictDetector;
    use crate::orchestration::conflict_notifier::{ConflictNotifier, NotificationConfig};
    use crate::orchestration::file_tracker::{FileTracker, ModificationType};

    use crate::orchestration::git_wrapper::GitWrapper;
    use crate::orchestration::identity::{AgentId, AgentIdentity};
    use crate::types::Namespace;
    use chrono::Utc;
    use std::path::PathBuf;
    use std::sync::{Arc, RwLock};

    /// Setup helper for creating a fully-configured branch coordinator
    async fn setup_test_coordinator() -> BranchCoordinator {
        let registry = Arc::new(RwLock::new(BranchRegistry::new()));

        let guard = Arc::new(BranchGuard::new(registry.clone(), PathBuf::from(".")));

        let conflict_detector = Arc::new(ConflictDetector::new());
        let file_tracker = Arc::new(FileTracker::new(conflict_detector));

        let notifier_config = NotificationConfig {
            enabled: true,
            notify_on_save: true,
            periodic_interval_minutes: 20,
            session_end_summary: true,
        };

        let notifier = Arc::new(ConflictNotifier::new(notifier_config, file_tracker));

        let git_wrapper = Arc::new(GitWrapper::new(registry.clone(), PathBuf::from(".")));

        let config = BranchCoordinatorConfig {
            enable_cross_process: false, // Disabled for unit tests
            auto_approve_readonly: true,
            default_mode: CoordinationMode::Isolated,
            mnemosyne_dir: None,
        };

        BranchCoordinator::new(registry, guard, notifier, git_wrapper, config).unwrap()
    }

    /// Helper to create test agent identity
    fn create_agent(role: AgentRole, is_coordinator: bool) -> AgentIdentity {
        AgentIdentity {
            id: AgentId::new(),
            role,
            namespace: Namespace::Global,
            branch: "main".to_string(),
            working_dir: PathBuf::from("."),
            spawned_at: Utc::now(),
            parent_id: None,
            is_coordinator,
        }
    }

    #[tokio::test]
    async fn test_readonly_auto_approval() {
        let coordinator = setup_test_coordinator().await;

        let agent = create_agent(AgentRole::Executor, false);

        let request = JoinRequest {
            agent_identity: agent,
            target_branch: "main".to_string(),
            intent: WorkIntent::ReadOnly,
            mode: CoordinationMode::Coordinated,
            work_items: vec![],
        };

        let response = coordinator.handle_join_request(request).await.unwrap();

        match response {
            JoinResponse::Approved { message, .. } => {
                assert!(message.contains("auto-approved"));
            }
            _ => panic!("Expected Approved response for read-only access"),
        }
    }

    #[tokio::test]
    async fn test_orchestrator_bypass() {
        let coordinator = setup_test_coordinator().await;

        let orchestrator = create_agent(AgentRole::Orchestrator, true);

        let request = JoinRequest {
            agent_identity: orchestrator,
            target_branch: "main".to_string(),
            intent: WorkIntent::FullBranch,
            mode: CoordinationMode::Isolated,
            work_items: vec![],
        };

        let response = coordinator.handle_join_request(request).await.unwrap();

        match response {
            JoinResponse::Approved { .. } => {
                // Success - orchestrator bypassed isolation rules
            }
            _ => panic!("Expected Approved response for orchestrator"),
        }
    }

    #[tokio::test]
    async fn test_coordinated_mode_multiple_agents() {
        let coordinator = setup_test_coordinator().await;

        let agent1 = create_agent(AgentRole::Executor, false);
        let agent2 = create_agent(AgentRole::Executor, false);

        // First agent joins
        let request1 = JoinRequest {
            agent_identity: agent1,
            target_branch: "feature/test".to_string(),
            intent: WorkIntent::Write(vec![PathBuf::from("src/module_a.rs")]),
            mode: CoordinationMode::Coordinated,
            work_items: vec![],
        };

        let response1 = coordinator.handle_join_request(request1).await.unwrap();

        match response1 {
            JoinResponse::Approved { .. } => {
                // First agent approved
            }
            _ => panic!("Expected Approved for first agent"),
        }

        // Second agent joins same branch (coordinated mode)
        let request2 = JoinRequest {
            agent_identity: agent2,
            target_branch: "feature/test".to_string(),
            intent: WorkIntent::Write(vec![PathBuf::from("src/module_b.rs")]),
            mode: CoordinationMode::Coordinated,
            work_items: vec![],
        };

        let response2 = coordinator.handle_join_request(request2).await.unwrap();

        match response2 {
            JoinResponse::RequiresCoordination {
                other_agents,
                message,
                ..
            } => {
                assert_eq!(other_agents.len(), 1);
                assert!(message.contains("Coordination required"));
            }
            _ => panic!("Expected RequiresCoordination for second agent"),
        }
    }

    #[tokio::test]
    async fn test_isolated_mode_blocks_second_agent() {
        let coordinator = setup_test_coordinator().await;

        let agent1 = create_agent(AgentRole::Executor, false);
        let agent2 = create_agent(AgentRole::Executor, false);

        // First agent joins in isolated mode
        let request1 = JoinRequest {
            agent_identity: agent1.clone(),
            target_branch: "feature/isolated".to_string(),
            intent: WorkIntent::FullBranch,
            mode: CoordinationMode::Isolated,
            work_items: vec![],
        };

        let response1 = coordinator.handle_join_request(request1).await.unwrap();

        match response1 {
            JoinResponse::Approved { .. } => {
                // First agent approved
            }
            _ => panic!("Expected Approved for first agent"),
        }

        // Second agent tries to join (should be denied due to isolation)
        let request2 = JoinRequest {
            agent_identity: agent2,
            target_branch: "feature/isolated".to_string(),
            intent: WorkIntent::FullBranch,
            mode: CoordinationMode::Isolated,
            work_items: vec![],
        };

        let response2 = coordinator.handle_join_request(request2).await.unwrap();

        match response2 {
            JoinResponse::Denied {
                reason,
                suggestions,
            } => {
                assert!(reason.contains("conflict") || reason.contains("assigned"));
                assert!(!suggestions.is_empty());
            }
            _ => panic!("Expected Denied for second agent in isolated mode"),
        }
    }

    #[tokio::test]
    async fn test_conflict_detection_same_file() {
        let conflict_detector = Arc::new(ConflictDetector::new());
        let file_tracker = Arc::new(FileTracker::new(conflict_detector));

        let agent1 = AgentId::new();
        let agent2 = AgentId::new();
        let path = PathBuf::from("src/main.rs");

        // Agent1 modifies file
        file_tracker
            .record_modification(&agent1, &path, ModificationType::Modified)
            .unwrap();

        // Agent2 modifies same file (conflict!)
        let conflicts = file_tracker
            .record_modification(&agent2, &path, ModificationType::Modified)
            .unwrap();

        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].agents.len(), 2);
    }

    #[tokio::test]
    async fn test_conflict_resolution_after_agent_release() {
        let conflict_detector = Arc::new(ConflictDetector::new());
        let file_tracker = Arc::new(FileTracker::new(conflict_detector));

        let agent1 = AgentId::new();
        let agent2 = AgentId::new();
        let path = PathBuf::from("src/main.rs");

        // Both agents modify same file
        file_tracker
            .record_modification(&agent1, &path, ModificationType::Modified)
            .unwrap();
        file_tracker
            .record_modification(&agent2, &path, ModificationType::Modified)
            .unwrap();

        let conflicts = file_tracker.get_active_conflicts().unwrap();
        assert_eq!(conflicts.len(), 1);

        // Agent1 completes work and releases
        file_tracker.clear_agent_files(&agent1).unwrap();

        // Conflict should be resolved
        let conflicts_after = file_tracker.get_active_conflicts().unwrap();
        assert_eq!(conflicts_after.len(), 0);
    }

    #[tokio::test]
    async fn test_release_assignment() {
        let coordinator = setup_test_coordinator().await;

        let agent = create_agent(AgentRole::Executor, false);
        let agent_id = agent.id.clone();

        // Agent joins
        let request = JoinRequest {
            agent_identity: agent,
            target_branch: "main".to_string(),
            intent: WorkIntent::FullBranch,
            mode: CoordinationMode::Isolated,
            work_items: vec![],
        };

        coordinator.handle_join_request(request).await.unwrap();

        // Verify assignment exists
        let assignments = coordinator.get_branch_assignments("main").await.unwrap();
        assert_eq!(assignments.len(), 1);

        // Release assignment
        coordinator.release_assignment(&agent_id).await.unwrap();

        // Verify assignment removed
        let assignments_after = coordinator.get_branch_assignments("main").await.unwrap();
        assert_eq!(assignments_after.len(), 0);
    }

    #[tokio::test]
    async fn test_multiple_branches_independent() {
        let coordinator = setup_test_coordinator().await;

        let agent1 = create_agent(AgentRole::Executor, false);
        let agent2 = create_agent(AgentRole::Executor, false);

        // Agent1 on branch A
        let request1 = JoinRequest {
            agent_identity: agent1,
            target_branch: "feature/a".to_string(),
            intent: WorkIntent::FullBranch,
            mode: CoordinationMode::Isolated,
            work_items: vec![],
        };

        let response1 = coordinator.handle_join_request(request1).await.unwrap();

        match response1 {
            JoinResponse::Approved { .. } => {
                // Success
            }
            _ => panic!("Expected Approved for agent1 on branch A"),
        }

        // Agent2 on branch B (should not conflict)
        let request2 = JoinRequest {
            agent_identity: agent2,
            target_branch: "feature/b".to_string(),
            intent: WorkIntent::FullBranch,
            mode: CoordinationMode::Isolated,
            work_items: vec![],
        };

        let response2 = coordinator.handle_join_request(request2).await.unwrap();

        match response2 {
            JoinResponse::Approved { .. } => {
                // Success - different branches are independent
            }
            _ => panic!("Expected Approved for agent2 on branch B"),
        }
    }

    #[tokio::test]
    async fn test_suggestions_for_denied_access() {
        let coordinator = setup_test_coordinator().await;

        let agent1 = create_agent(AgentRole::Executor, false);
        let agent2 = create_agent(AgentRole::Executor, false);

        // Agent1 takes isolated branch
        let request1 = JoinRequest {
            agent_identity: agent1,
            target_branch: "main".to_string(),
            intent: WorkIntent::FullBranch,
            mode: CoordinationMode::Isolated,
            work_items: vec![],
        };

        coordinator.handle_join_request(request1).await.unwrap();

        // Agent2 tries to join (will be denied)
        let request2 = JoinRequest {
            agent_identity: agent2,
            target_branch: "main".to_string(),
            intent: WorkIntent::FullBranch,
            mode: CoordinationMode::Isolated,
            work_items: vec![],
        };

        let response2 = coordinator.handle_join_request(request2).await.unwrap();

        match response2 {
            JoinResponse::Denied { suggestions, .. } => {
                // Should suggest alternatives
                assert!(!suggestions.is_empty());
                assert!(suggestions.iter().any(|s| s.contains("Coordinated")
                    || s.contains("ReadOnly")
                    || s.contains("Wait")
                    || s.contains("branch")));
            }
            _ => panic!("Expected Denied with suggestions"),
        }
    }

    #[tokio::test]
    async fn test_readonly_does_not_block_writers() {
        let coordinator = setup_test_coordinator().await;

        let reader = create_agent(AgentRole::Executor, false);
        let writer = create_agent(AgentRole::Executor, false);

        // Reader joins first
        let request_read = JoinRequest {
            agent_identity: reader,
            target_branch: "main".to_string(),
            intent: WorkIntent::ReadOnly,
            mode: CoordinationMode::Coordinated,
            work_items: vec![],
        };

        coordinator.handle_join_request(request_read).await.unwrap();

        // Writer should be able to join (coordinated mode)
        let request_write = JoinRequest {
            agent_identity: writer,
            target_branch: "main".to_string(),
            intent: WorkIntent::Write(vec![PathBuf::from("src/lib.rs")]),
            mode: CoordinationMode::Coordinated,
            work_items: vec![],
        };

        let response_write = coordinator
            .handle_join_request(request_write)
            .await
            .unwrap();

        match response_write {
            JoinResponse::RequiresCoordination { .. } | JoinResponse::Approved { .. } => {
                // Success - writer can work alongside reader
            }
            JoinResponse::Denied { .. } => {
                panic!("Writer should not be denied when reader is present");
            }
        }
    }
}
