//! Artifact management commands (constitution, specs, plans)

use clap::Subcommand;
use mnemosyne_core::{
    artifacts::{
        Artifact as ArtifactTrait,
    },
    error::Result,
    icons,
    ConnectionMode, LibsqlStorage,
};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use super::helpers::get_db_path;

#[derive(Debug, Subcommand)]
pub enum ArtifactCommands {
    /// Initialize artifact directory structure
    Init,

    /// Create a new project constitution
    CreateConstitution {
        /// Project name
        #[arg(short, long)]
        project: String,

        /// Core principles (can be specified multiple times)
        #[arg(short = 'P', long)]
        principle: Vec<String>,

        /// Quality gates (can be specified multiple times)
        #[arg(short = 'q', long)]
        quality_gate: Vec<String>,

        /// Constraints (can be specified multiple times)
        #[arg(short = 'c', long)]
        constraint: Vec<String>,

        /// Namespace for memory entry (default: project:PROJECT_NAME)
        #[arg(short, long)]
        namespace: Option<String>,
    },

    /// Create a new feature specification
    CreateFeatureSpec {
        /// Feature ID (kebab-case, e.g., "user-auth-jwt")
        #[arg(short, long)]
        id: String,

        /// Feature name (e.g., "User Authentication")
        #[arg(short, long)]
        name: String,

        /// Parent feature ID (for sub-features)
        #[arg(short, long)]
        parent: Option<String>,

        /// Functional requirements (can be specified multiple times)
        #[arg(short = 'r', long)]
        requirement: Vec<String>,

        /// Success criteria (can be specified multiple times)
        #[arg(short = 's', long)]
        success_criterion: Vec<String>,

        /// Constitution memory ID to link to
        #[arg(short = 'C', long)]
        constitution_id: Option<String>,

        /// Namespace for memory entry (default: project:CURRENT_PROJECT)
        #[arg(short = 'N', long)]
        namespace: Option<String>,
    },

    /// List all artifacts
    List {
        /// Filter by artifact type (constitution|spec|plan|tasks|checklist|clarification)
        #[arg(short, long)]
        artifact_type: Option<String>,
    },

    /// Show artifact details
    Show {
        /// Artifact ID or file path
        artifact: String,
    },

    /// Validate artifact structure and frontmatter
    Validate {
        /// Artifact file path
        path: String,
    },
}

/// Handle artifact command
pub async fn handle(command: ArtifactCommands, global_db_path: Option<String>) -> Result<()> {
    use mnemosyne_core::artifacts::{
        Constitution, FeatureSpec, Artifact as ArtifactTrait,
        ArtifactWorkflow, parse_frontmatter,
    };
    use mnemosyne_core::types::Namespace;
    use std::fs;

    match command {
        ArtifactCommands::CreateConstitution {
            project,
            principle,
            quality_gate,
            constraint,
            namespace,
        } => {
            println!("Creating project constitution for '{}'...", project);

            // Ensure artifact directory exists
            let artifacts_dir = PathBuf::from(".mnemosyne/artifacts");
            if !artifacts_dir.exists() {
                eprintln!("✗ Artifact directory not found. Run 'mnemosyne artifact init' first.");
                std::process::exit(1);
            }

            // Validate that at least one principle is provided
            if principle.is_empty() {
                eprintln!("✗ At least one principle is required (use --principle)");
                std::process::exit(1);
            }

            // Initialize storage and workflow
            let db_path = get_db_path(global_db_path.clone());
            let storage = Arc::new(
                LibsqlStorage::new_with_validation(ConnectionMode::Local(db_path), true).await?
            );
            let workflow = ArtifactWorkflow::new(artifacts_dir.clone(), storage)?;

            // Build constitution
            let mut builder = Constitution::builder(project.clone());
            for p in principle {
                builder = builder.principle(p);
            }
            for gate in quality_gate {
                builder = builder.quality_gate(gate);
            }
            for c in constraint {
                builder = builder.constraint(c);
            }
            let mut constitution = builder.build();

            // Determine namespace
            let ns = if let Some(ns_str) = namespace {
                // Parse namespace string (e.g., "project:myapp")
                if ns_str.starts_with("project:") {
                    let name = ns_str.strip_prefix("project:").unwrap().to_string();
                    Namespace::Project { name }
                } else if ns_str == "global" {
                    Namespace::Global
                } else {
                    eprintln!("✗ Invalid namespace format. Use 'global' or 'project:NAME'");
                    std::process::exit(1);
                }
            } else {
                // Default to project namespace
                Namespace::Project { name: project.clone() }
            };

            // Save constitution
            let memory_id = workflow.save_constitution(&mut constitution, ns).await?;

            println!("{} Constitution saved!", icons::status::success());
            println!("   Memory ID: {}", memory_id);
            println!("   File: {}", constitution.file_path().display());
            println!();
            println!("Next steps:");
            println!("  - View: mnemosyne artifact show constitution");
            println!("  - Edit: $EDITOR .mnemosyne/artifacts/{}", constitution.file_path().display());
            println!("  - Create feature spec: mnemosyne artifact create-feature-spec ...");

            Ok(())
        }
        ArtifactCommands::CreateFeatureSpec {
            id,
            name,
            parent,
            requirement,
            success_criterion,
            constitution_id,
            namespace,
        } => {
            println!("Creating feature specification '{}'...", name);

            // Ensure artifact directory exists
            let artifacts_dir = PathBuf::from(".mnemosyne/artifacts");
            if !artifacts_dir.exists() {
                eprintln!("✗ Artifact directory not found. Run 'mnemosyne artifact init' first.");
                std::process::exit(1);
            }

            // Initialize storage and workflow
            let db_path = get_db_path(global_db_path.clone());
            let storage = Arc::new(
                LibsqlStorage::new_with_validation(ConnectionMode::Local(db_path), true).await?
            );
            let workflow = ArtifactWorkflow::new(artifacts_dir.clone(), storage)?;

            // Build feature spec
            let mut builder = FeatureSpec::builder(id.clone(), name.clone());
            if let Some(p) = parent {
                builder = builder.parent_feature(p);
            }
            for req in requirement {
                builder = builder.requirement(req);
            }
            for criterion in success_criterion {
                builder = builder.success_criterion(criterion);
            }
            let mut spec = builder.build();

            // Determine namespace
            let ns = if let Some(ns_str) = namespace {
                // Parse namespace string
                if ns_str.starts_with("project:") {
                    let proj_name = ns_str.strip_prefix("project:").unwrap().to_string();
                    Namespace::Project { name: proj_name }
                } else if ns_str == "global" {
                    Namespace::Global
                } else {
                    eprintln!("✗ Invalid namespace format. Use 'global' or 'project:NAME'");
                    std::process::exit(1);
                }
            } else {
                // Try to infer project name from git root or use "default"
                let project_name = std::env::current_dir()
                    .ok()
                    .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()))
                    .unwrap_or_else(|| "default".to_string());
                Namespace::Project { name: project_name }
            };

            // Save feature spec
            let memory_id = workflow.save_feature_spec(&mut spec, ns, constitution_id).await?;

            println!("{} Feature spec saved!", icons::status::success());
            println!("   Memory ID: {}", memory_id);
            println!("   File: {}", spec.file_path().display());
            if let Some(ref const_id) = spec.metadata().references.first() {
                println!("   Linked to constitution: {}", const_id);
            }
            println!();
            println!("Next steps:");
            println!("  - View: mnemosyne artifact show {}", id);
            println!("  - Edit: $EDITOR .mnemosyne/artifacts/{}", spec.file_path().display());
            println!("  - List all specs: mnemosyne artifact list --artifact-type spec");

            Ok(())
        }
        ArtifactCommands::Init => {
            println!("Initializing artifact directory structure...");

            // Create artifact directories
            let base = PathBuf::from(".mnemosyne/artifacts");
            let subdirs = [
                "constitution",
                "specs",
                "plans",
                "tasks",
                "checklists",
                "clarifications",
            ];

            for subdir in &subdirs {
                let path = base.join(subdir);
                fs::create_dir_all(&path)?;
                println!("  ✓ Created {}", path.display());
            }

            // Create README
            let readme_path = base.join("README.md");
            let readme_content = r#"# Mnemosyne Artifacts

This directory contains specification workflow artifacts for structured specification-driven development.

## Structure

- `constitution/` - Project constitution defining principles and quality gates
- `specs/` - Feature specifications with user scenarios
- `plans/` - Implementation plans with technical architecture
- `tasks/` - Task breakdowns with dependencies
- `checklists/` - Quality checklists for validation
- `clarifications/` - Clarifications resolving ambiguities

## Usage

Use slash commands in Claude Code:
- `/project-constitution` - Create/update constitution
- `/feature-specify <description>` - Create feature spec
- `/feature-plan <feature-id>` - Create implementation plan
- `/feature-tasks <feature-id>` - Create task breakdown
- `/feature-checklist <feature-id>` - Create quality checklist

Or use CLI:
```bash
mnemosyne artifact list
mnemosyne artifact show <artifact-id>
mnemosyne artifact validate <path>
```

For more information, see: docs/specs/specification-artifacts.md
"#;
            fs::write(&readme_path, readme_content)?;
            println!("  ✓ Created {}", readme_path.display());

            println!();
            println!("✓ Artifact structure initialized successfully!");
            println!();
            println!("Next steps:");
            println!("  1. Create constitution: /project-constitution");
            println!("  2. Create feature spec: /feature-specify <description>");
            println!("  3. View artifacts: mnemosyne artifact list");
            Ok(())
        }
        ArtifactCommands::List { artifact_type } => {
            println!("Listing artifacts...");

            let base = PathBuf::from(".mnemosyne/artifacts");
            if !base.exists() {
                eprintln!("✗ Artifact directory not found. Run 'mnemosyne artifact init' first.");
                std::process::exit(1);
            }

            let search_dirs = if let Some(ref atype) = artifact_type {
                // Map type to directory
                let dir = match atype.as_str() {
                    "constitution" => "constitution",
                    "spec" | "feature_spec" => "specs",
                    "plan" | "implementation_plan" => "plans",
                    "tasks" | "task_breakdown" => "tasks",
                    "checklist" | "quality_checklist" => "checklists",
                    "clarification" => "clarifications",
                    _ => {
                        eprintln!("✗ Unknown artifact type: {}", atype);
                        eprintln!("Valid types: constitution, spec, plan, tasks, checklist, clarification");
                        std::process::exit(1);
                    }
                };
                vec![base.join(dir)]
            } else {
                // All directories
                vec![
                    base.join("constitution"),
                    base.join("specs"),
                    base.join("plans"),
                    base.join("tasks"),
                    base.join("checklists"),
                    base.join("clarifications"),
                ]
            };

            let mut found_any = false;
            for dir in search_dirs {
                if !dir.exists() {
                    continue;
                }

                let dir_name = dir.file_name().unwrap().to_string_lossy();
                let entries: Vec<_> = fs::read_dir(&dir)?
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path().extension().map_or(false, |ext| ext == "md")
                    })
                    .collect();

                if !entries.is_empty() {
                    println!("\n{}:", dir_name);
                    found_any = true;

                    for entry in entries {
                        let path = entry.path();
                        let name = path.file_name().unwrap().to_string_lossy();

                        // Try to parse frontmatter to get metadata
                        if let Ok(content) = fs::read_to_string(&path) {
                            if let Ok((frontmatter, _)) = parse_frontmatter(&content) {
                                let version = frontmatter.get("version")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown");
                                let status = frontmatter.get("status")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown");

                                println!("  • {} (v{}, {})", name, version, status);
                            } else {
                                println!("  • {}", name);
                            }
                        } else {
                            println!("  • {}", name);
                        }
                    }
                }
            }

            if !found_any {
                println!("No artifacts found.");
                println!("Create your first artifact with: /project-constitution");
            }

            Ok(())
        }
        ArtifactCommands::Show { artifact } => {
            // Try to find artifact by ID or path
            let path = if artifact.ends_with(".md") {
                PathBuf::from(artifact)
            } else {
                // Search for artifact by ID in all directories
                let base = PathBuf::from(".mnemosyne/artifacts");
                let search_dirs = [
                    base.join("constitution"),
                    base.join("specs"),
                    base.join("plans"),
                    base.join("tasks"),
                    base.join("checklists"),
                    base.join("clarifications"),
                ];

                let mut found_path: Option<PathBuf> = None;
                for dir in &search_dirs {
                    if !dir.exists() {
                        continue;
                    }

                    let artifact_file = format!("{}.md", artifact);
                    let candidate = dir.join(&artifact_file);
                    if candidate.exists() {
                        found_path = Some(candidate);
                        break;
                    }
                }

                found_path.unwrap_or_else(|| {
                    eprintln!("✗ Artifact not found: {}", artifact);
                    eprintln!("Try: mnemosyne artifact list");
                    std::process::exit(1);
                })
            };

            if !path.exists() {
                eprintln!("✗ File not found: {}", path.display());
                std::process::exit(1);
            }

            let content = fs::read_to_string(&path)?;
            println!("{}", content);
            Ok(())
        }
        ArtifactCommands::Validate { path } => {
            println!("Validating artifact: {}", path);

            let path_buf = PathBuf::from(&path);
            if !path_buf.exists() {
                eprintln!("✗ File not found: {}", path);
                std::process::exit(1);
            }

            let content = fs::read_to_string(&path_buf)?;

            // Parse frontmatter
            match parse_frontmatter(&content) {
                Ok((frontmatter, markdown)) => {
                    println!("✓ Valid YAML frontmatter");

                    // Check required fields
                    let required_fields = ["type", "id", "name", "version"];
                    let mut missing_fields = Vec::new();

                    for field in &required_fields {
                        if frontmatter.get(*field).is_none() {
                            missing_fields.push(*field);
                        }
                    }

                    if !missing_fields.is_empty() {
                        eprintln!("✗ Missing required fields: {}", missing_fields.join(", "));
                        std::process::exit(1);
                    }

                    println!("✓ All required fields present");

                    // Validate version format
                    if let Some(version) = frontmatter.get("version").and_then(|v| v.as_str()) {
                        if version.split('.').count() == 3 {
                            println!("✓ Valid semantic version: {}", version);
                        } else {
                            eprintln!("✗ Invalid version format: {} (expected X.Y.Z)", version);
                            std::process::exit(1);
                        }
                    }

                    // Check markdown content
                    if markdown.trim().is_empty() {
                        eprintln!("✗ Empty content (no markdown after frontmatter)");
                        std::process::exit(1);
                    }

                    println!("✓ Non-empty content ({} chars)", markdown.len());

                    println!();
                    println!("✓ Artifact is valid!");
                }
                Err(e) => {
                    eprintln!("✗ Invalid artifact: {}", e);
                    std::process::exit(1);
                }
            }

            Ok(())
        }
    }
}
