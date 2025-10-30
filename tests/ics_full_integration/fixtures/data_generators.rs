//! Data generators for creating realistic test data

use super::storage_fixture::create_test_memory;
use mnemosyne_core::types::{MemoryNote, MemoryType, Namespace};

/// Generate a batch of related memories
pub fn generate_memory_batch(
    base_topic: &str,
    count: usize,
    namespace: Namespace,
) -> Vec<MemoryNote> {
    let topics = [
        "authentication",
        "distributed systems",
        "caching",
        "database",
        "API design",
    ];

    let mut memories = Vec::new();
    for i in 0..count {
        let topic = topics[i % topics.len()];
        let content = format!("{} - {} implementation detail {}", base_topic, topic, i + 1);

        let memory = create_test_memory(
            &content,
            MemoryType::CodePattern,
            namespace.clone(),
            (5 + (i % 6)) as u8,
        );

        memories.push(memory);
    }

    memories
}

/// Generate a large dataset for performance testing
pub fn generate_large_dataset(count: usize, namespace: Namespace) -> Vec<MemoryNote> {
    let mut memories = Vec::new();

    for i in 0..count {
        let memory_type = match i % 7 {
            0 => MemoryType::ArchitectureDecision,
            1 => MemoryType::CodePattern,
            2 => MemoryType::BugFix,
            3 => MemoryType::Configuration,
            4 => MemoryType::Constraint,
            5 => MemoryType::Entity,
            _ => MemoryType::Insight,
        };

        let content = format!("Large dataset entry {} with detailed content about the system component and its interactions", i + 1);

        let mut memory = create_test_memory(
            &content,
            memory_type,
            namespace.clone(),
            ((i % 10) + 1) as u8,
        );

        // Add varied metadata
        memory.keywords = vec![
            format!("keyword{}", i % 20),
            "system".to_string(),
            "component".to_string(),
        ];
        memory.tags = vec![format!("Tag{}", i % 10)];
        memory.access_count = (i % 100) as u32;

        memories.push(memory);
    }

    memories
}

/// Generate memories with specific search patterns
pub fn generate_search_test_data(namespace: Namespace) -> Vec<MemoryNote> {
    vec![
        create_test_memory(
            "Authentication uses JWT tokens with 1-hour expiration",
            MemoryType::CodePattern,
            namespace.clone(),
            9,
        ),
        create_test_memory(
            "Database connection pool size set to 20",
            MemoryType::Configuration,
            namespace.clone(),
            7,
        ),
        create_test_memory(
            "API rate limiting: 100 requests per minute",
            MemoryType::ArchitectureDecision,
            namespace.clone(),
            8,
        ),
        create_test_memory(
            "Caching strategy uses Redis for session data",
            MemoryType::Insight,
            namespace.clone(),
            7,
        ),
        create_test_memory(
            "Distributed tracing implemented with OpenTelemetry",
            MemoryType::CodePattern,
            namespace.clone(),
            8,
        ),
    ]
}
