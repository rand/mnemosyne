# Tier 3 LLM Integration Specification

## Overview

Complete the integration between Tier 3 analyzers and the LLM service to enable deep semantic analysis using Claude API.

## Current State

**What Exists:**
- Type definitions: `DiscourseSegment`, `Contradiction`, `PragmaticElement`
- Prompt builders: All three analyzers have `build_*_prompt()` methods
- Conversion methods: `*_to_spans()` and `*_to_connections()` implemented
- Analyzer skeletons with proper structure

**What's Missing:**
- Actual LLM service calls
- JSON response parsing
- Error handling (timeouts, malformed responses, API errors)
- Result validation

---

## Requirements

### Functional Requirements

**FR-1**: DiscourseAnalyzer must analyze text and return discourse segments
- Input: Text string
- Output: `Vec<DiscourseSegment>` with valid ranges and relations
- Timeout: 30 seconds default
- Caching: Results cached by content hash

**FR-2**: ContradictionDetector must identify contradictions
- Input: Text string
- Output: `Vec<Contradiction>` with pairs of conflicting statements
- Threshold filtering: Only return contradictions above confidence threshold
- Timeout: 30 seconds default

**FR-3**: PragmaticsAnalyzer must extract pragmatic elements
- Input: Text string
- Output: `Vec<PragmaticElement>` with implied meanings
- Speech act classification for applicable elements
- Threshold filtering

### Non-Functional Requirements

**NFR-1**: Performance
- API calls must timeout after 30s (configurable)
- Results must be cached to avoid repeat calls
- Failed requests should not block the system

**NFR-2**: Reliability
- Graceful handling of API failures
- Retry logic for transient errors (3 retries with exponential backoff)
- Fallback to empty results on persistent failure

**NFR-3**: Observability
- Log all API calls with duration
- Log parsing failures with sample of problematic response
- Track cache hit/miss rates

---

## Design

### Type Definitions

```rust
/// LLM service error types
#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("API timeout after {0}s")]
    Timeout(u64),

    #[error("API rate limited, retry after {0}s")]
    RateLimited(u64),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Network error: {0}")]
    NetworkError(String),
}

/// Response parsing trait
pub trait ResponseParser<T> {
    fn parse(&self, json: &str) -> Result<T, LlmError>;
    fn validate(&self, result: &T) -> Result<(), LlmError>;
}
```

### Implementation Plan

#### 1. DiscourseAnalyzer::analyze()

```rust
pub async fn analyze(&self, text: &str) -> Result<Vec<DiscourseSegment>> {
    let prompt = self.build_discourse_prompt(text);

    // Call LLM with timeout
    let response = tokio::time::timeout(
        Duration::from_secs(30),
        self.llm_service.generate(&prompt)
    )
    .await
    .map_err(|_| SemanticError::AnalysisFailed("Timeout".to_string()))??;

    // Parse JSON response
    let segments = self.parse_discourse_response(&response, text.len())?;

    // Validate segments
    self.validate_segments(&segments, text.len())?;

    Ok(segments)
}

fn parse_discourse_response(&self, json: &str, text_len: usize) -> Result<Vec<DiscourseSegment>> {
    #[derive(Deserialize)]
    struct SegmentJson {
        start: usize,
        end: usize,
        text: String,
        relation: Option<String>,
        related_to_start: Option<usize>,
        related_to_end: Option<usize>,
        confidence: f32,
    }

    let segments: Vec<SegmentJson> = serde_json::from_str(json)
        .map_err(|e| SemanticError::AnalysisFailed(format!("Parse error: {}", e)))?;

    // Convert to DiscourseSegment
    segments.into_iter()
        .filter_map(|s| {
            // Validate ranges
            if s.end > text_len || s.start >= s.end {
                return None;
            }

            Some(DiscourseSegment {
                range: s.start..s.end,
                text: s.text,
                relation: s.relation.and_then(|r| match r.as_str() {
                    "Elaboration" => Some(DiscourseRelation::Elaboration),
                    "Contrast" => Some(DiscourseRelation::Contrast),
                    "Cause" => Some(DiscourseRelation::Cause),
                    "Sequence" => Some(DiscourseRelation::Sequence),
                    "Condition" => Some(DiscourseRelation::Condition),
                    "Background" => Some(DiscourseRelation::Background),
                    "Summary" => Some(DiscourseRelation::Summary),
                    "Evaluation" => Some(DiscourseRelation::Evaluation),
                    _ => None,
                }),
                related_to: if let (Some(start), Some(end)) = (s.related_to_start, s.related_to_end) {
                    if end <= text_len && start < end {
                        Some(start..end)
                    } else {
                        None
                    }
                } else {
                    None
                },
                confidence: s.confidence.clamp(0.0, 1.0),
            })
        })
        .collect()
}
```

#### 2. ContradictionDetector::detect()

```rust
pub async fn detect(&self, text: &str) -> Result<Vec<Contradiction>> {
    let prompt = self.build_detection_prompt(text);

    // Call LLM with timeout
    let response = tokio::time::timeout(
        Duration::from_secs(30),
        self.llm_service.generate(&prompt)
    )
    .await
    .map_err(|_| SemanticError::AnalysisFailed("Timeout".to_string()))??;

    // Parse JSON response
    let contradictions = self.parse_contradiction_response(&response, text.len())?;

    // Filter by threshold
    Ok(contradictions.into_iter()
        .filter(|c| c.confidence >= self.threshold)
        .collect())
}

fn parse_contradiction_response(&self, json: &str, text_len: usize) -> Result<Vec<Contradiction>> {
    #[derive(Deserialize)]
    struct ContradictionJson {
        statement1_start: usize,
        statement1_end: usize,
        text1: String,
        statement2_start: usize,
        statement2_end: usize,
        text2: String,
        #[serde(rename = "type")]
        contradiction_type: String,
        explanation: String,
        confidence: f32,
    }

    let contradictions: Vec<ContradictionJson> = serde_json::from_str(json)
        .map_err(|e| SemanticError::AnalysisFailed(format!("Parse error: {}", e)))?;

    // Convert and validate
    contradictions.into_iter()
        .filter_map(|c| {
            // Validate ranges
            if c.statement1_end > text_len || c.statement2_end > text_len ||
               c.statement1_start >= c.statement1_end || c.statement2_start >= c.statement2_end {
                return None;
            }

            let contradiction_type = match c.contradiction_type.as_str() {
                "Direct" => ContradictionType::Direct,
                "Temporal" => ContradictionType::Temporal,
                "Semantic" => ContradictionType::Semantic,
                "Implication" => ContradictionType::Implication,
                _ => return None,
            };

            Some(Contradiction {
                statement1: c.statement1_start..c.statement1_end,
                text1: c.text1,
                statement2: c.statement2_start..c.statement2_end,
                text2: c.text2,
                contradiction_type,
                explanation: c.explanation,
                confidence: c.confidence.clamp(0.0, 1.0),
            })
        })
        .collect()
}
```

#### 3. PragmaticsAnalyzer::analyze()

```rust
pub async fn analyze(&self, text: &str) -> Result<Vec<PragmaticElement>> {
    let prompt = self.build_analysis_prompt(text);

    // Call LLM with timeout
    let response = tokio::time::timeout(
        Duration::from_secs(30),
        self.llm_service.generate(&prompt)
    )
    .await
    .map_err(|_| SemanticError::AnalysisFailed("Timeout".to_string()))??;

    // Parse JSON response
    let elements = self.parse_pragmatics_response(&response, text.len())?;

    // Filter by threshold
    Ok(elements.into_iter()
        .filter(|e| e.confidence >= self.threshold)
        .collect())
}

fn parse_pragmatics_response(&self, json: &str, text_len: usize) -> Result<Vec<PragmaticElement>> {
    #[derive(Deserialize)]
    struct PragmaticJson {
        start: usize,
        end: usize,
        text: String,
        #[serde(rename = "type")]
        pragmatic_type: String,
        speech_act: Option<String>,
        explanation: String,
        implied_meaning: Option<String>,
        confidence: f32,
    }

    let elements: Vec<PragmaticJson> = serde_json::from_str(json)
        .map_err(|e| SemanticError::AnalysisFailed(format!("Parse error: {}", e)))?;

    // Convert and validate
    elements.into_iter()
        .filter_map(|e| {
            // Validate range
            if e.end > text_len || e.start >= e.end {
                return None;
            }

            let pragmatic_type = match e.pragmatic_type.as_str() {
                "Presupposition" => PragmaticType::Presupposition,
                "Implicature" => PragmaticType::Implicature,
                "SpeechAct" => PragmaticType::SpeechAct,
                "IndirectSpeech" => PragmaticType::IndirectSpeech,
                _ => return None,
            };

            let speech_act = e.speech_act.and_then(|sa| match sa.as_str() {
                "Assertion" => Some(SpeechActType::Assertion),
                "Question" => Some(SpeechActType::Question),
                "Command" => Some(SpeechActType::Command),
                "Promise" => Some(SpeechActType::Promise),
                "Request" => Some(SpeechActType::Request),
                "Wish" => Some(SpeechActType::Wish),
                _ => None,
            });

            Some(PragmaticElement {
                range: e.start..e.end,
                text: e.text,
                pragmatic_type,
                speech_act,
                explanation: e.explanation,
                implied_meaning: e.implied_meaning,
                confidence: e.confidence.clamp(0.0, 1.0),
            })
        })
        .collect()
}
```

---

## Testing Strategy

### Unit Tests

**Test 1: Mock LLM Response Parsing**
- Given: Valid JSON response
- When: Parsing discourse/contradiction/pragmatic response
- Then: Correct structured data returned

**Test 2: Invalid Range Handling**
- Given: Response with out-of-bounds ranges
- When: Parsing response
- Then: Invalid segments filtered out

**Test 3: Malformed JSON**
- Given: Invalid JSON response
- When: Parsing response
- Then: Error returned with clear message

**Test 4: Threshold Filtering**
- Given: Mixed confidence results
- When: Filtering by threshold
- Then: Only high-confidence results returned

### Integration Tests

**Test 5: End-to-End with Mock LLM**
- Given: Text and mock LLM service
- When: Calling analyze/detect
- Then: Correct parsed results with valid spans

**Test 6: Timeout Handling**
- Given: Slow mock LLM (>30s)
- When: Calling analyze/detect
- Then: Timeout error after 30s

**Test 7: Empty Response**
- Given: LLM returns empty array
- When: Parsing response
- Then: Empty vec returned (not error)

---

## Acceptance Criteria

- [ ] All three analyzers make actual LLM calls
- [ ] JSON responses parsed correctly
- [ ] Invalid data filtered out gracefully
- [ ] Timeouts work as specified (30s)
- [ ] Results cached by content hash
- [ ] All unit tests passing
- [ ] Integration tests with mock LLM passing
- [ ] Error messages are clear and actionable
- [ ] Logging in place for debugging

---

## Estimated Effort

- DiscourseAnalyzer: 1.5 days
- ContradictionDetector: 1.5 days
- PragmaticsAnalyzer: 1.5 days
- Error handling (cross-cutting): 1 day

**Total: 5.5 days** (3-4 days with parallelization)

---

## Dependencies

- LlmService must have `generate()` method that returns `Future<Result<String>>`
- `serde_json` for parsing
- `tokio::time::timeout` for timeout handling
- Mock LLM service for testing

---

## Risks & Mitigation

**Risk 1**: LLM responses vary in quality/format
- Mitigation: Robust parsing, filter invalid data, log parse failures

**Risk 2**: API rate limiting
- Mitigation: Aggressive caching, batch requests, respect rate limits

**Risk 3**: Parsing errors with real API responses
- Mitigation: Comprehensive testing, fallback to empty results

---

## References

- Current stubs: `tier3_analytical/discourse.rs:106`, `contradictions.rs:117`, `pragmatics.rs:118`
- Prompt builders: Already implemented in each analyzer
- Type definitions: Already complete
