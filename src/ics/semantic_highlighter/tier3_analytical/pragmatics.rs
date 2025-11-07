//! Pragmatic analysis using Claude API
//!
//! Analyzes pragmatic aspects of language:
//! - Presuppositions (implied assumptions)
//! - Implicatures (implied meanings)
//! - Speech acts (assertions, questions, commands, promises)
//! - Politeness and formality levels
//! - Intended vs literal meaning
//!
//! Uses Claude API for nuanced pragmatic understanding.

#![allow(dead_code)]

#[cfg(feature = "python")]
use super::dspy_integration::DSpySemanticBridge;
use crate::{
    ics::semantic_highlighter::{
        visualization::{Annotation, AnnotationType, HighlightSource, HighlightSpan},
        Result, SemanticError,
    },
    LlmService,
};
use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};
use std::ops::Range;
use std::sync::Arc;
use tracing::{debug, warn};

/// Type of pragmatic element
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PragmaticType {
    /// Presupposition (assumed background)
    Presupposition,
    /// Implicature (implied meaning)
    Implicature,
    /// Speech act classification
    SpeechAct,
    /// Indirect speech
    IndirectSpeech,
}

impl PragmaticType {
    fn _color(&self) -> Color {
        match self {
            PragmaticType::Presupposition => Color::LightBlue,
            PragmaticType::Implicature => Color::LightMagenta,
            PragmaticType::SpeechAct => Color::LightGreen,
            PragmaticType::IndirectSpeech => Color::LightYellow,
        }
    }

    fn _description(&self) -> &'static str {
        match self {
            PragmaticType::Presupposition => "Presupposition",
            PragmaticType::Implicature => "Implicature",
            PragmaticType::SpeechAct => "Speech act",
            PragmaticType::IndirectSpeech => "Indirect speech",
        }
    }
}

/// Speech act category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpeechActType {
    /// Making a statement
    Assertion,
    /// Asking a question
    Question,
    /// Issuing a command
    Command,
    /// Making a promise or commitment
    Promise,
    /// Making a request
    Request,
    /// Expressing a wish or desire
    Wish,
}

/// Pragmatic element detected in text
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PragmaticElement {
    /// Range in text
    pub range: Range<usize>,
    pub text: String,

    /// Type of pragmatic element
    pub pragmatic_type: PragmaticType,

    /// For speech acts, the specific type
    pub speech_act: Option<SpeechActType>,

    /// Explanation of the pragmatic meaning
    pub explanation: String,

    /// What is presupposed or implicated
    pub implied_meaning: Option<String>,

    /// Confidence score
    pub confidence: f32,
}

/// Pragmatics analyzer using Claude API or DSPy
#[derive(Clone)]
pub struct PragmaticsAnalyzer {
    _llm_service: Arc<LlmService>,
    threshold: f32,
    #[cfg(feature = "python")]
    dspy_bridge: Option<Arc<DSpySemanticBridge>>,
}

impl PragmaticsAnalyzer {
    pub fn new(llm_service: Arc<LlmService>) -> Self {
        Self {
            _llm_service: llm_service,
            threshold: 0.6,
            #[cfg(feature = "python")]
            dspy_bridge: None,
        }
    }

    /// Create analyzer with DSPy integration
    #[cfg(feature = "python")]
    pub fn with_dspy(llm_service: Arc<LlmService>, dspy_bridge: Arc<DSpySemanticBridge>) -> Self {
        Self {
            _llm_service: llm_service,
            threshold: 0.6,
            dspy_bridge: Some(dspy_bridge),
        }
    }

    /// Set confidence threshold
    pub fn with_threshold(mut self, threshold: f32) -> Self {
        self.threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Analyze pragmatic elements in text
    #[allow(unused_variables)] // text used with python feature
    pub async fn analyze(&self, text: &str) -> Result<Vec<PragmaticElement>> {
        // Use DSPy if available (preferred path)
        #[cfg(feature = "python")]
        if let Some(bridge) = &self.dspy_bridge {
            debug!("Using DSPy for pragmatics analysis");
            let elements = bridge.extract_pragmatics(text).await.map_err(|e| {
                SemanticError::AnalysisFailed(format!("DSPy pragmatics analysis failed: {}", e))
            })?;

            // Filter by threshold
            let filtered: Vec<_> = elements
                .into_iter()
                .filter(|e| e.confidence >= self.threshold)
                .collect();

            return Ok(filtered);
        }

        // Fallback: Direct LLM call (not yet implemented)
        debug!("DSPy not available, using direct LLM call (not implemented yet)");
        Err(SemanticError::AnalysisFailed(
            "Pragmatics analysis requires DSPy integration (enable 'python' feature)".to_string(),
        ))
    }

    /// Parse pragmatics response from LLM
    fn parse_pragmatics_response(
        &self,
        json: &str,
        text_len: usize,
    ) -> Result<Vec<PragmaticElement>> {
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
            .map_err(|e| SemanticError::AnalysisFailed(format!("JSON parse error: {}", e)))?;

        // Convert and validate
        let result = elements
            .into_iter()
            .filter_map(|e| {
                // Validate range
                if e.end > text_len || e.start >= e.end {
                    warn!(
                        "Invalid pragmatic element range: {}..{} (text len: {})",
                        e.start, e.end, text_len
                    );
                    return None;
                }

                // Parse pragmatic type
                let pragmatic_type = match e.pragmatic_type.as_str() {
                    "Presupposition" => PragmaticType::Presupposition,
                    "Implicature" => PragmaticType::Implicature,
                    "SpeechAct" => PragmaticType::SpeechAct,
                    "IndirectSpeech" => PragmaticType::IndirectSpeech,
                    other => {
                        warn!("Unknown pragmatic type: {}", other);
                        return None;
                    }
                };

                // Parse speech act if present
                let speech_act = e.speech_act.and_then(|sa| match sa.as_str() {
                    "Assertion" => Some(SpeechActType::Assertion),
                    "Question" => Some(SpeechActType::Question),
                    "Command" => Some(SpeechActType::Command),
                    "Promise" => Some(SpeechActType::Promise),
                    "Request" => Some(SpeechActType::Request),
                    "Wish" => Some(SpeechActType::Wish),
                    other => {
                        warn!("Unknown speech act type: {}", other);
                        None
                    }
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
            .collect();

        Ok(result)
    }

    /// Convert pragmatic elements to highlight spans
    pub fn elements_to_spans(&self, elements: &[PragmaticElement]) -> Vec<HighlightSpan> {
        elements
            .iter()
            .filter(|e| e.confidence >= self.threshold)
            .map(|element| {
                let style = Style::default()
                    .fg(element.pragmatic_type._color())
                    .add_modifier(Modifier::ITALIC);

                let annotation_text = if let Some(ref implied) = element.implied_meaning {
                    format!(
                        "{}: {} (implies: {})",
                        element.pragmatic_type._description(),
                        element.explanation,
                        implied
                    )
                } else {
                    format!(
                        "{}: {}",
                        element.pragmatic_type._description(),
                        element.explanation
                    )
                };

                HighlightSpan {
                    range: element.range.clone(),
                    style,
                    source: HighlightSource::Analytical,
                    annotation: Some(Annotation {
                        annotation_type: AnnotationType::Information,
                        underline: None,
                        tooltip: Some(annotation_text),
                    }),
                    confidence: element.confidence,
                    metadata: None,
                }
            })
            .collect()
    }

    /// Build prompt for pragmatic analysis
    fn build_analysis_prompt(&self, text: &str) -> String {
        format!(
            r#"Analyze the pragmatic aspects of the following text.

Identify:
1. Presuppositions - What assumptions does the text make?
2. Implicatures - What is implied but not explicitly stated?
3. Speech acts - What actions is the text performing? (asserting, questioning, commanding, promising, requesting, wishing)
4. Indirect speech - Where meaning differs from literal interpretation

Text:
{}

For each pragmatic element, provide:
1. The text segment (with character range)
2. Type (Presupposition, Implicature, SpeechAct, IndirectSpeech)
3. If speech act, specify type (Assertion, Question, Command, Promise, Request, Wish)
4. Explanation
5. Implied meaning (if applicable)
6. Confidence (0.0-1.0)

Respond in JSON format as an array of elements:
[
  {{
    "start": 0,
    "end": 20,
    "text": "segment text",
    "type": "Presupposition",
    "speech_act": null,
    "explanation": "explanation",
    "implied_meaning": "what is implied",
    "confidence": 0.8
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
    fn test_pragmatic_type_colors() {
        assert_eq!(PragmaticType::Presupposition._color(), Color::LightBlue);
        assert_eq!(PragmaticType::Implicature._color(), Color::LightMagenta);
        assert_eq!(PragmaticType::SpeechAct._color(), Color::LightGreen);
    }

    #[test]
    fn test_pragmatic_type_descriptions() {
        assert_eq!(
            PragmaticType::Presupposition._description(),
            "Presupposition"
        );
        assert_eq!(PragmaticType::Implicature._description(), "Implicature");
    }

    #[test]
    fn test_pragmatic_element_structure() {
        let element = PragmaticElement {
            range: 0..25,
            text: "Have you stopped lying?".to_string(),
            pragmatic_type: PragmaticType::Presupposition,
            speech_act: None,
            explanation: "Presupposes that you were lying".to_string(),
            implied_meaning: Some("You were lying before".to_string()),
            confidence: 0.85,
        };

        assert_eq!(element.pragmatic_type, PragmaticType::Presupposition);
        assert!(element.implied_meaning.is_some());
    }

    #[test]
    fn test_speech_act_types() {
        let speech_acts = vec![
            SpeechActType::Assertion,
            SpeechActType::Question,
            SpeechActType::Command,
            SpeechActType::Promise,
            SpeechActType::Request,
            SpeechActType::Wish,
        ];

        assert_eq!(speech_acts.len(), 6);
    }

    #[test]
    fn test_threshold_filtering() {
        let elements = vec![
            PragmaticElement {
                range: 0..10,
                text: "A".to_string(),
                pragmatic_type: PragmaticType::Presupposition,
                speech_act: None,
                explanation: "Test".to_string(),
                implied_meaning: None,
                confidence: 0.9,
            },
            PragmaticElement {
                range: 20..30,
                text: "B".to_string(),
                pragmatic_type: PragmaticType::Implicature,
                speech_act: None,
                explanation: "Test".to_string(),
                implied_meaning: None,
                confidence: 0.4,
            },
        ];

        // With threshold 0.6, only first should pass
        let high_conf: Vec<_> = elements.iter().filter(|e| e.confidence >= 0.6).collect();

        assert_eq!(high_conf.len(), 1);
        assert_eq!(high_conf[0].confidence, 0.9);
    }
}
