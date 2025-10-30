//! Symbol Resolution and Registry
//!
//! Tracks all symbols (@symbol), files (#path), holes, and entities
//! in ICS documents for:
//! - Symbol resolution (@symbol → definition)
//! - File resolution (#path → filesystem)
//! - Type-ahead completion
//! - Jump-to-definition
//! - Reference finding

use crate::ics::semantic::{SemanticAnalysis, TypedHole};
use crate::ics::editor::Position;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Symbol registry for a document
pub struct SymbolRegistry {
    /// @symbols - variables, functions, concepts
    symbols: HashMap<String, SymbolInfo>,

    /// #files - file paths, assets
    files: HashMap<PathBuf, FileInfo>,

    /// Typed holes from semantic analysis
    holes: HashMap<String, TypedHole>,

    /// Entities extracted from text (capitalized words, etc.)
    entities: HashMap<String, EntityInfo>,

    /// Reverse index: position → symbol
    position_index: Vec<(Position, String, SymbolKind)>,
}

/// Information about a symbol
#[derive(Debug, Clone)]
pub struct SymbolInfo {
    /// Symbol name (without @)
    pub name: String,

    /// Symbol kind
    pub kind: SymbolKind,

    /// Where symbol was first defined
    pub definition_location: Position,

    /// All references to this symbol
    pub references: Vec<Position>,

    /// Documentation comment if available
    pub doc_comment: Option<String>,

    /// Number of times symbol is referenced
    pub ref_count: usize,
}

/// Kind of symbol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    /// Variable or concept
    Variable,

    /// Function or operation
    Function,

    /// Type or class
    Type,

    /// General concept
    Concept,

    /// File reference
    File,

    /// Entity (from semantic analysis)
    Entity,
}

/// Information about a file reference
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// File path (relative to project root)
    pub path: PathBuf,

    /// Where file was first referenced
    pub definition_location: Position,

    /// All references to this file
    pub references: Vec<Position>,

    /// File resolution status
    pub resolution: FileResolution,
}

/// File resolution result
#[derive(Debug, Clone)]
pub enum FileResolution {
    /// File exists at this path
    Exists {
        absolute_path: PathBuf,
        size: u64,
        modified: std::time::SystemTime,
    },

    /// File not found, but here are suggestions
    NotFound {
        suggestions: Vec<PathBuf>,
    },

    /// Multiple files match (ambiguous)
    Ambiguous {
        candidates: Vec<PathBuf>,
    },
}

/// Information about an entity
#[derive(Debug, Clone)]
pub struct EntityInfo {
    /// Entity text
    pub text: String,

    /// First occurrence
    pub first_occurrence: Position,

    /// All occurrences
    pub occurrences: Vec<Position>,

    /// Frequency count
    pub frequency: usize,
}

impl SymbolRegistry {
    /// Create new empty registry
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            files: HashMap::new(),
            holes: HashMap::new(),
            entities: HashMap::new(),
            position_index: Vec::new(),
        }
    }

    /// Register a symbol from @mention
    pub fn register_symbol(&mut self, name: &str, pos: Position, kind: SymbolKind, doc: Option<String>) {
        let name = name.trim_start_matches('@').to_string();

        if let Some(info) = self.symbols.get_mut(&name) {
            // Symbol exists, add reference
            info.references.push(pos);
            info.ref_count += 1;
        } else {
            // New symbol
            self.symbols.insert(name.clone(), SymbolInfo {
                name: name.clone(),
                kind,
                definition_location: pos,
                references: vec![pos],
                doc_comment: doc,
                ref_count: 1,
            });
        }

        // Update position index
        self.position_index.push((pos, name, kind));
    }

    /// Register a file from #path mention
    pub fn register_file(&mut self, path: PathBuf, pos: Position, project_root: Option<&Path>) {
        let resolution = Self::resolve_file_path(&path, project_root);

        if let Some(info) = self.files.get_mut(&path) {
            // File exists, add reference
            info.references.push(pos);
        } else {
            // New file
            self.files.insert(path.clone(), FileInfo {
                path: path.clone(),
                definition_location: pos,
                references: vec![pos],
                resolution,
            });
        }

        // Update position index
        self.position_index.push((pos, path.to_string_lossy().to_string(), SymbolKind::File));
    }

    /// Register entity from semantic analysis
    pub fn register_entity(&mut self, text: &str, pos: Position) {
        if let Some(info) = self.entities.get_mut(text) {
            // Entity exists, add occurrence
            info.occurrences.push(pos);
            info.frequency += 1;
        } else {
            // New entity
            self.entities.insert(text.to_string(), EntityInfo {
                text: text.to_string(),
                first_occurrence: pos,
                occurrences: vec![pos],
                frequency: 1,
            });
        }
    }

    /// Resolve @symbol to definition
    pub fn resolve_symbol(&self, name: &str) -> Option<&SymbolInfo> {
        let name = name.trim_start_matches('@');
        self.symbols.get(name)
    }

    /// Check if #file exists on filesystem
    fn resolve_file_path(path: &Path, project_root: Option<&Path>) -> FileResolution {
        // Try absolute path first
        if path.is_absolute() && path.exists() {
            if let Ok(metadata) = std::fs::metadata(path) {
                return FileResolution::Exists {
                    absolute_path: path.to_path_buf(),
                    size: metadata.len(),
                    modified: metadata.modified().unwrap_or(std::time::SystemTime::now()),
                };
            }
        }

        // Try relative to project root
        if let Some(root) = project_root {
            let full_path = root.join(path);
            if full_path.exists() {
                if let Ok(metadata) = std::fs::metadata(&full_path) {
                    return FileResolution::Exists {
                        absolute_path: full_path,
                        size: metadata.len(),
                        modified: metadata.modified().unwrap_or(std::time::SystemTime::now()),
                    };
                }
            }
        }

        // File not found, provide suggestions
        FileResolution::NotFound {
            suggestions: Vec::new(),  // TODO: Fuzzy file search
        }
    }

    /// Get all symbols starting with prefix
    pub fn complete_symbol(&self, prefix: &str) -> Vec<CompletionCandidate> {
        let prefix_lower = prefix.trim_start_matches('@').to_lowercase();

        self.symbols
            .values()
            .filter(|sym| sym.name.to_lowercase().starts_with(&prefix_lower))
            .map(|sym| CompletionCandidate {
                text: format!("@{}", sym.name),
                kind: sym.kind,
                detail: sym.doc_comment.clone(),
                score: sym.ref_count as f32,  // More refs = higher score
            })
            .collect()
    }

    /// Get all files starting with prefix
    pub fn complete_file(&self, prefix: &str) -> Vec<CompletionCandidate> {
        let prefix_lower = prefix.trim_start_matches('#').to_lowercase();

        self.files
            .values()
            .filter(|file| {
                file.path.to_string_lossy().to_lowercase().contains(&prefix_lower)
            })
            .map(|file| CompletionCandidate {
                text: format!("#{}", file.path.display()),
                kind: SymbolKind::File,
                detail: match &file.resolution {
                    FileResolution::Exists { size, .. } => {
                        Some(format!("{} bytes", size))
                    }
                    FileResolution::NotFound { .. } => Some("not found".to_string()),
                    FileResolution::Ambiguous { candidates } => {
                        Some(format!("{} matches", candidates.len()))
                    }
                },
                score: file.references.len() as f32,
            })
            .collect()
    }

    /// Get symbol at position
    pub fn symbol_at_position(&self, pos: Position) -> Option<(&String, &SymbolKind)> {
        // Find closest symbol before or at position
        self.position_index
            .iter()
            .filter(|(p, _, _)| p.line == pos.line && p.column <= pos.column)
            .max_by_key(|(p, _, _)| p.column)
            .map(|(_, name, kind)| (name, kind))
    }

    /// Update registry from semantic analysis
    pub fn sync_from_analysis(&mut self, analysis: &SemanticAnalysis, _project_root: Option<&Path>) {
        // Update holes
        self.holes.clear();
        for hole in &analysis.holes {
            self.holes.insert(hole.name.clone(), hole.clone());
        }

        // Note: SemanticAnalysis.entities only contains counts (HashMap<String, usize>),
        // not position information. Entity positions are tracked separately when
        // they are registered via register_entity() during document parsing.
        // The semantic analyzer would need to be enhanced to track positions
        // if we want to sync entity positions from analysis.
    }

    /// Get all symbols in document
    pub fn all_symbols(&self) -> Vec<&SymbolInfo> {
        self.symbols.values().collect()
    }

    /// Get all holes
    pub fn all_holes(&self) -> Vec<&TypedHole> {
        self.holes.values().collect()
    }

    /// Clear registry
    pub fn clear(&mut self) {
        self.symbols.clear();
        self.files.clear();
        self.holes.clear();
        self.entities.clear();
        self.position_index.clear();
    }
}

/// Completion candidate
#[derive(Debug, Clone)]
pub struct CompletionCandidate {
    /// Text to insert (includes @ or # prefix)
    pub text: String,

    /// Symbol kind
    pub kind: SymbolKind,

    /// Detail/documentation
    pub detail: Option<String>,

    /// Relevance score (higher = more relevant)
    pub score: f32,
}

impl Default for SymbolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Shared symbol registry (thread-safe)
pub type SharedSymbolRegistry = Arc<RwLock<SymbolRegistry>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_symbol() {
        let mut registry = SymbolRegistry::new();

        registry.register_symbol("@foo", Position { line: 0, column: 0 }, SymbolKind::Variable, None);
        registry.register_symbol("@foo", Position { line: 5, column: 10 }, SymbolKind::Variable, None);

        let sym = registry.resolve_symbol("@foo").unwrap();
        assert_eq!(sym.name, "foo");
        assert_eq!(sym.ref_count, 2);
        assert_eq!(sym.references.len(), 2);
    }

    #[test]
    fn test_symbol_completion() {
        let mut registry = SymbolRegistry::new();

        registry.register_symbol("@foo", Position::default(), SymbolKind::Variable, None);
        registry.register_symbol("@foobar", Position::default(), SymbolKind::Function, None);
        registry.register_symbol("@baz", Position::default(), SymbolKind::Type, None);

        let completions = registry.complete_symbol("@fo");
        assert_eq!(completions.len(), 2);  // foo and foobar
        assert!(completions.iter().any(|c| c.text == "@foo"));
        assert!(completions.iter().any(|c| c.text == "@foobar"));
    }

    #[test]
    fn test_file_registration() {
        let mut registry = SymbolRegistry::new();

        registry.register_file(PathBuf::from("docs/spec.md"), Position::default(), None);

        let completions = registry.complete_file("#docs");
        assert_eq!(completions.len(), 1);
        assert_eq!(completions[0].text, "#docs/spec.md");
    }
}
