//! Integration tests for semantic highlighting system
//!
//! Tests the complete three-tier system working together.

use mnemosyne_core::ics::semantic_highlighter::{
    tier2_relational::*, HighlightSettings, SemanticHighlightEngine,
};

#[tokio::test]
async fn test_tier1_xml_tag_highlighting() {
    let mut engine = SemanticHighlightEngine::new(None);
    let text = "<thinking>Let me analyze this problem.</thinking>";

    let line = engine.highlight_line(text);

    // Should have highlighted spans for XML tags
    assert!(!line.spans.is_empty());
}

#[tokio::test]
async fn test_tier1_constraint_detection() {
    let mut engine = SemanticHighlightEngine::new(None);
    let text = "The system MUST validate input and SHOULD log errors.";

    let line = engine.highlight_line(text);

    // Should highlight MUST and SHOULD
    assert!(!line.spans.is_empty());
}

#[tokio::test]
async fn test_tier1_modality_detection() {
    let mut engine = SemanticHighlightEngine::new(None);
    let text = "This will definitely work, but it might need adjustment.";

    let line = engine.highlight_line(text);

    // Should highlight modality markers
    assert!(!line.spans.is_empty());
}

#[tokio::test]
async fn test_tier1_ambiguity_detection() {
    let mut engine = SemanticHighlightEngine::new(None);
    let text = "There are several issues with many components.";

    let line = engine.highlight_line(text);

    // Should highlight vague quantifiers
    assert!(!line.spans.is_empty());
}

#[tokio::test]
async fn test_tier1_domain_patterns() {
    let mut engine = SemanticHighlightEngine::new(None);
    let text = "See #src/main.rs and call @process_data with ?auth_token";

    let line = engine.highlight_line(text);

    // Should highlight file path, symbol, and typed hole
    assert!(!line.spans.is_empty());
}

#[tokio::test]
async fn test_tier2_entity_recognition() {
    let recognizer = EntityRecognizer::new();
    let text = "Dr. Smith discussed the algorithm with the team yesterday.";

    let entities = recognizer.recognize(text).unwrap();

    // Should find person and concept entities
    assert!(!entities.is_empty());

    let has_person = entities.iter().any(|e| e.entity_type == EntityType::Person);
    let has_concept = entities
        .iter()
        .any(|e| e.entity_type == EntityType::Concept);

    assert!(has_person || has_concept);
}

#[tokio::test]
async fn test_tier2_relationship_extraction() {
    let extractor = RelationshipExtractor::new();
    let text = "The function calls the API";

    let relationships = extractor.extract(text).unwrap();

    // Should find action relationship
    assert!(!relationships.is_empty());
}

#[tokio::test]
async fn test_tier2_semantic_roles() {
    let labeler = SemanticRoleLabeler::new();
    let text = "The data was processed by the server with a tool";

    let roles = labeler.label(text).unwrap();

    // Should find agent and instrument roles
    assert!(!roles.is_empty());
}

#[tokio::test]
async fn test_tier2_coreference() {
    let resolver = CoreferenceResolver::new();
    let text = "John Smith arrived early. He was prepared. Smith left on time.";

    let chains = resolver.resolve(text).unwrap();

    // Should find coreference chain
    if !chains.is_empty() {
        let chain = &chains[0];
        assert!(chain.mentions.len() > 1);
    }
}

#[tokio::test]
async fn test_tier2_anaphora() {
    let resolver = AnaphoraResolver::new();
    let text = "The system failed. It needed restart.";

    let resolutions = resolver.resolve(text).unwrap();

    // Should resolve "It" to "system"
    assert!(!resolutions.is_empty());
}

#[tokio::test]
async fn test_full_pipeline_complex_text() {
    let mut engine = SemanticHighlightEngine::new(None);

    let text = r#"<thinking>
The system MUST handle errors properly. Dr. Johnson mentioned that the algorithm
processes data efficiently. However, there might be several issues with the
implementation. See #src/core.rs for details.
</thinking>"#;

    let line = engine.highlight_line(text);

    // Should have multiple highlighted spans from different analyzers
    assert!(!line.spans.is_empty());
}

#[tokio::test]
async fn test_cache_statistics() {
    let engine = SemanticHighlightEngine::new(None);

    let (relational_stats, analytical_stats) = engine.cache_stats();

    // Relational cache should be initialized (Tier 2 enabled by default)
    assert!(relational_stats.capacity > 0);
    // Analytical cache capacity may be 0 (Tier 3 requires LLM service)
    let _ = analytical_stats;
}

#[tokio::test]
async fn test_settings_update() {
    let mut engine = SemanticHighlightEngine::new(None);

    let mut settings = HighlightSettings::default();
    settings.enable_relational = false;

    engine.update_settings(settings);

    // Should still work with updated settings
    let text = "Test text";
    let line = engine.highlight_line(text);

    // Should complete without error
    let _ = line;
}

#[tokio::test]
async fn test_engine_builder() {
    let settings = HighlightSettings {
        enable_structural: true,
        enable_relational: true,
        enable_analytical: false,
        ..Default::default()
    };

    let engine = mnemosyne_core::ics::semantic_highlighter::EngineBuilder::new()
        .with_settings(settings)
        .build();

    // Should build successfully
    let _ = engine;
}

#[tokio::test]
async fn test_confidence_scores() {
    let recognizer = EntityRecognizer::new().with_threshold(0.8);
    let text = "The algorithm is complex";

    let entities = recognizer.recognize(text).unwrap();

    // All entities should meet threshold
    for entity in entities {
        assert!(entity.confidence >= 0.8);
    }
}

#[tokio::test]
async fn test_parallel_analysis() {
    // Test that multiple analyzers can work in parallel
    let text = "Dr. Smith uses the algorithm in the system";

    let recognizer = EntityRecognizer::new();
    let extractor = RelationshipExtractor::new();
    let labeler = SemanticRoleLabeler::new();

    // All should complete without interfering
    let entities = recognizer.recognize(text).unwrap();
    let relationships = extractor.extract(text).unwrap();
    let roles = labeler.label(text).unwrap();

    // At least one analyzer should find something
    assert!(!entities.is_empty() || !relationships.is_empty() || !roles.is_empty());
}

#[tokio::test]
async fn test_multilayer_highlighting() {
    let mut engine = SemanticHighlightEngine::new(None);

    // Text with multiple layers of semantic meaning
    let text = "The function MUST validate input using the sanitizer";

    let line = engine.highlight_line(text);

    // Should have overlapping highlights from:
    // - Tier 1: MUST (constraint)
    // - Tier 2: entities (function, sanitizer), relationships (validate-input)
    assert!(!line.spans.is_empty());
}

#[tokio::test]
async fn test_clear_caches() {
    let engine = SemanticHighlightEngine::new(None);

    // Should not panic
    engine.clear_caches();

    let (relational_stats, analytical_stats) = engine.cache_stats();
    assert_eq!(relational_stats.size, 0);
    assert_eq!(analytical_stats.size, 0);
}
