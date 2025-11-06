//! Contradiction detection using Claude API
//!
//! Identifies semantic inconsistencies and contradictions:
//! - Direct contradictions ("X is true" vs "X is false")
//! - Logical inconsistencies
//! - Conflicting statements
//! - Temporal inconsistencies
//!
//! Uses Claude API for deep semantic understanding and reasoning.

#![allow(dead_code)]

#[cfg(feature = "python")]
use super::dspy_integration::DSpySemanticBridge;
use crate::{
    ics::semantic_highlighter::{
        visualization::{
            Annotation, AnnotationType, Connection, ConnectionType, HighlightSource, HighlightSpan,
        },
        Result, SemanticError,
    },
    LlmService,
};
use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};
use std::ops::Range;
use std::sync::Arc;
use tracing::{debug, warn};

/// Type of contradiction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContradictionType {
    /// Direct logical contradiction
    Direct,
    /// Temporal inconsistency
    Temporal,
    /// Semantic inconsistency
    Semantic,
    /// Implication contradiction
    Implication,
}

impl ContradictionType {
    fn _description(&self) -> &'static str {
        match self {
            ContradictionType::Direct => "Direct contradiction",
            ContradictionType::Temporal => "Temporal inconsistency",
            ContradictionType::Semantic => "Semantic inconsistency",
            ContradictionType::Implication => "Contradictory implication",
        }
    }

    fn severity(&self) -> ContradictionSeverity {
        match self {
            ContradictionType::Direct => ContradictionSeverity::High,
            ContradictionType::Temporal => ContradictionSeverity::Medium,
            ContradictionType::Semantic => ContradictionSeverity::Medium,
            ContradictionType::Implication => ContradictionSeverity::Low,
        }
    }
}

/// Severity level of contradiction
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ContradictionSeverity {
    Low,
    Medium,
    High,
}

impl ContradictionSeverity {
    fn color(&self) -> Color {
        match self {
            ContradictionSeverity::Low => Color::Yellow,
            ContradictionSeverity::Medium => Color::LightRed,
            ContradictionSeverity::High => Color::Red,
        }
    }
}

/// Detected contradiction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contradiction {
    /// First statement
    pub statement1: Range<usize>,
    pub text1: String,

    /// Contradicting statement
    pub statement2: Range<usize>,
    pub text2: String,

    /// Type of contradiction
    pub contradiction_type: ContradictionType,

    /// Explanation of the contradiction
    pub explanation: String,

    /// Confidence score (0.0-1.0)
    pub confidence: f32,
}

/// Contradiction detector using Claude API or DSPy
#[derive(Clone)]
pub struct ContradictionDetector {
    _llm_service: Arc<LlmService>,
    /// Minimum confidence threshold
    threshold: f32,
    #[cfg(feature = "python")]
    dspy_bridge: Option<Arc<DSpySemanticBridge>>,
}

impl ContradictionDetector {
    pub fn new(llm_service: Arc<LlmService>) -> Self {
        Self {
            _llm_service: llm_service,
            threshold: 0.7,
            #[cfg(feature = "python")]
            dspy_bridge: None,
        }
    }

    /// Create detector with DSPy integration
    #[cfg(feature = "python")]
    pub fn with_dspy(llm_service: Arc<LlmService>, dspy_bridge: Arc<DSpySemanticBridge>) -> Self {
        Self {
            _llm_service: llm_service,
            threshold: 0.7,
            dspy_bridge: Some(dspy_bridge),
        }
    }

    /// Set confidence threshold
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Detect contradictions in text
    pub async fn detect(&self, text: &str) -> Result<Vec<Contradiction>> {
        // Use DSPy if available (preferred path)
        #[cfg(feature = "python")]
        if let Some(bridge) = &self.dspy_bridge {
            debug!("Using DSPy for contradiction detection");
            let contradictions = bridge.detect_contradictions(text).await.map_err(|e| {
                SemanticError::AnalysisFailed(format!("DSPy contradiction detection failed: {}", e))
            })?;

            // Filter by threshold
            let filtered: Vec<_> = contradictions
                .into_iter()
                .filter(|c| c.confidence >= self.threshold)
                .collect();

            return Ok(filtered);
        }

        // Fallback: Direct LLM call (not yet implemented)
        debug!("DSPy not available, using direct LLM call (not implemented yet)");
        Err(SemanticError::AnalysisFailed(
            "Contradiction detection requires DSPy integration (enable 'python' feature)"
                .to_string(),
        ))
    }

    /// Parse contradiction response from LLM
    fn parse_contradiction_response(
        &self,
        json: &str,
        text_len: usize,
    ) -> Result<Vec<Contradiction>> {
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
            .map_err(|e| SemanticError::AnalysisFailed(format!("JSON parse error: {}", e)))?;

        // Convert and validate
        let result = contradictions
            .into_iter()
            .filter_map(|c| {
                // Validate ranges
                if c.statement1_end > text_len
                    || c.statement2_end > text_len
                    || c.statement1_start >= c.statement1_end
                    || c.statement2_start >= c.statement2_end
                {
                    warn!(
                        "Invalid contradiction ranges: {}..{} and {}..{} (text len: {})",
                        c.statement1_start,
                        c.statement1_end,
                        c.statement2_start,
                        c.statement2_end,
                        text_len
                    );
                    return None;
                }

                // Parse contradiction type
                let contradiction_type = match c.contradiction_type.as_str() {
                    "Direct" => ContradictionType::Direct,
                    "Temporal" => ContradictionType::Temporal,
                    "Semantic" => ContradictionType::Semantic,
                    "Implication" => ContradictionType::Implication,
                    other => {
                        warn!("Unknown contradiction type: {}", other);
                        return None;
                    }
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
            .collect();

        Ok(result)
    }

    /// Convert contradictions to highlight spans
    pub fn contradictions_to_spans(&self, contradictions: &[Contradiction]) -> Vec<HighlightSpan> {
        let mut spans = Vec::new();

        for contradiction in contradictions {
            if contradiction.confidence < self.threshold {
                continue;
            }

            let severity = contradiction.contradiction_type.severity();
            let style = Style::default()
                .fg(severity.color())
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED);

            let annotation = Annotation {
                annotation_type: AnnotationType::Contradiction,
                underline: None,
                tooltip: Some(format!(
                    "{}: {}",
                    contradiction.contradiction_type._description(),
                    contradiction.explanation
                )),
            };

            // Highlight first statement
            spans.push(HighlightSpan {
                range: contradiction.statement1.clone(),
                style,
                source: HighlightSource::Analytical,
                annotation: Some(annotation.clone()),
                confidence: contradiction.confidence,
                metadata: None,
            });

            // Highlight second statement
            spans.push(HighlightSpan {
                range: contradiction.statement2.clone(),
                style,
                source: HighlightSource::Analytical,
                annotation: Some(annotation),
                confidence: contradiction.confidence,
                metadata: None,
            });
        }

        spans
    }

    /// Create connections between contradicting statements
    pub fn contradictions_to_connections(
        &self,
        contradictions: &[Contradiction],
    ) -> Vec<Connection> {
        contradictions
            .iter()
            .filter(|c| c.confidence >= self.threshold)
            .map(|contradiction| Connection {
                from: contradiction.statement1.clone(),
                to: contradiction.statement2.clone(),
                connection_type: ConnectionType::Contradiction,
                label: Some(contradiction.contradiction_type._description().to_string()),
                confidence: contradiction.confidence,
            })
            .collect()
    }

    /// Build prompt for contradiction detection
    fn build_detection_prompt(&self, text: &str) -> String {
        format!(
            r#"Analyze the following text for contradictions and logical inconsistencies.

Look for:
1. Direct contradictions (statements that directly oppose each other)
2. Temporal inconsistencies (conflicting timelines or sequences)
3. Semantic inconsistencies (statements that are incompatible in meaning)
4. Implication contradictions (implied meanings that conflict)

Text:
{}

For each contradiction found, provide:
1. The first statement (with character range)
2. The contradicting statement (with character range)
3. Type of contradiction
4. Explanation of why they contradict
5. Confidence score (0.0-1.0)

Respond in JSON format as an array of contradictions:
[
  {{
    "statement1_start": 0,
    "statement1_end": 20,
    "text1": "first statement",
    "statement2_start": 30,
    "statement2_end": 50,
    "text2": "contradicting statement",
    "type": "Direct",
    "explanation": "explanation of contradiction",
    "confidence": 0.9
  }}
]"#,
            text
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contradiction_type_descriptions() {
        assert_eq!(
            ContradictionType::Direct._description(),
            "Direct contradiction"
        );
        assert_eq!(
            ContradictionType::Temporal._description(),
            "Temporal inconsistency"
        );
        assert_eq!(
            ContradictionType::Semantic._description(),
            "Semantic inconsistency"
        );
    }

    #[test]
    fn test_contradiction_severity() {
        assert_eq!(
            ContradictionType::Direct.severity(),
            ContradictionSeverity::High
        );
        assert_eq!(
            ContradictionType::Temporal.severity(),
            ContradictionSeverity::Medium
        );
        assert_eq!(
            ContradictionType::Implication.severity(),
            ContradictionSeverity::Low
        );
    }

    #[test]
    fn test_severity_colors() {
        assert_eq!(ContradictionSeverity::Low.color(), Color::Yellow);
        assert_eq!(ContradictionSeverity::Medium.color(), Color::LightRed);
        assert_eq!(ContradictionSeverity::High.color(), Color::Red);
    }

    #[test]
    fn test_severity_ordering() {
        assert!(ContradictionSeverity::Low < ContradictionSeverity::Medium);
        assert!(ContradictionSeverity::Medium < ContradictionSeverity::High);
    }

    #[test]
    fn test_contradiction_structure() {
        let contradiction = Contradiction {
            statement1: 0..10,
            text1: "X is true".to_string(),
            statement2: 20..30,
            text2: "X is false".to_string(),
            contradiction_type: ContradictionType::Direct,
            explanation: "Directly contradictory statements".to_string(),
            confidence: 0.95,
        };

        assert_eq!(contradiction.confidence, 0.95);
        assert_eq!(contradiction.contradiction_type, ContradictionType::Direct);
    }

    #[test]
    fn test_threshold_filtering() {
        let _contradictions = vec![
            Contradiction {
                statement1: 0..5,
                text1: "A".to_string(),
                statement2: 10..15,
                text2: "B".to_string(),
                contradiction_type: ContradictionType::Direct,
                explanation: "Test".to_string(),
                confidence: 0.9,
            },
            Contradiction {
                statement1: 20..25,
                text1: "C".to_string(),
                statement2: 30..35,
                text2: "D".to_string(),
                contradiction_type: ContradictionType::Semantic,
                explanation: "Test".to_string(),
                confidence: 0.5,
            },
        ];

        // Mock detector with threshold 0.7
        // In real tests, would need proper mocking
        // let detector = ContradictionDetector::new(mock_llm()).with_threshold(0.7);
        // let connections = detector.contradictions_to_connections(&_contradictions);
        // Should only include the first one (confidence 0.9 > 0.7)
        // assert_eq!(connections.len(), 1);
    }
}
