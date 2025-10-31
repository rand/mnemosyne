//! Visual connections between text spans

use std::ops::Range;

/// Type of connection between spans
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionType {
    /// Coreference (entity mention chain)
    Coreference,

    /// Anaphora (pronoun to antecedent)
    Anaphora,

    /// Discourse relation
    Discourse,

    /// Contradiction
    Contradiction,
}

/// A visual connection between two text spans
#[derive(Debug, Clone)]
pub struct Connection {
    /// Source span
    pub from: Range<usize>,

    /// Target span
    pub to: Range<usize>,

    /// Type of connection
    pub connection_type: ConnectionType,

    /// Label for the connection
    pub label: Option<String>,

    /// Confidence score
    pub confidence: f32,
}

impl Connection {
    pub fn new(from: Range<usize>, to: Range<usize>, connection_type: ConnectionType) -> Self {
        Self {
            from,
            to,
            connection_type,
            label: None,
            confidence: 1.0,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence;
        self
    }
}
