//! Integration tests for Tier 2 semantic analysis caching
//!
//! Tests the caching infrastructure for local NLP analysis results,
//! including serialization, deserialization, and cache invalidation.

use mnemosyne_core::ics::semantic_highlighter::{
    cache::{CachedResult, SemanticCache},
    settings::RelationalSettings,
    tier2_relational::{
        Entity, EntityType, RelationType, RelationalAnalyzer, Relationship, RoleAssignment,
        SemanticRole, Tier2AnalysisResult,
    },
};
use std::sync::Arc;

#[test]
fn test_tier2_analysis_result_serialization() {
    // Test Entity serialization
    let entity = Entity {
        entity_type: EntityType::Person,
        text: "Alice".to_string(),
        range: 0..5,
        confidence: 0.9,
    };

    let result = Tier2AnalysisResult::Entities(vec![entity]);
    let json = serde_json::to_value(&result).expect("Serialization should succeed");
    let deserialized: Tier2AnalysisResult =
        serde_json::from_value(json).expect("Deserialization should succeed");

    match deserialized {
        Tier2AnalysisResult::Entities(entities) => {
            assert_eq!(entities.len(), 1);
            assert_eq!(entities[0].text, "Alice");
            assert_eq!(entities[0].entity_type, EntityType::Person);
        }
        _ => panic!("Expected Entities variant"),
    }
}

#[test]
fn test_tier2_analysis_result_combined() {
    let entity = Entity {
        entity_type: EntityType::Organization,
        text: "Acme Corp".to_string(),
        range: 0..9,
        confidence: 0.85,
    };

    let relationship = Relationship {
        subject: 0..5,
        predicate: 6..10,
        object: 11..15,
        relation_type: RelationType::Action,
        confidence: 0.7,
    };

    let role = RoleAssignment {
        role: SemanticRole::Agent,
        range: 0..5,
        text: "Alice".to_string(),
        confidence: 0.8,
    };

    let result = Tier2AnalysisResult::Combined {
        entities: vec![entity],
        relationships: vec![relationship],
        roles: vec![role],
    };

    // Test serialization round-trip
    let json = serde_json::to_value(&result).expect("Serialization should succeed");
    let deserialized: Tier2AnalysisResult =
        serde_json::from_value(json).expect("Deserialization should succeed");

    match deserialized {
        Tier2AnalysisResult::Combined {
            entities,
            relationships,
            roles,
        } => {
            assert_eq!(entities.len(), 1);
            assert_eq!(relationships.len(), 1);
            assert_eq!(roles.len(), 1);
        }
        _ => panic!("Expected Combined variant"),
    }
}

#[test]
fn test_tier2_cache_insert_and_retrieve() {
    let cache = Arc::new(SemanticCache::new(100, 60));

    let entity = Entity {
        entity_type: EntityType::Location,
        text: "Paris".to_string(),
        range: 10..15,
        confidence: 0.9,
    };

    let result = Tier2AnalysisResult::Entities(vec![entity]);
    let json_value = serde_json::to_value(&result).expect("Serialization should succeed");

    // Insert into cache
    let range = 10..15;
    let cached = CachedResult::new(json_value);
    cache.relational.insert(range.clone(), cached);

    // Retrieve from cache
    let retrieved = cache
        .relational
        .get(&range)
        .expect("Cache should have entry");
    let deserialized: Tier2AnalysisResult =
        serde_json::from_value(retrieved.data).expect("Deserialization should succeed");

    match deserialized {
        Tier2AnalysisResult::Entities(entities) => {
            assert_eq!(entities.len(), 1);
            assert_eq!(entities[0].text, "Paris");
            assert_eq!(entities[0].entity_type, EntityType::Location);
        }
        _ => panic!("Expected Entities variant"),
    }
}

#[test]
fn test_tier2_cache_invalidation() {
    let cache = Arc::new(SemanticCache::new(100, 60));

    // Insert multiple cache entries
    for i in 0..3 {
        let start = i * 20;
        let end = start + 10;
        let range = start..end;

        let entity = Entity {
            entity_type: EntityType::Concept,
            text: format!("Entity{}", i),
            range: range.clone(),
            confidence: 0.8,
        };

        let result = Tier2AnalysisResult::Entities(vec![entity]);
        let json_value = serde_json::to_value(&result).expect("Serialization should succeed");
        let cached = CachedResult::new(json_value);
        cache.relational.insert(range, cached);
    }

    // Verify all entries exist
    assert!(cache.relational.get(&(0..10)).is_some());
    assert!(cache.relational.get(&(20..30)).is_some());
    assert!(cache.relational.get(&(40..50)).is_some());

    // Invalidate overlapping range (should remove middle entry)
    cache.relational.invalidate_range(&(15..35));

    // Verify middle entry removed
    assert!(
        cache.relational.get(&(0..10)).is_some(),
        "First entry should remain"
    );
    assert!(
        cache.relational.get(&(20..30)).is_none(),
        "Middle entry should be removed"
    );
    assert!(
        cache.relational.get(&(40..50)).is_some(),
        "Last entry should remain"
    );
}

#[test]
fn test_relational_analyzer_cache_hit() {
    let cache = Arc::new(SemanticCache::new(100, 60));
    let settings = RelationalSettings::default();
    let analyzer = RelationalAnalyzer::new(settings, Arc::clone(&cache));

    // Pre-populate cache
    let entity = Entity {
        entity_type: EntityType::Person,
        text: "Dr. Smith".to_string(),
        range: 0..9,
        confidence: 0.95,
    };

    let result = Tier2AnalysisResult::Entities(vec![entity.clone()]);
    let json_value = serde_json::to_value(&result).expect("Serialization should succeed");
    let cached = CachedResult::new(json_value);
    cache.relational.insert(0..9, cached);

    // Retrieve highlights from cache
    let text = "Dr. Smith";
    let spans = analyzer
        .get_cached_highlights(&(0..9), text)
        .expect("Should succeed");

    // Verify spans created from cached data
    assert!(!spans.is_empty(), "Should have highlights from cache");
}

#[test]
fn test_relational_analyzer_cache_miss() {
    let cache = Arc::new(SemanticCache::new(100, 60));
    let settings = RelationalSettings::default();
    let analyzer = RelationalAnalyzer::new(settings, cache);

    // Try to get highlights for range not in cache
    let text = "Some text";
    let spans = analyzer
        .get_cached_highlights(&(0..9), text)
        .expect("Should succeed");

    // Verify empty result on cache miss
    assert!(spans.is_empty(), "Should return empty on cache miss");
}

#[test]
fn test_highlight_cached_full_text() {
    let cache = Arc::new(SemanticCache::new(100, 60));
    let settings = RelationalSettings::default();
    let analyzer = RelationalAnalyzer::new(settings, cache);

    // Test with text that has recognizable entities
    let text = "Dr. Alice works at Acme Corp";

    // First call - cache miss, runs analysis
    let spans = analyzer.highlight_cached(text).expect("Should succeed");

    // Should have some highlights from synchronous analysis
    assert!(
        !spans.is_empty(),
        "Should detect entities in text (Dr. Alice, Acme Corp)"
    );
}

#[test]
fn test_entity_type_all_variants_serializable() {
    let variants = vec![
        EntityType::Person,
        EntityType::Organization,
        EntityType::Location,
        EntityType::Temporal,
        EntityType::Concept,
    ];

    for variant in variants {
        let json = serde_json::to_value(&variant).expect("Should serialize");
        let deserialized: EntityType = serde_json::from_value(json).expect("Should deserialize");
        assert_eq!(variant, deserialized);
    }
}

#[test]
fn test_relation_type_all_variants_serializable() {
    let variants = vec![
        RelationType::Action,
        RelationType::Attribution,
        RelationType::Possession,
        RelationType::Causation,
        RelationType::Comparison,
    ];

    for variant in variants {
        let json = serde_json::to_value(&variant).expect("Should serialize");
        let deserialized: RelationType = serde_json::from_value(json).expect("Should deserialize");
        assert_eq!(variant, deserialized);
    }
}

#[test]
fn test_semantic_role_all_variants_serializable() {
    let variants = vec![
        SemanticRole::Agent,
        SemanticRole::Patient,
        SemanticRole::Instrument,
        SemanticRole::Location,
        SemanticRole::Time,
        SemanticRole::Beneficiary,
    ];

    for variant in variants {
        let json = serde_json::to_value(&variant).expect("Should serialize");
        let deserialized: SemanticRole = serde_json::from_value(json).expect("Should deserialize");
        assert_eq!(variant, deserialized);
    }
}

#[test]
fn test_cache_stats_after_tier2_operations() {
    let cache = Arc::new(SemanticCache::new(100, 60));

    // Initially empty
    let (relational_stats, _) = cache.stats();
    assert_eq!(relational_stats.size, 0);

    // Add some entries
    for i in 0..5 {
        let range = (i * 10)..(i * 10 + 5);
        let entity = Entity {
            entity_type: EntityType::Concept,
            text: format!("Item{}", i),
            range: range.clone(),
            confidence: 0.8,
        };

        let result = Tier2AnalysisResult::Entities(vec![entity]);
        let json_value = serde_json::to_value(&result).expect("Serialization should succeed");
        let cached = CachedResult::new(json_value);
        cache.relational.insert(range, cached);
    }

    // Verify stats updated
    let (relational_stats, _) = cache.stats();
    assert_eq!(relational_stats.size, 5);
    assert_eq!(relational_stats.capacity, 100);
}
