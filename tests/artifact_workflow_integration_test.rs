//! Integration tests for artifact workflow
//!
//! Tests complete workflow including file I/O and memory creation

use mnemosyne_core::artifacts::{
    ArtifactWorkflow, ChecklistItem, ChecklistSection, Clarification, ClarificationItem,
    Constitution, FeatureSpec, ImplementationPlan, QualityChecklist, Task, TaskBreakdown,
    TaskPhase, UserScenario,
};
use mnemosyne_core::types::Namespace;
use mnemosyne_core::{ConnectionMode, LibsqlStorage};
use std::sync::Arc;

/// Helper to create temp directory and workflow
async fn setup_workflow() -> (tempfile::TempDir, ArtifactWorkflow) {
    let temp_dir = tempfile::tempdir().unwrap();
    let artifacts_dir = temp_dir.path().join(".mnemosyne/artifacts");
    std::fs::create_dir_all(&artifacts_dir).unwrap();

    // Create subdirectories
    for subdir in &[
        "constitution",
        "specs",
        "plans",
        "tasks",
        "checklists",
        "clarifications",
    ] {
        std::fs::create_dir_all(artifacts_dir.join(subdir)).unwrap();
    }

    // Create test database
    let db_path = temp_dir.path().join("test.db");
    let storage = Arc::new(
        LibsqlStorage::new_with_validation(
            ConnectionMode::Local(db_path.to_string_lossy().to_string()),
            true,
        )
        .await
        .unwrap(),
    );

    let workflow = ArtifactWorkflow::new(artifacts_dir, storage).unwrap();

    (temp_dir, workflow)
}

#[tokio::test]
async fn test_constitution_round_trip_workflow() {
    let (_temp_dir, workflow) = setup_workflow().await;

    // Create constitution
    let mut constitution = Constitution::builder("TestProject".to_string())
        .principle("Performance First")
        .quality_gate("90%+ coverage")
        .constraint("Rust-only")
        .build();

    let namespace = Namespace::Project {
        name: "test-project".to_string(),
    };

    // Save
    let memory_id = workflow
        .save_constitution(&mut constitution, namespace.clone())
        .await
        .unwrap();

    assert!(!memory_id.to_string().is_empty());
    assert!(constitution.metadata.memory_id.is_some());

    // Load
    let loaded = workflow.load_constitution().await.unwrap();

    assert_eq!(loaded.principles.len(), 1);
    assert_eq!(loaded.quality_gates.len(), 1);
    assert_eq!(loaded.constraints.len(), 1);
    assert_eq!(loaded.metadata.memory_id, constitution.metadata.memory_id);
}

#[tokio::test]
async fn test_feature_spec_workflow() {
    let (_temp_dir, workflow) = setup_workflow().await;

    let mut spec = FeatureSpec::builder("test-feature".to_string(), "Test Feature".to_string())
        .scenario(UserScenario {
            priority: "P0".to_string(),
            actor: "user".to_string(),
            goal: "do something".to_string(),
            benefit: "gain value".to_string(),
            acceptance_criteria: vec!["criterion 1".to_string()],
        })
        .requirement("Requirement 1")
        .success_criterion("Success 1")
        .build();

    let namespace = Namespace::Project {
        name: "test-project".to_string(),
    };

    // Save
    let memory_id = workflow
        .save_feature_spec(&mut spec, namespace.clone(), None)
        .await
        .unwrap();

    assert!(!memory_id.to_string().is_empty());
    assert!(spec.metadata.memory_id.is_some());

    // Load
    let loaded = workflow.load_feature_spec("test-feature").await.unwrap();

    assert_eq!(loaded.feature_id, "test-feature");
    assert_eq!(loaded.scenarios.len(), 1);
    assert_eq!(loaded.requirements.len(), 1);
    assert_eq!(loaded.success_criteria.len(), 1);
}

#[tokio::test]
async fn test_implementation_plan_workflow() {
    let (_temp_dir, workflow) = setup_workflow().await;

    let mut plan = ImplementationPlan::new(
        "test-feature".to_string(),
        "Test Feature Plan".to_string(),
        "Use pattern X".to_string(),
    );

    let namespace = Namespace::Project {
        name: "test-project".to_string(),
    };

    // Save
    let memory_id = workflow
        .save_implementation_plan(&mut plan, namespace.clone(), None)
        .await
        .unwrap();

    assert!(!memory_id.to_string().is_empty());
    assert!(plan.metadata.memory_id.is_some());

    // Load
    let loaded = workflow
        .load_implementation_plan("test-feature")
        .await
        .unwrap();

    assert_eq!(loaded.feature_id, "test-feature");
    assert_eq!(loaded.approach, "Use pattern X");
}

#[tokio::test]
async fn test_task_breakdown_workflow() {
    let (_temp_dir, workflow) = setup_workflow().await;

    let mut tasks =
        TaskBreakdown::new("test-feature".to_string(), "Test Feature Tasks".to_string());

    tasks.phases.push(TaskPhase {
        name: "Phase 1".to_string(),
        tasks: vec![Task {
            id: "T001".to_string(),
            description: "Task 1".to_string(),
            parallelizable: false,
            story: None,
            completed: false,
            depends_on: vec![],
        }],
    });

    let namespace = Namespace::Project {
        name: "test-project".to_string(),
    };

    // Save
    let memory_id = workflow
        .save_task_breakdown(&mut tasks, namespace.clone(), None)
        .await
        .unwrap();

    assert!(!memory_id.to_string().is_empty());
    assert!(tasks.metadata.memory_id.is_some());

    // Load
    let loaded = workflow.load_task_breakdown("test-feature").await.unwrap();

    assert_eq!(loaded.feature_id, "test-feature");
    assert_eq!(loaded.phases.len(), 1);
    assert_eq!(loaded.phases[0].tasks.len(), 1);
}

#[tokio::test]
async fn test_quality_checklist_workflow() {
    let (_temp_dir, workflow) = setup_workflow().await;

    let mut checklist = QualityChecklist::new(
        "test-feature".to_string(),
        "Test Feature Checklist".to_string(),
    );

    checklist.add_section(ChecklistSection {
        name: "Tests".to_string(),
        items: vec![ChecklistItem {
            description: "Unit tests".to_string(),
            completed: true,
            notes: None,
        }],
    });

    let namespace = Namespace::Project {
        name: "test-project".to_string(),
    };

    // Save
    let memory_id = workflow
        .save_quality_checklist(&mut checklist, namespace.clone(), None)
        .await
        .unwrap();

    assert!(!memory_id.to_string().is_empty());
    assert!(checklist.metadata.memory_id.is_some());

    // Load
    let loaded = workflow
        .load_quality_checklist("test-feature")
        .await
        .unwrap();

    assert_eq!(loaded.feature_id, "test-feature");
    assert_eq!(loaded.sections.len(), 1);
    assert_eq!(loaded.sections[0].items.len(), 1);
}

#[tokio::test]
async fn test_clarification_workflow() {
    let (_temp_dir, workflow) = setup_workflow().await;

    let mut clarification = Clarification::new(
        "test-feature".to_string(),
        "Test Feature Clarifications".to_string(),
    );

    clarification.add_item(ClarificationItem {
        id: "Q001".to_string(),
        question: "Use caching?".to_string(),
        context: "Performance unclear".to_string(),
        decision: Some("Yes, use Redis".to_string()),
        rationale: Some("Improves response times".to_string()),
        spec_updates: vec!["Added caching requirement".to_string()],
    });

    let namespace = Namespace::Project {
        name: "test-project".to_string(),
    };

    // Save
    let memory_id = workflow
        .save_clarification(&mut clarification, namespace.clone(), None)
        .await
        .unwrap();

    assert!(!memory_id.to_string().is_empty());
    assert!(clarification.metadata.memory_id.is_some());

    // Load
    let loaded = workflow.load_clarification("test-feature").await.unwrap();

    assert_eq!(loaded.feature_id, "test-feature");
    assert_eq!(loaded.items.len(), 1);
    assert_eq!(loaded.items[0].id, "Q001");
}

#[tokio::test]
async fn test_complete_workflow_chain() {
    let (_temp_dir, workflow) = setup_workflow().await;

    let namespace = Namespace::Project {
        name: "test-project".to_string(),
    };

    // 1. Constitution
    let mut constitution = Constitution::builder("ChainTest".to_string())
        .principle("Quality First")
        .build();

    let constitution_id = workflow
        .save_constitution(&mut constitution, namespace.clone())
        .await
        .unwrap();

    // 2. Feature Spec (linked to constitution)
    let mut spec = FeatureSpec::builder("chain-feature".to_string(), "Chain Feature".to_string())
        .requirement("Req 1")
        .build();

    let spec_id = workflow
        .save_feature_spec(
            &mut spec,
            namespace.clone(),
            Some(constitution_id.to_string()),
        )
        .await
        .unwrap();

    assert!(spec
        .metadata
        .references
        .contains(&constitution_id.to_string()));

    // 3. Implementation Plan (linked to spec)
    let mut plan = ImplementationPlan::new(
        "chain-feature".to_string(),
        "Chain Plan".to_string(),
        "Use recommended patterns".to_string(),
    );

    let plan_id = workflow
        .save_implementation_plan(&mut plan, namespace.clone(), Some(spec_id.to_string()))
        .await
        .unwrap();

    assert!(plan.metadata.references.contains(&spec_id.to_string()));

    // 4. Task Breakdown (linked to plan)
    let mut tasks = TaskBreakdown::new("chain-feature".to_string(), "Chain Tasks".to_string());

    let _tasks_id = workflow
        .save_task_breakdown(&mut tasks, namespace.clone(), Some(plan_id.to_string()))
        .await
        .unwrap();

    assert!(tasks.metadata.references.contains(&plan_id.to_string()));

    // 5. Quality Checklist (linked to spec)
    let mut checklist =
        QualityChecklist::new("chain-feature".to_string(), "Chain Checklist".to_string());

    let _checklist_id = workflow
        .save_quality_checklist(&mut checklist, namespace.clone(), Some(spec_id.to_string()))
        .await
        .unwrap();

    assert!(checklist.metadata.references.contains(&spec_id.to_string()));

    // 6. Clarification (linked to spec)
    let mut clarification = Clarification::new(
        "chain-feature".to_string(),
        "Chain Clarifications".to_string(),
    );

    let _clarification_id = workflow
        .save_clarification(
            &mut clarification,
            namespace.clone(),
            Some(spec_id.to_string()),
        )
        .await
        .unwrap();

    assert!(clarification
        .metadata
        .references
        .contains(&spec_id.to_string()));

    // Verify all artifacts can be loaded
    let _loaded_constitution = workflow.load_constitution().await.unwrap();
    let _loaded_spec = workflow.load_feature_spec("chain-feature").await.unwrap();
    let _loaded_plan = workflow
        .load_implementation_plan("chain-feature")
        .await
        .unwrap();
    let _loaded_tasks = workflow.load_task_breakdown("chain-feature").await.unwrap();
    let _loaded_checklist = workflow
        .load_quality_checklist("chain-feature")
        .await
        .unwrap();
    let _loaded_clarification = workflow.load_clarification("chain-feature").await.unwrap();
}
