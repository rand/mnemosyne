//! Example usage of the semantic highlighting system
//!
//! Demonstrates the public API and various configuration options.
//!
//! Note: Individual analyzer APIs are internal. This example shows
//! the public-facing engine API that coordinates all analyzers.

use mnemosyne_core::ics::semantic_highlighter::{
    EngineBuilder, HighlightSettings, SemanticHighlightEngine,
};

fn main() {
    println!("=== Semantic Highlighting System Examples ===\n");

    example_basic_highlighting();
    example_multilayer_highlighting();
    example_custom_configuration();
    example_cache_management();
    example_different_text_types();
}

/// Example 1: Basic highlighting with default settings
fn example_basic_highlighting() {
    println!("Example 1: Basic Highlighting");
    println!("─────────────────────────────");

    let mut engine = SemanticHighlightEngine::new(None);

    let texts = vec![
        "The system MUST validate input before processing.",
        "<thinking>Let me analyze this problem carefully.</thinking>",
        "Dr. Smith mentioned that the algorithm is efficient.",
        "There are several issues with many components.",
        "See #src/main.rs and call @process_data",
    ];

    for text in texts {
        let line = engine.highlight_line(text);
        println!("Input:  {}", text);
        println!("Spans:  {} highlights detected", line.spans.len());
        println!();
    }
}

/// Example 2: Multi-layer highlighting (all tiers at once)
fn example_multilayer_highlighting() {
    println!("Example 2: Multi-layer Highlighting");
    println!("────────────────────────────────────");

    let mut engine = SemanticHighlightEngine::new(None);

    // Complex text with multiple semantic layers
    let complex_text = r#"<thinking>
The system MUST validate input properly. Dr. Johnson mentioned that several
components might need refactoring. See #src/validator.rs for implementation.
</thinking>"#;

    println!("Analyzing complex text:");
    println!("{}\n", complex_text);

    for line in complex_text.lines() {
        let highlighted = engine.highlight_line(line);
        println!(
            "Line: '{}' → {} highlights",
            line.trim(),
            highlighted.spans.len()
        );
    }

    println!("\nThis text triggers:");
    println!("  - Tier 1: XML tags (<thinking>), constraints (MUST), modality (might),");
    println!("           ambiguity (several), domain patterns (#src/validator.rs)");
    println!("  - Tier 2: Entities (Dr. Johnson, components), relationships, roles");
    println!();
}

/// Example 3: Custom configuration
fn example_custom_configuration() {
    println!("Example 3: Custom Configuration");
    println!("────────────────────────────────");

    // Tier 1 only (fastest, minimal analysis)
    let tier1_settings = HighlightSettings {
        enable_structural: true,
        enable_relational: false,
        enable_analytical: false,
        ..Default::default()
    };

    let mut tier1_engine = EngineBuilder::new().with_settings(tier1_settings).build();

    let text = "The system MUST validate input. See #src/main.rs";
    let line = tier1_engine.highlight_line(text);
    println!("Tier 1 only: {} highlights", line.spans.len());
    println!("  (XML tags, constraints, modality, ambiguity, domain patterns)");

    // Tier 1 + 2 (comprehensive local analysis)
    let tier12_settings = HighlightSettings {
        enable_structural: true,
        enable_relational: true,
        enable_analytical: false,
        ..Default::default()
    };

    let mut tier12_engine = EngineBuilder::new().with_settings(tier12_settings).build();

    let line = tier12_engine.highlight_line(text);
    println!("\nTier 1+2: {} highlights", line.spans.len());
    println!("  (+ entities, relationships, roles, coreference, anaphora)");

    println!();
}

/// Example 4: Cache management
fn example_cache_management() {
    println!("Example 4: Cache Management");
    println!("───────────────────────────");

    let mut engine = SemanticHighlightEngine::new(None);

    // Process some text
    let texts = vec![
        "Dr. Smith works on the algorithm.",
        "The system MUST validate input.",
        "Dr. Smith works on the algorithm.", // Duplicate
    ];

    for text in &texts {
        let _ = engine.highlight_line(text);
    }

    // Get cache statistics
    let (relational_stats, analytical_stats) = engine.cache_stats();
    println!(
        "Relational cache: {} entries / {} capacity ({:.1}% utilization)",
        relational_stats.size,
        relational_stats.capacity,
        relational_stats.utilization() * 100.0
    );
    println!(
        "Analytical cache: {} entries / {} capacity ({:.1}% utilization)",
        analytical_stats.size,
        analytical_stats.capacity,
        analytical_stats.utilization() * 100.0
    );

    // Clear caches
    engine.clear_caches();
    println!("\nCaches cleared");

    // Process again - caches should be empty
    for text in &texts {
        let _ = engine.highlight_line(text);
    }

    let (relational_stats, analytical_stats) = engine.cache_stats();
    println!(
        "After clear - Relational: {} entries",
        relational_stats.size
    );
    println!(
        "              Analytical: {} entries",
        analytical_stats.size
    );

    println!();
}

/// Example 5: Different text types
fn example_different_text_types() {
    println!("Example 5: Different Text Types");
    println!("────────────────────────────────");

    let mut engine = SemanticHighlightEngine::new(None);

    // Agentic context
    let agentic = "<thinking>Let me carefully analyze this problem.</thinking>";
    let line = engine.highlight_line(agentic);
    println!("Agentic context: {} highlights", line.spans.len());

    // Technical specification
    let spec = "The API MUST return 200 OK and SHOULD include ETag header.";
    let line = engine.highlight_line(spec);
    println!("Technical spec: {} highlights", line.spans.len());

    // Natural language
    let natural = "Dr. Johnson mentioned that several algorithms might work.";
    let line = engine.highlight_line(natural);
    println!("Natural language: {} highlights", line.spans.len());

    // Code references
    let code_ref = "See #src/parser.rs and implement @parse_tokens with ?error_recovery";
    let line = engine.highlight_line(code_ref);
    println!("Code references: {} highlights", line.spans.len());

    // Mixed content
    let mixed = r#"<example>
The validator MUST check input. Dr. Smith implemented this in #validator.rs
using @validate_input with ?custom_rules. Several edge cases remain unclear.
</example>"#;

    println!("\nMixed content analysis:");
    for line in mixed.lines() {
        let highlighted = engine.highlight_line(line);
        if !line.trim().is_empty() {
            println!(
                "  '{}...' → {} highlights",
                &line.trim()[..line.trim().len().min(40)],
                highlighted.spans.len()
            );
        }
    }

    println!();
}
