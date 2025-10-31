//! Inline annotations (icons, underlines, etc.)

use serde::{Deserialize, Serialize};

/// Type of annotation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnnotationType {
    /// Information/definition (ⓘ)
    Information,

    /// Warning/ambiguity (⚠)
    Warning,

    /// Contradiction (⚡)
    Contradiction,

    /// Coreference link (🔗)
    CorefLink,

    /// Discourse relation (➜)
    DiscourseRelation,

    /// Presupposition (?)
    Presupposition,

    /// Custom icon
    Custom(String),
}

impl AnnotationType {
    /// Get the icon for this annotation type
    pub fn icon(&self) -> &str {
        match self {
            Self::Information => "ⓘ",
            Self::Warning => "⚠",
            Self::Contradiction => "⚡",
            Self::CorefLink => "🔗",
            Self::DiscourseRelation => "➜",
            Self::Presupposition => "?",
            Self::Custom(s) => s,
        }
    }
}

/// Type of underline style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnderlineStyle {
    /// Solid line (━━━) - High confidence
    Solid,

    /// Dashed line (╍╍╍) - Medium confidence
    Dashed,

    /// Dotted line (┄┄┄) - Low confidence
    Dotted,

    /// Wavy line (∿∿∿) - Problem/warning
    Wavy,

    /// Double line (═══) - Emphasis
    Double,
}

/// An annotation on a text span
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    /// Type of annotation
    pub annotation_type: AnnotationType,

    /// Underline style
    pub underline: Option<UnderlineStyle>,

    /// Tooltip text
    pub tooltip: Option<String>,
}

impl Annotation {
    pub fn new(annotation_type: AnnotationType) -> Self {
        Self {
            annotation_type,
            underline: None,
            tooltip: None,
        }
    }

    pub fn with_underline(mut self, style: UnderlineStyle) -> Self {
        self.underline = Some(style);
        self
    }

    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
}
