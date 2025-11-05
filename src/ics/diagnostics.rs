//! Beautiful Terminal Diagnostics with Ariadne
#![allow(clippy::explicit_counter_loop)]
//!
//! Provides ariadne/miette-style diagnostic rendering for:
//! - Validation errors and warnings
//! - Semantic analysis issues
//! - Typed holes and ambiguities
//! - Syntax and structure problems
//!
//! Features:
//! - Source code context with line numbers
//! - Color-coded severity levels
//! - Inline suggestions and fixes
//! - Multi-line span support

use crate::ics::editor::{Diagnostic, Position, Severity};
use ariadne::{Color, Label, Report, ReportKind, Source};
use std::collections::HashMap;

/// Diagnostic renderer using ariadne
pub struct DiagnosticRenderer {
    /// Cache for file sources
    source_cache: HashMap<String, String>,
}

impl DiagnosticRenderer {
    /// Create new diagnostic renderer
    pub fn new() -> Self {
        Self {
            source_cache: HashMap::new(),
        }
    }

    /// Add a source file to the cache
    pub fn add_source(&mut self, filename: String, content: String) {
        self.source_cache.insert(filename, content);
    }

    /// Render a single diagnostic to string
    ///
    /// # Arguments
    /// * `diagnostic` - The diagnostic to render
    /// * `filename` - The source file name
    /// * `source` - The complete source text
    ///
    /// # Returns
    /// Rendered diagnostic as a string with ANSI colors
    pub fn render_diagnostic(
        &self,
        diagnostic: &Diagnostic,
        filename: &str,
        source: &str,
    ) -> String {
        let mut buf = Vec::new();

        // Convert Severity to ReportKind and Color
        let (kind, color) = match diagnostic.severity {
            Severity::Error => (ReportKind::Error, Color::Red),
            Severity::Warning => (ReportKind::Warning, Color::Yellow),
            Severity::Hint => (ReportKind::Advice, Color::Blue),
        };

        // Calculate byte offset from position
        let offset = self.position_to_offset(source, &diagnostic.position);

        // Create label with span
        let span = offset..(offset + diagnostic.length.max(1));
        let label = Label::new((filename, span.clone()))
            .with_message(&diagnostic.message)
            .with_color(color);

        // Build report
        let mut report = Report::build(kind, filename, offset)
            .with_message(&diagnostic.message)
            .with_label(label);

        // Add suggestion as a note if available
        if let Some(suggestion) = &diagnostic.suggestion {
            report = report.with_note(format!("Suggestion: {}", suggestion));
        }

        // Write report to buffer
        report
            .finish()
            .write((filename, Source::from(source)), &mut buf)
            .expect("Failed to write diagnostic");

        String::from_utf8_lossy(&buf).to_string()
    }

    /// Render multiple diagnostics
    ///
    /// Groups diagnostics by file and renders them together
    pub fn render_diagnostics(
        &self,
        diagnostics: &[(String, Diagnostic)],
        sources: &HashMap<String, String>,
    ) -> String {
        let mut output = String::new();

        // Group diagnostics by file
        let mut by_file: HashMap<String, Vec<&Diagnostic>> = HashMap::new();
        for (filename, diagnostic) in diagnostics {
            by_file
                .entry(filename.clone())
                .or_default()
                .push(diagnostic);
        }

        // Render each file's diagnostics
        for (filename, file_diagnostics) in by_file {
            if let Some(source) = sources.get(&filename) {
                for diagnostic in file_diagnostics {
                    let rendered = self.render_diagnostic(diagnostic, &filename, source);
                    output.push_str(&rendered);
                    output.push('\n');
                }
            }
        }

        output
    }

    /// Render a diagnostic for a buffer (single unnamed source)
    pub fn render_for_buffer(&self, diagnostic: &Diagnostic, source: &str) -> String {
        self.render_diagnostic(diagnostic, "<buffer>", source)
    }

    /// Convert Position to byte offset
    fn position_to_offset(&self, source: &str, position: &Position) -> usize {
        let mut offset = 0;
        let mut current_line = 0;

        for line in source.lines() {
            if current_line == position.line {
                // Found the right line, add column offset
                return offset + position.column.min(line.len());
            }
            offset += line.len() + 1; // +1 for newline
            current_line += 1;
        }

        // If position is beyond end of file, return file length
        source.len()
    }

    /// Clear source cache
    pub fn clear_cache(&mut self) {
        self.source_cache.clear();
    }
}

impl Default for DiagnosticRenderer {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating diagnostics with ariadne-style formatting
pub struct DiagnosticBuilder {
    position: Position,
    length: usize,
    severity: Severity,
    message: String,
    suggestion: Option<String>,
}

impl DiagnosticBuilder {
    /// Start building a new diagnostic
    pub fn new(position: Position, severity: Severity) -> Self {
        Self {
            position,
            length: 1,
            severity,
            message: String::new(),
            suggestion: None,
        }
    }

    /// Set the length of the affected span
    pub fn with_length(mut self, length: usize) -> Self {
        self.length = length;
        self
    }

    /// Set the diagnostic message
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }

    /// Set a suggestion for fixing the issue
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Build the diagnostic
    pub fn build(self) -> Diagnostic {
        Diagnostic {
            position: self.position,
            length: self.length,
            severity: self.severity,
            message: self.message,
            suggestion: self.suggestion,
        }
    }
}

/// Format a diagnostic for display in the diagnostics panel
pub fn format_diagnostic_summary(diagnostic: &Diagnostic) -> String {
    let severity_icon = match diagnostic.severity {
        Severity::Error => "✗",
        Severity::Warning => "⚠",
        Severity::Hint => "ℹ",
    };

    format!(
        "{} [{}:{}] {}",
        severity_icon,
        diagnostic.position.line + 1,
        diagnostic.position.column + 1,
        diagnostic.message
    )
}

/// Collect diagnostics from semantic analysis
pub fn diagnostics_from_semantic(
    semantic: &crate::ics::semantic::SemanticAnalysis,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Create diagnostics from typed holes
    for hole in &semantic.holes {
        let position = Position {
            line: hole.line,
            column: hole.column,
        };

        let severity = match hole.kind {
            crate::ics::semantic::HoleKind::Contradiction => Severity::Error,
            crate::ics::semantic::HoleKind::Undefined => Severity::Error,
            crate::ics::semantic::HoleKind::Ambiguous => Severity::Warning,
            crate::ics::semantic::HoleKind::Incomplete => Severity::Warning,
            crate::ics::semantic::HoleKind::Unknown => Severity::Hint,
        };

        let suggestion = if !hole.suggestions.is_empty() {
            Some(format!("Possible: {}", hole.suggestions.join(", ")))
        } else {
            None
        };

        let diagnostic = DiagnosticBuilder::new(position, severity)
            .with_length(hole.name.len())
            .with_message(format!("{:?}: {}", hole.kind, hole.name))
            .with_suggestion(suggestion.unwrap_or_default())
            .build();

        diagnostics.push(diagnostic);
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_renderer() {
        let renderer = DiagnosticRenderer::new();

        let source = "let x = foo;\nlet y = bar;";
        let diagnostic = Diagnostic {
            position: Position { line: 0, column: 8 },
            length: 3,
            severity: Severity::Error,
            message: "Undefined variable 'foo'".to_string(),
            suggestion: Some("Did you mean 'for'?".to_string()),
        };

        let output = renderer.render_for_buffer(&diagnostic, source);

        // Verify output contains key elements
        assert!(output.contains("Error"));
        assert!(output.contains("Undefined variable"));
        assert!(output.contains("Suggestion"));
    }

    #[test]
    fn test_position_to_offset() {
        let renderer = DiagnosticRenderer::new();

        let source = "line1\nline2\nline3";

        // Start of file
        let offset = renderer.position_to_offset(source, &Position { line: 0, column: 0 });
        assert_eq!(offset, 0);

        // Start of second line
        let offset = renderer.position_to_offset(source, &Position { line: 1, column: 0 });
        assert_eq!(offset, 6); // "line1\n" = 6 bytes

        // Middle of second line
        let offset = renderer.position_to_offset(source, &Position { line: 1, column: 3 });
        assert_eq!(offset, 9); // "line1\nlin" = 9 bytes
    }

    #[test]
    fn test_diagnostic_builder() {
        let diagnostic = DiagnosticBuilder::new(
            Position {
                line: 5,
                column: 10,
            },
            Severity::Warning,
        )
        .with_length(5)
        .with_message("Test warning")
        .with_suggestion("Fix it like this")
        .build();

        assert_eq!(diagnostic.position.line, 5);
        assert_eq!(diagnostic.position.column, 10);
        assert_eq!(diagnostic.length, 5);
        assert_eq!(diagnostic.severity, Severity::Warning);
        assert_eq!(diagnostic.message, "Test warning");
        assert_eq!(diagnostic.suggestion, Some("Fix it like this".to_string()));
    }

    #[test]
    fn test_format_diagnostic_summary() {
        let diagnostic = Diagnostic {
            position: Position {
                line: 10,
                column: 5,
            },
            length: 3,
            severity: Severity::Error,
            message: "Test error".to_string(),
            suggestion: None,
        };

        let summary = format_diagnostic_summary(&diagnostic);
        assert!(summary.contains("✗")); // Error icon
        assert!(summary.contains("[11:6]")); // Line:column (1-indexed)
        assert!(summary.contains("Test error"));
    }

    #[test]
    fn test_diagnostics_from_semantic() {
        use crate::ics::semantic::{HoleKind, SemanticAnalysis, TypedHole};

        let mut analysis = SemanticAnalysis::default();
        analysis.holes.push(TypedHole {
            name: "undefined_var".to_string(),
            kind: HoleKind::Undefined,
            line: 5,
            column: 10,
            context: "let x = undefined_var".to_string(),
            suggestions: vec!["defined_var".to_string()],
        });

        let diagnostics = diagnostics_from_semantic(&analysis);

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].severity, Severity::Error);
        assert!(diagnostics[0].message.contains("Undefined"));
    }
}
