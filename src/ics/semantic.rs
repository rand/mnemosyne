//! Semantic analysis for context documents
//!
//! Provides background analysis to extract:
//! - Subject-predicate-object triples
//! - Relationships and dependencies
//! - Ambiguities and contradictions
//! - Typed holes (unknowns that need resolution)
//!
//! Runs asynchronously without blocking the UI

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::mpsc;

/// Semantic triple (subject-predicate-object)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Triple {
    /// Subject
    pub subject: String,
    /// Predicate (relationship/action)
    pub predicate: String,
    /// Object
    pub object: String,
    /// Source line in document
    pub source_line: usize,
    /// Confidence score (0-100)
    pub confidence: u8,
}

/// Typed hole - something that needs resolution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypedHole {
    /// Name/identifier of the hole
    pub name: String,
    /// Type of hole
    pub kind: HoleKind,
    /// Position in document
    pub line: usize,
    /// Column position
    pub column: usize,
    /// Context around the hole
    pub context: String,
    /// Possible resolutions
    pub suggestions: Vec<String>,
}

/// Kind of typed hole
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum HoleKind {
    /// Unknown symbol/reference
    Unknown,
    /// Ambiguous term (multiple meanings)
    Ambiguous,
    /// Missing definition
    Undefined,
    /// Contradictory statement
    Contradiction,
    /// Incomplete specification
    Incomplete,
}

impl HoleKind {
    /// Get color for hole kind
    pub fn color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            HoleKind::Unknown => Color::Rgb(180, 180, 120), // Soft yellow
            HoleKind::Ambiguous => Color::Rgb(180, 160, 180), // Soft purple
            HoleKind::Undefined => Color::Rgb(180, 140, 140), // Soft red
            HoleKind::Contradiction => Color::Rgb(200, 120, 120), // Stronger red
            HoleKind::Incomplete => Color::Rgb(160, 180, 180), // Soft cyan
        }
    }

    /// Get icon for hole kind
    pub fn icon(&self) -> &'static str {
        match self {
            HoleKind::Unknown => "?",
            HoleKind::Ambiguous => "~",
            HoleKind::Undefined => "!",
            HoleKind::Contradiction => "✗",
            HoleKind::Incomplete => "…",
        }
    }
}

/// Semantic analysis result
#[derive(Debug, Clone, Default)]
pub struct SemanticAnalysis {
    /// Extracted triples
    pub triples: Vec<Triple>,
    /// Typed holes found
    pub holes: Vec<TypedHole>,
    /// Entity mentions (name -> count)
    pub entities: HashMap<String, usize>,
    /// Relationships between entities
    pub relationships: Vec<(String, String, String)>, // (from, relation, to)
}

/// Semantic analyzer
pub struct SemanticAnalyzer {
    /// Channel for sending analysis requests
    tx: mpsc::UnboundedSender<AnalysisRequest>,
    /// Channel for receiving results
    rx: mpsc::UnboundedReceiver<SemanticAnalysis>,
    /// Whether analysis is currently in progress
    analyzing: bool,
}

/// Analysis request
struct AnalysisRequest {
    /// Text to analyze
    text: String,
}

impl SemanticAnalyzer {
    /// Create new semantic analyzer
    pub fn new() -> Self {
        let (request_tx, mut request_rx) = mpsc::unbounded_channel::<AnalysisRequest>();
        let (result_tx, result_rx) = mpsc::unbounded_channel::<SemanticAnalysis>();

        // Spawn background analysis task with error recovery
        tokio::spawn(async move {
            while let Some(request) = request_rx.recv().await {
                // Catch any panics in analysis to prevent task death
                let analysis = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    Self::analyze_text(&request.text)
                }));

                match analysis {
                    Ok(result) => {
                        let _ = result_tx.send(result);
                    }
                    Err(panic_err) => {
                        // Analysis panicked - send empty result as fallback
                        let panic_msg = panic_err
                            .downcast_ref::<&str>()
                            .map(|s| *s)
                            .or_else(|| panic_err.downcast_ref::<String>().map(|s| s.as_str()))
                            .unwrap_or("unknown panic");

                        eprintln!("ERROR: Semantic analysis panicked: {}", panic_msg);
                        let _ = result_tx.send(SemanticAnalysis::default());
                    }
                }
            }
        });

        Self {
            tx: request_tx,
            rx: result_rx,
            analyzing: false,
        }
    }

    /// Request analysis of text (non-blocking)
    ///
    /// # Errors
    ///
    /// Returns error if the background analysis task has died (receiver dropped)
    pub fn analyze(&mut self, text: String) -> Result<()> {
        let request = AnalysisRequest { text };

        self.tx
            .send(request)
            .context("Failed to send analysis request: background task may have died")?;
        self.analyzing = true;

        Ok(())
    }

    /// Try to receive analysis result (non-blocking)
    pub fn try_recv(&mut self) -> Option<SemanticAnalysis> {
        let result = self.rx.try_recv().ok();
        if result.is_some() {
            self.analyzing = false;
        }
        result
    }

    /// Check if analysis is currently in progress
    pub fn is_analyzing(&self) -> bool {
        self.analyzing
    }

    /// Analyze text and extract semantic information
    fn analyze_text(text: &str) -> SemanticAnalysis {
        let line_count = text.lines().count();

        // Pre-allocate with estimated capacity (reduces reallocations)
        let mut triples = Vec::with_capacity(line_count / 5); // ~20% of lines have triples
        let mut holes = Vec::with_capacity(line_count / 10); // ~10% of lines have holes
        let mut entities = HashMap::with_capacity(line_count / 2); // ~50% entities/line
        let mut relationships = Vec::with_capacity(line_count / 5);

        // Extract simple triples from text
        for (line_idx, line) in text.lines().enumerate() {
            // Compute lowercase once per line for efficiency
            let lower = line.to_lowercase();

            // Look for patterns like "X is Y", "X has Y", "X requires Y"
            if let Some(triple) = Self::extract_triple(line, &lower, line_idx) {
                // Build relationship while we have the triple (avoid clone later)
                relationships.push((
                    triple.subject.clone(),
                    triple.predicate.clone(),
                    triple.object.clone(),
                ));
                triples.push(triple);
            }

            // Look for typed holes (pass lowercase to avoid recomputation)
            Self::find_holes_into(line, &lower, line_idx, &mut holes);

            // Extract entity mentions
            Self::extract_entities(line, &mut entities);
        }

        SemanticAnalysis {
            triples,
            holes,
            entities,
            relationships,
        }
    }

    /// Extract a semantic triple from a line
    ///
    /// Takes pre-computed lowercase for efficiency
    fn extract_triple(line: &str, lower: &str, line_idx: usize) -> Option<Triple> {
        // Pattern: "X is Y"
        if let Some(is_pos) = lower.find(" is ") {
            let subject = line[..is_pos].trim().to_string();
            let object = line[is_pos + 4..].trim().to_string();

            if !subject.is_empty() && !object.is_empty() {
                return Some(Triple {
                    subject,
                    predicate: "is".to_string(),
                    object,
                    source_line: line_idx,
                    confidence: 80,
                });
            }
        }

        // Pattern: "X has Y"
        if let Some(has_pos) = lower.find(" has ") {
            let subject = line[..has_pos].trim().to_string();
            let object = line[has_pos + 5..].trim().to_string();

            if !subject.is_empty() && !object.is_empty() {
                return Some(Triple {
                    subject,
                    predicate: "has".to_string(),
                    object,
                    source_line: line_idx,
                    confidence: 75,
                });
            }
        }

        // Pattern: "X requires Y"
        if let Some(req_pos) = lower.find(" requires ") {
            let subject = line[..req_pos].trim().to_string();
            let object = line[req_pos + 10..].trim().to_string();

            if !subject.is_empty() && !object.is_empty() {
                return Some(Triple {
                    subject,
                    predicate: "requires".to_string(),
                    object,
                    source_line: line_idx,
                    confidence: 85,
                });
            }
        }

        None
    }

    /// Find typed holes in a line and push them into the provided vector
    ///
    /// Takes pre-computed lowercase for efficiency
    fn find_holes_into(line: &str, lower: &str, line_idx: usize, holes: &mut Vec<TypedHole>) {
        // Look for TODO/TBD/FIXME markers
        if lower.contains("todo") || lower.contains("tbd") || lower.contains("fixme") {
            holes.push(TypedHole {
                name: "Incomplete".to_string(),
                kind: HoleKind::Incomplete,
                line: line_idx,
                column: 0,
                context: line.to_string(),
                suggestions: vec!["Complete this section".to_string()],
            });
        }

        // Look for contradictions (words like "however", "but actually")
        if lower.contains("however")
            || lower.contains("but actually")
            || lower.contains("contradiction")
        {
            holes.push(TypedHole {
                name: "Potential contradiction".to_string(),
                kind: HoleKind::Contradiction,
                line: line_idx,
                column: 0,
                context: line.to_string(),
                suggestions: vec!["Review for consistency".to_string()],
            });
        }

        // Look for undefined references (e.g., @undefined, #missing)
        for (col, word) in line.split_whitespace().enumerate() {
            if (word.starts_with('@') || word.starts_with('#')) && word.len() > 1 {
                // This could be an undefined reference
                // In a real implementation, we'd check against known symbols
                if word.contains("undefined")
                    || word.contains("missing")
                    || word.contains("unknown")
                {
                    holes.push(TypedHole {
                        name: word.to_string(),
                        kind: HoleKind::Undefined,
                        line: line_idx,
                        column: col,
                        context: line.to_string(),
                        suggestions: vec![format!("Define {}", word)],
                    });
                }
            }
        }
    }

    /// Extract entity mentions from a line
    fn extract_entities(line: &str, entities: &mut HashMap<String, usize>) {
        // Extract capitalized words (potential entities)
        for word in line.split_whitespace() {
            let clean = word.trim_matches(|c: char| !c.is_alphanumeric());
            // Safe: check length before accessing first char
            if clean.len() > 2 {
                if let Some(first_char) = clean.chars().next() {
                    if first_char.is_uppercase() {
                        *entities.entry(clean.to_string()).or_insert(0) += 1;
                    }
                }
            }
        }

        // Extract symbol references (@symbol, #file)
        for word in line.split_whitespace() {
            if word.starts_with('@') || word.starts_with('#') {
                let clean =
                    word.trim_matches(|c: char| !c.is_alphanumeric() && c != '@' && c != '#');
                if clean.len() > 2 {
                    *entities.entry(clean.to_string()).or_insert(0) += 1;
                }
            }
        }
    }
}

impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_triple_extraction() {
        let text =
            "The system is distributed.\nThe agent has memory.\nService requires authentication.";
        let analysis = SemanticAnalyzer::analyze_text(text);

        assert_eq!(analysis.triples.len(), 3);

        // Check "is" triple
        assert!(analysis
            .triples
            .iter()
            .any(|t| t.subject == "The system" && t.predicate == "is"));

        // Check "has" triple
        assert!(analysis
            .triples
            .iter()
            .any(|t| t.subject == "The agent" && t.predicate == "has"));

        // Check "requires" triple
        assert!(analysis
            .triples
            .iter()
            .any(|t| t.subject == "Service" && t.predicate == "requires"));
    }

    #[test]
    fn test_hole_detection() {
        let text = "TODO: implement this feature\nThis is complete.";
        let analysis = SemanticAnalyzer::analyze_text(text);

        assert!(!analysis.holes.is_empty());
        assert!(analysis
            .holes
            .iter()
            .any(|h| h.kind == HoleKind::Incomplete));
    }

    #[test]
    fn test_entity_extraction() {
        let text = "The Orchestrator manages the Agent and calls Process() with #config.";
        let analysis = SemanticAnalyzer::analyze_text(text);

        assert!(analysis.entities.contains_key("Orchestrator"));
        assert!(analysis.entities.contains_key("Agent"));
        assert!(analysis.entities.contains_key("Process"));
        assert!(analysis.entities.contains_key("#config"));
    }

    #[test]
    fn test_hole_kind_colors() {
        for kind in [
            HoleKind::Unknown,
            HoleKind::Ambiguous,
            HoleKind::Undefined,
            HoleKind::Contradiction,
            HoleKind::Incomplete,
        ] {
            let _ = kind.color(); // Just verify it doesn't panic
            assert!(!kind.icon().is_empty());
        }
    }

    #[test]
    fn test_contradiction_detection() {
        let text = "The system is fast. However, the system is slow.";
        let analysis = SemanticAnalyzer::analyze_text(text);

        assert!(analysis
            .holes
            .iter()
            .any(|h| h.kind == HoleKind::Contradiction));
    }

    #[tokio::test]
    async fn test_analyzing_state() {
        let mut analyzer = SemanticAnalyzer::new();

        // Initially not analyzing
        assert!(!analyzer.is_analyzing());

        // After triggering analysis, should be analyzing
        analyzer.analyze("test text".to_string()).unwrap();
        assert!(analyzer.is_analyzing());

        // Note: In real usage, try_recv() would eventually return a result
        // and set analyzing to false, but that's async so can't test here
    }
}
