//! Color schemes for semantic highlighting

use ratatui::style::Color;

/// Color scheme for semantic highlighting
#[derive(Debug, Clone)]
pub struct ColorScheme {
    /// Entity type colors
    pub person: Color,
    pub organization: Color,
    pub location: Color,
    pub concept: Color,
    pub temporal: Color,

    /// Relationship colors
    pub subject: Color,
    pub predicate: Color,
    pub object: Color,

    /// Quality indicator colors
    pub certain: Color,
    pub probable: Color,
    pub uncertain: Color,
    pub ambiguous: Color,
    pub contradictory: Color,

    /// Discourse relation colors
    pub elaboration: Color,
    pub contrast: Color,
    pub cause: Color,
    pub temporal_relation: Color,

    /// Constraint colors
    pub must: Color,
    pub should: Color,
    pub may: Color,
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self {
            // Entity colors
            person: Color::Rgb(255, 215, 0),       // Warm yellow #FFD700
            organization: Color::Rgb(65, 105, 225), // Corporate blue #4169E1
            location: Color::Rgb(46, 139, 87),     // Earth green #2E8B57
            concept: Color::Rgb(147, 112, 219),    // Abstract purple #9370DB
            temporal: Color::Rgb(255, 140, 0),     // Clock orange #FF8C00

            // Relationship colors
            subject: Color::Blue,
            predicate: Color::Green,
            object: Color::Rgb(255, 140, 0), // Orange

            // Quality colors
            certain: Color::Green,
            probable: Color::Blue,
            uncertain: Color::Yellow,
            ambiguous: Color::Rgb(255, 165, 0), // Orange
            contradictory: Color::Red,

            // Discourse colors
            elaboration: Color::Blue,
            contrast: Color::Rgb(255, 140, 0), // Orange
            cause: Color::Green,
            temporal_relation: Color::Rgb(147, 112, 219), // Purple

            // Constraint colors
            must: Color::Red,
            should: Color::Yellow,
            may: Color::Blue,
        }
    }
}

impl ColorScheme {
    /// Get color for entity type
    pub fn entity_color(&self, entity_type: &str) -> Color {
        match entity_type.to_lowercase().as_str() {
            "person" | "per" => self.person,
            "organization" | "org" => self.organization,
            "location" | "loc" | "gpe" => self.location,
            "concept" => self.concept,
            "temporal" | "date" | "time" => self.temporal,
            _ => Color::White,
        }
    }

    /// Get color for certainty level
    pub fn certainty_color(&self, level: &str) -> Color {
        match level.to_lowercase().as_str() {
            "certain" | "high" => self.certain,
            "probable" | "medium" => self.probable,
            "uncertain" | "low" => self.uncertain,
            "ambiguous" => self.ambiguous,
            _ => Color::White,
        }
    }
}
