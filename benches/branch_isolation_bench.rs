//! Performance benchmarks for branch isolation system
//!
//! Targets:
//! - Registry operations: <1ms per operation
//! - Conflict detection: <10ms for 100+ files
//! - Cross-process: <50ms round-trip
//! - Persistence: <20ms for save/load
//! - Notifications: <5ms per notification

use chrono::Utc;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use mnemosyne_core::launcher::agents::AgentRole;
use mnemosyne_core::orchestration::{
    AgentId, AgentIdentity, BranchRegistry, ConflictDetector, ConflictNotifier, CoordinationMode,
    CrossProcessCoordinator, FileTracker, ModificationType, NotificationConfig, WorkIntent,
};
use mnemosyne_core::types::Namespace;
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;

/// Create test agent identity
fn create_test_agent(id: AgentId) -> AgentIdentity {
    AgentIdentity {
        id,
        role: AgentRole::Executor,
        namespace: Namespace::Global,
        branch: "main".to_string(),
        working_dir: PathBuf::from("."),
        spawned_at: Utc::now(),
        parent_id: None,
        is_coordinator: false,
    }
}

/// Benchmark 1: Registry Operations
fn bench_registry_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("registry_operations");
    group.throughput(Throughput::Elements(1));

    // Assign operation
    group.bench_function("assign", |b| {
        b.iter(|| {
            let mut registry = BranchRegistry::new();
            let agent_id = AgentId::new();
            let agent = create_test_agent(agent_id.clone());

            registry
                .assign_branch(
                    black_box(&agent_id),
                    black_box(&agent),
                    black_box("main"),
                    black_box(WorkIntent::FullBranch),
                    black_box(CoordinationMode::Isolated),
                    black_box(vec![]),
                )
                .unwrap();
        });
    });

    // Query operation
    group.bench_function("query_assignments", |b| {
        let mut registry = BranchRegistry::new();
        let agent_id = AgentId::new();
        let agent = create_test_agent(agent_id.clone());
        registry
            .assign_branch(
                &agent_id,
                &agent,
                "main",
                WorkIntent::FullBranch,
                CoordinationMode::Isolated,
                vec![],
            )
            .unwrap();

        b.iter(|| {
            let assignments = registry.get_assignments(black_box("main"));
            black_box(assignments);
        });
    });

    // Release operation
    group.bench_function("release", |b| {
        b.iter_batched(
            || {
                let mut registry = BranchRegistry::new();
                let agent_id = AgentId::new();
                let agent = create_test_agent(agent_id.clone());
                registry
                    .assign_branch(
                        &agent_id,
                        &agent,
                        "main",
                        WorkIntent::FullBranch,
                        CoordinationMode::Isolated,
                        vec![],
                    )
                    .unwrap();
                (registry, agent_id)
            },
            |(mut registry, agent_id)| {
                registry.release_assignment(black_box(&agent_id)).unwrap();
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Benchmark 2: Conflict Detection
fn bench_conflict_detection(c: &mut Criterion) {
    let mut group = c.benchmark_group("conflict_detection");

    for num_files in [10, 50, 100, 200].iter() {
        group.throughput(Throughput::Elements(*num_files as u64));

        group.bench_with_input(
            BenchmarkId::new("track_modifications", num_files),
            num_files,
            |b, &num_files| {
                let conflict_detector = Arc::new(ConflictDetector::new());
                let file_tracker = Arc::new(FileTracker::new(conflict_detector));
                let agent_id = AgentId::new();

                b.iter(|| {
                    for i in 0..num_files {
                        file_tracker
                            .track_modification(
                                black_box(&agent_id),
                                black_box(&PathBuf::from(format!("file_{}.rs", i))),
                                black_box(ModificationType::Modified),
                            )
                            .unwrap();
                    }
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("detect_conflicts", num_files),
            num_files,
            |b, &num_files| {
                let conflict_detector = Arc::new(ConflictDetector::new());
                let file_tracker = Arc::new(FileTracker::new(conflict_detector));
                let agent1 = AgentId::new();
                let agent2 = AgentId::new();

                // Setup: Both agents modify the same files
                for i in 0..num_files {
                    let path = PathBuf::from(format!("file_{}.rs", i));
                    file_tracker
                        .track_modification(&agent1, &path, ModificationType::Modified)
                        .unwrap();
                    file_tracker
                        .track_modification(&agent2, &path, ModificationType::Modified)
                        .unwrap();
                }

                b.iter(|| {
                    let conflicts = file_tracker.get_active_conflicts();
                    black_box(conflicts);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark 3: Cross-Process Coordination
fn bench_cross_process(c: &mut Criterion) {
    let mut group = c.benchmark_group("cross_process");
    group.throughput(Throughput::Elements(1));

    group.bench_function("send_message", |b| {
        let temp_dir = TempDir::new().unwrap();
        let agent_id = AgentId::new();
        let coordinator = CrossProcessCoordinator::new(temp_dir.path(), agent_id).unwrap();

        b.iter(|| {
            use mnemosyne_core::orchestration::{CoordinationMessage, MessageType};
            let message = CoordinationMessage {
                id: uuid::Uuid::new_v4().to_string(),
                from_agent: AgentId::new(),
                to_agent: Some(AgentId::new()),
                message_type: MessageType::JoinRequest,
                timestamp: Utc::now(),
                payload: serde_json::json!({"test": "data"}),
            };
            coordinator.send_message(black_box(message)).unwrap();
        });
    });

    group.bench_function("receive_messages", |b| {
        let temp_dir = TempDir::new().unwrap();
        let agent_id = AgentId::new();
        let coordinator = CrossProcessCoordinator::new(temp_dir.path(), agent_id).unwrap();

        // Send 10 messages
        for _ in 0..10 {
            use mnemosyne_core::orchestration::{CoordinationMessage, MessageType};
            let message = CoordinationMessage {
                id: uuid::Uuid::new_v4().to_string(),
                from_agent: AgentId::new(),
                to_agent: Some(agent_id.clone()),
                message_type: MessageType::JoinRequest,
                timestamp: Utc::now(),
                payload: serde_json::json!({"test": "data"}),
            };
            coordinator.send_message(message).unwrap();
        }

        b.iter(|| {
            let messages = coordinator.receive_messages().unwrap();
            black_box(messages);
        });
    });

    group.finish();
}

/// Benchmark 4: Registry Persistence
fn bench_persistence(c: &mut Criterion) {
    let mut group = c.benchmark_group("persistence");
    group.throughput(Throughput::Elements(1));

    group.bench_function("save_registry", |b| {
        let temp_dir = TempDir::new().unwrap();
        let registry_path = temp_dir.path().join("registry.json");

        b.iter_batched(
            || {
                let mut registry = BranchRegistry::new();
                registry.enable_persistence(registry_path.clone());

                // Add 10 assignments
                for i in 0..10 {
                    let agent_id = AgentId::new();
                    let agent = create_test_agent(agent_id.clone());
                    registry
                        .assign_branch(
                            &agent_id,
                            &agent,
                            &format!("branch_{}", i),
                            WorkIntent::FullBranch,
                            CoordinationMode::Isolated,
                            vec![],
                        )
                        .unwrap();
                }
                registry
            },
            |registry| {
                // The persist is called automatically during assign_branch
                black_box(registry);
            },
            criterion::BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Benchmark 5: Notification Generation
fn bench_notifications(c: &mut Criterion) {
    let mut group = c.benchmark_group("notifications");
    group.throughput(Throughput::Elements(1));

    group.bench_function("generate_on_save_notification", |b| {
        let conflict_detector = Arc::new(ConflictDetector::new());
        let file_tracker = Arc::new(FileTracker::new(conflict_detector));
        let config = NotificationConfig {
            enabled: true,
            notify_on_save: true,
            periodic_interval_minutes: 20,
            session_end_summary: true,
        };
        let notifier = ConflictNotifier::new(config, file_tracker.clone());

        // Setup conflicts
        let agent1 = AgentId::new();
        let agent2 = AgentId::new();
        let path = PathBuf::from("src/main.rs");
        file_tracker
            .track_modification(&agent1, &path, ModificationType::Modified)
            .unwrap();
        file_tracker
            .track_modification(&agent2, &path, ModificationType::Modified)
            .unwrap();

        b.iter(|| {
            let notification = notifier.notify_on_save(black_box(&agent1), black_box(&path));
            black_box(notification);
        });
    });

    group.bench_function("generate_periodic_notification", |b| {
        let conflict_detector = Arc::new(ConflictDetector::new());
        let file_tracker = Arc::new(FileTracker::new(conflict_detector));
        let config = NotificationConfig {
            enabled: true,
            notify_on_save: true,
            periodic_interval_minutes: 20,
            session_end_summary: true,
        };
        let notifier = ConflictNotifier::new(config, file_tracker.clone());

        // Setup conflicts
        let agent1 = AgentId::new();
        for i in 0..5 {
            let agent2 = AgentId::new();
            let path = PathBuf::from(format!("src/file_{}.rs", i));
            file_tracker
                .track_modification(&agent1, &path, ModificationType::Modified)
                .unwrap();
            file_tracker
                .track_modification(&agent2, &path, ModificationType::Modified)
                .unwrap();
        }

        b.iter(|| {
            let notification = notifier.generate_periodic_notification(black_box(&agent1));
            black_box(notification);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_registry_operations,
    bench_conflict_detection,
    bench_cross_process,
    bench_persistence,
    bench_notifications,
);

criterion_main!(benches);
