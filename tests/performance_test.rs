//! Performance and reliability tests for ICS integration
//!
//! Tests stress conditions, throughput, and resource usage

use std::fs;
use std::time::{Duration, Instant};
use tempfile::TempDir;

#[test]
fn test_rapid_file_creation() {
    let temp_dir = TempDir::new().unwrap();

    let start = Instant::now();

    // Create 100 files rapidly
    for i in 0..100 {
        let file_path = temp_dir.path().join(format!("file_{}.md", i));
        let content = format!("# File {}\n\nContent for file {}", i, i);
        fs::write(&file_path, content).unwrap();
        assert!(file_path.exists());
    }

    let elapsed = start.elapsed();

    // Should complete in under 1 second on modern hardware
    assert!(elapsed < Duration::from_secs(1),
            "File creation too slow: {:?}", elapsed);

    println!("Created 100 files in {:?} ({:.2} files/sec)",
             elapsed, 100.0 / elapsed.as_secs_f64());
}

#[test]
fn test_large_json_serialization() {
    use serde_json::json;

    // Create large JSON structure
    let large_json = json!({
        "session_id": "test-123",
        "timestamp": "2025-11-04T20:00:00Z",
        "action": "edit",
        "file_path": "/tmp/test.md",
        "template": "feature",
        "readonly": false,
        "panel": "memory",
        "context": {
            "conversation_summary": "x".repeat(10000), // 10KB summary
            "relevant_memories": (0..100).map(|i| format!("mem_{}", i)).collect::<Vec<_>>(),
            "related_files": (0..50).map(|i| format!("file_{}.rs", i)).collect::<Vec<_>>(),
        }
    });

    let start = Instant::now();

    // Serialize
    let json_str = serde_json::to_string(&large_json).unwrap();
    let serialize_time = start.elapsed();

    // Deserialize
    let start = Instant::now();
    let _parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
    let deserialize_time = start.elapsed();

    println!("JSON size: {} bytes", json_str.len());
    println!("Serialization: {:?}", serialize_time);
    println!("Deserialization: {:?}", deserialize_time);

    // Should be fast (< 10ms for this size)
    assert!(serialize_time < Duration::from_millis(10));
    assert!(deserialize_time < Duration::from_millis(10));
}

#[test]
fn test_session_directory_stress() {
    let temp_dir = TempDir::new().unwrap();
    let session_dir = temp_dir.path().join(".claude").join("sessions");
    fs::create_dir_all(&session_dir).unwrap();

    let start = Instant::now();

    // Rapidly create and delete coordination files
    for i in 0..50 {
        let intent_file = session_dir.join(format!("edit-intent-{}.json", i));
        let result_file = session_dir.join(format!("edit-result-{}.json", i));

        // Write
        fs::write(&intent_file, "{}").unwrap();
        fs::write(&result_file, "{}").unwrap();

        // Verify
        assert!(intent_file.exists());
        assert!(result_file.exists());

        // Delete
        fs::remove_file(&intent_file).unwrap();
        fs::remove_file(&result_file).unwrap();

        // Verify cleanup
        assert!(!intent_file.exists());
        assert!(!result_file.exists());
    }

    let elapsed = start.elapsed();
    println!("50 create-verify-delete cycles in {:?}", elapsed);

    // Should be fast
    assert!(elapsed < Duration::from_secs(1));
}

#[test]
fn test_template_content_access_performance() {
    // Simulate accessing template content many times
    let templates = ["api", "architecture", "bugfix", "feature", "refactor"];

    let start = Instant::now();

    for _ in 0..10000 {
        for template in &templates {
            // Simulate template lookup and access
            let _content = match *template {
                "api" => "# API Design Context\n...",
                "architecture" => "# Architecture Decision\n...",
                "bugfix" => "# Bug Fix Context\n...",
                "feature" => "# Feature Implementation\n...",
                "refactor" => "# Refactoring Context\n...",
                _ => unreachable!(),
            };
        }
    }

    let elapsed = start.elapsed();
    println!("10000 template lookups in {:?}", elapsed);

    // Should be nearly instant (all in-memory)
    assert!(elapsed < Duration::from_millis(100));
}

#[test]
fn test_concurrent_file_reads() {
    use std::sync::Arc;
    use std::thread;

    let temp_dir = TempDir::new().unwrap();
    let test_file = temp_dir.path().join("shared.md");

    // Create a file to read
    fs::write(&test_file, "# Shared Content\n\nThis is shared.").unwrap();

    let test_file = Arc::new(test_file);
    let start = Instant::now();

    // Spawn 10 threads that all read the same file
    let handles: Vec<_> = (0..10)
        .map(|_i| {
            let file = Arc::clone(&test_file);
            thread::spawn(move || {
                for _ in 0..100 {
                    let content = fs::read_to_string(file.as_ref()).unwrap();
                    assert!(content.contains("Shared Content"));
                }
            })
        })
        .collect();

    // Wait for all threads
    for handle in handles {
        handle.join().unwrap();
    }

    let elapsed = start.elapsed();
    println!("10 threads × 100 reads in {:?}", elapsed);

    // Should complete reasonably fast
    assert!(elapsed < Duration::from_secs(5));
}

#[test]
fn test_memory_efficiency_large_content() {
    let temp_dir = TempDir::new().unwrap();
    let large_file = temp_dir.path().join("large.md");

    // Create 10MB file
    let large_content = "x".repeat(10 * 1024 * 1024);

    let start = Instant::now();
    fs::write(&large_file, &large_content).unwrap();
    let write_time = start.elapsed();

    let start = Instant::now();
    let read_content = fs::read_to_string(&large_file).unwrap();
    let read_time = start.elapsed();

    assert_eq!(read_content.len(), large_content.len());

    println!("10MB file write: {:?}", write_time);
    println!("10MB file read: {:?}", read_time);

    // Should handle large files efficiently (< 1 second each)
    assert!(write_time < Duration::from_secs(1));
    assert!(read_time < Duration::from_secs(1));
}

#[test]
fn test_pathbuf_operations_performance() {
    let temp_dir = TempDir::new().unwrap();

    let start = Instant::now();

    // Create and manipulate many PathBufs
    for i in 0..10000 {
        let path = temp_dir.path().join(format!("dir{}", i)).join("subdir").join("file.md");
        let _parent = path.parent();
        let _filename = path.file_name();
        let _extension = path.extension();
        let _as_str = path.to_string_lossy();
    }

    let elapsed = start.elapsed();
    println!("10000 PathBuf operations in {:?}", elapsed);

    // Should be fast (all in-memory)
    assert!(elapsed < Duration::from_millis(100));
}

#[test]
fn test_json_parse_error_recovery_performance() {
    // Test that we can quickly detect and recover from malformed JSON
    let bad_jsons = vec![
        "{ invalid json }",
        "{",
        "}",
        "{}}}",
        "{\"incomplete\": ",
        "{\"key\": }",
        "[1, 2, ",
    ];

    let start = Instant::now();

    for bad_json in bad_jsons {
        let result: Result<serde_json::Value, _> = serde_json::from_str(bad_json);
        // Should fail fast
        assert!(result.is_err(), "Expected error for: {}", bad_json);
    }

    let elapsed = start.elapsed();
    println!("7 malformed JSON errors detected in {:?}", elapsed);

    // Error detection should be nearly instant
    assert!(elapsed < Duration::from_millis(10));
}

#[test]
fn test_directory_creation_idempotency() {
    let temp_dir = TempDir::new().unwrap();
    let nested_dir = temp_dir.path()
        .join("level1")
        .join("level2")
        .join("level3")
        .join("level4");

    let start = Instant::now();

    // Create directory structure 100 times (should be idempotent)
    for _ in 0..100 {
        fs::create_dir_all(&nested_dir).unwrap();
        assert!(nested_dir.exists());
    }

    let elapsed = start.elapsed();
    println!("100 idempotent directory creations in {:?}", elapsed);

    // Should be fast (no-op after first)
    assert!(elapsed < Duration::from_millis(500));
}
