//! Syntax highlighting support

use std::path::Path;

/// Supported languages
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    Markdown,
    Rust,
    Python,
    TypeScript,
    JavaScript,
    JSON,
    TOML,
    YAML,
    Bash,
    Go,
    C,
    Cpp,
    PlainText,
}

impl Language {
    /// Detect language from file path
    pub fn from_path(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?;
        match ext {
            "md" | "markdown" => Some(Language::Markdown),
            "rs" => Some(Language::Rust),
            "py" => Some(Language::Python),
            "ts" => Some(Language::TypeScript),
            "js" | "jsx" => Some(Language::JavaScript),
            "json" => Some(Language::JSON),
            "toml" => Some(Language::TOML),
            "yaml" | "yml" => Some(Language::YAML),
            "sh" | "bash" | "zsh" => Some(Language::Bash),
            "go" => Some(Language::Go),
            "c" | "h" => Some(Language::C),
            "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Some(Language::Cpp),
            _ => None,
        }
    }

    /// Get file extension for language
    pub fn extension(&self) -> &'static str {
        match self {
            Language::Markdown => "md",
            Language::Rust => "rs",
            Language::Python => "py",
            Language::TypeScript => "ts",
            Language::JavaScript => "js",
            Language::JSON => "json",
            Language::TOML => "toml",
            Language::YAML => "yaml",
            Language::Bash => "sh",
            Language::Go => "go",
            Language::C => "c",
            Language::Cpp => "cpp",
            Language::PlainText => "txt",
        }
    }
}

/// Syntax highlight kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighlightKind {
    // Standard syntax highlights
    Keyword,
    String,
    Number,
    Comment,
    Function,
    Type,
    Variable,
    Operator,
    Punctuation,

    // ICS-specific highlights
    /// File reference: #file/path
    IcsFileRef,
    /// Symbol reference: @symbol
    IcsSymbolRef,
    /// Unresolved typed hole: ?hole_name
    IcsHole,
    /// Ambiguous reference (multiple matches)
    IcsAmbiguous,

    // Semantic highlights (from DSPy analysis)
    /// Subject in semantic triple
    IcsSubject,
    /// Object in semantic triple
    IcsObject,
    /// Predicate in semantic triple
    IcsPredicate,
}

impl HighlightKind {
    /// Get color for highlight kind (stub for now)
    pub fn color(&self) -> (u8, u8, u8) {
        match self {
            HighlightKind::Keyword => (197, 134, 192),
            HighlightKind::String => (152, 195, 121),
            HighlightKind::Number => (209, 154, 102),
            HighlightKind::Comment => (92, 99, 112),
            HighlightKind::Function => (97, 175, 239),
            HighlightKind::Type => (229, 192, 123),
            HighlightKind::Variable => (224, 108, 117),
            HighlightKind::Operator => (86, 182, 194),
            HighlightKind::Punctuation => (171, 178, 191),

            HighlightKind::IcsFileRef => (97, 175, 239),
            HighlightKind::IcsSymbolRef => (198, 120, 221),
            HighlightKind::IcsHole => (224, 108, 117),
            HighlightKind::IcsAmbiguous => (229, 192, 123),

            HighlightKind::IcsSubject => (152, 195, 121),
            HighlightKind::IcsObject => (86, 182, 194),
            HighlightKind::IcsPredicate => (209, 154, 102),
        }
    }
}
