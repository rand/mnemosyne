# Semantic Highlighting System

Advanced multi-tier semantic analysis and highlighting for natural language text, markdown, and agentic context.

## Overview

The semantic highlighting system provides real-time to background semantic analysis across three performance tiers:

- **Tier 1: Structural** (<5ms) - Real-time pattern matching
- **Tier 2: Relational** (<200ms) - Incremental local NLP
- **Tier 3: Analytical** (2s+) - Deep semantic analysis via Claude API (optional)

## Architecture

```
┌─────────────────────────────────────────────────────┐
│            SemanticHighlightEngine                  │
│  ┌───────────────────────────────────────────────┐  │
│  │  Priority-based Span Merger                   │  │
│  └───────────────────────────────────────────────┘  │
│                      ▲                               │
│          ┌───────────┼───────────┐                  │
│          │           │           │                   │
│  ┌───────▼─────┐ ┌──▼────────┐ ┌▼──────────────┐   │
│  │   Tier 1    │ │  Tier 2   │ │   Tier 3      │   │
│  │ Structural  │ │ Relational│ │  Analytical   │   │
│  │   <5ms      │ │  <200ms   │ │  2s+ (async)  │   │
│  └─────────────┘ └───────────┘ └───────────────┘   │
│         │              │              │              │
│    Always on      Optional       Optional           │
│     (local)       (local)     (Claude API)          │
└─────────────────────────────────────────────────────┘
```

## Features

### Tier 1: Structural Pattern Matching

**Real-time (<5ms), always-on, 100% local**

1. **XML Tag Analyzer** (`xml_tags.rs`)
   - Highlights agentic context tags: `<thinking>`, `<example>`, etc.
   - Validates nesting and matching
   - Detects mismatched/unclosed tags

2. **RFC 2119 Constraint Detector** (`constraints.rs`)
   - Keywords: MUST, SHALL, SHOULD, MAY, MUST NOT, etc.
   - Severity-based coloring (red=mandatory, yellow=recommended, green=optional)

3. **Modality/Hedging Analyzer** (`modality.rs`)
   - Certainty levels: certain, probable, uncertain, conditional
   - Dictionary-based + phrase detection
   - Color-coded by confidence level

4. **Ambiguity Detector** (`ambiguity.rs`)
   - Vague quantifiers: "several", "many", "some"
   - Unclear references: sentence-initial "this", "that"
   - Configurable severity threshold

5. **Domain Pattern Matcher** (`domain_patterns.rs`)
   - File paths: `#src/main.rs`
   - Symbol references: `@function_name`, `@Type::method`
   - Typed holes: `?placeholder`
   - URLs, code blocks, inline code

### Tier 2: Relational NLP Analysis

**Incremental (<200ms), optional, 100% local**

1. **Entity Recognizer** (`entities.rs`)
   - Types: PERSON, ORGANIZATION, LOCATION, TEMPORAL, CONCEPT
   - Rule-based with dictionary support
   - Confidence scoring and overlap resolution

2. **Relationship Extractor** (`relationships.rs`)
   - Subject-Verb-Object (SVO) triples
   - Relation types: Action, Attribution, Possession, Causation, Comparison
   - Visual connections between entities

3. **Semantic Role Labeler** (`semantic_roles.rs`)
   - Roles: Agent, Patient, Instrument, Location, Time, Beneficiary
   - Preposition-based heuristics
   - Active and passive voice support

4. **Coreference Resolver** (`coreference.rs`)
   - Links mentions of same entity
   - Distance-based scoring (500-char window)
   - Proper names, pronouns, nominals

5. **Anaphora Resolver** (`anaphora.rs`)
   - Resolves pronouns to antecedents
   - Grammatical agreement checking
   - 300-character lookback window

### Tier 3: Analytical Deep Semantics

**Background (2s+), optional, requires Claude API**

1. **Discourse Analyzer** (`discourse.rs`)
   - Rhetorical Structure Theory (RST) relations
   - Coherence scoring
   - Topic flow analysis

2. **Contradiction Detector** (`contradictions.rs`)
   - Four types: Direct, Temporal, Semantic, Implication
   - Severity levels with explanations
   - Visual connections between conflicting statements

3. **Pragmatics Analyzer** (`pragmatics.rs`)
   - Presuppositions (implied assumptions)
   - Implicatures (implied meanings)
   - Speech act classification
   - Indirect speech detection

4. **Request Batcher** (`batching.rs`)
   - Request aggregation
   - Token bucket rate limiting
   - Content-hash deduplication
   - Priority-based scheduling

## Usage

### Basic Usage

```rust
use mnemosyne_core::ics::semantic_highlighter::SemanticHighlightEngine;

// Create engine (Tier 1 + 2 only)
let mut engine = SemanticHighlightEngine::new(None);

// Highlight a line
let text = "The system MUST validate input properly.";
let highlighted_line = engine.highlight_line(text);

// highlighted_line is a ratatui::text::Line with styled spans
```

### With Claude API (Tier 3)

```rust
use mnemosyne_core::{
    ics::semantic_highlighter::SemanticHighlightEngine,
    LlmService,
};
use std::sync::Arc;

// Create LLM service with API key
let llm_service = Arc::new(LlmService::new("your-api-key"));

// Create engine with all three tiers
let mut engine = SemanticHighlightEngine::new(Some(llm_service));

// Highlight synchronously (Tier 1 + 2)
let line = engine.highlight_line(text);

// Request background analysis (Tier 3)
tokio::spawn(async move {
    engine.request_analysis(AnalysisRequest::Full).await
});
```

### Custom Configuration

```rust
use mnemosyne_core::ics::semantic_highlighter::{
    EngineBuilder, HighlightSettings, RelationalSettings,
};

let settings = HighlightSettings {
    enable_structural: true,
    enable_relational: true,
    enable_analytical: false,  // Disable Tier 3
    relational: RelationalSettings {
        confidence_threshold: 0.7,
        max_coref_distance: 500,
        debounce_ms: 100,
    },
    ..Default::default()
};

let engine = EngineBuilder::new()
    .with_settings(settings)
    .build();
```

### Individual Analyzers

```rust
use mnemosyne_core::ics::semantic_highlighter::tier1_structural::*;
use mnemosyne_core::ics::semantic_highlighter::tier2_relational::*;

// Tier 1: XML tags
let xml_analyzer = XmlTagAnalyzer::new();
let spans = xml_analyzer.analyze(text)?;

// Tier 2: Entity recognition
let entity_recognizer = EntityRecognizer::new()
    .with_threshold(0.8);
let entities = entity_recognizer.recognize(text)?;

// Tier 2: Coreference resolution
let coref_resolver = CoreferenceResolver::new()
    .with_max_distance(500)
    .with_threshold(0.6);
let chains = coref_resolver.resolve(text)?;
```

## Configuration

### Settings Structure

```rust
pub struct HighlightSettings {
    // Enable/disable tiers
    pub enable_structural: bool,
    pub enable_relational: bool,
    pub enable_analytical: bool,

    // Tier-specific settings
    pub relational: RelationalSettings,
    pub analytical: AnalyticalSettings,
    pub visual: VisualSettings,
}

pub struct RelationalSettings {
    pub confidence_threshold: f32,  // 0.0-1.0
    pub max_coref_distance: usize,  // characters
    pub debounce_ms: u64,           // milliseconds
}

pub struct AnalyticalSettings {
    pub rate_limit_rpm: usize,      // requests per minute
    pub max_batch_size: usize,      // requests per batch
    pub batch_wait_ms: u64,         // max wait time
    pub cache_ttl_seconds: u64,     // cache lifetime
    pub auto_analyze: bool,         // auto-trigger analysis
}
```

### Visual Configuration

```rust
pub struct VisualSettings {
    pub show_icons: bool,           // Show annotation icons
    pub show_connections: bool,     // Show visual connections
    pub show_confidence: bool,      // Show confidence scores
}
```

## Color Scheme

### Entity Types
- **PERSON**: Warm yellow (#FFD700)
- **ORGANIZATION**: Corporate blue (#4169E1)
- **LOCATION**: Earth green (#2E8B57)
- **TEMPORAL**: Clock orange (#FF8C00)
- **CONCEPT**: Abstract purple (#9370DB)

### Modality Levels
- **Certain**: Green
- **Probable**: Yellow
- **Uncertain**: Magenta
- **Conditional**: Cyan

### Constraints (RFC 2119)
- **Mandatory (MUST)**: Red
- **Prohibited (MUST NOT)**: Magenta
- **Recommended (SHOULD)**: Yellow
- **Optional (MAY)**: Green

### Contradictions
- **High severity**: Red
- **Medium severity**: Light red
- **Low severity**: Yellow

## Performance Characteristics

### Tier 1: Structural
- **Latency**: <5ms
- **Throughput**: >10,000 lines/sec
- **Memory**: Minimal (stateless patterns)
- **Caching**: None needed

### Tier 2: Relational
- **Latency**: <200ms (incremental)
- **Throughput**: >1,000 lines/sec
- **Memory**: LRU cache (configurable)
- **Caching**: Range-based LRU

### Tier 3: Analytical
- **Latency**: 2-5s (async)
- **Throughput**: Batched (5-10 req/batch)
- **Memory**: Content-hash cache
- **Caching**: Aggressive (hours TTL)

## Caching Strategy

### Tier 2: LRU Cache
- Key: Text range
- Invalidation: LRU eviction
- Hit rate: ~80% for typical editing

### Tier 3: Content-Hash Cache
- Key: SHA-256 of text content
- Invalidation: TTL-based
- Hit rate: ~95% for stable text
- Deduplication: Automatic

## Rate Limiting

Tier 3 uses token bucket algorithm:
- Configurable RPM (requests per minute)
- Burst capacity: 2x rate
- Automatic backoff on limit
- Per-batch rate consumption

## Integration

### With IcsEditor

```rust
use mnemosyne_core::ics::{
    IcsEditor,
    semantic_highlighter::SemanticHighlightEngine,
};

// Create highlighter
let highlighter = SemanticHighlightEngine::new(None);

// In editor rendering loop
for (line_idx, line_text) in editor.visible_lines() {
    let highlighted = highlighter.highlight_line(line_text);
    // Render highlighted line
}

// On text change (debounced)
highlighter.schedule_analysis(&editor.text(), changed_range);
```

### With ratatui

```rust
use ratatui::{
    widgets::{Paragraph, Block},
    text::Line,
};

let text = editor.current_line();
let highlighted: Line = highlighter.highlight_line(text);

let paragraph = Paragraph::new(highlighted)
    .block(Block::default().title("Code"));
```

## Testing

Run integration tests:
```bash
cargo test --test semantic_highlighter_integration
```

Run specific tier tests:
```bash
cargo test --lib tier1_structural
cargo test --lib tier2_relational
cargo test --lib tier3_analytical
```

## Examples

See `examples/semantic_highlighting.rs` for complete examples of:
- Basic highlighting
- Custom configuration
- Individual analyzers
- Async Tier 3 analysis
- Cache management

## Limitations

### Current Limitations
1. Tier 2 uses heuristics (not full parsing)
2. Tier 3 requires Claude API key
3. No incremental re-analysis yet (planned)
4. Limited language-specific parsing

### Known Issues
1. False positives in entity recognition for uncommon names
2. Coreference resolution struggles with complex nested references
3. Contradiction detection requires clear semantic context

## Future Enhancements

### Planned Features
- [ ] Incremental re-analysis for edited regions
- [ ] Language-specific parsers (using tree-sitter)
- [ ] Custom user-defined patterns
- [ ] Multi-document coreference
- [ ] Explanation UI for contradictions
- [ ] Confidence visualization

### Performance Improvements
- [ ] Parallel Tier 2 analysis
- [ ] Streaming Tier 3 results
- [ ] Smarter cache warming
- [ ] Adaptive batch sizing

## Contributing

When adding new analyzers:

1. Choose appropriate tier based on performance
2. Implement with confidence scoring
3. Add comprehensive tests
4. Document color scheme choices
5. Update this documentation

## References

- [RFC 2119](https://www.rfc-editor.org/rfc/rfc2119) - Requirement levels
- [Rhetorical Structure Theory](https://en.wikipedia.org/wiki/Rhetorical_structure_theory) - Discourse relations
- [Semantic Role Labeling](https://en.wikipedia.org/wiki/Semantic_role_labeling) - Role theory
- [Coreference Resolution](https://en.wikipedia.org/wiki/Coreference) - Entity linking

## License

MIT - See LICENSE file for details
