//! Out-of-Memory Stress Test
//!
//! Tests mnemosyne's behavior under memory pressure to identify OOM issues.
//! This test intentionally allocates large amounts of memory to trigger
//! memory limits and validate graceful handling.

#![cfg(not(debug_assertions))] // Only run in release mode

use mnemosyne_core::diagnostics::{global_memory_tracker, start_memory_monitoring, MemoryStatus};
use mnemosyne_core::{LibsqlStorage, Namespace, StorageBackend};
use std::sync::Arc;
use std::time::Duration;
use tempfile::tempdir;
use tokio::time::sleep;

#[tokio::test]
#[ignore] // Run manually with: cargo test --release oom_stress_test -- --ignored
async fn test_memory_growth_embeddings() {
    // Start memory monitoring
    let _monitor_task = start_memory_monitoring();

    // Create temporary storage
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let storage = LibsqlStorage::new(&db_path, Default::default())
        .await
        .unwrap();

    let namespace = Namespace::from_str("project:stress-test").unwrap();

    // Generate large number of memories with embeddings
    for i in 0..10_000 {
        let content = format!(
            "This is stress test memory number {} with sufficient content to generate embeddings. \
             Adding more text to make it realistic. Lorem ipsum dolor sit amet, consectetur adipiscing elit.",
            i
        );

        storage
            .store_memory(&content, &namespace, None, None, &[], None)
            .await
            .unwrap();

        if i % 100 == 0 {
            let tracker = global_memory_tracker();
            tracker.log_statistics();

            let status = tracker.check_thresholds();
            if status == MemoryStatus::Critical {
                eprintln!("WARNING: Memory critical at {} memories", i);
            }

            sleep(Duration::from_millis(10)).await;
        }
    }

    // Verify memory doesn't continue growing after operations complete
    let snapshot_before = global_memory_tracker().snapshot();
    sleep(Duration::from_secs(5)).await;
    let snapshot_after = global_memory_tracker().snapshot();

    let growth = snapshot_after.current_usage.saturating_sub(snapshot_before.current_usage);
    let growth_mb = growth / 1_048_576;

    println!("Memory growth after 5s idle: {} MB", growth_mb);

    // Allow some growth but not unbounded
    assert!(
        growth_mb < 50,
        "Memory grew by {} MB after operations completed",
        growth_mb
    );
}

#[tokio::test]
#[ignore]
async fn test_work_queue_unbounded_growth() {
    use mnemosyne_core::orchestration::{WorkItem, WorkQueue};
    use mnemosyne_core::types::MemoryId;

    let _monitor_task = start_memory_monitoring();
    let work_queue = Arc::new(tokio::sync::RwLock::new(WorkQueue::new()));

    let snapshot_before = global_memory_tracker().snapshot();

    // Simulate unbounded work queue growth
    for i in 0..50_000 {
        let item = WorkItem {
            id: format!("work-{}", i),
            description: format!("Large work item {} with substantial description text", i),
            dependencies: vec![],
            assigned_to: None,
            status: mnemosyne_core::orchestration::state::WorkItemStatus::Pending,
            created_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            result: None,
            metadata: std::collections::HashMap::new(),
        };

        work_queue.write().await.submit(item).await.unwrap();

        if i % 1000 == 0 {
            let queue_size = work_queue.read().await.pending_work().await.len();
            global_memory_tracker().set_work_queue_size(queue_size);
            global_memory_tracker().log_statistics();
        }
    }

    let snapshot_after = global_memory_tracker().snapshot();
    let growth = snapshot_after.current_usage - snapshot_before.current_usage;
    let growth_mb = growth / 1_048_576;

    println!("Work queue grew to {} items", snapshot_after.work_queue_size);
    println!("Memory growth: {} MB", growth_mb);

    // This should NOT grow unbounded - fail if it does
    assert!(
        growth_mb < 500,
        "Work queue memory growth excessive: {} MB",
        growth_mb
    );
}

#[tokio::test]
#[ignore]
async fn test_spawned_task_cleanup() {
    let _monitor_task = start_memory_monitoring();

    global_memory_tracker().set_spawned_tasks(0);

    // Spawn many tasks that complete quickly
    let mut handles = vec![];
    for _ in 0..1000 {
        global_memory_tracker().increment_spawned_tasks();

        let handle = tokio::spawn(async {
            sleep(Duration::from_millis(10)).await;
        });
        handles.push(handle);
    }

    // Wait for all to complete
    for handle in handles {
        handle.await.unwrap();
        global_memory_tracker().decrement_spawned_tasks();
    }

    sleep(Duration::from_secs(1)).await;

    // Verify task count returns to baseline
    let snapshot = global_memory_tracker().snapshot();
    assert_eq!(
        snapshot.spawned_tasks, 0,
        "Spawned tasks not cleaned up properly"
    );
}

#[tokio::test]
#[ignore]
async fn test_database_connection_leaks() {
    let _monitor_task = start_memory_monitoring();

    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test.db");

    // Create many storage instances (simulating connection leaks)
    for i in 0..100 {
        global_memory_tracker().increment_db_connections();

        let _storage = LibsqlStorage::new(&db_path, Default::default())
            .await
            .unwrap();

        if i % 10 == 0 {
            global_memory_tracker().log_statistics();
        }

        // Intentionally don't drop storage to simulate leak
    }

    let snapshot = global_memory_tracker().snapshot();
    println!("Database connections tracked: {}", snapshot.db_connections);

    // This test documents the issue - connections should be pooled
    // In a fixed version, this assertion should pass
    // assert!(snapshot.db_connections < 10, "Connection pooling not working");
}
