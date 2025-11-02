//! Example: Specification Workflow
//!
//! Demonstrates the complete specification workflow:
//! 1. Initialize artifact directory
//! 2. Create project constitution
//! 3. Create feature specification
//! 4. Create implementation plan
//! 5. Create task breakdown
//! 6. Create quality checklist
//! 7. Create clarifications
//! 8. Link artifacts with memory system
//!
//! Run with: cargo run --example specification_workflow

use mnemosyne_core::artifacts::{
    Constitution, FeatureSpec, UserScenario, ImplementationPlan, TaskBreakdown,
    TaskPhase, Task, QualityChecklist, ChecklistSection, ChecklistItem,
    Clarification, ClarificationItem, ArtifactWorkflow,
};
use mnemosyne_core::types::Namespace;
use mnemosyne_core::{ConnectionMode, LibsqlStorage};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Specification Workflow Example ===\n");

    // 1. Setup: Create temp directory and database
    let temp_dir = std::env::temp_dir().join(format!("mnemosyne_example_{}", std::process::id()));
    let artifacts_dir = temp_dir.join(".mnemosyne/artifacts");
    std::fs::create_dir_all(&artifacts_dir)?;

    println!("📁 Created artifact directory: {}\n", artifacts_dir.display());

    // Create subdirectories
    for subdir in &["constitution", "specs", "plans", "tasks", "checklists", "clarifications"] {
        std::fs::create_dir_all(artifacts_dir.join(subdir))?;
    }

    // Create temporary database for example
    let db_path = temp_dir.join("example.db");
    let storage = Arc::new(
        LibsqlStorage::new_with_validation(
            ConnectionMode::Local(db_path.to_string_lossy().to_string()),
            true, // Create if missing
        )
        .await?
    );

    println!("✅ Initialized memory storage\n");

    // 2. Create workflow coordinator
    let workflow = ArtifactWorkflow::new(artifacts_dir.clone(), storage)?;

    // 3. Create project constitution
    println!("📜 Creating project constitution...\n");

    let mut constitution = Constitution::builder("ExampleProject".to_string())
        .principle("Performance First: Sub-200ms p95 latency for all queries")
        .principle("Type Safety: Leverage Rust's type system for correctness")
        .principle("Developer Experience: Clear APIs and helpful error messages")
        .quality_gate("90%+ test coverage on critical paths")
        .quality_gate("All compiler warnings addressed")
        .quality_gate("Documentation for all public APIs")
        .constraint("Rust-only for core logic (performance-critical)")
        .constraint("No external API dependencies without approval")
        .build();

    let namespace = Namespace::Project {
        name: "example-project".to_string(),
    };

    let constitution_memory_id = workflow
        .save_constitution(&mut constitution, namespace.clone())
        .await?;

    println!("✅ Constitution saved!");
    println!("   Memory ID: {}", constitution_memory_id);
    println!("   File: {}/constitution/project-constitution.md\n", artifacts_dir.display());

    // 4. Create feature specification
    println!("🎯 Creating feature specification...\n");

    let mut feature_spec = FeatureSpec::builder(
        "user-authentication".to_string(),
        "User Authentication".to_string(),
    )
    .scenario(UserScenario {
        priority: "P0".to_string(),
        actor: "developer".to_string(),
        goal: "authenticate with JWT tokens".to_string(),
        benefit: "maintain stateless sessions".to_string(),
        acceptance_criteria: vec![
            "Token issued on successful login".to_string(),
            "Token validated on protected endpoints".to_string(),
            "Token expires after 24 hours".to_string(),
        ],
    })
    .scenario(UserScenario {
        priority: "P1".to_string(),
        actor: "developer".to_string(),
        goal: "refresh authentication tokens".to_string(),
        benefit: "avoid frequent re-authentication".to_string(),
        acceptance_criteria: vec![
            "Refresh token provided with access token".to_string(),
            "Refresh endpoint validates refresh token".to_string(),
            "Refresh tokens stored securely".to_string(),
        ],
    })
    .requirement("Use RS256 algorithm for JWT signing")
    .requirement("Store refresh tokens in HTTP-only cookies")
    .requirement("Implement token revocation endpoint")
    .success_criterion("Authentication latency < 100ms p95")
    .success_criterion("Support 10,000 concurrent sessions")
    .build();

    let spec_memory_id = workflow
        .save_feature_spec(
            &mut feature_spec,
            namespace.clone(),
            Some(constitution_memory_id.to_string()),
        )
        .await?;

    println!("✅ Feature spec saved!");
    println!("   Memory ID: {}", spec_memory_id);
    println!("   File: {}/specs/user-authentication.md", artifacts_dir.display());
    println!("   Linked to constitution: {}\n", constitution_memory_id);

    // 5. Create implementation plan
    println!("📋 Creating implementation plan...\n");

    let mut plan = ImplementationPlan::new(
        "user-authentication".to_string(),
        "JWT Authentication Implementation Plan".to_string(),
        "Using jsonwebtoken crate with RS256 algorithm for asymmetric signing. \
         Refresh tokens stored in HTTP-only cookies with secure flag. \
         Token revocation via blacklist in Redis.".to_string(),
    );

    plan.dependencies.push("jsonwebtoken = \"9.2\"".to_string());
    plan.dependencies.push("redis = { version = \"0.23\", features = [\"tokio-comp\"] }".to_string());

    let plan_memory_id = workflow
        .save_implementation_plan(
            &mut plan,
            namespace.clone(),
            Some(spec_memory_id.to_string()),
        )
        .await?;

    println!("✅ Implementation plan saved!");
    println!("   Memory ID: {}", plan_memory_id);
    println!("   File: {}/plans/user-authentication-plan.md", artifacts_dir.display());
    println!("   Linked to spec: {}\n", spec_memory_id);

    // 6. Create task breakdown
    println!("📝 Creating task breakdown...\n");

    let mut tasks = TaskBreakdown::new(
        "user-authentication".to_string(),
        "JWT Authentication Tasks".to_string(),
    );

    tasks.phases.push(TaskPhase {
        name: "Setup".to_string(),
        tasks: vec![
            Task {
                id: "T001".to_string(),
                description: "Add jsonwebtoken and redis dependencies".to_string(),
                parallelizable: false,
                story: None,
                completed: false,
                depends_on: vec![],
            },
            Task {
                id: "T002".to_string(),
                description: "Create keys directory and generate RS256 keypair".to_string(),
                parallelizable: false,
                story: None,
                completed: false,
                depends_on: vec![],
            },
        ],
    });

    tasks.phases.push(TaskPhase {
        name: "P0: User Login with Token".to_string(),
        tasks: vec![
            Task {
                id: "T003".to_string(),
                description: "[P] Implement token generation function".to_string(),
                parallelizable: true,
                story: None,
                completed: false,
                depends_on: vec!["T001".to_string(), "T002".to_string()],
            },
            Task {
                id: "T004".to_string(),
                description: "[P] Implement token validation middleware".to_string(),
                parallelizable: true,
                story: None,
                completed: false,
                depends_on: vec!["T001".to_string(), "T002".to_string()],
            },
            Task {
                id: "T005".to_string(),
                description: "Integrate login endpoint with token generation".to_string(),
                parallelizable: false,
                story: None,
                completed: false,
                depends_on: vec!["T003".to_string()],
            },
        ],
    });

    tasks.phases.push(TaskPhase {
        name: "P1: Token Refresh".to_string(),
        tasks: vec![
            Task {
                id: "T006".to_string(),
                description: "Implement refresh token storage in Redis".to_string(),
                parallelizable: false,
                story: None,
                completed: false,
                depends_on: vec!["T003".to_string()],
            },
            Task {
                id: "T007".to_string(),
                description: "Add refresh endpoint with token validation".to_string(),
                parallelizable: false,
                story: None,
                completed: false,
                depends_on: vec!["T006".to_string()],
            },
        ],
    });

    let tasks_memory_id = workflow
        .save_task_breakdown(
            &mut tasks,
            namespace.clone(),
            Some(plan_memory_id.to_string()),
        )
        .await?;

    println!("✅ Task breakdown saved!");
    println!("   Memory ID: {}", tasks_memory_id);
    println!("   File: {}/tasks/user-authentication-tasks.md", artifacts_dir.display());
    println!("   Linked to plan: {}\n", plan_memory_id);

    // 7. Create quality checklist
    println!("✅ Creating quality checklist...\n");

    let mut checklist = QualityChecklist::new(
        "user-authentication".to_string(),
        "JWT Authentication Quality Checklist".to_string(),
    );

    checklist.add_section(ChecklistSection {
        name: "Functional Requirements".to_string(),
        items: vec![
            ChecklistItem {
                description: "Token generation works with correct claims".to_string(),
                completed: false,
                notes: None,
            },
            ChecklistItem {
                description: "Token validation rejects invalid/expired tokens".to_string(),
                completed: false,
                notes: None,
            },
            ChecklistItem {
                description: "Refresh token flow works end-to-end".to_string(),
                completed: false,
                notes: Some("Test with expired access token".to_string()),
            },
        ],
    });

    checklist.add_section(ChecklistSection {
        name: "Non-Functional Requirements".to_string(),
        items: vec![
            ChecklistItem {
                description: "Performance: Authentication latency < 100ms p95".to_string(),
                completed: false,
                notes: None,
            },
            ChecklistItem {
                description: "Security: No secrets in git".to_string(),
                completed: false,
                notes: Some("Verify .gitignore includes keys/".to_string()),
            },
            ChecklistItem {
                description: "Security: Keys stored securely with proper permissions".to_string(),
                completed: false,
                notes: None,
            },
        ],
    });

    checklist.add_section(ChecklistSection {
        name: "Testing".to_string(),
        items: vec![
            ChecklistItem {
                description: "Unit tests: 90%+ coverage".to_string(),
                completed: false,
                notes: None,
            },
            ChecklistItem {
                description: "Integration tests for token lifecycle".to_string(),
                completed: false,
                notes: None,
            },
        ],
    });

    let checklist_memory_id = workflow
        .save_quality_checklist(
            &mut checklist,
            namespace.clone(),
            Some(spec_memory_id.to_string()),
        )
        .await?;

    println!("✅ Quality checklist saved!");
    println!("   Memory ID: {}", checklist_memory_id);
    println!("   File: {}/checklists/user-authentication-checklist.md", artifacts_dir.display());
    println!("   Linked to spec: {}\n", spec_memory_id);

    // 8. Create clarifications
    println!("❓ Creating clarifications...\n");

    let mut clarification = Clarification::new(
        "user-authentication".to_string(),
        "JWT Authentication Clarifications".to_string(),
    );

    clarification.add_item(ClarificationItem {
        id: "Q001".to_string(),
        question: "Should we support refresh tokens or only short-lived access tokens?".to_string(),
        context: "The spec mentions stateless sessions but doesn't specify refresh mechanism.".to_string(),
        decision: Some("Use refresh tokens stored in HTTP-only cookies".to_string()),
        rationale: Some("Improves security (refresh tokens can be revoked) and UX (no manual re-auth every 24h)".to_string()),
        spec_updates: vec![
            "Added P1 scenario for refresh token flow".to_string(),
            "Updated security requirements to include token revocation".to_string(),
        ],
    });

    clarification.add_item(ClarificationItem {
        id: "Q002".to_string(),
        question: "Which JWT algorithm should we use?".to_string(),
        context: "Security best practices need to be clarified".to_string(),
        decision: Some("Use RS256 asymmetric signing".to_string()),
        rationale: Some("Better key rotation and security properties than HS256".to_string()),
        spec_updates: vec!["Added RS256 requirement to spec".to_string()],
    });

    clarification.add_item(ClarificationItem {
        id: "Q003".to_string(),
        question: "Should we implement password recovery in this feature?".to_string(),
        context: "Out of scope for MVP?".to_string(),
        decision: None,
        rationale: None,
        spec_updates: vec![],
    });

    let clarification_memory_id = workflow
        .save_clarification(
            &mut clarification,
            namespace.clone(),
            Some(spec_memory_id.to_string()),
        )
        .await?;

    println!("✅ Clarifications saved!");
    println!("   Memory ID: {}", clarification_memory_id);
    println!("   File: {}/clarifications/user-authentication-clarifications.md", artifacts_dir.display());
    println!("   Linked to spec: {}", spec_memory_id);
    println!("   Status: {} resolved, {} pending\n",
        clarification.items.iter().filter(|i| i.decision.is_some()).count(),
        clarification.items.iter().filter(|i| i.decision.is_none()).count()
    );

    // 9. Verify files were created
    println!("📂 Verifying artifact files...\n");

    let constitution_path = artifacts_dir.join("constitution/project-constitution.md");
    let spec_path = artifacts_dir.join("specs/user-authentication.md");

    if constitution_path.exists() {
        let content = std::fs::read_to_string(&constitution_path)?;
        println!("✅ Constitution file exists ({} bytes)", content.len());
        println!("   Preview: {}", &content.lines().take(5).collect::<Vec<_>>().join("\n"));
        println!();
    }

    if spec_path.exists() {
        let content = std::fs::read_to_string(&spec_path)?;
        println!("✅ Feature spec file exists ({} bytes)", content.len());
        println!("   Preview: {}", &content.lines().take(5).collect::<Vec<_>>().join("\n"));
        println!();
    }

    // 10. Demonstrate loading all artifacts
    println!("📥 Loading artifacts from files...\n");

    let loaded_constitution = workflow.load_constitution().await?;
    println!("✅ Loaded constitution:");
    println!("   Principles: {}", loaded_constitution.principles.len());
    println!("   Quality Gates: {}", loaded_constitution.quality_gates.len());
    println!("   Constraints: {}", loaded_constitution.constraints.len());
    println!();

    let loaded_spec = workflow.load_feature_spec("user-authentication").await?;
    println!("✅ Loaded feature spec:");
    println!("   Scenarios: {}", loaded_spec.scenarios.len());
    println!("   Requirements: {}", loaded_spec.requirements.len());
    println!("   Success Criteria: {}", loaded_spec.success_criteria.len());
    println!();

    let loaded_plan = workflow.load_implementation_plan("user-authentication").await?;
    println!("✅ Loaded implementation plan:");
    println!("   Approach: {} chars", loaded_plan.approach.len());
    println!("   Dependencies: {}", loaded_plan.dependencies.len());
    println!();

    let loaded_tasks = workflow.load_task_breakdown("user-authentication").await?;
    let total_tasks: usize = loaded_tasks.phases.iter().map(|p| p.tasks.len()).sum();
    println!("✅ Loaded task breakdown:");
    println!("   Phases: {}", loaded_tasks.phases.len());
    println!("   Total Tasks: {}", total_tasks);
    println!();

    let loaded_checklist = workflow.load_quality_checklist("user-authentication").await?;
    let total_items: usize = loaded_checklist.sections.iter().map(|s| s.items.len()).sum();
    println!("✅ Loaded quality checklist:");
    println!("   Sections: {}", loaded_checklist.sections.len());
    println!("   Total Items: {}", total_items);
    println!("   Completion: {:.1}%", loaded_checklist.completion_percentage());
    println!();

    let loaded_clarification = workflow.load_clarification("user-authentication").await?;
    let resolved = loaded_clarification.items.iter().filter(|i| i.decision.is_some()).count();
    println!("✅ Loaded clarifications:");
    println!("   Total Questions: {}", loaded_clarification.items.len());
    println!("   Resolved: {}", resolved);
    println!("   Pending: {}", loaded_clarification.items.len() - resolved);
    println!();

    // 11. Cleanup
    println!("🧹 Cleaning up...");
    std::fs::remove_dir_all(&temp_dir)?;
    println!("✅ Removed temporary directory\n");

    println!("=== Workflow Complete ===");
    println!("\nThis example demonstrated:");
    println!("  ✓ Constitution creation with builder pattern");
    println!("  ✓ Feature spec creation with scenarios");
    println!("  ✓ Implementation plan with architecture decisions");
    println!("  ✓ Task breakdown with phases and dependencies");
    println!("  ✓ Quality checklist with sections and completion tracking");
    println!("  ✓ Clarifications with resolved/pending status");
    println!("  ✓ Memory entry creation with linking (constitution → spec → plan → tasks)");
    println!("  ✓ Artifact file persistence to .mnemosyne/artifacts/");
    println!("  ✓ Artifact loading from files with full data preservation");

    Ok(())
}
