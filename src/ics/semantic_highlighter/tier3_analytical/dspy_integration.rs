// ! DSPy integration for Tier 3 semantic analysis
//!
//! This module provides a DSPy-powered bridge for semantic highlighting operations.
//! It replaces direct LLM API calls with systematic prompt optimization via DSPy.
//!
//! # Architecture
//!
//! ```text
//! Tier 3 Analyzers → DSpySemanticBridge → Python DSPy → SemanticModule → LLM
//! ```
//!
//! # Benefits
//!
//! - **Systematic Optimization**: Prompts optimized via teleprompters
//! - **Structured Output**: Type-safe JSON schemas via DSPy signatures
//! - **Shared Infrastructure**: Reuses DSpyService from agent orchestration
//! - **Batch Processing**: Efficient handling of multiple analysis requests
//!
//! # Usage
//!
//! ```rust,no_run
//! use mnemosyne_core::ics::semantic_highlighter::tier3_analytical::dspy_integration::DSpySemanticBridge;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let bridge = DSpySemanticBridge::new()?;
//!
//! // Discourse analysis
//! let segments = bridge.analyze_discourse("Text to analyze").await?;
//!
//! // Contradiction detection
//! let contradictions = bridge.detect_contradictions("Contradictory text").await?;
//!
//! // Pragmatics extraction
//! let elements = bridge.extract_pragmatics("Text with implications").await?;
//! # Ok(())
//! # }
//! ```

use super::{
    Contradiction, ContradictionType, DiscourseRelation, DiscourseSegment, PragmaticElement,
    PragmaticType, SpeechActType,
};
use crate::error::{MnemosyneError, Result};
use pyo3::prelude::*;
use pyo3::types::PyList;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

/// DSPy bridge for Tier 3 semantic analysis
///
/// Provides type-safe interface to Python DSPy SemanticModule.
/// Thread-safe and async-friendly.
#[derive(Clone)]
pub struct DSpySemanticBridge {
    /// Python DSPy service instance (holds GIL when accessed)
    service: Arc<Mutex<Py<PyAny>>>,
}

impl DSpySemanticBridge {
    /// Create a new DSPy semantic bridge
    ///
    /// Initializes Python interpreter and imports DSPy semantic module.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Python interpreter initialization fails
    /// - DSPy service module import fails
    /// - SemanticModule instantiation fails
    pub fn new() -> Result<Self> {
        Python::with_gil(|py| {
            // Import the DSPy service module
            let dspy_service_mod = py
                .import_bound("mnemosyne.orchestration.dspy_service")
                .map_err(|e| {
                    error!("Failed to import DSPy service module: {}", e);
                    MnemosyneError::Other(format!("DSPy service import failed: {}", e))
                })?;

            // Get DSpyService class
            let service_class = dspy_service_mod.getattr("DSpyService").map_err(|e| {
                error!("Failed to get DSpyService class: {}", e);
                MnemosyneError::Other(format!("DSpyService class not found: {}", e))
            })?;

            // Instantiate service
            let service = service_class.call0().map_err(|e| {
                error!("Failed to instantiate DSpyService: {}", e);
                MnemosyneError::Other(format!("DSpyService instantiation failed: {}", e))
            })?;

            info!("DSPy semantic bridge initialized successfully");

            Ok(Self {
                service: Arc::new(Mutex::new(service.unbind().into())),
            })
        })
    }

    /// Analyze discourse structure of text
    ///
    /// Identifies discourse segments and relations between them.
    ///
    /// # Arguments
    ///
    /// * `text` - Text to analyze (max ~2000 tokens for efficiency)
    ///
    /// # Returns
    ///
    /// Vector of discourse segments with ranges and relations
    pub async fn analyze_discourse(&self, text: &str) -> Result<Vec<DiscourseSegment>> {
        debug!("Analyzing discourse structure for {} chars", text.len());

        let service = self.service.clone();
        let text = text.to_string();

        let result = tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                let service_guard = service.blocking_lock();
                let service_ref = service_guard.bind(py);

                // Get semantic module
                let semantic_module =
                    service_ref
                        .call_method0("get_semantic_module")
                        .map_err(|e| {
                            error!("Failed to get semantic module: {}", e);
                            MnemosyneError::Other(format!("Semantic module not found: {}", e))
                        })?;

                // Call analyze_discourse
                let prediction = semantic_module
                    .call_method1("analyze_discourse", (&text,))
                    .map_err(|e| {
                        error!("DSPy discourse analysis failed: {}", e);
                        MnemosyneError::Other(format!("Discourse analysis failed: {}", e))
                    })?;

                // Extract segments from prediction
                Self::parse_discourse_segments(&prediction)
            })
        })
        .await
        .map_err(|e| {
            error!("Tokio spawn_blocking failed: {}", e);
            MnemosyneError::Other(format!("Async execution failed: {}", e))
        })??;

        debug!("Extracted {} discourse segments", result.len());
        Ok(result)
    }

    /// Detect contradictions in text
    ///
    /// Identifies pairs of contradictory statements.
    ///
    /// # Arguments
    ///
    /// * `text` - Text to analyze
    ///
    /// # Returns
    ///
    /// Vector of contradictions with statement ranges and types
    pub async fn detect_contradictions(&self, text: &str) -> Result<Vec<Contradiction>> {
        debug!("Detecting contradictions in {} chars", text.len());

        let service = self.service.clone();
        let text = text.to_string();

        let result = tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                let service_guard = service.blocking_lock();
                let service_ref = service_guard.bind(py);

                // Get semantic module
                let semantic_module =
                    service_ref
                        .call_method0("get_semantic_module")
                        .map_err(|e| {
                            error!("Failed to get semantic module: {}", e);
                            MnemosyneError::Other(format!("Semantic module not found: {}", e))
                        })?;

                // Call detect_contradictions
                let prediction = semantic_module
                    .call_method1("detect_contradictions", (&text,))
                    .map_err(|e| {
                        error!("DSPy contradiction detection failed: {}", e);
                        MnemosyneError::Other(format!("Contradiction detection failed: {}", e))
                    })?;

                // Extract contradictions from prediction
                Self::parse_contradictions(&prediction)
            })
        })
        .await
        .map_err(|e| {
            error!("Tokio spawn_blocking failed: {}", e);
            MnemosyneError::Other(format!("Async execution failed: {}", e))
        })??;

        debug!("Detected {} contradictions", result.len());
        Ok(result)
    }

    /// Extract pragmatic elements from text
    ///
    /// Identifies implied meanings, presuppositions, and speech acts.
    ///
    /// # Arguments
    ///
    /// * `text` - Text to analyze
    ///
    /// # Returns
    ///
    /// Vector of pragmatic elements with ranges and types
    pub async fn extract_pragmatics(&self, text: &str) -> Result<Vec<PragmaticElement>> {
        debug!("Extracting pragmatics from {} chars", text.len());

        let service = self.service.clone();
        let text = text.to_string();

        let result = tokio::task::spawn_blocking(move || {
            Python::with_gil(|py| {
                let service_guard = service.blocking_lock();
                let service_ref = service_guard.bind(py);

                // Get semantic module
                let semantic_module =
                    service_ref
                        .call_method0("get_semantic_module")
                        .map_err(|e| {
                            error!("Failed to get semantic module: {}", e);
                            MnemosyneError::Other(format!("Semantic module not found: {}", e))
                        })?;

                // Call extract_pragmatics
                let prediction = semantic_module
                    .call_method1("extract_pragmatics", (&text,))
                    .map_err(|e| {
                        error!("DSPy pragmatics extraction failed: {}", e);
                        MnemosyneError::Other(format!("Pragmatics extraction failed: {}", e))
                    })?;

                // Extract elements from prediction
                Self::parse_pragmatic_elements(&prediction)
            })
        })
        .await
        .map_err(|e| {
            error!("Tokio spawn_blocking failed: {}", e);
            MnemosyneError::Other(format!("Async execution failed: {}", e))
        })??;

        debug!("Extracted {} pragmatic elements", result.len());
        Ok(result)
    }

    /// Parse discourse segments from DSPy prediction
    fn parse_discourse_segments(prediction: &Bound<PyAny>) -> Result<Vec<DiscourseSegment>> {
        let segments_py = prediction.getattr("segments").map_err(|e| {
            error!("Failed to get segments attribute: {}", e);
            MnemosyneError::Other(format!("Missing segments in prediction: {}", e))
        })?;

        // Convert Python list to Rust Vec - iterate manually
        let segments_list: &Bound<PyList> = segments_py.downcast().map_err(|e| {
            error!("Segments is not a list: {}", e);
            MnemosyneError::Other(format!("Segments must be a list: {}", e))
        })?;

        let mut result = Vec::new();
        for item in segments_list.iter() {
            // Convert Python dict to JSON string then parse
            let json_str = item.str().map_err(|e| {
                error!("Failed to convert segment to string: {}", e);
                MnemosyneError::Other(format!("Segment conversion failed: {}", e))
            })?;

            let json_str_rust: String = json_str.extract().map_err(|e| {
                error!("Failed to extract string: {}", e);
                MnemosyneError::Other(format!("String extraction failed: {}", e))
            })?;

            match serde_json::from_str::<Value>(&json_str_rust) {
                Ok(seg_json) => {
                    if let Some(segment) = Self::json_to_discourse_segment(&seg_json) {
                        result.push(segment);
                    } else {
                        warn!("Skipping invalid discourse segment");
                    }
                }
                Err(e) => {
                    warn!("Failed to parse segment JSON: {}", e);
                }
            }
        }

        Ok(result)
    }

    /// Parse contradictions from DSPy prediction
    fn parse_contradictions(prediction: &Bound<PyAny>) -> Result<Vec<Contradiction>> {
        let contradictions_py = prediction.getattr("contradictions").map_err(|e| {
            error!("Failed to get contradictions attribute: {}", e);
            MnemosyneError::Other(format!("Missing contradictions in prediction: {}", e))
        })?;

        let contradictions_list: &Bound<PyList> = contradictions_py.downcast().map_err(|e| {
            error!("Contradictions is not a list: {}", e);
            MnemosyneError::Other(format!("Contradictions must be a list: {}", e))
        })?;

        let mut result = Vec::new();
        for item in contradictions_list.iter() {
            let json_str = item.str().map_err(|e| {
                error!("Failed to convert contradiction to string: {}", e);
                MnemosyneError::Other(format!("Contradiction conversion failed: {}", e))
            })?;

            let json_str_rust: String = json_str.extract().map_err(|e| {
                error!("Failed to extract string: {}", e);
                MnemosyneError::Other(format!("String extraction failed: {}", e))
            })?;

            match serde_json::from_str::<Value>(&json_str_rust) {
                Ok(contra_json) => {
                    if let Some(contradiction) = Self::json_to_contradiction(&contra_json) {
                        result.push(contradiction);
                    } else {
                        warn!("Skipping invalid contradiction");
                    }
                }
                Err(e) => {
                    warn!("Failed to parse contradiction JSON: {}", e);
                }
            }
        }

        Ok(result)
    }

    /// Parse pragmatic elements from DSPy prediction
    fn parse_pragmatic_elements(prediction: &Bound<PyAny>) -> Result<Vec<PragmaticElement>> {
        let elements_py = prediction.getattr("elements").map_err(|e| {
            error!("Failed to get elements attribute: {}", e);
            MnemosyneError::Other(format!("Missing elements in prediction: {}", e))
        })?;

        let elements_list: &Bound<PyList> = elements_py.downcast().map_err(|e| {
            error!("Elements is not a list: {}", e);
            MnemosyneError::Other(format!("Elements must be a list: {}", e))
        })?;

        let mut result = Vec::new();
        for item in elements_list.iter() {
            let json_str = item.str().map_err(|e| {
                error!("Failed to convert element to string: {}", e);
                MnemosyneError::Other(format!("Element conversion failed: {}", e))
            })?;

            let json_str_rust: String = json_str.extract().map_err(|e| {
                error!("Failed to extract string: {}", e);
                MnemosyneError::Other(format!("String extraction failed: {}", e))
            })?;

            match serde_json::from_str::<Value>(&json_str_rust) {
                Ok(elem_json) => {
                    if let Some(element) = Self::json_to_pragmatic_element(&elem_json) {
                        result.push(element);
                    } else {
                        warn!("Skipping invalid pragmatic element");
                    }
                }
                Err(e) => {
                    warn!("Failed to parse element JSON: {}", e);
                }
            }
        }

        Ok(result)
    }

    /// Convert JSON value to DiscourseSegment
    fn json_to_discourse_segment(json: &Value) -> Option<DiscourseSegment> {
        let start = json.get("start")?.as_u64()? as usize;
        let end = json.get("end")?.as_u64()? as usize;
        let text = json.get("text")?.as_str()?.to_string();
        let confidence = json.get("confidence")?.as_f64()? as f32;

        let relation = json.get("relation").and_then(|r| {
            r.as_str().and_then(|s| match s {
                "Elaboration" => Some(DiscourseRelation::Elaboration),
                "Contrast" => Some(DiscourseRelation::Contrast),
                "Cause" => Some(DiscourseRelation::Cause),
                "Sequence" => Some(DiscourseRelation::Sequence),
                "Condition" => Some(DiscourseRelation::Condition),
                "Background" => Some(DiscourseRelation::Background),
                "Summary" => Some(DiscourseRelation::Summary),
                "Evaluation" => Some(DiscourseRelation::Evaluation),
                _ => None,
            })
        });

        let related_to = if let (Some(related_start), Some(related_end)) = (
            json.get("related_to_start").and_then(|v| v.as_u64()),
            json.get("related_to_end").and_then(|v| v.as_u64()),
        ) {
            Some(related_start as usize..related_end as usize)
        } else {
            None
        };

        Some(DiscourseSegment {
            range: start..end,
            text,
            relation,
            related_to,
            confidence,
        })
    }

    /// Convert JSON value to Contradiction
    fn json_to_contradiction(json: &Value) -> Option<Contradiction> {
        let statement1_start = json.get("statement1_start")?.as_u64()? as usize;
        let statement1_end = json.get("statement1_end")?.as_u64()? as usize;
        let text1 = json.get("text1")?.as_str()?.to_string();
        let statement2_start = json.get("statement2_start")?.as_u64()? as usize;
        let statement2_end = json.get("statement2_end")?.as_u64()? as usize;
        let text2 = json.get("text2")?.as_str()?.to_string();
        let explanation = json.get("explanation")?.as_str()?.to_string();
        let confidence = json.get("confidence")?.as_f64()? as f32;

        let contradiction_type = match json.get("type")?.as_str()? {
            "Direct" => ContradictionType::Direct,
            "Temporal" => ContradictionType::Temporal,
            "Semantic" => ContradictionType::Semantic,
            "Implication" => ContradictionType::Implication,
            _ => return None,
        };

        Some(Contradiction {
            statement1: statement1_start..statement1_end,
            text1,
            statement2: statement2_start..statement2_end,
            text2,
            contradiction_type,
            explanation,
            confidence,
        })
    }

    /// Convert JSON value to PragmaticElement
    fn json_to_pragmatic_element(json: &Value) -> Option<PragmaticElement> {
        let start = json.get("start")?.as_u64()? as usize;
        let end = json.get("end")?.as_u64()? as usize;
        let text = json.get("text")?.as_str()?.to_string();
        let explanation = json.get("explanation")?.as_str()?.to_string();
        let confidence = json.get("confidence")?.as_f64()? as f32;

        let pragmatic_type = match json.get("type")?.as_str()? {
            "Presupposition" => PragmaticType::Presupposition,
            "Implicature" => PragmaticType::Implicature,
            "SpeechAct" => PragmaticType::SpeechAct,
            "IndirectSpeech" => PragmaticType::IndirectSpeech,
            _ => return None,
        };

        let speech_act = json.get("speech_act").and_then(|sa| {
            sa.as_str().and_then(|s| match s {
                "Assertion" => Some(SpeechActType::Assertion),
                "Question" => Some(SpeechActType::Question),
                "Command" => Some(SpeechActType::Command),
                "Promise" => Some(SpeechActType::Promise),
                "Request" => Some(SpeechActType::Request),
                "Wish" => Some(SpeechActType::Wish),
                _ => None,
            })
        });

        let implied_meaning = json
            .get("implied_meaning")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Some(PragmaticElement {
            range: start..end,
            text,
            pragmatic_type,
            speech_act,
            explanation,
            implied_meaning,
            confidence,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_to_discourse_segment() {
        let json = serde_json::json!({
            "start": 0,
            "end": 10,
            "text": "Test text",
            "relation": "Elaboration",
            "related_to_start": 10,
            "related_to_end": 20,
            "confidence": 0.9
        });

        let segment = DSpySemanticBridge::json_to_discourse_segment(&json).unwrap();
        assert_eq!(segment.range, 0..10);
        assert_eq!(segment.text, "Test text");
        assert!(matches!(
            segment.relation,
            Some(DiscourseRelation::Elaboration)
        ));
        assert_eq!(segment.confidence, 0.9);
    }

    #[test]
    fn test_json_to_contradiction() {
        let json = serde_json::json!({
            "statement1_start": 0,
            "statement1_end": 10,
            "text1": "Auth required",
            "statement2_start": 20,
            "statement2_end": 35,
            "text2": "No auth needed",
            "type": "Direct",
            "explanation": "Contradictory statements",
            "confidence": 0.95
        });

        let contradiction = DSpySemanticBridge::json_to_contradiction(&json).unwrap();
        assert_eq!(contradiction.statement1, 0..10);
        assert_eq!(contradiction.text1, "Auth required");
        assert!(matches!(
            contradiction.contradiction_type,
            ContradictionType::Direct
        ));
        assert_eq!(contradiction.confidence, 0.95);
    }
}
