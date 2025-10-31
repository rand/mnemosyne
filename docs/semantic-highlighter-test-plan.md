# Semantic Highlighter Test Plan

## Overview

Comprehensive test plan for validating correctness, performance, and reliability of the three-tier semantic highlighting system.

## Testing Strategy

### Approach

**Test-Driven Development (TDD)**:
1. Write tests first for new features
2. Implement until tests pass
3. Refactor with confidence
4. Repeat

**Test Pyramid**:
```
      /\
     /E2E\         5% (slow, brittle)
    /------\
   /  Intg  \      15% (moderate)
  /----------\
 /    Unit    \    80% (fast, focused)
/--------------\
```

### Coverage Targets

- **Overall**: 70%+ line coverage
- **Critical path** (Tier 1, core analyzers): 90%+
- **Business logic** (analysis algorithms): 80%+
- **UI layer** (rendering): 60%+
- **Infrastructure** (cache, batching): 75%+

### Test Organization

```
tests/
├── unit/
│   ├── tier1_structural/      # Tier 1 analyzer tests
│   ├── tier2_relational/      # Tier 2 analyzer tests
│   ├── tier3_analytical/      # Tier 3 analyzer tests
│   ├── cache/                 # Cache system tests
│   └── visualization/         # Span merging, rendering tests
├── integration/
│   ├── engine_integration.rs  # Full engine tests
│   ├── tier_coordination.rs   # Multi-tier interaction
│   └── ics_integration.rs     # ICS editor integration
├── e2e/
│   ├── real_world_documents.rs # Full document analysis
│   └── performance.rs         # Performance benchmarks
└── property/
    ├── cache_properties.rs    # Property-based cache tests
    └── span_properties.rs     # Span merging properties
```

---

## Test Types

### 1. Unit Tests (80%)

**Purpose**: Test individual functions/methods in isolation

**Characteristics**:
- Fast (<10ms per test)
- No I/O (mocked LLM, no network)
- Focused on single responsibility
- High coverage (90%+)

**Examples**:

```rust
// Tier 1: XML Tag Analyzer
#[test]
fn test_xml_tag_detection() {
    let analyzer = XmlTagAnalyzer::new();
    let text = "<thinking>Test</thinking>";
    let tags = analyzer.find_tags(text).unwrap();

    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].name, "thinking");
    assert_eq!(tags[0].range, 0..29);
}

#[test]
fn test_nested_xml_tags() {
    let analyzer = XmlTagAnalyzer::new();
    let text = "<outer><inner>Content</inner></outer>";
    let tags = analyzer.find_tags(text).unwrap();

    assert_eq!(tags.len(), 2);
    assert!(tags[0].range.contains(&tags[1].range.start));
}

// Tier 2: Entity Recognition
#[test]
fn test_person_entity_detection() {
    let recognizer = EntityRecognizer::new();
    let text = "Dr. Smith discussed the results with Jane Doe.";
    let entities = recognizer.recognize(text).unwrap();

    let people: Vec<_> = entities.iter()
        .filter(|e| e.entity_type == EntityType::Person)
        .collect();

    assert_eq!(people.len(), 2);
    assert!(people.iter().any(|e| e.text.contains("Smith")));
    assert!(people.iter().any(|e| e.text.contains("Doe")));
}

// Tier 3: LLM Response Parsing
#[test]
fn test_discourse_response_parsing() {
    let analyzer = DiscourseAnalyzer::new(mock_llm());
    let json = r#"[{
        "start": 0,
        "end": 20,
        "text": "First segment",
        "relation": "Elaboration",
        "confidence": 0.85
    }]"#;

    let segments = analyzer.parse_discourse_response(json, 100).unwrap();
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].relation, Some(DiscourseRelation::Elaboration));
}

// Cache
#[test]
fn test_cache_expiration() {
    let cache = AnalyticalCache::new(0); // 0 second TTL
    cache.insert_with_content("test", CachedResult::new("data"));

    std::thread::sleep(Duration::from_millis(10));

    assert!(cache.get_by_content("test").is_none());
}
```

### 2. Integration Tests (15%)

**Purpose**: Test component interactions

**Characteristics**:
- Moderate speed (100ms-1s per test)
- May use mock LLM service
- Tests multiple components together
- Focus on boundaries and data flow

**Examples**:

```rust
// Full Engine Pipeline
#[test]
fn test_full_engine_pipeline() {
    let mut engine = SemanticHighlightEngine::new(None);
    let text = "<thinking>The system MUST validate input. John said the API processes data.</thinking>";

    let line = engine.highlight_line(text);

    // Should have spans from multiple tiers
    assert!(!line.spans.is_empty());

    // Tier 1: XML tags
    assert!(line.spans.iter().any(|s| s.source == HighlightSource::Structural));

    // Tier 2: Entities (John, API, system)
    // Note: May be cached or not, depending on timing
}

// Tier Coordination
#[test]
fn test_tier_priority_merging() {
    let mut engine = SemanticHighlightEngine::new(None);
    let text = "MUST validate"; // Both structural (MUST) and potential entity (validate)

    let line = engine.highlight_line(text);

    // Higher priority tier should win
    // Structural (Tier 1) should override any conflicts
    let must_span = line.spans.iter().find(|s| text[s.range.clone()].contains("MUST"));
    if let Some(span) = must_span {
        // Should be structural or higher priority
        assert!(span.source >= HighlightSource::Structural);
    }
}

// ICS Integration
#[tokio::test]
async fn test_text_change_triggers_analysis() {
    let mut editor = IcsEditor::new();
    let buffer_id = editor.new_buffer(None);
    let buffer = editor.buffer_mut(buffer_id).unwrap();

    // Enable semantic analysis
    buffer.enable_semantic_analysis(mock_llm(), HighlightSettings::default());

    // Insert text
    buffer.insert("Test content").unwrap();

    // Verify schedule_analysis was called (check internal state or mock)
    // This requires additional instrumentation or mock verification
}
```

### 3. End-to-End Tests (5%)

**Purpose**: Test complete user workflows

**Characteristics**:
- Slow (1-10s per test)
- Real or production-like LLM service
- Full system integration
- Focus on user stories

**Examples**:

```rust
// Real Document Analysis
#[tokio::test]
#[ignore] // Expensive, run manually or in CI only
async fn test_real_document_analysis() {
    let mut engine = SemanticHighlightEngine::new(Some(real_llm_service()));

    let document = std::fs::read_to_string("tests/fixtures/sample_spec.md").unwrap();

    // Request full analysis
    engine.request_analysis(document.clone(), AnalysisRequestType::Full).await.unwrap();

    // Wait for analysis to complete
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Check results
    for line in document.lines() {
        let highlighted = engine.highlight_line(line);
        // Should have semantic highlights
        // (exact assertions depend on document content)
    }

    // Check cache
    let (rel_stats, ana_stats) = engine.cache_stats();
    assert!(rel_stats.size > 0 || ana_stats.size > 0);
}

// Performance Benchmark
#[test]
fn bench_tier1_performance() {
    let mut engine = SemanticHighlightEngine::new(None);
    let text = "MUST <thinking>analyze</thinking> with high priority";

    let start = Instant::now();
    for _ in 0..1000 {
        let _ = engine.highlight_line(text);
    }
    let elapsed = start.elapsed();

    let avg_time = elapsed.as_micros() / 1000;
    assert!(avg_time < 5000, "Tier 1 should be <5ms, got {}μs", avg_time);
}
```

### 4. Property-Based Tests

**Purpose**: Test invariants across many inputs

**Characteristics**:
- Uses property testing (e.g., proptest, quickcheck)
- Tests properties, not specific outputs
- Finds edge cases automatically

**Examples**:

```rust
use proptest::prelude::*;

proptest! {
    // Span Merging Properties
    #[test]
    fn prop_span_merging_preserves_order(spans in arbitrary_spans()) {
        let mut merger = SpanMerger::new();
        for span in spans {
            merger.add(span);
        }

        let merged = merger.merge();

        // Property: Merged spans should be ordered by start position
        for window in merged.windows(2) {
            assert!(window[0].range.start <= window[1].range.start);
        }
    }

    #[test]
    fn prop_span_merging_no_overlap(spans in arbitrary_spans()) {
        let mut merger = SpanMerger::new();
        for span in spans {
            merger.add(span);
        }

        let merged = merger.merge();

        // Property: Merged spans should not overlap
        for window in merged.windows(2) {
            assert!(window[0].range.end <= window[1].range.start);
        }
    }

    // Cache Properties
    #[test]
    fn prop_cache_get_after_set(content in ".*", data in ".*") {
        let cache = AnalyticalCache::new(3600);
        let result = CachedResult::new(data.clone());

        cache.insert_with_content(&content, result);
        let retrieved = cache.get_by_content(&content);

        // Property: What you put in, you get out (within TTL)
        assert_eq!(retrieved.unwrap().data, data);
    }
}

fn arbitrary_spans() -> impl Strategy<Value = Vec<HighlightSpan>> {
    prop::collection::vec(
        (0..100usize, 0..50usize, 0..5u8).prop_map(|(start, len, priority)| {
            HighlightSpan::new(
                start..start + len,
                Style::default(),
                match priority {
                    0 => HighlightSource::Plain,
                    1 => HighlightSource::Syntax,
                    2 => HighlightSource::Structural,
                    3 => HighlightSource::Relational,
                    _ => HighlightSource::Analytical,
                },
            )
        }),
        0..20,
    )
}
```

### 5. Performance Tests

**Purpose**: Validate performance requirements

**Characteristics**:
- Benchmarks with criterion.rs
- Measure latency, throughput
- Regression detection
- Resource monitoring

**Examples**:

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_tier1(c: &mut Criterion) {
    let mut engine = SemanticHighlightEngine::new(None);
    let text = "MUST <thinking>analyze</thinking> with @high priority #file.rs ?hole";

    c.bench_function("tier1_highlight_line", |b| {
        b.iter(|| {
            engine.highlight_line(black_box(text))
        });
    });
}

fn bench_tier2(c: &mut Criterion) {
    let mut engine = SemanticHighlightEngine::new(None);
    let text = "Dr. Smith discussed the API with John. He said the system processes data efficiently.";

    // Warm up cache
    engine.highlight_line(text);

    c.bench_function("tier2_cached", |b| {
        b.iter(|| {
            engine.highlight_line(black_box(text))
        });
    });
}

criterion_group!(benches, bench_tier1, bench_tier2);
criterion_main!(benches);
```

---

## Feature-Specific Test Cases

### Tier 1: Structural

**XML Tags**:
- [ ] Single tag detection
- [ ] Nested tags
- [ ] Unclosed tags (error case)
- [ ] Self-closing tags
- [ ] Tags with attributes

**RFC 2119 Keywords**:
- [ ] All keywords (MUST, SHOULD, MAY, etc.)
- [ ] Case sensitivity
- [ ] Keywords in context (not mid-word)

**Modality Detection**:
- [ ] All modality levels (certain, likely, possible, speculative)
- [ ] Negation handling
- [ ] Confidence scoring

**Ambiguity Detection**:
- [ ] Vague language (some, many, few)
- [ ] Hedging phrases (might be, could be)
- [ ] Clear vs ambiguous text

**Domain Patterns**:
- [ ] File paths (#file.rs)
- [ ] Symbols (@symbol)
- [ ] Typed holes (?hole)

### Tier 2: Relational

**Entity Recognition**:
- [ ] Person names (full, partial)
- [ ] Organizations
- [ ] Locations
- [ ] Technical terms
- [ ] Confidence thresholds

**Relationships**:
- [ ] SVO extraction (subject-verb-object)
- [ ] Dependency relations
- [ ] Confidence scoring

**Semantic Roles**:
- [ ] Agent detection
- [ ] Patient detection
- [ ] Instrument, Location, Time
- [ ] Beneficiary

**Coreference Resolution**:
- [ ] Pronoun resolution (he, she, it)
- [ ] Partial name matching
- [ ] Nominal references ("the system")
- [ ] Chain construction

**Anaphora Resolution**:
- [ ] Personal pronouns
- [ ] Demonstratives (this, that)
- [ ] Possessives
- [ ] Distance-based resolution

### Tier 3: Analytical

**Discourse Analysis**:
- [ ] Segment detection
- [ ] Relation classification (all 8 types)
- [ ] LLM call with timeout
- [ ] JSON parsing
- [ ] Invalid range filtering

**Contradiction Detection**:
- [ ] Direct contradictions
- [ ] Temporal contradictions
- [ ] Semantic contradictions
- [ ] Implication contradictions
- [ ] Confidence filtering

**Pragmatics Analysis**:
- [ ] Presupposition detection
- [ ] Implicature detection
- [ ] Speech act classification
- [ ] Indirect speech

**Error Handling**:
- [ ] Timeout handling (30s)
- [ ] Retry logic (3 attempts, exponential backoff)
- [ ] Malformed JSON
- [ ] API rate limiting
- [ ] Network errors

### Infrastructure

**Caching**:
- [ ] LRU eviction (Tier 2)
- [ ] Content-hash deduplication (Tier 3)
- [ ] TTL expiration
- [ ] Cache invalidation
- [ ] Stats tracking

**Batching**:
- [ ] Request aggregation
- [ ] Priority ordering
- [ ] Rate limiting (token bucket)
- [ ] Deduplication
- [ ] Batch size limits

**Background Processing**:
- [ ] Request queueing
- [ ] Concurrent processing
- [ ] Result storage
- [ ] Error handling
- [ ] Cancellation

**Incremental Analysis**:
- [ ] Dirty region tracking
- [ ] Region merging
- [ ] Debouncing (250ms)
- [ ] Cache invalidation on change

---

## Test Fixtures

```
tests/fixtures/
├── documents/
│   ├── sample_spec.md           # Technical specification
│   ├── conversation.txt         # Multi-speaker dialogue
│   ├── code_with_comments.rs    # Source code with docs
│   └── academic_paper.md        # Formal writing
├── llm_responses/
│   ├── discourse_valid.json     # Valid discourse response
│   ├── discourse_invalid.json   # Malformed JSON
│   ├── contradiction_valid.json
│   └── pragmatics_valid.json
└── expected_outputs/
    ├── spec_tier1_spans.json    # Expected Tier 1 output
    ├── spec_tier2_spans.json
    └── spec_tier3_spans.json
```

---

## Test Infrastructure

### Mocks

```rust
/// Mock LLM service for testing
pub struct MockLlmService {
    responses: Arc<Mutex<VecDeque<String>>>,
    delay: Duration,
}

impl MockLlmService {
    pub fn new() -> Self {
        Self {
            responses: Arc::new(Mutex::new(VecDeque::new())),
            delay: Duration::from_millis(100),
        }
    }

    pub fn add_response(&self, response: String) {
        self.responses.lock().unwrap().push_back(response);
    }

    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }
}

#[async_trait]
impl LlmService for MockLlmService {
    async fn generate(&self, _prompt: &str) -> Result<String> {
        tokio::time::sleep(self.delay).await;

        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| anyhow::anyhow!("No more mock responses"))
    }
}
```

### Test Helpers

```rust
pub mod helpers {
    /// Create engine with preset fixtures
    pub fn engine_with_fixtures() -> SemanticHighlightEngine {
        let settings = HighlightSettings::default();
        let llm = Arc::new(MockLlmService::new());
        SemanticHighlightEngine::with_settings(settings, Some(llm))
    }

    /// Load fixture file
    pub fn load_fixture(name: &str) -> String {
        std::fs::read_to_string(format!("tests/fixtures/{}", name))
            .expect("Fixture file not found")
    }

    /// Assert spans match expected
    pub fn assert_spans_eq(actual: &[HighlightSpan], expected: &[HighlightSpan]) {
        assert_eq!(actual.len(), expected.len(), "Span count mismatch");
        for (a, e) in actual.iter().zip(expected) {
            assert_eq!(a.range, e.range, "Range mismatch");
            assert_eq!(a.source, e.source, "Source mismatch");
        }
    }
}
```

---

## CI/CD Integration

### GitHub Actions Workflow

```yaml
name: Semantic Highlighter Tests

on: [push, pull_request]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Run unit tests
        run: cargo test --lib

      - name: Run integration tests
        run: cargo test --test '*'

      - name: Run benchmarks (comparison only)
        run: cargo bench --no-run

      - name: Check coverage
        run: |
          cargo install cargo-tarpaulin
          cargo tarpaulin --out Xml

      - name: Upload coverage
        uses: codecov/codecov-action@v3
```

---

## Acceptance Criteria

### Phase-Specific Criteria

**Phase 2 (Tier 3 LLM Integration)**:
- [ ] All Tier 3 analyzers have unit tests
- [ ] Mock LLM service used in tests
- [ ] JSON parsing tests (valid, invalid, edge cases)
- [ ] Error handling tests (timeout, retry, malformed)
- [ ] Test coverage >80%

**Phase 3 (Background Processing)**:
- [ ] Batch processing tests with mock requests
- [ ] Concurrent processing tests
- [ ] Rate limiting tests
- [ ] Error handling and retry tests
- [ ] Result storage tests

**Phase 4 (Incremental Analysis)**:
- [ ] Dirty region tracking tests
- [ ] Debouncing tests
- [ ] Cache invalidation tests
- [ ] Property tests for region merging

**Phase 5 (ICS Integration)**:
- [ ] Text change hook tests
- [ ] Rendering integration tests
- [ ] Settings update tests
- [ ] Command execution tests

**Phase 6 (Comprehensive Testing)**:
- [ ] All unit tests passing
- [ ] All integration tests passing
- [ ] Performance benchmarks meet targets
- [ ] Property tests find no violations
- [ ] Coverage targets met (70%+ overall)

---

## Performance Targets

| Tier | Target | Measurement | Acceptance |
|------|--------|-------------|------------|
| 1    | <5ms   | Per-line highlight | 95th percentile |
| 2    | <200ms | Full analysis (uncached) | 95th percentile |
| 2    | <2ms   | Cached lookup | 99th percentile |
| 3    | 2-10s  | API call (per document) | Median |
| 3    | <1ms   | Cached lookup | 99th percentile |

---

## Estimated Effort

- Test infrastructure setup: 1 day
- Unit tests (all tiers): 2 days
- Integration tests: 1 day
- Property tests: 0.5 days
- Performance benchmarks: 0.5 days
- CI/CD integration: 0.5 days

**Total: 5.5 days**

---

## References

- Testing best practices: <https://matklad.github.io/2021/05/31/how-to-test.html>
- Property testing: <https://github.com/proptest-rs/proptest>
- Criterion benchmarks: <https://github.com/bheisler/criterion.rs>
