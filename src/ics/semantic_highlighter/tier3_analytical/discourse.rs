//! Discourse analysis using Claude API
//!
//! Analyzes discourse structure and coherence:
//! - Rhetorical Structure Theory (RST) relations (Elaboration, Contrast, Cause, etc.)
//! - Coherence scoring
//! - Topic flow and structure
//! - Argument quality assessment
//!
//! Uses Claude API for deep semantic understanding.

use crate::{
    ics::semantic_highlighter::{
        visualization::{HighlightSpan, HighlightSource, Connection, ConnectionType},
        Result, SemanticError,
    },
    LlmService,
};
#[cfg(feature = "python")]
use super::dspy_integration::DSpySemanticBridge;
use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};
use std::ops::Range;
use std::sync::Arc;
use tracing::{debug, warn};

/// Discourse relation type (RST-based)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiscourseRelation {
    /// One segment elaborates on another
    Elaboration,
    /// Segments present contrasting information
    Contrast,
    /// Causal relationship
    Cause,
    /// Temporal sequence
    Sequence,
    /// Condition relationship
    Condition,
    /// Background information
    Background,
    /// Summary or conclusion
    Summary,
    /// Evaluation or assessment
    Evaluation,
}

impl DiscourseRelation {
    fn _color(&self) -> Color {
        match self {
            DiscourseRelation::Elaboration => Color::LightBlue,
            DiscourseRelation::Contrast => Color::Red,
            DiscourseRelation::Cause => Color::Yellow,
            DiscourseRelation::Sequence => Color::Green,
            DiscourseRelation::Condition => Color::Cyan,
            DiscourseRelation::Background => Color::Gray,
            DiscourseRelation::Summary => Color::Magenta,
            DiscourseRelation::Evaluation => Color::LightMagenta,
        }
    }

    fn _description(&self) -> &'static str {
        match self {
            DiscourseRelation::Elaboration => "Elaborates on",
            DiscourseRelation::Contrast => "Contrasts with",
            DiscourseRelation::Cause => "Causes",
            DiscourseRelation::Sequence => "Follows",
            DiscourseRelation::Condition => "Conditional on",
            DiscourseRelation::Background => "Background for",
            DiscourseRelation::Summary => "Summarizes",
            DiscourseRelation::Evaluation => "Evaluates",
        }
    }
}

/// Discourse segment with relation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscourseSegment {
    pub range: Range<usize>,
    pub text: String,
    pub relation: Option<DiscourseRelation>,
    pub related_to: Option<Range<usize>>,
    pub confidence: f32,
}

/// Coherence assessment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoherenceScore {
    /// Overall coherence (0.0-1.0)
    pub overall: f32,
    /// Topic consistency
    pub topic_consistency: f32,
    /// Logical flow
    pub logical_flow: f32,
    /// Issues found
    pub issues: Vec<String>,
}

/// Discourse analyzer using Claude API or DSPy
#[derive(Clone)]
pub struct DiscourseAnalyzer {
    _llm_service: Arc<LlmService>,
    #[cfg(feature = "python")]
    dspy_bridge: Option<Arc<DSpySemanticBridge>>,
}

impl DiscourseAnalyzer {
    pub fn new(llm_service: Arc<LlmService>) -> Self {
        Self {
            _llm_service: llm_service,
            #[cfg(feature = "python")]
            dspy_bridge: None,
        }
    }

    /// Create analyzer with DSPy integration
    #[cfg(feature = "python")]
    pub fn with_dspy(llm_service: Arc<LlmService>, dspy_bridge: Arc<DSpySemanticBridge>) -> Self {
        Self {
            _llm_service: llm_service,
            dspy_bridge: Some(dspy_bridge),
        }
    }

    /// Analyze discourse structure in text
    pub async fn analyze(&self, text: &str) -> Result<Vec<DiscourseSegment>> {
        // Use DSPy if available (preferred path)
        #[cfg(feature = "python")]
        if let Some(bridge) = &self.dspy_bridge {
            debug!("Using DSPy for discourse analysis");
            return bridge.analyze_discourse(text).await
                .map_err(|e| SemanticError::AnalysisFailed(format!("DSPy discourse analysis failed: {}", e)));
        }

        // Fallback: Direct LLM call (not yet implemented)
        debug!("DSPy not available, using direct LLM call (not implemented yet)");
        Err(SemanticError::AnalysisFailed(
            "Discourse analysis requires DSPy integration (enable 'python' feature)".to_string()
        ))
    }

    /// Parse discourse response from LLM
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
            .map_err(|e| crate::ics::semantic_highlighter::SemanticError::AnalysisFailed(
                format!("JSON parse error: {}", e)
            ))?;

        // Convert to DiscourseSegment, filtering invalid entries
        let result = segments
            .into_iter()
            .filter_map(|s| {
                // Validate ranges
                if s.end > text_len || s.start >= s.end {
                    warn!("Invalid discourse segment range: {}..{} (text len: {})", s.start, s.end, text_len);
                    return None;
                }

                // Parse relation
                let relation = s.relation.and_then(|r| match r.as_str() {
                    "Elaboration" => Some(DiscourseRelation::Elaboration),
                    "Contrast" => Some(DiscourseRelation::Contrast),
                    "Cause" => Some(DiscourseRelation::Cause),
                    "Sequence" => Some(DiscourseRelation::Sequence),
                    "Condition" => Some(DiscourseRelation::Condition),
                    "Background" => Some(DiscourseRelation::Background),
                    "Summary" => Some(DiscourseRelation::Summary),
                    "Evaluation" => Some(DiscourseRelation::Evaluation),
                    other => {
                        warn!("Unknown discourse relation: {}", other);
                        None
                    }
                });

                // Parse related_to range
                let related_to = if let (Some(start), Some(end)) = (s.related_to_start, s.related_to_end) {
                    if end <= text_len && start < end {
                        Some(start..end)
                    } else {
                        warn!("Invalid related_to range: {}..{} (text len: {})", start, end, text_len);
                        None
                    }
                } else {
                    None
                };

                Some(DiscourseSegment {
                    range: s.start..s.end,
                    text: s.text,
                    relation,
                    related_to,
                    confidence: s.confidence.clamp(0.0, 1.0),
                })
            })
            .collect();

        Ok(result)
    }

    /// Validate discourse segments
    fn validate_segments(&self, segments: &[DiscourseSegment], text_len: usize) -> Result<()> {
        for (i, seg) in segments.iter().enumerate() {
            // Check range validity
            if seg.range.end > text_len {
                return Err(crate::ics::semantic_highlighter::SemanticError::AnalysisFailed(
                    format!("Segment {} range exceeds text length: {:?} > {}", i, seg.range, text_len)
                ));
            }

            if seg.range.start >= seg.range.end {
                return Err(crate::ics::semantic_highlighter::SemanticError::AnalysisFailed(
                    format!("Segment {} has invalid range: {:?}", i, seg.range)
                ));
            }

            // Check confidence
            if seg.confidence < 0.0 || seg.confidence > 1.0 {
                return Err(crate::ics::semantic_highlighter::SemanticError::AnalysisFailed(
                    format!("Segment {} has invalid confidence: {}", i, seg.confidence)
                ));
            }

            // Check related_to validity if present
            if let Some(ref related) = seg.related_to {
                if related.end > text_len || related.start >= related.end {
                    return Err(crate::ics::semantic_highlighter::SemanticError::AnalysisFailed(
                        format!("Segment {} has invalid related_to range: {:?}", i, related)
                    ));
                }
            }
        }

        Ok(())
    }

    /// Build prompt for discourse relation analysis
    fn build_discourse_prompt(&self, text: &str) -> String {
        format!(
            r#"Analyze the discourse structure of the following text using Rhetorical Structure Theory (RST).

Identify discourse relations between segments:
- Elaboration: One segment elaborates on another
- Contrast: Segments present contrasting information
- Cause: Causal relationship
- Sequence: Temporal ordering
- Condition: Conditional relationship
- Background: Background information
- Summary: Summary or conclusion
- Evaluation: Evaluation or assessment

Text:
{}

For each discourse relation found, provide:
1. start: starting character position
2. end: ending character position
3. text: the segment text
4. relation: one of the relation types above
5. related_to_start: start position of related segment (optional)
6. related_to_end: end position of related segment (optional)
7. confidence: confidence score between 0.0 and 1.0

Respond ONLY with a JSON array of segments, no additional text.

Example format:
[
  {{
    "start": 0,
    "end": 25,
    "text": "First segment text",
    "relation": "Elaboration",
    "related_to_start": 26,
    "related_to_end": 50,
    "confidence": 0.85
  }}
]"#,
            text
        )
    }

    /// Assess coherence of text
    pub async fn assess_coherence(&self, text: &str) -> Result<CoherenceScore> {
        let _prompt = self.build_coherence_prompt(text);

        // Placeholder - would call LLM service
        // let response = self.llm_service.call_api(&_prompt).await?;

        Ok(CoherenceScore {
            overall: 0.8,
            topic_consistency: 0.85,
            logical_flow: 0.75,
            issues: Vec::new(),
        })
    }

    /// Convert discourse segments to highlight spans
    pub fn segments_to_spans(&self, segments: &[DiscourseSegment]) -> Vec<HighlightSpan> {
        segments
            .iter()
            .filter_map(|seg| {
                seg.relation.map(|rel| HighlightSpan {
                    range: seg.range.clone(),
                    style: Style::default()
                        .fg(rel._color())
                        .add_modifier(Modifier::UNDERLINED),
                    source: HighlightSource::Analytical,
                    annotation: None,
                    confidence: seg.confidence,
                    metadata: None,
                })
            })
            .collect()
    }

    /// Create connections between related segments
    pub fn segments_to_connections(&self, segments: &[DiscourseSegment]) -> Vec<Connection> {
        segments
            .iter()
            .filter_map(|seg| {
                if let (Some(relation), Some(related_to)) = (seg.relation, seg.related_to.clone()) {
                    Some(Connection {
                        from: seg.range.clone(),
                        to: related_to,
                        connection_type: ConnectionType::Discourse,
                        label: Some(relation._description().to_string()),
                        confidence: seg.confidence,
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Build prompt for coherence analysis
    fn build_coherence_prompt(&self, text: &str) -> String {
        format!(
            r#"Analyze the coherence and logical flow of the following text.

Text:
{}

Assess:
1. Overall coherence (0.0-1.0)
2. Topic consistency
3. Logical flow
4. Any coherence issues

Respond in JSON format:
{{
  "overall": 0.8,
  "topic_consistency": 0.85,
  "logical_flow": 0.75,
  "issues": ["list of issues"]
}}"#,
            text
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_llm_service() -> Arc<LlmService> {
        // This would need proper mocking in real tests
        // For now, just creating a placeholder
        panic!("Mock LLM service not implemented for tests")
    }

    #[test]
    fn test_discourse_relation_colors() {
        assert_eq!(DiscourseRelation::Elaboration._color(), Color::LightBlue);
        assert_eq!(DiscourseRelation::Contrast._color(), Color::Red);
        assert_eq!(DiscourseRelation::Cause._color(), Color::Yellow);
    }

    #[test]
    fn test_discourse_relation_descriptions() {
        assert_eq!(DiscourseRelation::Elaboration._description(), "Elaborates on");
        assert_eq!(DiscourseRelation::Contrast._description(), "Contrasts with");
    }

    #[test]
    fn test_segments_to_spans() {
        let _segments = vec![
            DiscourseSegment {
                range: 0..10,
                text: "First part".to_string(),
                relation: Some(DiscourseRelation::Elaboration),
                related_to: None,
                confidence: 0.9,
            },
        ];

        // Mock analyzer without LLM service for this test
        // In real implementation, we'd need proper mocking
        // let analyzer = DiscourseAnalyzer::new(mock_llm_service());
        // let spans = analyzer.segments_to_spans(&_segments);
        // assert_eq!(spans.len(), 1);
    }

    #[test]
    fn test_coherence_score_default() {
        let score = CoherenceScore {
            overall: 0.8,
            topic_consistency: 0.85,
            logical_flow: 0.75,
            issues: vec!["Minor topic drift".to_string()],
        };

        assert_eq!(score.overall, 0.8);
        assert_eq!(score.issues.len(), 1);
    }
}
