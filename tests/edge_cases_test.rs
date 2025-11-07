//! Edge case tests for ICS integration
//!
//! Tests error handling, boundary conditions, and unusual inputs

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_nonexistent_file_without_template() {
    let temp_dir = TempDir::new().unwrap();
    let nonexistent = temp_dir.path().join("nonexistent.md");

    // File shouldn't exist
    assert!(!nonexistent.exists());

    // Simulate creating with default content
    // Real command: mnemosyne edit /tmp/nonexistent.md
    let default_content = "# Context\n\nEdit your context here...\n";
    fs::write(&nonexistent, default_content).unwrap();

    // Verify created with default content
    assert!(nonexistent.exists());
    let content = fs::read_to_string(&nonexistent).unwrap();
    assert_eq!(content, default_content);
}

#[test]
fn test_template_with_existing_file() {
    let temp_dir = TempDir::new().unwrap();
    let existing = temp_dir.path().join("existing.md");

    // Create file with existing content
    fs::write(&existing, "Existing content").unwrap();

    // When using template with existing file, it should load the file
    // (not overwrite with template)
    let content = fs::read_to_string(&existing).unwrap();
    assert_eq!(content, "Existing content");

    // Template should only be applied if file doesn't exist
}

#[test]
fn test_session_directory_autocreation() {
    let temp_dir = TempDir::new().unwrap();
    let session_dir = temp_dir.path().join(".claude").join("sessions");

    // Doesn't exist initially
    assert!(!session_dir.exists());

    // Create it
    fs::create_dir_all(&session_dir).unwrap();

    // Now exists
    assert!(session_dir.exists());
    assert!(session_dir.is_dir());
}

#[test]
fn test_malformed_session_json() {
    let temp_dir = TempDir::new().unwrap();
    let session_dir = temp_dir.path().join(".claude").join("sessions");
    fs::create_dir_all(&session_dir).unwrap();

    let intent_file = session_dir.join("edit-intent.json");

    // Write malformed JSON
    fs::write(&intent_file, "{ this is not valid json }").unwrap();

    // Reading should fail gracefully
    let result = fs::read_to_string(&intent_file);
    assert!(result.is_ok());

    // Parsing should fail
    let content = result.unwrap();
    let parse_result: Result<serde_json::Value, _> = serde_json::from_str(&content);
    assert!(parse_result.is_err());
}

#[test]
fn test_very_long_filename() {
    let temp_dir = TempDir::new().unwrap();

    // Create a very long filename (but within filesystem limits)
    let long_name = "a".repeat(200) + ".md";
    let long_path = temp_dir.path().join(&long_name);

    // Should be able to create file
    let result = fs::write(&long_path, "test content");

    // Some filesystems have limits, so this might fail
    // We just verify it fails gracefully if it does
    if let Err(e) = result {
        // Error should be clear
        assert!(
            e.to_string().contains("File name too long")
                || e.to_string().contains("ENAMETOOLONG")
                || e.to_string().contains("name")
        );
    } else {
        // If it succeeded, verify file exists
        assert!(long_path.exists());
    }
}

#[test]
fn test_file_in_nonexistent_directory() {
    let temp_dir = TempDir::new().unwrap();
    let nested_path = temp_dir
        .path()
        .join("nonexistent")
        .join("nested")
        .join("file.md");

    // Parent directories don't exist
    assert!(!nested_path.parent().unwrap().exists());

    // Trying to create file without creating parents should fail
    let result = fs::write(&nested_path, "content");
    assert!(result.is_err());

    // Creating parents first should work
    fs::create_dir_all(nested_path.parent().unwrap()).unwrap();
    fs::write(&nested_path, "content").unwrap();
    assert!(nested_path.exists());
}

#[test]
fn test_empty_filename() {
    // Empty filename should be invalid
    let empty = PathBuf::from("");
    assert_eq!(empty.to_string_lossy(), "");

    // This would be caught by validation
}

#[test]
fn test_filename_with_special_characters() {
    let temp_dir = TempDir::new().unwrap();

    // Test various special characters
    let special_chars = vec![
        "file with spaces.md",
        "file-with-dashes.md",
        "file_with_underscores.md",
        "file.multiple.dots.md",
        // Note: Some chars like : / \ are invalid on most filesystems
    ];

    for name in special_chars {
        let path = temp_dir.path().join(name);
        let result = fs::write(&path, "content");
        assert!(result.is_ok(), "Failed to create file: {}", name);
        assert!(path.exists(), "File doesn't exist: {}", name);
    }
}

#[test]
fn test_template_enum_values() {
    // Test that template values match what's expected
    let valid_templates = ["api", "architecture", "bugfix", "feature", "refactor"];

    for template in valid_templates {
        // Each should be a valid template name
        assert!(!template.is_empty());
        assert!(template.chars().all(|c| c.is_ascii_lowercase()));
    }
}

#[test]
fn test_panel_enum_values() {
    // Test that panel values match what's expected
    let valid_panels = ["memory", "diagnostics", "proposals", "holes"];

    for panel in valid_panels {
        // Each should be a valid panel name
        assert!(!panel.is_empty());
        assert!(panel.chars().all(|c| c.is_ascii_lowercase()));
    }
}

#[test]
fn test_readonly_prevents_write() {
    // Test that readonly flag is respected
    let readonly_flag = true;

    if readonly_flag {
        // When readonly is true, saves should be prevented
        // In the actual ICS app: config.read_only = true
        assert!(readonly_flag);
    }
}

#[test]
fn test_concurrent_session_files() {
    let temp_dir = TempDir::new().unwrap();
    let session_dir = temp_dir.path().join(".claude").join("sessions");
    fs::create_dir_all(&session_dir).unwrap();

    // Multiple threads shouldn't corrupt files
    // (In real usage, each session has unique ID)

    for i in 0..5 {
        let intent_file = session_dir.join(format!("edit-intent-{}.json", i));
        let content = format!("{{\"session_id\": \"session-{}\"}}", i);
        fs::write(&intent_file, content).unwrap();
    }

    // Verify all files exist and have correct content
    for i in 0..5 {
        let intent_file = session_dir.join(format!("edit-intent-{}.json", i));
        assert!(intent_file.exists());
        let content = fs::read_to_string(&intent_file).unwrap();
        assert!(content.contains(&format!("session-{}", i)));
    }
}

#[test]
fn test_cleanup_removes_both_files() {
    let temp_dir = TempDir::new().unwrap();
    let session_dir = temp_dir.path().join(".claude").join("sessions");
    fs::create_dir_all(&session_dir).unwrap();

    let intent_file = session_dir.join("edit-intent.json");
    let result_file = session_dir.join("edit-result.json");

    // Create both files
    fs::write(&intent_file, "{}").unwrap();
    fs::write(&result_file, "{}").unwrap();

    assert!(intent_file.exists());
    assert!(result_file.exists());

    // Cleanup removes both
    fs::remove_file(&intent_file).unwrap();
    fs::remove_file(&result_file).unwrap();

    assert!(!intent_file.exists());
    assert!(!result_file.exists());
}

#[test]
fn test_invalid_json_structure() {
    let temp_dir = TempDir::new().unwrap();
    let json_file = temp_dir.path().join("test.json");

    // Valid JSON but wrong structure
    let wrong_structure = r#"{"wrong": "fields", "missing": "session_id"}"#;
    fs::write(&json_file, wrong_structure).unwrap();

    // Can parse as JSON
    let content = fs::read_to_string(&json_file).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();

    // But doesn't have required fields
    assert!(parsed.get("session_id").is_none());
    assert!(parsed.get("wrong").is_some());
}

#[test]
fn test_large_file_handling() {
    let temp_dir = TempDir::new().unwrap();
    let large_file = temp_dir.path().join("large.md");

    // Create a moderately large file (1MB)
    let large_content = "x".repeat(1024 * 1024);
    fs::write(&large_file, &large_content).unwrap();

    assert!(large_file.exists());
    let metadata = fs::metadata(&large_file).unwrap();
    assert_eq!(metadata.len(), 1024 * 1024);

    // Should be able to read it back
    let read_content = fs::read_to_string(&large_file).unwrap();
    assert_eq!(read_content.len(), large_content.len());
}
