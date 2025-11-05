//! E2E Integration Test: Multi-Instance Branch Isolation
//!
//! This test simulates two concurrent mnemosyne instances working on different
//! branches to verify complete isolation. It validates the core use case that
//! motivated the worktree isolation feature.
//!
//! # Test Scenario
//!
//! 1. Two "instances" (simulated as test agents) start simultaneously
//! 2. Instance A works on feature-alpha branch
//! 3. Instance B works on feature-beta branch
//! 4. Both make independent git operations (commits, branch switches)
//! 5. Verify neither instance is affected by the other's operations
//! 6. Cleanup both worktrees successfully

use mnemosyne_core::orchestration::{identity::AgentId, WorktreeManager};
use std::fs;
use std::process::Command;
use tempfile::TempDir;

/// Setup a realistic test repository with history
fn setup_realistic_repo() -> (TempDir, WorktreeManager) {
    let temp_dir = tempfile::tempdir().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to init git");

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to set git email");

    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to set git name");

    // Create initial commit on main
    fs::write(repo_path.join("README.md"), "# Test Project\n\nInitial commit.")
        .expect("Failed to write README");

    fs::create_dir_all(repo_path.join("src")).expect("Failed to create src dir");
    fs::write(repo_path.join("src/main.rs"), "fn main() {\n    println!(\"v1.0\");\n}\n")
        .expect("Failed to write main.rs");

    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .expect("Failed to git add");

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to commit");

    // Create base branches (simulating existing project state)
    Command::new("git")
        .args(["branch", "feature-alpha"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to create branch");

    Command::new("git")
        .args(["branch", "feature-beta"])
        .current_dir(repo_path)
        .output()
        .expect("Failed to create branch");

    let manager = WorktreeManager::new(repo_path.to_path_buf())
        .expect("Failed to create worktree manager");

    (temp_dir, manager)
}

/// Get current commit hash for a worktree
fn get_commit_hash(worktree_path: &std::path::Path) -> String {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(worktree_path)
        .output()
        .expect("Failed to get commit hash");

    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Get current branch name for a worktree
fn get_branch_name(worktree_path: &std::path::Path) -> String {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(worktree_path)
        .output()
        .expect("Failed to get branch name");

    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

#[test]
fn test_e2e_multi_instance_complete_isolation() {
    println!("\n=== E2E Test: Multi-Instance Branch Isolation ===\n");

    let (temp_dir, manager) = setup_realistic_repo();
    let repo_path = temp_dir.path();

    // === PHASE 1: Instance Startup ===
    println!("Phase 1: Starting two instances...");

    let instance_a = AgentId::new();
    let instance_b = AgentId::new();

    let worktree_a = manager
        .create_worktree(&instance_a, "feature-alpha")
        .expect("Failed to create worktree A");

    let worktree_b = manager
        .create_worktree(&instance_b, "feature-beta")
        .expect("Failed to create worktree B");

    println!("  ✓ Instance A: {} on feature-alpha", instance_a);
    println!("  ✓ Instance B: {} on feature-beta", instance_b);

    // Verify initial state
    assert_eq!(get_branch_name(&worktree_a), "feature-alpha");
    assert_eq!(get_branch_name(&worktree_b), "feature-beta");
    println!("  ✓ Both instances on correct branches\n");

    // === PHASE 2: Instance A Makes Changes ===
    println!("Phase 2: Instance A makes changes on feature-alpha...");

    fs::write(
        worktree_a.join("feature-a.txt"),
        "Feature A implementation",
    )
    .expect("Failed to write A file");

    Command::new("git")
        .args(["add", "."])
        .current_dir(&worktree_a)
        .output()
        .expect("Failed to git add in A");

    Command::new("git")
        .args(["commit", "-m", "Add feature A"])
        .current_dir(&worktree_a)
        .output()
        .expect("Failed to commit in A");

    let commit_a_after = get_commit_hash(&worktree_a);
    let commit_b_after = get_commit_hash(&worktree_b);

    println!("  ✓ Instance A committed changes");
    println!("  ✓ Instance A commit: {}", &commit_a_after[..8]);

    // Critical assertion: B should NOT see A's commit
    assert_ne!(
        commit_a_after, commit_b_after,
        "Instance B should not see Instance A's commits"
    );
    println!("  ✓ Instance B unaffected (different commit hash)\n");

    // === PHASE 3: Instance B Makes Independent Changes ===
    println!("Phase 3: Instance B makes independent changes on feature-beta...");

    fs::write(
        worktree_b.join("feature-b.txt"),
        "Feature B implementation",
    )
    .expect("Failed to write B file");

    Command::new("git")
        .args(["add", "."])
        .current_dir(&worktree_b)
        .output()
        .expect("Failed to git add in B");

    Command::new("git")
        .args(["commit", "-m", "Add feature B"])
        .current_dir(&worktree_b)
        .output()
        .expect("Failed to commit in B");

    let commit_b_final = get_commit_hash(&worktree_b);
    let commit_a_final = get_commit_hash(&worktree_a);

    println!("  ✓ Instance B committed changes");
    println!("  ✓ Instance B commit: {}", &commit_b_final[..8]);

    // Critical assertion: A should NOT see B's changes
    assert_eq!(
        commit_a_final, commit_a_after,
        "Instance A should not see Instance B's commits"
    );
    println!("  ✓ Instance A unaffected (commit unchanged)\n");

    // === PHASE 4: Verify File Isolation ===
    println!("Phase 4: Verifying file isolation...");

    assert!(
        worktree_a.join("feature-a.txt").exists(),
        "Instance A should have its file"
    );
    assert!(
        !worktree_a.join("feature-b.txt").exists(),
        "Instance A should NOT see Instance B's files"
    );

    assert!(
        worktree_b.join("feature-b.txt").exists(),
        "Instance B should have its file"
    );
    assert!(
        !worktree_b.join("feature-a.txt").exists(),
        "Instance B should NOT see Instance A's files"
    );

    println!("  ✓ Instance A has feature-a.txt only");
    println!("  ✓ Instance B has feature-b.txt only");
    println!("  ✓ Complete file isolation confirmed\n");

    // === PHASE 5: Branch Switch in Instance A ===
    println!("Phase 5: Instance A switches branches...");

    // Create and switch to a new branch (can't switch to main - it's used by main worktree)
    let output = Command::new("git")
        .args(["switch", "-c", "feature-gamma"])
        .current_dir(&worktree_a)
        .output()
        .expect("Failed to switch to feature-gamma in A");

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Git switch failed: {}", stderr);
        panic!("Failed to switch branches");
    }

    let branch_a_switched = get_branch_name(&worktree_a);
    let branch_b_unchanged = get_branch_name(&worktree_b);

    assert_eq!(branch_a_switched, "feature-gamma", "Instance A should be on feature-gamma");
    assert_eq!(
        branch_b_unchanged, "feature-beta",
        "Instance B should still be on feature-beta"
    );

    println!("  ✓ Instance A switched to feature-gamma");
    println!("  ✓ Instance B still on feature-beta (unaffected)");
    println!("  ✓ Branch operations are isolated\n");

    // === PHASE 6: Verify Main Worktree Unaffected ===
    println!("Phase 6: Verifying main worktree unaffected...");

    let main_branch = get_branch_name(repo_path);
    assert_eq!(
        main_branch, "main",
        "Main worktree should still be on main"
    );

    let main_has_feature_a = repo_path.join("feature-a.txt").exists();
    let main_has_feature_b = repo_path.join("feature-b.txt").exists();

    assert!(
        !main_has_feature_a,
        "Main worktree should not have feature A file"
    );
    assert!(
        !main_has_feature_b,
        "Main worktree should not have feature B file"
    );

    println!("  ✓ Main worktree on main branch");
    println!("  ✓ Main worktree unaffected by instance operations\n");

    // === PHASE 7: Cleanup ===
    println!("Phase 7: Cleaning up worktrees...");

    manager
        .remove_worktree(&instance_a)
        .expect("Failed to cleanup worktree A");
    manager
        .remove_worktree(&instance_b)
        .expect("Failed to cleanup worktree B");

    assert!(!worktree_a.exists(), "Worktree A should be removed");
    assert!(!worktree_b.exists(), "Worktree B should be removed");

    println!("  ✓ Both worktrees cleaned up successfully\n");

    // === SUCCESS ===
    println!("=== ✅ E2E Test PASSED: Complete Isolation Verified ===");
    println!("\nKey Validations:");
    println!("  • Two instances worked on different branches simultaneously");
    println!("  • Commits in one instance didn't affect the other");
    println!("  • File changes were completely isolated");
    println!("  • Branch operations were independent");
    println!("  • Main worktree remained untouched");
    println!("  • Cleanup succeeded without errors");
    println!("\nThis confirms the worktree isolation system works correctly");
    println!("for real-world multi-instance scenarios.\n");
}

#[test]
fn test_e2e_edge_case_nonexistent_branch() {
    println!("\n=== E2E Edge Case: Non-Existent Branch ===\n");

    let (temp_dir, manager) = setup_realistic_repo();

    let instance_id = AgentId::new();

    println!("Attempting to create worktree on non-existent branch...");

    // This should auto-create the branch
    let worktree = manager
        .create_worktree(&instance_id, "feature-new")
        .expect("Failed to create worktree with new branch");

    println!("  ✓ Worktree created successfully");

    // Verify branch was created
    let branch = get_branch_name(&worktree);
    assert_eq!(branch, "feature-new");

    println!("  ✓ Branch 'feature-new' was auto-created");
    println!("  ✓ Worktree is on correct branch");

    // Cleanup
    manager
        .remove_worktree(&instance_id)
        .expect("Failed to cleanup");

    println!("\n=== ✅ Edge Case Test PASSED ===\n");
}

#[test]
fn test_e2e_edge_case_stale_worktree() {
    println!("\n=== E2E Edge Case: Stale Worktree ===\n");

    let (temp_dir, manager) = setup_realistic_repo();

    let instance_id = AgentId::new();

    println!("Creating initial worktree...");
    let worktree_path = manager
        .create_worktree(&instance_id, "feature-alpha")
        .expect("Failed to create worktree");

    println!("  ✓ Worktree created at: {}", worktree_path.display());

    // Simulate crash - don't clean up properly
    println!("\nSimulating crash (no cleanup)...");
    println!("Attempting to create worktree again with same agent ID...");

    // This should detect existing worktree and clean it up
    let worktree_path_2 = manager
        .create_worktree(&instance_id, "feature-alpha")
        .expect("Failed to recreate worktree");

    println!("  ✓ Worktree recreated successfully");
    assert_eq!(worktree_path, worktree_path_2);
    println!("  ✓ Same path reused after cleanup");

    // Verify it's functional
    assert_eq!(get_branch_name(&worktree_path_2), "feature-alpha");
    println!("  ✓ Recreated worktree is functional");

    // Cleanup
    manager
        .remove_worktree(&instance_id)
        .expect("Failed to cleanup");

    println!("\n=== ✅ Edge Case Test PASSED ===\n");
}
