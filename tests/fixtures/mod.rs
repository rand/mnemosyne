//! Test data fixtures and sample memories

use chrono::{Duration, Utc};
use mnemosyne_core::{MemoryId, MemoryNote, MemoryType, Namespace};

pub struct TestData {
    pub database_memories: Vec<MemoryNote>,
    pub api_memories: Vec<MemoryNote>,
    pub testing_memories: Vec<MemoryNote>,
    pub misc_memories: Vec<MemoryNote>,
}

impl TestData {
    /// Load comprehensive test data with variety of content, dates, and importance
    pub fn load() -> Self {
        let now = Utc::now();

        Self {
            database_memories: vec![
                create_memory(
                    "Decided to use PostgreSQL for better ACID guarantees and complex queries",
                    "PostgreSQL chosen for relational data",
                    vec!["database", "postgresql", "architecture"],
                    vec!["database", "architecture"],
                    MemoryType::ArchitectureDecision,
                    9,
                    now - Duration::days(5),
                ),
                create_memory(
                    "Database connection pool size set to 20 based on load testing",
                    "Connection pool sized to 20",
                    vec!["database", "configuration", "performance"],
                    vec!["database", "config"],
                    MemoryType::Configuration,
                    6,
                    now - Duration::days(15),
                ),
                create_memory(
                    "All database queries must complete in under 200ms p95",
                    "Query performance requirement: <200ms",
                    vec!["database", "performance", "requirement"],
                    vec!["database", "performance"],
                    MemoryType::Constraint,
                    7,
                    now - Duration::days(20),
                ),
                create_memory(
                    "Use prepared statements to prevent SQL injection attacks",
                    "Prepared statements for SQL safety",
                    vec!["database", "security", "sql"],
                    vec!["database", "security"],
                    MemoryType::CodePattern,
                    8,
                    now - Duration::days(30),
                ),
                create_memory(
                    "Database migration failed due to missing foreign key constraint",
                    "Fixed migration with proper FK",
                    vec!["database", "migration", "bug"],
                    vec!["database", "bugfix"],
                    MemoryType::BugFix,
                    5,
                    now - Duration::days(45),
                ),
            ],
            api_memories: vec![
                create_memory(
                    "REST API design follows JSON:API specification for consistency",
                    "API follows JSON:API spec",
                    vec!["api", "rest", "json", "architecture"],
                    vec!["api", "architecture"],
                    MemoryType::ArchitectureDecision,
                    8,
                    now - Duration::days(3),
                ),
                create_memory(
                    "API rate limit set to 100 requests per minute per user",
                    "Rate limit: 100 req/min/user",
                    vec!["api", "rate-limiting", "configuration"],
                    vec!["api", "config"],
                    MemoryType::Configuration,
                    7,
                    now - Duration::days(10),
                ),
                create_memory(
                    "All API endpoints must return results in under 500ms",
                    "API latency requirement: <500ms",
                    vec!["api", "performance", "requirement"],
                    vec!["api", "performance"],
                    MemoryType::Constraint,
                    8,
                    now - Duration::days(12),
                ),
                create_memory(
                    "Use exponential backoff for retry logic in API clients",
                    "Exponential backoff for retries",
                    vec!["api", "retry", "pattern"],
                    vec!["api", "pattern"],
                    MemoryType::CodePattern,
                    7,
                    now - Duration::days(25),
                ),
                create_memory(
                    "Fixed race condition in concurrent API request handling",
                    "Race condition fix in API handler",
                    vec!["api", "concurrency", "bug"],
                    vec!["api", "bugfix"],
                    MemoryType::BugFix,
                    6,
                    now - Duration::days(35),
                ),
            ],
            testing_memories: vec![
                create_memory(
                    "Use property-based testing for complex business logic validation",
                    "Property-based testing for business logic",
                    vec!["testing", "property", "validation"],
                    vec!["testing", "strategy"],
                    MemoryType::CodePattern,
                    7,
                    now - Duration::days(8),
                ),
                create_memory(
                    "Test coverage target: 70% overall, 90% for critical paths",
                    "Coverage targets: 70% overall, 90% critical",
                    vec!["testing", "coverage", "requirement"],
                    vec!["testing", "quality"],
                    MemoryType::Constraint,
                    6,
                    now - Duration::days(18),
                ),
                create_memory(
                    "Integration tests run in isolated Docker containers",
                    "Integration tests use Docker",
                    vec!["testing", "integration", "docker"],
                    vec!["testing", "infrastructure"],
                    MemoryType::Configuration,
                    5,
                    now - Duration::days(22),
                ),
                create_memory(
                    "Fixed flaky test caused by timing assumption",
                    "Flaky test fix: remove timing",
                    vec!["testing", "flaky", "bug"],
                    vec!["testing", "bugfix"],
                    MemoryType::BugFix,
                    4,
                    now - Duration::days(40),
                ),
                create_memory(
                    "E2E tests should verify complete user workflows",
                    "E2E tests for complete workflows",
                    vec!["testing", "e2e", "workflow"],
                    vec!["testing", "strategy"],
                    MemoryType::Insight,
                    6,
                    now - Duration::days(50),
                ),
            ],
            misc_memories: vec![
                create_memory(
                    "Team prefers TypeScript over JavaScript for better type safety",
                    "Team preference: TypeScript",
                    vec!["preference", "typescript", "language"],
                    vec!["team", "preference"],
                    MemoryType::Preference,
                    5,
                    now - Duration::days(7),
                ),
                create_memory(
                    "Code review checklist includes security, performance, and tests",
                    "Review checklist: security, perf, tests",
                    vec!["process", "review", "checklist"],
                    vec!["process", "quality"],
                    MemoryType::Insight,
                    6,
                    now - Duration::days(14),
                ),
                create_memory(
                    "Deployment happens every Friday at 2pm EST",
                    "Deployment schedule: Fri 2pm EST",
                    vec!["deployment", "schedule", "process"],
                    vec!["process", "deployment"],
                    MemoryType::Configuration,
                    4,
                    now - Duration::days(28),
                ),
                create_memory(
                    "Monitoring dashboard: https://grafana.example.com/dashboards/main",
                    "Grafana dashboard link",
                    vec!["monitoring", "grafana", "reference"],
                    vec!["infrastructure", "reference"],
                    MemoryType::Reference,
                    3,
                    now - Duration::days(33),
                ),
                create_memory(
                    "User entity has relationships: profile, orders, payments",
                    "User entity relationships",
                    vec!["entity", "user", "relationships"],
                    vec!["domain", "entity"],
                    MemoryType::Entity,
                    7,
                    now - Duration::days(42),
                ),
            ],
        }
    }

    /// Get all memories as a single vec
    pub fn all(&self) -> Vec<MemoryNote> {
        let mut all = Vec::new();
        all.extend(self.database_memories.clone());
        all.extend(self.api_memories.clone());
        all.extend(self.testing_memories.clone());
        all.extend(self.misc_memories.clone());
        all
    }
}

/// Helper to create a memory with specified parameters
fn create_memory(
    content: &str,
    summary: &str,
    keywords: Vec<&str>,
    tags: Vec<&str>,
    memory_type: MemoryType,
    importance: u8,
    created_at: chrono::DateTime<Utc>,
) -> MemoryNote {
    MemoryNote {
        id: MemoryId::new(),
        namespace: Namespace::Global,
        created_at,
        updated_at: created_at,
        content: content.to_string(),
        summary: summary.to_string(),
        keywords: keywords.into_iter().map(String::from).collect(),
        tags: tags.into_iter().map(String::from).collect(),
        context: "test".to_string(),
        memory_type,
        importance,
        confidence: 0.8,
        links: vec![],
        related_files: vec![],
        related_entities: vec![],
        access_count: 0,
        last_accessed_at: created_at,
        expires_at: None,
        is_archived: false,
        superseded_by: None,
        embedding: None,
        embedding_model: "test".to_string(),
    }
}
