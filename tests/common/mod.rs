//! Common test utilities and helpers

use mnemosyne_core::{
    ConfigManager, ConnectionMode, LibsqlStorage, LlmConfig, LlmService, MemoryId, MemoryNote,
    MemoryType, Namespace,
};
use std::sync::Arc;
use tempfile::TempDir;
use uuid;

/// Create an in-memory LibSQL storage for testing
pub async fn create_test_storage() -> LibsqlStorage {
    // Use a temporary file instead of :memory: because libSQL's :memory: mode
    // creates isolated databases per connection, so migrations wouldn't persist
    let temp_file = format!("/tmp/mnemosyne_test_{}.db", uuid::Uuid::new_v4());
    let storage = LibsqlStorage::new_with_validation(
        ConnectionMode::Local(temp_file.clone()),
        true, // create_if_missing - required for test databases
    )
    .await
    .expect("Failed to create test storage");

    // Clean up temp file on drop would be ideal, but for now tests are fast enough
    // that OS cleanup is fine

    storage
}

/// Create a test LLM service with empty API key (for non-LLM tests)
pub fn create_test_llm_service() -> Arc<LlmService> {
    Arc::new(
        LlmService::new(LlmConfig {
            api_key: String::new(),
            model: "claude-haiku-4-5-20251001".to_string(),
            max_tokens: 1024,
            temperature: 0.7,
        })
        .expect("Failed to create test LLM service"),
    )
}

/// Create a test LLM service with real API key from environment
pub fn create_real_llm_service() -> Option<Arc<LlmService>> {
    ConfigManager::new()
        .ok()
        .and_then(|cm| cm.get_api_key().ok())
        .and_then(|key| {
            LlmService::new(LlmConfig {
                api_key: key,
                model: "claude-haiku-4-5-20251001".to_string(),
                max_tokens: 1024,
                temperature: 0.7,
            })
            .ok()
        })
        .map(Arc::new)
}

/// Create a temporary directory with a git repo and CLAUDE.md
pub fn create_test_project(name: &str) -> TempDir {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");

    // Initialize git repo
    std::process::Command::new("git")
        .args(["init"])
        .current_dir(temp_dir.path())
        .output()
        .expect("Failed to init git repo");

    // Create CLAUDE.md with project name
    let claude_md_content = format!(
        r#"---
project: {}
description: Test project for integration testing
---

# {}

This is a test project.
"#,
        name, name
    );

    std::fs::write(temp_dir.path().join("CLAUDE.md"), claude_md_content)
        .expect("Failed to write CLAUDE.md");

    temp_dir
}

/// Sample memory for testing
pub fn sample_memory(content: &str, memory_type: MemoryType, importance: u8) -> MemoryNote {
    MemoryNote {
        id: MemoryId::new(),
        namespace: Namespace::Global,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        content: content.to_string(),
        summary: format!("Summary of: {}", content),
        keywords: vec!["test".to_string(), "sample".to_string()],
        tags: vec!["test".to_string()],
        context: "test context".to_string(),
        memory_type,
        importance,
        confidence: 0.8,
        links: vec![],
        related_files: vec![],
        related_entities: vec![],
        access_count: 0,
        last_accessed_at: chrono::Utc::now(),
        expires_at: None,
        is_archived: false,
        superseded_by: None,
        embedding: None,
        embedding_model: "test".to_string(),
    }
}
