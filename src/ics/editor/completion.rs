//! Smart completions for context documents
//!
//! Provides intelligent, non-intrusive completions:
//! - Symbol completion (functions, variables, types)
//! - Keyword completion
//! - File path completion
//! - Fuzzy matching using nucleo

use super::Position;
use anyhow::Result;
use nucleo::Matcher;

/// Completion item
#[derive(Debug, Clone)]
pub struct CompletionItem {
    /// Label to display
    pub label: String,
    /// Text to insert
    pub insert_text: String,
    /// Kind of completion
    pub kind: CompletionKind,
    /// Optional detail/description
    pub detail: Option<String>,
    /// Relevance score (higher = more relevant)
    pub score: i32,
}

/// Completion kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    /// Keyword
    Keyword,
    /// Variable
    Variable,
    /// Function
    Function,
    /// Type
    Type,
    /// File path
    File,
    /// Memory reference
    Memory,
    /// Symbol
    Symbol,
}

impl CompletionKind {
    /// Get display character for kind
    pub fn icon(&self) -> &'static str {
        match self {
            CompletionKind::Keyword => "K",
            CompletionKind::Variable => "V",
            CompletionKind::Function => "F",
            CompletionKind::Type => "T",
            CompletionKind::File => "f",
            CompletionKind::Memory => "M",
            CompletionKind::Symbol => "S",
        }
    }

    /// Get color for kind
    pub fn color(&self) -> ratatui::style::Color {
        use ratatui::style::Color;
        match self {
            CompletionKind::Keyword => Color::Rgb(180, 160, 200),
            CompletionKind::Variable => Color::Rgb(180, 200, 180),
            CompletionKind::Function => Color::Rgb(200, 180, 160),
            CompletionKind::Type => Color::Rgb(160, 180, 200),
            CompletionKind::File => Color::Rgb(200, 200, 160),
            CompletionKind::Memory => Color::Rgb(200, 160, 180),
            CompletionKind::Symbol => Color::Rgb(180, 180, 180),
        }
    }
}

/// Completion engine
pub struct CompletionEngine {
    /// Fuzzy matcher
    matcher: Matcher,
    /// Available symbols
    symbols: Vec<String>,
    /// Keywords for context documents
    keywords: Vec<String>,
}

impl Default for CompletionEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl CompletionEngine {
    /// Create new completion engine
    pub fn new() -> Self {
        let keywords = vec![
            // Context engineering keywords
            "context".to_string(),
            "memory".to_string(),
            "agent".to_string(),
            "orchestrator".to_string(),
            "optimizer".to_string(),
            "reviewer".to_string(),
            "executor".to_string(),
            "task".to_string(),
            "goal".to_string(),
            "constraint".to_string(),
            "requirement".to_string(),
            "spec".to_string(),
            "implementation".to_string(),
            "test".to_string(),
            "verify".to_string(),
            "validate".to_string(),
            // Markdown/formatting
            "heading".to_string(),
            "list".to_string(),
            "table".to_string(),
            "code".to_string(),
            "link".to_string(),
            "image".to_string(),
            "quote".to_string(),
        ];

        Self {
            matcher: Matcher::new(nucleo::Config::DEFAULT),
            symbols: Vec::new(),
            keywords,
        }
    }

    /// Update symbols from text
    pub fn update_symbols(&mut self, text: &str) {
        self.symbols.clear();

        // Extract simple symbols (words starting with capital letter or containing #, @)
        for word in text.split_whitespace() {
            if word.len() > 2 {
                // Function-like: word(
                if word.contains('(') {
                    let name = word.split('(').next().unwrap();
                    if !self.symbols.contains(&name.to_string()) {
                        self.symbols.push(name.to_string());
                    }
                }
                // File reference: #path
                else if word.starts_with('#') {
                    if !self.symbols.contains(&word.to_string()) {
                        self.symbols.push(word.to_string());
                    }
                }
                // Symbol reference: @symbol
                else if word.starts_with('@') {
                    if !self.symbols.contains(&word.to_string()) {
                        self.symbols.push(word.to_string());
                    }
                }
                // Type-like: Capitalized
                else if word.chars().next().unwrap().is_uppercase() {
                    let clean = word.trim_end_matches(|c: char| !c.is_alphanumeric());
                    if !clean.is_empty() && !self.symbols.contains(&clean.to_string()) {
                        self.symbols.push(clean.to_string());
                    }
                }
            }
        }
    }

    /// Get completions for prefix at position
    pub fn complete(&mut self, prefix: &str, _position: Position) -> Vec<CompletionItem> {
        if prefix.is_empty() {
            return Vec::new();
        }

        let mut completions = Vec::new();

        // Clone collections to avoid borrow checker issues
        let keywords = self.keywords.clone();
        let symbols = self.symbols.clone();

        // Match against keywords
        for keyword in &keywords {
            if let Some(score) = self.fuzzy_match(prefix, keyword) {
                completions.push(CompletionItem {
                    label: keyword.clone(),
                    insert_text: keyword.clone(),
                    kind: CompletionKind::Keyword,
                    detail: Some("Keyword".to_string()),
                    score,
                });
            }
        }

        // Match against symbols
        for symbol in &symbols {
            if let Some(score) = self.fuzzy_match(prefix, symbol) {
                let kind = if symbol.starts_with('#') {
                    CompletionKind::File
                } else if symbol.starts_with('@') {
                    CompletionKind::Memory
                } else if symbol.contains('(') {
                    CompletionKind::Function
                } else if symbol.chars().next().unwrap().is_uppercase() {
                    CompletionKind::Type
                } else {
                    CompletionKind::Symbol
                };

                completions.push(CompletionItem {
                    label: symbol.clone(),
                    insert_text: symbol.clone(),
                    kind,
                    detail: Some(format!("{:?}", kind)),
                    score,
                });
            }
        }

        // Sort by score (descending)
        completions.sort_by(|a, b| b.score.cmp(&a.score));

        // Limit to top 10
        completions.truncate(10);

        completions
    }

    /// Fuzzy match two strings
    fn fuzzy_match(&mut self, pattern: &str, text: &str) -> Option<i32> {
        use nucleo::Utf32Str;

        // Convert to UTF-32 strings for nucleo
        let pattern_chars: Vec<char> = pattern.chars().collect();
        let text_chars: Vec<char> = text.chars().collect();

        let pattern_utf32 = Utf32Str::Ascii(pattern.as_bytes());
        let text_utf32 = Utf32Str::Ascii(text.as_bytes());

        self.matcher
            .fuzzy_match(text_utf32, pattern_utf32)
            .map(|score| score as i32)
    }

    /// Get word at position
    pub fn word_at_position(text: &str, position: Position) -> Option<(String, usize, usize)> {
        let lines: Vec<&str> = text.lines().collect();
        if position.line >= lines.len() {
            return None;
        }

        let line = lines[position.line];
        if position.column > line.len() {
            return None;
        }

        // Find word boundaries
        let before = &line[..position.column];
        let after = &line[position.column..];

        let start = before
            .rfind(|c: char| !c.is_alphanumeric() && c != '_' && c != '#' && c != '@')
            .map(|i| i + 1)
            .unwrap_or(0);

        let end = after
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| position.column + i)
            .unwrap_or(line.len());

        let word = line[start..end].to_string();
        Some((word, start, end))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_completion_engine() {
        let mut engine = CompletionEngine::new();

        // Test keyword completion
        let completions = engine.complete("con", Position { line: 0, column: 3 });
        assert!(!completions.is_empty());
        assert!(completions.iter().any(|c| c.label.contains("context")));
    }

    #[test]
    fn test_symbol_extraction() {
        let mut engine = CompletionEngine::new();

        let text = "The Orchestrator agent calls process() function with #file/path and @memory";
        engine.update_symbols(text);

        // Should extract symbols
        assert!(engine.symbols.contains(&"Orchestrator".to_string()));
        assert!(engine.symbols.contains(&"process".to_string()));
        assert!(engine.symbols.contains(&"#file/path".to_string()));
        assert!(engine.symbols.contains(&"@memory".to_string()));
    }

    #[test]
    fn test_word_at_position() {
        let text = "hello world test";

        // Get word at "wo|rld"
        let result = CompletionEngine::word_at_position(text, Position { line: 0, column: 8 });
        assert_eq!(result, Some(("world".to_string(), 6, 11)));

        // Get word at "|hello"
        let result = CompletionEngine::word_at_position(text, Position { line: 0, column: 0 });
        assert_eq!(result, Some(("hello".to_string(), 0, 5)));
    }

    #[test]
    fn test_completion_kinds() {
        // Test icon and color methods exist for all kinds
        for kind in [
            CompletionKind::Keyword,
            CompletionKind::Variable,
            CompletionKind::Function,
            CompletionKind::Type,
            CompletionKind::File,
            CompletionKind::Memory,
        ] {
            assert!(!kind.icon().is_empty());
            let _ = kind.color(); // Just verify it doesn't panic
        }
    }
}
