//! Common regex patterns for semantic highlighting
//!
//! This module provides pre-compiled regex patterns used across
//! multiple analyzers for efficient pattern matching.

use once_cell::sync::Lazy;
use regex::Regex;

/// Common patterns used across analyzers
pub struct CommonPatterns;

impl CommonPatterns {
    /// XML-style tags: <tag>, </tag>, <tag attr="value">
    pub fn xml_tag() -> &'static Regex {
        static PATTERN: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r#"<(/?)([a-zA-Z][a-zA-Z0-9_-]*)((?:\s+[a-zA-Z][a-zA-Z0-9_-]*\s*=\s*(?:"[^"]*"|'[^']*'))*)\s*(/?)>"#)
                .expect("Valid XML tag regex")
        });
        &PATTERN
    }

    /// Self-closing XML tags: <tag/>
    pub fn xml_self_closing() -> &'static Regex {
        static PATTERN: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"<([a-zA-Z][a-zA-Z0-9_-]*)\s*/>")
                .expect("Valid self-closing XML regex")
        });
        &PATTERN
    }

    /// RFC 2119 constraint keywords (MUST, SHALL, MAY, etc.)
    pub fn rfc2119_keywords() -> &'static Regex {
        static PATTERN: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"\b(MUST|SHALL|REQUIRED|SHOULD|RECOMMENDED|MAY|OPTIONAL|MUST NOT|SHALL NOT|SHOULD NOT|NOT RECOMMENDED)\b")
                .expect("Valid RFC 2119 regex")
        });
        &PATTERN
    }

    /// Markdown code blocks: ```language or ```
    pub fn code_fence() -> &'static Regex {
        static PATTERN: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"^```([a-zA-Z0-9_+-]*)")
                .expect("Valid code fence regex")
        });
        &PATTERN
    }

    /// Inline code: `code`
    pub fn inline_code() -> &'static Regex {
        static PATTERN: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"`([^`]+)`")
                .expect("Valid inline code regex")
        });
        &PATTERN
    }

    /// File paths: #/path/to/file or #file.txt
    pub fn file_path() -> &'static Regex {
        static PATTERN: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"#([/a-zA-Z0-9._-]+(?:/[a-zA-Z0-9._-]+)*)")
                .expect("Valid file path regex")
        });
        &PATTERN
    }

    /// Symbol references: @symbol or @Type::method
    pub fn symbol_reference() -> &'static Regex {
        static PATTERN: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"@([a-zA-Z_][a-zA-Z0-9_]*(?:::[a-zA-Z_][a-zA-Z0-9_]*)*)")
                .expect("Valid symbol reference regex")
        });
        &PATTERN
    }

    /// Typed holes: ?hole or ?typed_hole
    pub fn typed_hole() -> &'static Regex {
        static PATTERN: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"\?([a-zA-Z_][a-zA-Z0-9_]*)")
                .expect("Valid typed hole regex")
        });
        &PATTERN
    }

    /// URLs: http:// or https://
    pub fn url() -> &'static Regex {
        static PATTERN: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"https?://[^\s<>]+")
                .expect("Valid URL regex")
        });
        &PATTERN
    }

    /// Ambiguous words and phrases
    pub fn ambiguous_phrases() -> &'static Regex {
        static PATTERN: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"\b(some|various|several|many|few|might|could|perhaps|maybe|possibly|probably|unclear|ambiguous|vague|approximately|roughly|about|around)\b")
                .expect("Valid ambiguous phrases regex")
        });
        &PATTERN
    }

    /// Vague quantifiers
    pub fn vague_quantifiers() -> &'static Regex {
        static PATTERN: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"\b(a lot of|lots of|a bunch of|a few|some|several|many|most|all|none|any)\b")
                .expect("Valid vague quantifiers regex")
        });
        &PATTERN
    }

    /// Time expressions
    pub fn temporal_expressions() -> &'static Regex {
        static PATTERN: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"\b(yesterday|today|tomorrow|now|later|soon|recently|currently|previously|before|after|when|while|during|since|until)\b")
                .expect("Valid temporal expressions regex")
        });
        &PATTERN
    }

    /// Causal indicators
    pub fn causal_indicators() -> &'static Regex {
        static PATTERN: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"\b(because|since|as|therefore|thus|hence|consequently|so|then|if|when|unless)\b")
                .expect("Valid causal indicators regex")
        });
        &PATTERN
    }

    /// Contrast indicators
    pub fn contrast_indicators() -> &'static Regex {
        static PATTERN: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"\b(but|however|although|though|yet|nevertheless|nonetheless|despite|instead|rather|whereas|while)\b")
                .expect("Valid contrast indicators regex")
        });
        &PATTERN
    }

    /// Elaboration indicators
    pub fn elaboration_indicators() -> &'static Regex {
        static PATTERN: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"\b(specifically|namely|particularly|especially|for example|for instance|such as|including|like|e\.g\.|i\.e\.)\b")
                .expect("Valid elaboration indicators regex")
        });
        &PATTERN
    }

    /// Pronouns (for anaphora detection)
    pub fn pronouns() -> &'static Regex {
        static PATTERN: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"\b(he|she|it|they|them|his|her|its|their|him|this|that|these|those|who|which|what)\b")
                .expect("Valid pronouns regex")
        });
        &PATTERN
    }

    /// Sentence boundaries
    pub fn sentence_boundary() -> &'static Regex {
        static PATTERN: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"[.!?]\s+")
                .expect("Valid sentence boundary regex")
        });
        &PATTERN
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xml_tag_pattern() {
        let text = "<thinking>some content</thinking>";
        let matches: Vec<_> = CommonPatterns::xml_tag()
            .find_iter(text)
            .map(|m| m.as_str())
            .collect();
        assert_eq!(matches, vec!["<thinking>", "</thinking>"]);
    }

    #[test]
    fn test_rfc2119_pattern() {
        let text = "The system MUST validate input and SHOULD log errors";
        let matches: Vec<_> = CommonPatterns::rfc2119_keywords()
            .find_iter(text)
            .map(|m| m.as_str())
            .collect();
        assert_eq!(matches, vec!["MUST", "SHOULD"]);
    }

    #[test]
    fn test_file_path_pattern() {
        let text = "See #src/main.rs and #config.toml for details";
        let matches: Vec<_> = CommonPatterns::file_path()
            .find_iter(text)
            .map(|m| m.as_str())
            .collect();
        assert_eq!(matches, vec!["#src/main.rs", "#config.toml"]);
    }

    #[test]
    fn test_typed_hole_pattern() {
        let text = "Implement ?auth_handler and ?database_connection";
        let matches: Vec<_> = CommonPatterns::typed_hole()
            .find_iter(text)
            .map(|m| m.as_str())
            .collect();
        assert_eq!(matches, vec!["?auth_handler", "?database_connection"]);
    }

    #[test]
    fn test_ambiguous_phrases_pattern() {
        let text = "There might be several issues, perhaps around 10";
        let matches: Vec<_> = CommonPatterns::ambiguous_phrases()
            .find_iter(text)
            .map(|m| m.as_str())
            .collect();
        assert!(matches.contains(&"might"));
        assert!(matches.contains(&"several"));
        assert!(matches.contains(&"perhaps"));
        assert!(matches.contains(&"around"));
    }
}
