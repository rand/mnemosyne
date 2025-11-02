//! Example: Specification Workflow
//!
//! Demonstrates the complete specification workflow:
//! 1. Initialize artifact directory
//! 2. Create project constitution
//! 3. Create feature specification
//! 4. Link artifacts with memory system
//!
//! Run with: cargo run --example specification_workflow

use mnemosyne_core::artifacts::{
    Constitution, FeatureSpec, UserScenario, ArtifactWorkflow,
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

    // 5. Verify files were created
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

    // 6. Demonstrate loading artifacts
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

    // 7. Cleanup
    println!("🧹 Cleaning up...");
    std::fs::remove_dir_all(&temp_dir)?;
    println!("✅ Removed temporary directory\n");

    println!("=== Workflow Complete ===");
    println!("\nThis example demonstrated:");
    println!("  ✓ Constitution creation with builder pattern");
    println!("  ✓ Feature spec creation with scenarios");
    println!("  ✓ Memory entry creation with linking");
    println!("  ✓ Artifact file persistence");
    println!("  ✓ Artifact loading from files");

    Ok(())
}
