//! Inline validation for context documents
//!
//! Provides gentle, non-intrusive validation checks:
//! - Bracket matching
//! - Quote matching
//! - Basic structure validation
//! - Soft warnings (not blocking errors)

use super::Position;

/// Validation diagnostic
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Position of the issue
    pub position: Position,
    /// Length of the affected text
    pub length: usize,
    /// Severity level
    pub severity: Severity,
    /// Diagnostic message
    pub message: String,
    /// Optional fix suggestion
    pub suggestion: Option<String>,
}

/// Diagnostic severity
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Informational hint
    Hint,
    /// Warning (not blocking)
    Warning,
    /// Error (more serious but still not blocking in ICS)
    Error,
}

impl Severity {
    /// Get color for severity
    pub fn color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            Severity::Hint => Color::Rgb(140, 140, 160), // Soft blue-gray
            Severity::Warning => Color::Rgb(200, 180, 120), // Soft yellow
            Severity::Error => Color::Rgb(200, 140, 140), // Soft red
        }
    }
}

/// Text validator
pub struct Validator {
    /// Whether to enable validation
    enabled: bool,
}

impl Default for Validator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator {
    /// Create new validator
    pub fn new() -> Self {
        Self { enabled: true }
    }

    /// Enable/disable validation
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Validate text and return diagnostics
    pub fn validate(&self, text: &str) -> Vec<Diagnostic> {
        if !self.enabled {
            return Vec::new();
        }

        let mut diagnostics = Vec::new();

        // Check bracket matching
        self.check_brackets(text, &mut diagnostics);

        // Check quote matching
        self.check_quotes(text, &mut diagnostics);

        // Check line length (soft suggestion)
        self.check_line_length(text, &mut diagnostics);

        diagnostics
    }

    /// Check for unmatched brackets
    fn check_brackets(&self, text: &str, diagnostics: &mut Vec<Diagnostic>) {
        let pairs = [('(', ')'), ('[', ']'), ('{', '}')];

        for (open, close) in pairs {
            let mut stack = Vec::new();
            let mut line = 0;
            let mut column = 0;

            for ch in text.chars() {
                if ch == '\n' {
                    line += 1;
                    column = 0;
                    continue;
                }

                if ch == open {
                    stack.push((line, column));
                } else if ch == close {
                    if stack.is_empty() {
                        diagnostics.push(Diagnostic {
                            position: Position { line, column },
                            length: 1,
                            severity: Severity::Warning,
                            message: format!("Unmatched closing '{}'", close),
                            suggestion: Some(format!(
                                "Remove '{}' or add matching '{}'",
                                close, open
                            )),
                        });
                    } else {
                        stack.pop();
                    }
                }

                column += 1;
            }

            // Check for unmatched opening brackets
            for (line, column) in stack {
                diagnostics.push(Diagnostic {
                    position: Position { line, column },
                    length: 1,
                    severity: Severity::Warning,
                    message: format!("Unmatched opening '{}'", open),
                    suggestion: Some(format!("Add matching '{}' or remove '{}'", close, open)),
                });
            }
        }
    }

    /// Check for unmatched quotes
    fn check_quotes(&self, text: &str, diagnostics: &mut Vec<Diagnostic>) {
        let quotes = ['"', '\'', '`'];

        for quote in quotes {
            let mut in_quote = false;
            let mut quote_start = (0, 0);
            let mut line = 0;
            let mut column = 0;

            for ch in text.chars() {
                if ch == '\n' {
                    line += 1;
                    column = 0;
                    continue;
                }

                if ch == quote {
                    if in_quote {
                        // Closing quote
                        in_quote = false;
                    } else {
                        // Opening quote
                        in_quote = true;
                        quote_start = (line, column);
                    }
                }

                column += 1;
            }

            // Check for unclosed quote
            if in_quote {
                diagnostics.push(Diagnostic {
                    position: Position {
                        line: quote_start.0,
                        column: quote_start.1,
                    },
                    length: 1,
                    severity: Severity::Hint,
                    message: format!("Unclosed quote '{}'", quote),
                    suggestion: Some(format!("Add closing '{}'", quote)),
                });
            }
        }
    }

    /// Check for overly long lines (soft suggestion)
    fn check_line_length(&self, text: &str, diagnostics: &mut Vec<Diagnostic>) {
        const SOFT_LIMIT: usize = 100;
        const HARD_LIMIT: usize = 120;

        for (line_idx, line) in text.lines().enumerate() {
            let len = line.len();

            if len > HARD_LIMIT {
                diagnostics.push(Diagnostic {
                    position: Position {
                        line: line_idx,
                        column: HARD_LIMIT,
                    },
                    length: len - HARD_LIMIT,
                    severity: Severity::Hint,
                    message: format!("Line is {} characters (consider breaking at 120)", len),
                    suggestion: Some("Break into multiple lines for readability".to_string()),
                });
            } else if len > SOFT_LIMIT {
                diagnostics.push(Diagnostic {
                    position: Position {
                        line: line_idx,
                        column: SOFT_LIMIT,
                    },
                    length: len - SOFT_LIMIT,
                    severity: Severity::Hint,
                    message: format!("Line is {} characters", len),
                    suggestion: None,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bracket_matching() {
        let validator = Validator::new();

        // Valid brackets
        let text = "This is (valid [text] with {brackets})";
        let diagnostics = validator.validate(text);
        let bracket_issues: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.contains("bracket") || d.message.contains("Unmatched"))
            .collect();
        assert_eq!(bracket_issues.len(), 0);

        // Unmatched opening
        let text = "This has (unmatched";
        let diagnostics = validator.validate(text);
        assert!(diagnostics
            .iter()
            .any(|d| d.message.contains("Unmatched opening")));

        // Unmatched closing
        let text = "This has unmatched)";
        let diagnostics = validator.validate(text);
        assert!(diagnostics
            .iter()
            .any(|d| d.message.contains("Unmatched closing")));
    }

    #[test]
    fn test_quote_matching() {
        let validator = Validator::new();

        // Valid quotes
        let text = r#"This is "valid" text with 'quotes'"#;
        let diagnostics = validator.validate(text);
        let quote_issues: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.contains("quote"))
            .collect();
        assert_eq!(quote_issues.len(), 0);

        // Unclosed quote
        let text = r#"This has "unclosed"#;
        let diagnostics = validator.validate(text);
        assert!(diagnostics
            .iter()
            .any(|d| d.message.contains("Unclosed quote")));
    }

    #[test]
    fn test_line_length() {
        let validator = Validator::new();

        // Short line
        let text = "Short line";
        let diagnostics = validator.validate(text);
        let length_issues: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.message.contains("characters"))
            .collect();
        assert_eq!(length_issues.len(), 0);

        // Long line
        let text = "a".repeat(150);
        let diagnostics = validator.validate(&text);
        assert!(diagnostics.iter().any(|d| d.message.contains("characters")));
    }

    #[test]
    fn test_disabled_validator() {
        let mut validator = Validator::new();
        validator.set_enabled(false);

        let text = "This has (unmatched";
        let diagnostics = validator.validate(text);
        assert_eq!(diagnostics.len(), 0);
    }
}
