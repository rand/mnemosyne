//! Test fixtures for ICS E2E tests
//!
//! Provides sample data for testing:
//! - Documents (markdown, TOML, JSON)
//! - Memories with varying importance
//! - Proposals from different agents
//! - Diagnostics (errors, warnings, hints)

use mnemosyne_core::ics::editor::{Diagnostic, Position, Severity};
use mnemosyne_core::ics::{ChangeProposal, ProposalStatus};
use mnemosyne_core::types::{MemoryId, MemoryNote, MemoryType, Namespace};
use std::time::SystemTime;

/// Sample markdown document with semantic patterns
pub fn sample_markdown_doc() -> &'static str {
    r#"# Project Context

## Architecture

The system is distributed across multiple nodes.
The agent has memory persistence.
Service requires authentication for all endpoints.

TODO: Add rate limiting documentation

## Components

The Orchestrator manages the Executor and coordinates work.
The Optimizer has caching capabilities.
However, the system performance is inconsistent.

### Undefined References

We need to configure @rate_limiter and #monitoring_config.

"#
}

/// Sample TOML configuration
pub fn sample_toml_doc() -> &'static str {
    r#"[project]
name = "mnemosyne"
version = "1.0.0"

[dependencies]
tokio = "1.0"
anyhow = "1.0"

# TODO: Add more dependencies

[features]
default = ["storage", "llm"]
storage = []
llm = []
"#
}

/// Sample JSON data
pub fn sample_json_doc() -> &'static str {
    r#"{
  "config": {
    "enabled": true,
    "timeout": 5000,
    "retries": 3
  },
  "agents": [
    {
      "name": "orchestrator",
      "priority": 1
    },
    {
      "name": "executor",
      "priority": 2
    }
  ]
}
"#
}

/// Large document for performance testing (1000+ lines)
pub fn large_document() -> String {
    let mut doc = String::with_capacity(50000);

    doc.push_str("# Large Document Test\n\n");

    for i in 0..100 {
        doc.push_str(&format!("## Section {}\n\n", i + 1));
        doc.push_str("The system is distributed.\n");
        doc.push_str("The agent has memory.\n");
        doc.push_str("Service requires authentication.\n\n");

        doc.push_str("### Subsection Details\n\n");
        for j in 0..10 {
            doc.push_str(&format!("Line {} with content about the system.\n", j + 1));
        }
        doc.push_str("\n");
    }

    doc
}

/// Create sample memories for testing
pub fn sample_memories() -> Vec<MemoryNote> {
    vec![
        MemoryNote {
            id: MemoryId::new(),
            namespace: Namespace::Global,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            content: "The system uses Rust for performance and memory safety".to_string(),
            summary: "Rust architecture decision".to_string(),
            keywords: vec![
                "rust".to_string(),
                "performance".to_string(),
                "architecture".to_string(),
            ],
            tags: vec!["Architecture".to_string()],
            context: "Technical decision for programming language".to_string(),
            memory_type: MemoryType::ArchitectureDecision,
            importance: 9,
            confidence: 0.95,
            links: vec![],
            related_files: vec![],
            related_entities: vec!["Rust".to_string(), "System".to_string()],
            access_count: 5,
            last_accessed_at: chrono::Utc::now(),
            expires_at: None,
            is_archived: false,
            superseded_by: None,
            embedding: None,
            embedding_model: "test".to_string(),
        },
        MemoryNote {
            id: MemoryId::new(),
            namespace: Namespace::Global,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            content: "Authentication uses JWT tokens with 1-hour expiration".to_string(),
            summary: "JWT authentication pattern".to_string(),
            keywords: vec![
                "jwt".to_string(),
                "auth".to_string(),
                "security".to_string(),
            ],
            tags: vec!["Security".to_string()],
            context: "API security implementation".to_string(),
            memory_type: MemoryType::CodePattern,
            importance: 7,
            confidence: 0.90,
            links: vec![],
            related_files: vec![],
            related_entities: vec!["JWT".to_string(), "Authentication".to_string()],
            access_count: 3,
            last_accessed_at: chrono::Utc::now(),
            expires_at: None,
            is_archived: false,
            superseded_by: None,
            embedding: None,
            embedding_model: "test".to_string(),
        },
        MemoryNote {
            id: MemoryId::new(),
            namespace: Namespace::Global,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            content: "Distributed architecture uses event-driven communication".to_string(),
            summary: "Event-driven architecture".to_string(),
            keywords: vec![
                "distributed".to_string(),
                "events".to_string(),
                "architecture".to_string(),
            ],
            tags: vec!["Architecture".to_string()],
            context: "System design pattern".to_string(),
            memory_type: MemoryType::ArchitectureDecision,
            importance: 8,
            confidence: 0.88,
            links: vec![],
            related_files: vec![],
            related_entities: vec!["System".to_string(), "Events".to_string()],
            access_count: 7,
            last_accessed_at: chrono::Utc::now(),
            expires_at: None,
            is_archived: false,
            superseded_by: None,
            embedding: None,
            embedding_model: "test".to_string(),
        },
    ]
}

/// Create sample proposals for testing
pub fn sample_proposals() -> Vec<ChangeProposal> {
    vec![
        ChangeProposal {
            id: "prop-1".to_string(),
            agent: "agent:semantic".to_string(),
            description: "Fix TODO marker".to_string(),
            original: "TODO: Add rate limiting documentation".to_string(),
            proposed: "Rate limiting: Configure max 100 requests/minute per client".to_string(),
            line_range: (10, 10),
            created_at: SystemTime::now(),
            status: ProposalStatus::Pending,
            rationale: "TODO marker detected - proposing concrete implementation".to_string(),
        },
        ChangeProposal {
            id: "prop-2".to_string(),
            agent: "agent:optimizer".to_string(),
            description: "Resolve contradiction".to_string(),
            original: "However, the system performance is inconsistent".to_string(),
            proposed: "System performance varies with load - optimization pending".to_string(),
            line_range: (16, 16),
            created_at: SystemTime::now(),
            status: ProposalStatus::Pending,
            rationale: "Contradiction detected - providing more nuanced description".to_string(),
        },
        ChangeProposal {
            id: "prop-3".to_string(),
            agent: "agent:reviewer".to_string(),
            description: "Define undefined reference".to_string(),
            original: "@rate_limiter".to_string(),
            proposed: "@rate_limiter: Token bucket algorithm, 100 req/min".to_string(),
            line_range: (20, 20),
            created_at: SystemTime::now(),
            status: ProposalStatus::Pending,
            rationale: "Undefined symbol detected - adding definition".to_string(),
        },
    ]
}

/// Create sample diagnostics for testing
pub fn sample_diagnostics() -> Vec<Diagnostic> {
    vec![
        Diagnostic {
            position: Position { line: 5, column: 0 },
            length: 4,
            severity: Severity::Warning,
            message: "TODO marker found - incomplete section".to_string(),
            suggestion: Some("Complete or remove TODO".to_string()),
        },
        Diagnostic {
            position: Position {
                line: 15,
                column: 0,
            },
            length: 7,
            severity: Severity::Error,
            message: "Contradictory statements detected".to_string(),
            suggestion: Some("Resolve contradiction".to_string()),
        },
        Diagnostic {
            position: Position {
                line: 20,
                column: 20,
            },
            length: 12,
            severity: Severity::Error,
            message: "Undefined reference: @rate_limiter".to_string(),
            suggestion: Some("Define @rate_limiter".to_string()),
        },
        Diagnostic {
            position: Position {
                line: 20,
                column: 37,
            },
            length: 17,
            severity: Severity::Error,
            message: "Undefined reference: #monitoring_config".to_string(),
            suggestion: Some("Define #monitoring_config".to_string()),
        },
    ]
}

/// Document with validation errors (brackets, quotes, line length)
pub fn document_with_validation_errors() -> &'static str {
    r#"# Test Document

This line has an unclosed bracket [like this

This line has an unclosed quote "like this

This line is extremely long and exceeds the maximum line length limit by including a lot of unnecessary verbose content that should be broken into multiple lines for better readability and maintainability according to style guidelines

func() { unclosed brace
"#
}
