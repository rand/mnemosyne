//! Typed Holes Navigation and Management
//!
//! Provides navigation and AI-assisted resolution for typed holes:
//! - Jump to next/previous hole (]h, [h)
//! - Go to hole by index (gh)
//! - List all holes with context
//! - AI suggestions for hole resolution
//!
//! Holes track ambiguities, contradictions, undefined references,
//! and incomplete specifications in ICS documents.

use crate::ics::editor::Position;
use crate::ics::semantic::{HoleKind, TypedHole};
use std::collections::HashMap;

/// Hole navigation tracker
pub struct HoleNavigator {
    /// All holes in document (sorted by position)
    holes: Vec<TypedHole>,

    /// Current hole index (if navigating)
    current_index: Option<usize>,

    /// Hole resolution history
    resolutions: HashMap<String, HoleResolution>,
}

/// Resolution status for a hole
#[derive(Debug, Clone)]
pub struct HoleResolution {
    /// Hole name
    pub hole_name: String,

    /// Resolution strategy applied
    pub strategy: ResolutionStrategy,

    /// AI-generated suggestions
    pub ai_suggestions: Vec<String>,

    /// User's selected resolution (if any)
    pub selected_resolution: Option<String>,

    /// Whether resolution was accepted
    pub accepted: bool,
}

/// Strategy for resolving a hole
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionStrategy {
    /// Define the missing symbol
    Define,

    /// Clarify ambiguous reference
    Clarify,

    /// Fix contradiction
    FixContradiction,

    /// Complete incomplete specification
    Complete,

    /// Skip (defer resolution)
    Skip,
}

impl HoleNavigator {
    /// Create new hole navigator
    pub fn new() -> Self {
        Self {
            holes: Vec::new(),
            current_index: None,
            resolutions: HashMap::new(),
        }
    }

    /// Update holes from semantic analysis
    pub fn update_holes(&mut self, holes: Vec<TypedHole>) {
        // Sort holes by position (line, then column)
        let mut sorted_holes = holes;
        sorted_holes.sort_by(|a, b| a.line.cmp(&b.line).then_with(|| a.column.cmp(&b.column)));

        self.holes = sorted_holes;

        // Reset current index if out of bounds
        if let Some(idx) = self.current_index {
            if idx >= self.holes.len() {
                self.current_index = None;
            }
        }
    }

    /// Get all holes
    pub fn holes(&self) -> &[TypedHole] {
        &self.holes
    }

    /// Get hole count
    pub fn hole_count(&self) -> usize {
        self.holes.len()
    }

    /// Jump to next hole after given position
    pub fn next_hole(&mut self, current_pos: Position) -> Option<&TypedHole> {
        if self.holes.is_empty() {
            return None;
        }

        // Find first hole after current position
        for (idx, hole) in self.holes.iter().enumerate() {
            if hole.line > current_pos.line
                || (hole.line == current_pos.line && hole.column > current_pos.column)
            {
                self.current_index = Some(idx);
                return Some(hole);
            }
        }

        // Wrap around to first hole
        self.current_index = Some(0);
        self.holes.first()
    }

    /// Jump to previous hole before given position
    pub fn previous_hole(&mut self, current_pos: Position) -> Option<&TypedHole> {
        if self.holes.is_empty() {
            return None;
        }

        // Find last hole before current position
        for (idx, hole) in self.holes.iter().enumerate().rev() {
            if hole.line < current_pos.line
                || (hole.line == current_pos.line && hole.column < current_pos.column)
            {
                self.current_index = Some(idx);
                return Some(hole);
            }
        }

        // Wrap around to last hole
        let last_idx = self.holes.len() - 1;
        self.current_index = Some(last_idx);
        self.holes.last()
    }

    /// Go to hole by index
    pub fn go_to_hole(&mut self, index: usize) -> Option<&TypedHole> {
        if index < self.holes.len() {
            self.current_index = Some(index);
            Some(&self.holes[index])
        } else {
            None
        }
    }

    /// Get current hole (if navigating)
    pub fn current_hole(&self) -> Option<&TypedHole> {
        self.current_index.and_then(|idx| self.holes.get(idx))
    }

    /// Get current hole index
    pub fn current_index(&self) -> Option<usize> {
        self.current_index
    }

    /// Generate AI suggestions for a hole
    pub fn generate_suggestions(&mut self, hole: &TypedHole) -> Vec<String> {
        // Check if we already have suggestions for this hole
        if let Some(resolution) = self.resolutions.get(&hole.name) {
            if !resolution.ai_suggestions.is_empty() {
                return resolution.ai_suggestions.clone();
            }
        }

        // Generate suggestions based on hole kind
        let suggestions = match hole.kind {
            HoleKind::Unknown => {
                vec![
                    format!("Define '{}' with explicit context", hole.name),
                    format!("Reference existing symbol similar to '{}'", hole.name),
                    "Add clarifying comment".to_string(),
                ]
            }
            HoleKind::Ambiguous => {
                vec![
                    format!("Disambiguate '{}' by adding context", hole.name),
                    "Use more specific terminology".to_string(),
                    "Add type annotation or qualifier".to_string(),
                ]
            }
            HoleKind::Undefined => {
                vec![
                    format!("Define '{}' before using it", hole.name),
                    format!("Import or reference '{}' from elsewhere", hole.name),
                    "Check for typos in the name".to_string(),
                ]
            }
            HoleKind::Contradiction => {
                vec![
                    "Resolve conflicting statements".to_string(),
                    "Clarify which statement is correct".to_string(),
                    "Add conditional context to explain contradiction".to_string(),
                ]
            }
            HoleKind::Incomplete => {
                vec![
                    format!("Complete the specification for '{}'", hole.name),
                    "Add missing details or constraints".to_string(),
                    "Provide concrete examples".to_string(),
                ]
            }
        };

        // Store resolution with AI suggestions
        self.resolutions.insert(
            hole.name.clone(),
            HoleResolution {
                hole_name: hole.name.clone(),
                strategy: Self::suggest_strategy(&hole.kind),
                ai_suggestions: suggestions.clone(),
                selected_resolution: None,
                accepted: false,
            },
        );

        suggestions
    }

    /// Suggest resolution strategy for hole kind
    fn suggest_strategy(kind: &HoleKind) -> ResolutionStrategy {
        match kind {
            HoleKind::Unknown => ResolutionStrategy::Define,
            HoleKind::Ambiguous => ResolutionStrategy::Clarify,
            HoleKind::Undefined => ResolutionStrategy::Define,
            HoleKind::Contradiction => ResolutionStrategy::FixContradiction,
            HoleKind::Incomplete => ResolutionStrategy::Complete,
        }
    }

    /// Accept a resolution for a hole
    pub fn accept_resolution(&mut self, hole_name: &str, resolution: String) {
        if let Some(res) = self.resolutions.get_mut(hole_name) {
            res.selected_resolution = Some(resolution);
            res.accepted = true;
        }
    }

    /// Get resolution for a hole
    pub fn get_resolution(&self, hole_name: &str) -> Option<&HoleResolution> {
        self.resolutions.get(hole_name)
    }

    /// Get all unresolved holes
    pub fn unresolved_holes(&self) -> Vec<&TypedHole> {
        self.holes
            .iter()
            .filter(|hole| {
                self.resolutions
                    .get(&hole.name)
                    .map_or(true, |res| !res.accepted)
            })
            .collect()
    }

    /// Get holes by kind
    pub fn holes_by_kind(&self, kind: HoleKind) -> Vec<&TypedHole> {
        self.holes.iter().filter(|h| h.kind == kind).collect()
    }

    /// Clear all holes
    pub fn clear(&mut self) {
        self.holes.clear();
        self.current_index = None;
    }

    /// Clear resolutions
    pub fn clear_resolutions(&mut self) {
        self.resolutions.clear();
    }
}

impl Default for HoleNavigator {
    fn default() -> Self {
        Self::new()
    }
}

/// Format hole for display
pub fn format_hole(hole: &TypedHole, show_context: bool) -> String {
    let icon = hole.kind.icon();
    let kind_str = format!("{:?}", hole.kind);

    let base = format!(
        "{} [{}:{}] {} - {}",
        icon,
        hole.line + 1,
        hole.column + 1,
        kind_str,
        hole.name
    );

    if show_context && !hole.context.is_empty() {
        format!("{}\n  Context: {}", base, hole.context)
    } else {
        base
    }
}

/// Format hole with suggestions
pub fn format_hole_with_suggestions(hole: &TypedHole, suggestions: &[String]) -> String {
    let mut output = format_hole(hole, true);

    if !suggestions.is_empty() {
        output.push_str("\n  Suggestions:");
        for (i, suggestion) in suggestions.iter().enumerate() {
            output.push_str(&format!("\n    {}. {}", i + 1, suggestion));
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_hole(name: &str, line: usize, column: usize, kind: HoleKind) -> TypedHole {
        TypedHole {
            name: name.to_string(),
            kind,
            line,
            column,
            context: format!("test context for {}", name),
            suggestions: vec![],
        }
    }

    #[test]
    fn test_hole_navigator_basics() {
        let mut nav = HoleNavigator::new();

        assert_eq!(nav.hole_count(), 0);
        assert!(nav.current_hole().is_none());

        let holes = vec![
            create_test_hole("foo", 5, 10, HoleKind::Undefined),
            create_test_hole("bar", 10, 5, HoleKind::Unknown),
            create_test_hole("baz", 3, 0, HoleKind::Ambiguous),
        ];

        nav.update_holes(holes);

        // Should be sorted by position
        assert_eq!(nav.hole_count(), 3);
        assert_eq!(nav.holes()[0].name, "baz"); // line 3
        assert_eq!(nav.holes()[1].name, "foo"); // line 5
        assert_eq!(nav.holes()[2].name, "bar"); // line 10
    }

    #[test]
    fn test_hole_navigation() {
        let mut nav = HoleNavigator::new();

        let holes = vec![
            create_test_hole("first", 1, 0, HoleKind::Unknown),
            create_test_hole("second", 5, 10, HoleKind::Undefined),
            create_test_hole("third", 10, 0, HoleKind::Ambiguous),
        ];

        nav.update_holes(holes);

        // Next hole from start
        let next = nav.next_hole(Position { line: 0, column: 0 });
        assert!(next.is_some());
        assert_eq!(next.unwrap().name, "first");

        // Next hole from middle
        let next = nav.next_hole(Position { line: 3, column: 0 });
        assert_eq!(next.unwrap().name, "second");

        // Previous hole from end
        let prev = nav.previous_hole(Position {
            line: 15,
            column: 0,
        });
        assert_eq!(prev.unwrap().name, "third");

        // Go to specific hole
        let hole = nav.go_to_hole(1);
        assert_eq!(hole.unwrap().name, "second");
        assert_eq!(nav.current_index(), Some(1));
    }

    #[test]
    fn test_hole_suggestions() {
        let mut nav = HoleNavigator::new();

        let hole = create_test_hole("undefined_var", 5, 10, HoleKind::Undefined);

        let suggestions = nav.generate_suggestions(&hole);

        assert!(!suggestions.is_empty());
        assert!(suggestions
            .iter()
            .any(|s| s.contains("Define 'undefined_var'")));

        // Should cache suggestions
        let cached = nav.generate_suggestions(&hole);
        assert_eq!(suggestions, cached);
    }

    #[test]
    fn test_hole_resolution() {
        let mut nav = HoleNavigator::new();

        let hole = create_test_hole("test", 1, 0, HoleKind::Unknown);
        nav.generate_suggestions(&hole);

        // Accept resolution
        nav.accept_resolution("test", "Define test explicitly".to_string());

        let resolution = nav.get_resolution("test");
        assert!(resolution.is_some());
        assert!(resolution.unwrap().accepted);
        assert_eq!(
            resolution.unwrap().selected_resolution,
            Some("Define test explicitly".to_string())
        );
    }

    #[test]
    fn test_holes_by_kind() {
        let mut nav = HoleNavigator::new();

        let holes = vec![
            create_test_hole("a", 1, 0, HoleKind::Undefined),
            create_test_hole("b", 2, 0, HoleKind::Unknown),
            create_test_hole("c", 3, 0, HoleKind::Undefined),
        ];

        nav.update_holes(holes);

        let undefined = nav.holes_by_kind(HoleKind::Undefined);
        assert_eq!(undefined.len(), 2);
        assert!(undefined.iter().any(|h| h.name == "a"));
        assert!(undefined.iter().any(|h| h.name == "c"));
    }

    #[test]
    fn test_format_hole() {
        let hole = create_test_hole("test_hole", 10, 5, HoleKind::Ambiguous);

        let formatted = format_hole(&hole, false);
        assert!(formatted.contains("~")); // Ambiguous icon
        assert!(formatted.contains("[11:6]")); // 1-indexed position
        assert!(formatted.contains("test_hole"));

        let with_context = format_hole(&hole, true);
        assert!(with_context.contains("Context:"));
    }
}
