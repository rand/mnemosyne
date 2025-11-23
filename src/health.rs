//! Health check system for mnemosyne
#![allow(clippy::collapsible_else_if)]
//!
//! Provides comprehensive health diagnostics including:
//! - Database integrity and schema validation
//! - Migration consistency checks
//! - Hook system verification
//! - Memory statistics and growth
//! - Performance benchmarks
//! - Actor health monitoring

use crate::error::Result;
use crate::storage::libsql::LibsqlStorage;
use crate::storage::StorageBackend;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;
use tracing::{debug, info};

/// Health check status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

/// Individual health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub name: String,
    pub status: CheckStatus,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl CheckResult {
    pub fn pass(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Pass,
            message: message.into(),
            details: None,
        }
    }

    pub fn warn(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Warn,
            message: message.into(),
            details: None,
        }
    }

    pub fn fail(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: CheckStatus::Fail,
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

/// Overall health check summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthSummary {
    pub status: CheckStatus,
    pub checks: Vec<CheckResult>,
    pub summary: HealthStats,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStats {
    pub total_checks: usize,
    pub passed: usize,
    pub warnings: usize,
    pub errors: usize,
}

/// Run all health checks
pub async fn run_health_checks(
    storage: &LibsqlStorage,
    verbose: bool,
    fix: bool,
) -> Result<HealthSummary> {
    info!("🏥 Starting mnemosyne health checks...");

    let mut checks = Vec::new();

    // Phase 1: Database Health (CRITICAL)
    checks.extend(check_database_health(storage, verbose, fix).await?);

    // Phase 2: Schema Validation (HIGH)
    checks.extend(check_schema_validation(storage, verbose, fix).await?);

    // Phase 3: Migration Consistency (HIGH)
    checks.extend(check_migration_consistency(storage, verbose, fix).await?);

    // Phase 4: Hook System (MEDIUM)
    checks.extend(check_hook_system(verbose, fix).await?);

    // Phase 5: Memory Statistics (MEDIUM)
    checks.extend(check_memory_statistics(storage, verbose).await?);

    // Phase 6: Performance (MEDIUM)
    checks.extend(check_performance(storage, verbose).await?);

    // Phase 7: Worktree Cleanup (LOW)
    checks.extend(check_worktree_cleanup(verbose, fix).await?);

    // Phase 8: Version Updates (INFO)
    checks.extend(check_version_updates(verbose).await?);

    // Calculate summary
    let passed = checks
        .iter()
        .filter(|c| c.status == CheckStatus::Pass)
        .count();
    let warnings = checks
        .iter()
        .filter(|c| c.status == CheckStatus::Warn)
        .count();
    let errors = checks
        .iter()
        .filter(|c| c.status == CheckStatus::Fail)
        .count();

    let overall_status = if errors > 0 {
        CheckStatus::Fail
    } else if warnings > 0 {
        CheckStatus::Warn
    } else {
        CheckStatus::Pass
    };

    Ok(HealthSummary {
        status: overall_status,
        checks,
        summary: HealthStats {
            total_checks: passed + warnings + errors,
            passed,
            warnings,
            errors,
        },
    })
}

/// Check database health (file exists, readable, not corrupted)
async fn check_database_health(
    storage: &LibsqlStorage,
    _verbose: bool,
    _fix: bool,
) -> Result<Vec<CheckResult>> {
    debug!("Checking database health...");
    let mut results = Vec::new();

    // Check database file exists and is readable
    let db_path = storage.db_path();
    if db_path.exists() {
        results.push(CheckResult::pass(
            "database_exists",
            format!("Database file exists: {}", db_path.display()),
        ));

        // Check database integrity
        match storage.check_integrity().await {
            Ok(true) => {
                results.push(CheckResult::pass(
                    "database_integrity",
                    "Database integrity check passed",
                ));
            }
            Ok(false) => {
                results.push(CheckResult::fail(
                    "database_integrity",
                    "Database integrity check failed - database may be corrupted",
                ));
            }
            Err(e) => {
                results.push(
                    CheckResult::fail("database_integrity", "Failed to check database integrity")
                        .with_details(serde_json::json!({ "error": e.to_string() })),
                );
            }
        }
    } else {
        results.push(CheckResult::fail(
            "database_exists",
            format!("Database file not found: {}", db_path.display()),
        ));
    }

    Ok(results)
}

/// Check schema validation (tables exist, correct structure)
async fn check_schema_validation(
    storage: &LibsqlStorage,
    _verbose: bool,
    _fix: bool,
) -> Result<Vec<CheckResult>> {
    debug!("Checking schema validation...");
    let mut results = Vec::new();

    // Check required tables exist
    let required_tables = vec![
        "memories",
        "memories_fts",
        "audit_log",
        "work_items",
        "_migrations_applied",
    ];

    for table in required_tables {
        match storage.table_exists(table).await {
            Ok(true) => {
                results.push(CheckResult::pass(
                    format!("table_{}", table),
                    format!("Table '{}' exists", table),
                ));
            }
            Ok(false) => {
                results.push(CheckResult::fail(
                    format!("table_{}", table),
                    format!("Required table '{}' not found", table),
                ));
            }
            Err(e) => {
                results.push(
                    CheckResult::fail(
                        format!("table_{}", table),
                        format!("Failed to check table '{}'", table),
                    )
                    .with_details(serde_json::json!({ "error": e.to_string() })),
                );
            }
        }
    }

    Ok(results)
}

/// Check migration consistency
async fn check_migration_consistency(
    storage: &LibsqlStorage,
    _verbose: bool,
    _fix: bool,
) -> Result<Vec<CheckResult>> {
    debug!("Checking migration consistency...");
    let mut results = Vec::new();

    // Get applied migrations from database
    match storage.get_applied_migrations().await {
        Ok(applied_migrations) => {
            results.push(
                CheckResult::pass(
                    "migrations_table",
                    format!("{} migrations applied", applied_migrations.len()),
                )
                .with_details(serde_json::json!({
                    "count": applied_migrations.len(),
                    "migrations": applied_migrations,
                })),
            );
        }
        Err(e) => {
            results.push(
                CheckResult::fail("migrations_table", "Failed to query applied migrations")
                    .with_details(serde_json::json!({ "error": e.to_string() })),
            );
        }
    }

    Ok(results)
}

/// Check hook system status
async fn check_hook_system(_verbose: bool, fix: bool) -> Result<Vec<CheckResult>> {
    debug!("Checking hook system...");
    let mut results = Vec::new();

    // Check .claude/hooks directory exists
    let hooks_dir = PathBuf::from(".claude/hooks");
    if hooks_dir.exists() && hooks_dir.is_dir() {
        results.push(CheckResult::pass(
            "hooks_directory",
            format!("Hooks directory exists: {}", hooks_dir.display()),
        ));

        // Check for expected hook scripts
        let expected_hooks = vec!["post-commit.sh", "session-start.sh", "pre-compact.sh"];

        for hook in expected_hooks {
            let hook_path = hooks_dir.join(hook);
            if hook_path.exists() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let metadata = std::fs::metadata(&hook_path)?;
                    let permissions = metadata.permissions();
                    let is_executable = permissions.mode() & 0o111 != 0;

                    if is_executable {
                        results.push(CheckResult::pass(
                            format!("hook_{}", hook),
                            format!("Hook '{}' exists and is executable", hook),
                        ));
                    } else {
                        if fix {
                            // Attempt to make executable
                            #[cfg(unix)]
                            {
                                use std::fs;
                                let mut perms = permissions;
                                perms.set_mode(perms.mode() | 0o111);
                                if let Err(e) = fs::set_permissions(&hook_path, perms) {
                                    results.push(CheckResult::warn(
                                        format!("hook_{}", hook),
                                        format!(
                                            "Hook '{}' not executable, fix failed: {}",
                                            hook, e
                                        ),
                                    ));
                                } else {
                                    results.push(CheckResult::pass(
                                        format!("hook_{}", hook),
                                        format!("Hook '{}' made executable (fixed)", hook),
                                    ));
                                }
                            }
                        } else {
                            results.push(CheckResult::warn(
                                format!("hook_{}", hook),
                                format!("Hook '{}' exists but is not executable", hook),
                            ));
                        }
                    }
                }
                #[cfg(not(unix))]
                {
                    results.push(CheckResult::pass(
                        format!("hook_{}", hook),
                        format!("Hook '{}' exists", hook),
                    ));
                }
            } else {
                results.push(CheckResult::warn(
                    format!("hook_{}", hook),
                    format!("Hook '{}' not found", hook),
                ));
            }
        }

        // Check for snapshot directory
        let snapshot_dir = PathBuf::from(".claude/context-snapshots");
        if snapshot_dir.exists() && snapshot_dir.is_dir() {
            results.push(CheckResult::pass(
                "snapshot_directory",
                "Context snapshot directory exists",
            ));
        } else {
            if fix {
                if let Err(e) = std::fs::create_dir_all(&snapshot_dir) {
                    results.push(CheckResult::warn(
                        "snapshot_directory",
                        format!("Snapshot directory missing, fix failed: {}", e),
                    ));
                } else {
                    results.push(CheckResult::pass(
                        "snapshot_directory",
                        "Snapshot directory created (fixed)",
                    ));
                }
            } else {
                results.push(CheckResult::warn(
                    "snapshot_directory",
                    "Context snapshot directory not found",
                ));
            }
        }
    } else {
        results.push(CheckResult::warn(
            "hooks_directory",
            format!("Hooks directory not found: {}", hooks_dir.display()),
        ));
    }

    Ok(results)
}

/// Check memory statistics
async fn check_memory_statistics(
    storage: &LibsqlStorage,
    _verbose: bool,
) -> Result<Vec<CheckResult>> {
    debug!("Checking memory statistics...");
    let mut results = Vec::new();

    // Get total memory count
    match StorageBackend::count_memories(storage, None).await {
        Ok(count) => {
            results.push(
                CheckResult::pass("memory_count", format!("Total memories: {}", count))
                    .with_details(serde_json::json!({ "count": count })),
            );

            if count == 0 {
                results.push(CheckResult::warn(
                    "memory_count_zero",
                    "No memories found - system may not be storing memories correctly",
                ));
            }
        }
        Err(e) => {
            results.push(
                CheckResult::fail("memory_count", "Failed to count memories")
                    .with_details(serde_json::json!({ "error": e.to_string() })),
            );
        }
    }

    // Get importance distribution
    match storage.get_importance_distribution().await {
        Ok(distribution) => {
            let critical = distribution.get(&9).unwrap_or(&0) + distribution.get(&10).unwrap_or(&0);
            let high = distribution.get(&7).unwrap_or(&0) + distribution.get(&8).unwrap_or(&0);

            results.push(
                CheckResult::pass(
                    "importance_distribution",
                    format!("Importance: {} critical, {} high", critical, high),
                )
                .with_details(serde_json::json!(distribution)),
            );
        }
        Err(e) => {
            results.push(
                CheckResult::warn(
                    "importance_distribution",
                    "Failed to get importance distribution",
                )
                .with_details(serde_json::json!({ "error": e.to_string() })),
            );
        }
    }

    Ok(results)
}

/// Check performance benchmarks
async fn check_performance(storage: &LibsqlStorage, _verbose: bool) -> Result<Vec<CheckResult>> {
    debug!("Checking performance...");
    let mut results = Vec::new();

    // Benchmark keyword search performance
    let start = Instant::now();
    match storage.keyword_search("test", None).await {
        Ok(_) => {
            let elapsed = start.elapsed();
            let elapsed_ms = elapsed.as_millis();

            if elapsed_ms < 100 {
                results.push(
                    CheckResult::pass(
                        "search_performance",
                        format!("Search performance: {}ms (target: <100ms)", elapsed_ms),
                    )
                    .with_details(serde_json::json!({
                        "elapsed_ms": elapsed_ms,
                        "target_ms": 100,
                        "ratio": 100.0 / elapsed_ms as f64,
                    })),
                );
            } else {
                results.push(
                    CheckResult::warn(
                        "search_performance",
                        format!(
                            "Search performance: {}ms (exceeds 100ms target)",
                            elapsed_ms
                        ),
                    )
                    .with_details(serde_json::json!({
                        "elapsed_ms": elapsed_ms,
                        "target_ms": 100,
                    })),
                );
            }
        }
        Err(e) => {
            results.push(
                CheckResult::fail("search_performance", "Failed to benchmark search")
                    .with_details(serde_json::json!({ "error": e.to_string() })),
            );
        }
    }

    Ok(results)
}

/// Check and cleanup stale git worktrees
async fn check_worktree_cleanup(_verbose: bool, fix: bool) -> Result<Vec<CheckResult>> {
    debug!("Checking worktree status...");
    let mut results = Vec::new();

    // Check if we're in a git repository
    let is_git_repo = std::process::Command::new("git")
        .args(["rev-parse", "--git-dir"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !is_git_repo {
        results.push(CheckResult::pass(
            "worktree_check",
            "Not in git repository, worktree cleanup not applicable",
        ));
        return Ok(results);
    }

    // Get repository root
    let repo_root = match std::env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            results.push(
                CheckResult::fail("worktree_check", "Failed to get current directory")
                    .with_details(serde_json::json!({ "error": e.to_string() })),
            );
            return Ok(results);
        }
    };

    // Initialize worktree manager
    use crate::orchestration::WorktreeManager;
    let manager = match WorktreeManager::new(repo_root) {
        Ok(m) => m,
        Err(e) => {
            results.push(
                CheckResult::fail("worktree_check", "Failed to initialize worktree manager")
                    .with_details(serde_json::json!({ "error": e.to_string() })),
            );
            return Ok(results);
        }
    };

    // List all worktrees
    let worktrees = match manager.list_worktrees() {
        Ok(wt) => wt,
        Err(e) => {
            results.push(
                CheckResult::fail("worktree_check", "Failed to list worktrees")
                    .with_details(serde_json::json!({ "error": e.to_string() })),
            );
            return Ok(results);
        }
    };

    // Count non-main worktrees (potential stale worktrees)
    let agent_worktrees: Vec<_> = worktrees.iter().filter(|wt| !wt.is_main).collect();

    if agent_worktrees.is_empty() {
        results.push(CheckResult::pass(
            "worktree_check",
            "No agent worktrees found",
        ));
        return Ok(results);
    }

    // If fix mode, cleanup all stale worktrees (no active agents)
    if fix {
        let cleaned = match manager.cleanup_stale(&[]) {
            Ok(cleaned) => cleaned,
            Err(e) => {
                results.push(
                    CheckResult::fail("worktree_cleanup", "Failed to cleanup worktrees")
                        .with_details(serde_json::json!({ "error": e.to_string() })),
                );
                return Ok(results);
            }
        };

        if cleaned.is_empty() {
            results.push(CheckResult::pass(
                "worktree_cleanup",
                "No stale worktrees to clean",
            ));
        } else {
            results.push(
                CheckResult::pass(
                    "worktree_cleanup",
                    format!("Cleaned {} stale worktree(s)", cleaned.len()),
                )
                .with_details(serde_json::json!({
                    "cleaned_count": cleaned.len(),
                })),
            );
        }
    } else {
        // Just report status
        results.push(
            CheckResult::warn(
                "worktree_check",
                format!(
                    "Found {} agent worktree(s) - run with --fix to clean up stale worktrees",
                    agent_worktrees.len()
                ),
            )
            .with_details(serde_json::json!({
                "worktree_count": agent_worktrees.len(),
                "worktrees": agent_worktrees.iter().map(|wt| {
                    serde_json::json!({
                        "path": wt.path.display().to_string(),
                        "branch": wt.branch,
                    })
                }).collect::<Vec<_>>(),
            })),
        );
    }

    Ok(results)
}

/// Print health summary to console
pub fn print_health_summary(summary: &HealthSummary, verbose: bool) {
    println!("{} Mnemosyne Health Check", crate::icons::system::gear());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    for check in &summary.checks {
        let icon = match check.status {
            CheckStatus::Pass => crate::icons::status::success(),
            CheckStatus::Warn => crate::icons::status::warning(),
            CheckStatus::Fail => crate::icons::status::error(),
        };

        let status_text = match check.status {
            CheckStatus::Pass => "PASS",
            CheckStatus::Warn => "WARN",
            CheckStatus::Fail => "FAIL",
        };

        println!("{} {} - {}", icon, check.name, status_text);

        if verbose || check.status != CheckStatus::Pass {
            println!("   {}", check.message);
            if let Some(details) = &check.details {
                if verbose {
                    println!(
                        "   Details: {}",
                        serde_json::to_string_pretty(details).unwrap()
                    );
                }
            }
        }
    }

    println!();
    println!(
        "Overall: {} ({} passed, {} warnings, {} errors)",
        match summary.status {
            CheckStatus::Pass => format!("{} HEALTHY", crate::icons::status::success()),
            CheckStatus::Warn => format!("{}  WARNINGS", crate::icons::status::warning()),
            CheckStatus::Fail => format!("{} ERRORS", crate::icons::status::error()),
        },
        summary.summary.passed,
        summary.summary.warnings,
        summary.summary.errors
    );
}

/// Check for available version updates
async fn check_version_updates(_verbose: bool) -> Result<Vec<CheckResult>> {
    debug!("Checking for version updates...");
    let mut results = Vec::new();

    // Create version checker
    let checker = match crate::version_check::VersionChecker::new() {
        Ok(c) => c,
        Err(e) => {
            results.push(CheckResult::fail(
                "version_checker_init",
                format!("Failed to initialize version checker: {}", e),
            ));
            return Ok(results);
        }
    };

    // Check all tools with 5-second timeout
    let check_future = checker.check_all_tools();
    let tool_results =
        match tokio::time::timeout(std::time::Duration::from_secs(5), check_future).await {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                results.push(CheckResult::warn(
                    "version_check_failed",
                    format!("Version check failed: {}", e),
                ));
                return Ok(results);
            }
            Err(_) => {
                results.push(CheckResult::warn(
                    "version_check_timeout",
                    "Version check timed out (network may be unavailable)".to_string(),
                ));
                return Ok(results);
            }
        };

    // Check each tool
    for info in tool_results {
        let tool_name = format!("{}_version", info.tool.name());

        if !info.is_installed {
            results.push(CheckResult::warn(
                &tool_name,
                format!("{} is not installed (optional)", info.tool.display_name()),
            ));
            continue;
        }

        if info.update_available {
            let message = if let (Some(installed), Some(latest)) = (&info.installed, &info.latest) {
                format!(
                    "{} update available: {} → {} (run 'mnemosyne update' to update)",
                    info.tool.display_name(),
                    installed,
                    latest
                )
            } else {
                format!("{} update available", info.tool.display_name())
            };

            results.push(CheckResult::warn(&tool_name, message));
        } else if let Some(version) = &info.installed {
            results.push(CheckResult::pass(
                &tool_name,
                format!("{} is up to date ({})", info.tool.display_name(), version),
            ));
        }
    }

    if results.is_empty() {
        results.push(CheckResult::pass(
            "version_checks",
            "All version checks completed".to_string(),
        ));
    }

    Ok(results)
}
