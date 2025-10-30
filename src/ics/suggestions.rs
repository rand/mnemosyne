//! Type-Ahead Completion Engine
//!
//! Provides real-time completion suggestions for:
//! - @symbols (variables, functions, concepts)
//! - #files (project files)
//! - Memory content from storage backend
//!
//! Features:
//! - Context detection (@ vs #)
//! - Fuzzy matching with scoring
//! - Filesystem caching for performance
//! - Storage backend integration

use crate::ics::symbols::{CompletionCandidate, SharedSymbolRegistry, SymbolKind};
use crate::storage::{MemorySortOrder, StorageBackend};
use crate::types::Namespace;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

/// Completion context type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionContext {
    /// Completing a @symbol
    Symbol,

    /// Completing a #file path
    File,

    /// No completion context
    None,
}

/// Completion engine
pub struct CompletionEngine {
    /// Symbol registry
    symbol_registry: SharedSymbolRegistry,

    /// Storage backend for memory-based suggestions
    storage: Arc<dyn StorageBackend>,

    /// Current namespace for memory queries
    namespace: Namespace,

    /// Filesystem cache
    file_cache: Arc<RwLock<FileSystemCache>>,

    /// Project root for file resolution
    project_root: Option<PathBuf>,
}

/// Filesystem cache for file completions
struct FileSystemCache {
    /// Cached file paths
    files: Vec<PathBuf>,

    /// Last scan time
    last_scan: Option<Instant>,

    /// Cache validity duration
    validity: Duration,
}

impl FileSystemCache {
    fn new() -> Self {
        Self {
            files: Vec::new(),
            last_scan: None,
            validity: Duration::from_secs(60), // Re-scan every 60 seconds
        }
    }

    fn is_valid(&self) -> bool {
        if let Some(last_scan) = self.last_scan {
            last_scan.elapsed() < self.validity
        } else {
            false
        }
    }

    fn update(&mut self, files: Vec<PathBuf>) {
        self.files = files;
        self.last_scan = Some(Instant::now());
    }
}

impl CompletionEngine {
    /// Create new completion engine
    pub fn new(
        symbol_registry: SharedSymbolRegistry,
        storage: Arc<dyn StorageBackend>,
        namespace: Namespace,
        project_root: Option<PathBuf>,
    ) -> Self {
        Self {
            symbol_registry,
            storage,
            namespace,
            file_cache: Arc::new(RwLock::new(FileSystemCache::new())),
            project_root,
        }
    }

    /// Detect completion context at cursor position
    ///
    /// Returns:
    /// - (CompletionContext::Symbol, prefix) if cursor is after @
    /// - (CompletionContext::File, prefix) if cursor is after #
    /// - (CompletionContext::None, "") otherwise
    pub fn detect_context(&self, line: &str, column: usize) -> (CompletionContext, String) {
        if column == 0 || column > line.len() {
            return (CompletionContext::None, String::new());
        }

        // Look backwards from cursor to find @ or #
        let prefix_chars: Vec<char> = line.chars().take(column).collect();

        // Find the last @ or # before cursor
        for i in (0..prefix_chars.len()).rev() {
            match prefix_chars[i] {
                '@' => {
                    // Found @, extract prefix after it
                    let prefix: String = prefix_chars[(i + 1)..].iter().collect();
                    return (CompletionContext::Symbol, prefix);
                }
                '#' => {
                    // Found #, extract prefix after it
                    let prefix: String = prefix_chars[(i + 1)..].iter().collect();
                    return (CompletionContext::File, prefix);
                }
                // Stop at whitespace or special chars (unless part of path)
                c if c.is_whitespace() => break,
                _ => continue,
            }
        }

        (CompletionContext::None, String::new())
    }

    /// Get completions based on context
    pub async fn get_completions(&self, line: &str, column: usize) -> Vec<CompletionCandidate> {
        let (context, prefix) = self.detect_context(line, column);

        match context {
            CompletionContext::Symbol => self.complete_symbol(&prefix).await,
            CompletionContext::File => self.complete_file(&prefix).await,
            CompletionContext::None => Vec::new(),
        }
    }

    /// Get symbol completions
    async fn complete_symbol(&self, prefix: &str) -> Vec<CompletionCandidate> {
        // Get completions from symbol registry
        let registry_completions = {
            let registry = self.symbol_registry.read().unwrap();
            registry.complete_symbol(&format!("@{}", prefix))
        };

        // Get completions from memory storage
        let memory_completions = self.complete_from_memories(prefix).await;

        // Merge and deduplicate
        let mut all_completions = registry_completions;
        all_completions.extend(memory_completions);

        // Sort by score (highest first)
        all_completions.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Deduplicate by text
        let mut seen = HashMap::new();
        all_completions.retain(|c| seen.insert(c.text.clone(), ()).is_none());

        all_completions
    }

    /// Get file completions
    async fn complete_file(&self, prefix: &str) -> Vec<CompletionCandidate> {
        // Get completions from symbol registry (for previously referenced files)
        let mut registry_completions = {
            let registry = self.symbol_registry.read().unwrap();
            registry.complete_file(&format!("#{}", prefix))
        };

        // Get completions from filesystem
        let filesystem_completions = self.scan_filesystem(prefix).await;

        // Merge
        registry_completions.extend(filesystem_completions);

        // Sort by score
        registry_completions.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Deduplicate
        let mut seen = HashMap::new();
        registry_completions.retain(|c| seen.insert(c.text.clone(), ()).is_none());

        registry_completions
    }

    /// Complete from memories in storage
    async fn complete_from_memories(&self, prefix: &str) -> Vec<CompletionCandidate> {
        // Search for memories with matching content
        // This is a simplified version - in production you'd use semantic search
        let prefix_lower = prefix.to_lowercase();

        match self
            .storage
            .list_memories(
                Some(self.namespace.clone()),
                20,
                MemorySortOrder::Importance,
            )
            .await
        {
            Ok(memories) => {
                memories
                    .into_iter()
                    .filter_map(|memory| {
                        // Check if summary or content contains the prefix
                        if memory.summary.to_lowercase().contains(&prefix_lower)
                            || memory.content.to_lowercase().contains(&prefix_lower)
                        {
                            Some(CompletionCandidate {
                                text: format!("@{}", memory.summary),
                                kind: SymbolKind::Concept,
                                detail: Some(format!(
                                    "{} (importance: {})",
                                    memory.content.chars().take(50).collect::<String>(),
                                    memory.importance
                                )),
                                score: memory.importance as f32,
                            })
                        } else {
                            None
                        }
                    })
                    .collect()
            }
            Err(_) => Vec::new(),
        }
    }

    /// Scan filesystem for file completions
    async fn scan_filesystem(&self, prefix: &str) -> Vec<CompletionCandidate> {
        // Check cache validity
        let needs_scan = {
            let cache = self.file_cache.read().unwrap();
            !cache.is_valid()
        };

        // Refresh cache if needed
        if needs_scan {
            if let Some(root) = &self.project_root {
                let files = Self::walk_directory(root, 3); // Max depth 3
                let mut cache = self.file_cache.write().unwrap();
                cache.update(files);
            }
        }

        // Filter cached files by prefix
        let prefix_lower = prefix.to_lowercase();
        let cache = self.file_cache.read().unwrap();

        cache
            .files
            .iter()
            .filter_map(|path| {
                let path_str = path.to_string_lossy();
                if path_str.to_lowercase().contains(&prefix_lower) {
                    // Score based on how early the prefix appears
                    let score = if let Some(pos) = path_str.to_lowercase().find(&prefix_lower) {
                        // Earlier matches score higher
                        100.0 - (pos as f32)
                    } else {
                        0.0
                    };

                    Some(CompletionCandidate {
                        text: format!("#{}", path_str),
                        kind: SymbolKind::File,
                        detail: Self::file_detail(path),
                        score: score.max(0.0),
                    })
                } else {
                    None
                }
            })
            .collect()
    }

    /// Walk directory recursively to build file list
    fn walk_directory(root: &Path, max_depth: usize) -> Vec<PathBuf> {
        let mut files = Vec::new();

        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                let path = entry.path();

                // Skip hidden files and directories
                if let Some(name) = path.file_name() {
                    if name.to_string_lossy().starts_with('.') {
                        continue;
                    }
                }

                // Skip target, node_modules, etc.
                if let Some(name) = path.file_name() {
                    let name_str = name.to_string_lossy();
                    if name_str == "target"
                        || name_str == "node_modules"
                        || name_str == ".git"
                        || name_str == "dist"
                        || name_str == "build"
                    {
                        continue;
                    }
                }

                if path.is_file() {
                    // Make path relative to root
                    if let Ok(relative) = path.strip_prefix(root) {
                        files.push(relative.to_path_buf());
                    }
                } else if path.is_dir() && max_depth > 0 {
                    // Recurse into subdirectories
                    let mut subfiles = Self::walk_directory(&path, max_depth - 1);
                    files.append(&mut subfiles);
                }
            }
        }

        files
    }

    /// Get file detail string
    fn file_detail(path: &Path) -> Option<String> {
        if let Ok(metadata) = std::fs::metadata(path) {
            let size = metadata.len();
            let size_str = if size < 1024 {
                format!("{} B", size)
            } else if size < 1024 * 1024 {
                format!("{} KB", size / 1024)
            } else {
                format!("{} MB", size / (1024 * 1024))
            };

            Some(size_str)
        } else {
            None
        }
    }

    /// Update project root
    pub fn set_project_root(&mut self, root: PathBuf) {
        self.project_root = Some(root);
        // Invalidate cache to force rescan
        let mut cache = self.file_cache.write().unwrap();
        cache.last_scan = None;
    }

    /// Force refresh of filesystem cache
    pub fn refresh_cache(&self) {
        let mut cache = self.file_cache.write().unwrap();
        cache.last_scan = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ics::editor::Position;
    use crate::ics::symbols::SymbolRegistry;
    use crate::storage::libsql::ConnectionMode;
    use crate::LibsqlStorage;
    use std::sync::Arc;
    use tempfile::TempDir;

    async fn create_test_engine() -> (CompletionEngine, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("test.db");

        let storage = Arc::new(
            LibsqlStorage::new_with_validation(
                ConnectionMode::Local(db_path.to_str().unwrap().to_string()),
                true,
            )
            .await
            .expect("Failed to create storage"),
        );

        let namespace = Namespace::Session {
            project: "test".to_string(),
            session_id: "test-session".to_string(),
        };

        let registry = Arc::new(RwLock::new(SymbolRegistry::new()));

        let engine = CompletionEngine::new(
            registry,
            storage,
            namespace,
            Some(temp_dir.path().to_path_buf()),
        );

        (engine, temp_dir)
    }

    #[test]
    fn test_detect_context_symbol() {
        let (engine, _temp) = tokio_test::block_on(create_test_engine());

        let (context, prefix) = engine.detect_context("This is @fo", 11);
        assert_eq!(context, CompletionContext::Symbol);
        assert_eq!(prefix, "fo");

        let (context, prefix) = engine.detect_context("@symbol", 7);
        assert_eq!(context, CompletionContext::Symbol);
        assert_eq!(prefix, "symbol");
    }

    #[test]
    fn test_detect_context_file() {
        let (engine, _temp) = tokio_test::block_on(create_test_engine());

        let (context, prefix) = engine.detect_context("See #src/m", 10);
        assert_eq!(context, CompletionContext::File);
        assert_eq!(prefix, "src/m");
    }

    #[test]
    fn test_detect_context_none() {
        let (engine, _temp) = tokio_test::block_on(create_test_engine());

        let (context, prefix) = engine.detect_context("No completion here", 10);
        assert_eq!(context, CompletionContext::None);
        assert_eq!(prefix, "");
    }

    #[tokio::test]
    async fn test_symbol_completion_integration() {
        let (engine, _temp) = create_test_engine().await;

        // Register a symbol
        {
            let mut registry = engine.symbol_registry.write().unwrap();
            registry.register_symbol(
                "@test_symbol",
                Position { line: 0, column: 0 },
                SymbolKind::Variable,
                Some("Test symbol".to_string()),
            );
        }

        // Get completions
        let completions = engine.get_completions("@te", 3).await;

        assert!(!completions.is_empty());
        assert!(completions.iter().any(|c| c.text == "@test_symbol"));
    }
}
