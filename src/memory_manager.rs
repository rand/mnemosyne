//! Simple, high-level memory API for personal AI agents.
//!
//! ## Design goals (inspired by Mem0, Letta, Zep, SuperLocalMemory)
//! - Agent namespace isolation – each agent owns its own private memory scope
//! - Flat text I/O – plain strings in, plain strings out
//! - Proactive lifecycle – `forget()` manages recall quality
//! - Graph-aware recall – related memories surfaced via `MemoryLink`
//! - Offline-capable – local embeddings used automatically
//! - Bare-minimum API – store / recall / list / forget / update
//!
//! ## Quick start
//!
//! ```no_run
//! use mnemosyne_core::MemoryManager;
//!
//! # fn main() -> mnemosyne_core::error::Result<()> {
//! # tokio::runtime::Runtime::new().unwrap().block_on(async {
//! // Open (or create) a per-agent database
//! let agent = MemoryManager::new("my-agent").await?;
//!
//! // Store a memory (auto-enriched by default)
//! let id = agent.store("User prefers dark mode in all apps.").await?;
//!
//! // Semantic recall – highest fidelity hits first
//! let hits = agent.recall("What UI theme does the user like?", 5).await?;
//! for r in &hits {
//!     println!("[{:.2}] {}", r.score, r.memory.summary);
//! }
//!
//! // Soft-delete a wrong or superseded memory
//! agent.forget(&id).await?;
//! # Ok(())
//! # })
//! # }
//! ```

use std::path::PathBuf;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::{
    error::{MnemosyneError, Result},
    storage::{libsql::ConnectionMode, MemorySortOrder, StorageBackend},
    types::{MemoryId, MemoryNote, Namespace, SearchResult},
};

/// Re-export so consumers can write `MemoryConfig::tags(vec![MemoryType::Feature])`.
pub use crate::types::MemoryType;

/// Per-operation configuration for [`MemoryManager`].
#[derive(Debug, Clone, Default)]
pub struct MemoryConfig {
    /// Namespace override. Defaults to the agent's private `Agent` namespace.
    pub namespace: Option<Namespace>,
    /// Skip LLM enrichment (skip summarisation + tagging).
    pub skip_enrich: bool,
    /// Maximum recall results.
    pub max_results: Option<usize>,
    /// Only recall memories with importance ≥ this value (1-10).
    pub min_importance: Option<u8>,
    /// Tags to attach to a newly stored memory.
    pub tags: Vec<String>,
    /// Memory type for a newly stored memory.
    pub memory_type: Option<MemoryType>,
}

impl MemoryConfig {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn namespace(mut self, ns: Namespace) -> Self {
        self.namespace = Some(ns);
        self
    }
    pub fn skip_enrich(mut self) -> Self {
        self.skip_enrich = true;
        self
    }
    pub fn max_results(mut self, n: usize) -> Self {
        self.max_results = Some(n);
        self
    }
    pub fn min_importance(mut self, n: u8) -> Self {
        self.min_importance = Some(n);
        self
    }
    pub fn tags(mut self, tags: impl Into<Vec<String>>) -> Self {
        self.tags = tags.into();
        self
    }
    pub fn memory_type(mut self, memory_type: MemoryType) -> Self {
        self.memory_type = Some(memory_type);
        self
    }
}

/// Ergonomic, agent-first memory API.
///
/// Owns a [`LibsqlStorage`] in `Arc<Mutex<…>>` – thread-safe and `Clone`.
/// All default operations target the agent's private [`Namespace::Agent`] scope.
/// The agent identifier is also used as a database filename, so it is restricted
/// to portable filename characters.
#[derive(Clone)]
pub struct MemoryManager {
    agent_id: String,
    storage: Arc<Mutex<crate::storage::libsql::LibsqlStorage>>,
    default_namespace: Namespace,
}

impl std::fmt::Debug for MemoryManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MemoryManager")
            .field("agent_id", &self.agent_id)
            .field("default_namespace", &self.default_namespace)
            .finish_non_exhaustive()
    }
}

fn validate_agent_id(agent_id: &str) -> Result<()> {
    if agent_id.is_empty()
        || agent_id == "."
        || agent_id == ".."
        || !agent_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err(MnemosyneError::ValidationError(
            "agent_id must contain only ASCII letters, digits, '-', '_' or '.'".to_string(),
        ));
    }
    Ok(())
}

// ----------------------------------------------------------------------- factory

impl MemoryManager {
    /// Open (or create) the agent's database at the default location
    /// (`~/.mnemosyne/<agent_id>.db`).
    ///
    /// # Errors
    /// Fails if the database cannot be opened or initialised.
    pub async fn new(agent_id: impl Into<String>) -> Result<Self> {
        Self::new_with_path(agent_id, None).await
    }

    /// Open the agent's database at an explicit file path.
    pub async fn new_with_path(
        agent_id: impl Into<String>,
        db_path: Option<PathBuf>,
    ) -> Result<Self> {
        let agent_id: String = agent_id.into();
        validate_agent_id(&agent_id)?;
        let path = match db_path {
            Some(p) => p.display().to_string(),
            None => {
                let mut dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
                dir.push(".mnemosyne");
                let _ = std::fs::create_dir_all(&dir);
                dir.push(format!("{}.db", agent_id));
                dir.display().to_string()
            }
        };
        let mode = ConnectionMode::Local(path);
        let init = crate::storage::libsql::LibsqlStorage::new_with_validation(mode, true).await?;
        Ok(Self {
            agent_id: agent_id.clone(),
            storage: Arc::new(Mutex::new(init)),
            default_namespace: Namespace::Agent { agent_id },
        })
    }

    /// Open with an explicit [`ConnectionMode`] (e.g. remote libsql / Turso).
    pub async fn with_connection(
        agent_id: impl Into<String>,
        mode: ConnectionMode,
    ) -> Result<Self> {
        let agent_id: String = agent_id.into();
        validate_agent_id(&agent_id)?;
        let init = crate::storage::libsql::LibsqlStorage::new_with_validation(mode, true).await?;
        Ok(Self {
            agent_id: agent_id.clone(),
            storage: Arc::new(Mutex::new(init)),
            default_namespace: Namespace::Agent { agent_id },
        })
    }

    // --------------------------------------------------------------------- read

    /// Recall semantically similar memories via hybrid search.
    ///
    /// Results ranked highest score first.
    pub async fn recall(
        &self,
        query: impl Into<String>,
        limit: usize,
    ) -> Result<Vec<SearchResult>> {
        self.recall_with_config(query, limit, MemoryConfig::new())
            .await
    }

    /// Recall with full configuration (namespace, importance filter, …).
    pub async fn recall_with_config(
        &self,
        query: impl Into<String>,
        limit: usize,
        config: MemoryConfig,
    ) -> Result<Vec<SearchResult>> {
        let q: String = query.into();
        let ns = config
            .namespace
            .unwrap_or_else(|| self.default_namespace.clone());
        let min_imp = config.min_importance;

        let mut results = {
            let guard = self.storage.lock().await;
            let inner: &crate::storage::libsql::LibsqlStorage = &*guard;
            inner
                .hybrid_search(&q, Some(ns.clone()), limit * 2, true)
                .await?
        };

        if let Some(min) = min_imp {
            results.retain(|r| r.memory.importance >= min);
        }
        results.truncate(limit);
        results.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Ok(results)
    }

    /// List memories sorted by recency, importance, or access count.
    pub async fn list(&self, limit: usize, sort_by: MemorySortOrder) -> Result<Vec<MemoryNote>> {
        self.list_with_config(limit, sort_by, MemoryConfig::new())
            .await
    }

    /// List with optional tag filter.
    pub async fn list_with_config(
        &self,
        limit: usize,
        sort_by: MemorySortOrder,
        config: MemoryConfig,
    ) -> Result<Vec<MemoryNote>> {
        let ns = config
            .namespace
            .unwrap_or_else(|| self.default_namespace.clone());
        let guard = self.storage.lock().await;
        let inner: &crate::storage::libsql::LibsqlStorage = &*guard;
        let mut notes = inner.list_memories(Some(ns), limit, sort_by).await?;
        if !config.tags.is_empty() {
            let lower: Vec<String> = config.tags.iter().map(|t| t.to_lowercase()).collect();
            notes.retain(|n| n.tags.iter().any(|t| lower.contains(&t.to_lowercase())));
        }
        Ok(notes)
    }

    /// Fetch a memory by id, or `Ok(None)` if not found / archived.
    pub async fn get(&self, id: &MemoryId) -> Result<Option<MemoryNote>> {
        let guard = self.storage.lock().await;
        let inner: &crate::storage::libsql::LibsqlStorage = &*guard;
        // StorageBackend::get_memory takes MemoryId by value; returns Result<MemoryNote>.
        // We wrap in Option for the public API.
        match inner.get_memory(*id).await {
            Ok(m) if m.id == *id => Ok(Some(m)),
            Ok(_) => Ok(None),
            Err(MnemosyneError::MemoryNotFound(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    // -------------------------------------------------------------------- write

    /// Store a plain-text memory in the agent's default namespace.
    ///
    /// The note receives a bounded summary and can be embedded when the backing
    /// storage has an embedding service configured. Use
    /// [`store_with_config`](Self::store_with_config) with `.skip_enrich()` to
    /// skip the optional embedding step.
    pub async fn store(&self, content: impl Into<String>) -> Result<MemoryId> {
        self.store_with_config(content, MemoryConfig::new()).await
    }

    /// Store with full configuration.
    pub async fn store_with_config(
        &self,
        content: impl Into<String>,
        config: MemoryConfig,
    ) -> Result<MemoryId> {
        let ns = config
            .namespace
            .unwrap_or_else(|| self.default_namespace.clone());
        let content_str = content.into();

        let summary = crate::utils::string::truncate_at_char_boundary(&content_str, 200);
        let note = MemoryNote {
            id: MemoryId::new(),
            namespace: ns.clone(),
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            content: content_str.clone(),
            summary,
            keywords: Vec::new(),
            tags: config.tags.clone(),
            context: String::new(),
            memory_type: config
                .memory_type
                .unwrap_or(crate::types::MemoryType::Insight),
            importance: config.min_importance.unwrap_or(5),
            confidence: 0.8,
            links: Vec::new(),
            related_files: Vec::new(),
            related_entities: Vec::new(),
            access_count: 0,
            last_accessed_at: chrono::Utc::now(),
            expires_at: None,
            is_archived: false,
            superseded_by: None,
            embedding: None,
            embedding_model: String::new(),
        };

        let note_id = note.id;

        // Persist via StorageBackend trait
        {
            let guard = self.storage.lock().await;
            let inner: &crate::storage::libsql::LibsqlStorage = &*guard;
            <crate::storage::libsql::LibsqlStorage as StorageBackend>::store_memory(inner, &note)
                .await?;
        }

        // Optional embedding
        if !config.skip_enrich {
            let guard = self.storage.lock().await;
            let inner: &crate::storage::libsql::LibsqlStorage = &*guard;
            let _ = inner
                .generate_and_store_embedding(&note_id, &content_str)
                .await;
        }

        Ok(note_id)
    }

    /// Natural-language alias for [`store`].
    pub async fn remember(&self, content: impl Into<String>) -> Result<MemoryId> {
        self.store(content).await
    }

    /// Update an existing memory; only `Some(..)` fields are changed.
    ///
    /// Changing `content` clears the embedding column (reconsolidation re-embeds).
    pub async fn update(
        &self,
        id: &MemoryId,
        content: Option<String>,
        importance: Option<u8>,
        tags: Option<Vec<String>>,
    ) -> Result<MemoryNote> {
        let guard = self.storage.lock().await;
        let inner: &crate::storage::libsql::LibsqlStorage = &*guard;

        // Use MemoryUpdates struct: build update, then apply via trait
        // Use MemoryId by value (Copy); trait impl takes `id: MemoryId`.
        let current = inner.get_memory(*id).await?;

        // Apply partial updates in-memory
        let mut note = current;
        if let Some(c) = content {
            note.content = c;
            note.updated_at = chrono::Utc::now();
            note.embedding = None;
        }
        if let Some(i) = importance {
            note.importance = i;
        }
        if let Some(t) = tags {
            note.tags = t;
        }

        // Persist the updated MemoryNote (not MemoryUpdates)
        <crate::storage::libsql::LibsqlStorage as StorageBackend>::update_memory(inner, &note)
            .await?;
        // get_memory takes MemoryId by value; returns Result<MemoryNote> directly.
        Ok(inner.get_memory(*id).await?)
    }

    // -------------------------------------------------------------------- lifecycle

    /// Soft-delete (archive) a memory.
    pub async fn forget(&self, id: &MemoryId) -> Result<()> {
        let guard = self.storage.lock().await;
        let inner: &crate::storage::libsql::LibsqlStorage = &*guard;
        <crate::storage::libsql::LibsqlStorage as StorageBackend>::archive_memory(inner, *id)
            .await?;
        Ok(())
    }
}

use crate::utils::is_trivial_prompt;

impl MemoryManager {
    /// The agent identifier for this manager.
    #[inline]
    pub fn agent_id(&self) -> &str {
        &self.agent_id
    }

    /// Prefetch context for the given query, returning plain text ready for
    /// agent prompt injection.
    ///
    /// Mirrors the `prefetch_all` contract in hermes-agent's `MemoryManager`:
    /// returns merged plain-text blocks from each provider.  Failures in any
    /// provider are swallowed so the agent turn never blocks on memory.
    ///
    /// Returns empty string when the query is a trivial greeting ("hi",
    /// "thanks") or when no provider returns content.
    pub async fn prefetch(&self, query: impl Into<String>) -> String {
        self.prefetch_with_config(query, MemoryConfig::new()).await
    }

    /// Prefetch context with namespace and result-count configuration.
    pub async fn prefetch_with_config(
        &self,
        query: impl Into<String>,
        config: MemoryConfig,
    ) -> String {
        let q = query.into();
        if is_trivial_prompt(&q) {
            return String::new();
        }
        // One-shot recall: pull top results and join their content into
        // a single string to feed back into the model as context.
        let limit = config.max_results.unwrap_or(5);
        match self.recall_with_config(&q, limit, config).await {
            Ok(hits) => {
                let parts: Vec<String> = hits
                    .into_iter()
                    .filter(|r| r.score > 0.0)
                    .map(|r| {
                        format!(
                            "[relevance={:.2}] {}\n---",
                            r.score,
                            r.memory.content.trim()
                        )
                    })
                    .collect();
                parts.join("\n\n")
            }
            Err(_) => String::new(),
        }
    }

    /// Like [`prefetch`] but includes the `<memory-context>` fence block
    /// so the output is directly injectable into the user message.
    ///
    /// The fenced block isolates memory context from user input so
    /// the model treats it as reference data, not new conversation.
    pub fn build_context_block(&self, prefetched_text: impl AsRef<str>) -> String {
        build_memory_context_block(prefetched_text.as_ref())
    }

    /// Sync a completed turn to memory — store a user+assistant exchange
    /// so the agent's persistent memory captures what happened this turn.
    ///
    /// Mirrors `MemoryManager::sync_all` in hermes-agent.  The exchange is
    /// stored as a single memory note tagged `\"turn_sync\"` in the agent's
    /// default namespace.  Enrichment (LLM summary/embedding) is skipped for
    /// speed — call `mnemosyne embed` or run evolution to backfill later.
    ///
    /// Errors are returned (not swallowed) so the agent can decide, but the
    /// call is cheap: it is a single INSERT.
    pub async fn sync(&self, user_text: &str, assistant_text: &str) -> Result<MemoryId> {
        self.sync_with_config(user_text, assistant_text, MemoryConfig::new())
            .await
    }

    /// Sync a completed turn with an optional namespace, tags, and memory type.
    pub async fn sync_with_config(
        &self,
        user_text: &str,
        assistant_text: &str,
        config: MemoryConfig,
    ) -> Result<MemoryId> {
        let ns = config
            .namespace
            .unwrap_or_else(|| self.default_namespace.clone());
        let content = format!(
            "User: {}\n\nAssistant: {}",
            user_text.trim(),
            assistant_text.trim()
        );
        let now = chrono::Utc::now();
        let note = MemoryNote {
            id: MemoryId::new(),
            namespace: ns.clone(),
            created_at: now,
            updated_at: now,
            content: content.clone(),
            summary: crate::utils::string::truncate_at_char_boundary(&content, 200),
            keywords: Vec::new(),
            tags: {
                let mut tags = vec!["turn_sync".to_string()];
                tags.extend(config.tags);
                tags
            },
            context: "Agent turn sync".to_string(),
            memory_type: config
                .memory_type
                .unwrap_or(crate::types::MemoryType::Insight),
            importance: 5,
            confidence: 0.7,
            links: Vec::new(),
            related_files: Vec::new(),
            related_entities: Vec::new(),
            access_count: 0,
            last_accessed_at: now,
            expires_at: None,
            is_archived: false,
            superseded_by: None,
            embedding: None,
            embedding_model: String::new(),
        };

        let note_id = note.id;
        {
            let guard = self.storage.lock().await;
            let inner: &crate::storage::libsql::LibsqlStorage = &*guard;
            <crate::storage::libsql::LibsqlStorage as StorageBackend>::store_memory(inner, &note)
                .await?;
        }

        Ok(note_id)
    }

    // -------------------------------------------------------------------- best-effort wrappers

    /// Best-effort recall: never propagates errors.
    ///
    /// Agent code can call this without defensive `try/except` wrappers.
    pub async fn recall_best_effort(
        &self,
        query: impl Into<String>,
        limit: usize,
    ) -> Vec<SearchResult> {
        self.recall(query, limit).await.unwrap_or_default()
    }

    /// Best-effort forget: logs and swallows any error.
    pub async fn forget_best_effort(&self, id: &MemoryId) {
        if let Err(e) = self.forget(id).await {
            tracing::debug!("forget_best_effort: ignoring error for {}: {}", id, e);
        }
    }
}

/// Wrap prefetched memory context in a `<memory-context>` fence block.
///
/// Re-exported from [`crate::agent_context`] for backward compatibility.
pub use crate::agent_context::build_memory_context_block;

// ===================================================================== tests

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mn_mgr_new_sets_agent_id() {
        let mgr = MemoryManager::new("t").await.unwrap();
        assert_eq!(mgr.agent_id(), "t");
    }

    #[tokio::test]
    async fn mn_mgr_rejects_path_traversal_agent_ids() {
        let err = MemoryManager::new("../outside").await.unwrap_err();
        assert!(matches!(err, MnemosyneError::ValidationError(_)));
    }

    #[tokio::test]
    async fn mn_mgr_defaults_to_agent_namespace() {
        let mgr = MemoryManager::new("ns").await.unwrap();
        assert!(matches!(mgr.default_namespace, Namespace::Agent { .. } if mgr.agent_id() == "ns"));
    }

    #[tokio::test]
    async fn mn_mgr_store_recall_round_trip() {
        let aid = format!("rt-{}", uuid::Uuid::new_v4());
        let mgr = MemoryManager::new(&aid).await.unwrap();
        mgr.store("Cats have five toes front, four back.")
            .await
            .unwrap();
        let hits = mgr.recall("How many cat toes?", 5).await.unwrap();
        assert!(!hits.is_empty(), "need >= 1 result, got {:?}", hits);
        assert!(hits.iter().any(|r| r.score >= 0.0));
    }

    #[tokio::test]
    async fn mn_mgr_namespace_isolation() {
        let a = MemoryManager::new(format!("ia-{}", uuid::Uuid::new_v4()))
            .await
            .unwrap();
        let b = MemoryManager::new(format!("ib-{}", uuid::Uuid::new_v4()))
            .await
            .unwrap();
        a.store("secret A").await.unwrap();
        b.store("secret B").await.unwrap();
        let r = a.recall("secret A", 10).await.unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].memory.summary, "secret A");
    }

    #[tokio::test]
    async fn mn_mgr_list_and_tag_filter() {
        let aid = format!("lst-{}", uuid::Uuid::new_v4());
        let mgr = MemoryManager::new(&aid).await.unwrap();
        mgr.store("alpha").await.unwrap();
        mgr.store_with_config("beta", MemoryConfig::new().tags(vec!["t".into()]))
            .await
            .unwrap();
        let all = mgr.list(10, MemorySortOrder::Recent).await.unwrap();
        assert!(all.len() >= 2, "need >= 2, got {}", all.len());
        let tagged = mgr
            .list_with_config(
                10,
                MemorySortOrder::Recent,
                MemoryConfig::new().tags(vec!["t".into()]),
            )
            .await
            .unwrap();
        assert_eq!(tagged.len(), 1);
        assert!(tagged[0].tags.contains(&"t".to_string()));
    }

    #[tokio::test]
    async fn mn_mgr_forget_removes_from_recall() {
        let aid = format!("fg-{}", uuid::Uuid::new_v4());
        let mgr = MemoryManager::new(&aid).await.unwrap();
        let id = mgr.store("to forget").await.unwrap();
        mgr.forget(&id).await.unwrap();
        assert!(mgr.recall("to forget", 10).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn mn_mgr_update_changes_fields() {
        let aid = format!("upd-{}", uuid::Uuid::new_v4());
        let mgr = MemoryManager::new(&aid).await.unwrap();
        let id = mgr.store("orig").await.unwrap();
        let u = mgr
            .update(&id, Some("revised".into()), Some(9), None)
            .await
            .unwrap();
        assert_eq!(u.importance, 9);
        assert!(u.content.contains("revised"));
    }

    #[tokio::test]
    async fn mn_mgr_get_missing_returns_none() {
        assert!(MemoryManager::new("gn")
            .await
            .unwrap()
            .get(&MemoryId::new())
            .await
            .unwrap()
            .is_none());
    }

    // ── Tests for the agent-usability API added for hermes-agent parity ──

    #[tokio::test]
    async fn mn_mgr_prefetch_returns_text() {
        let aid = format!("pf-{}", uuid::Uuid::new_v4());
        let mgr = MemoryManager::new(&aid).await.unwrap();
        mgr.store_with_config(
            "Cats have five toes on front paws.",
            MemoryConfig::new().skip_enrich(),
        )
        .await
        .unwrap();
        let text = mgr.prefetch("cat toes").await;
        assert!(
            text.contains("Cats"),
            "prefetch should return text content, got: {text}"
        );
    }

    #[tokio::test]
    async fn mn_mgr_prefetch_trivial_returns_empty() {
        let mgr = MemoryManager::new("trivial").await.unwrap();
        assert_eq!(mgr.prefetch("hi").await, "");
        assert_eq!(mgr.prefetch("thanks").await, "");
        assert_eq!(mgr.prefetch("ok").await, "");
    }

    #[tokio::test]
    async fn mn_mgr_context_block_wraps_text() {
        let mgr = MemoryManager::new("cb").await.unwrap();
        let block = mgr.build_context_block("The user prefers dark mode.");
        assert!(block.contains("<memory-context>"));
        assert!(block.contains("dark mode"));
        assert!(block.contains("System note"));
    }

    #[tokio::test]
    async fn mn_mgr_context_block_strips_prewrapped() {
        // When the prefetched text already contains a context block,
        // build_context_block calls sanitize_context to strip it, preventing
        // a double-wrapped <memory-context> block. The sanitized content
        // (the "Hello" that was *between* the tags) is preserved.
        let mgr = MemoryManager::new("strip").await.unwrap();
        let input = "<memory-context>\n[System note: old]\nHello\n</memory-context>";
        let block = mgr.build_context_block(input);
        // After sanitize_context strips the block, the remaining text is
        // "\nHello\n" (leading content before <memory-context> minus
        // the block). Since input *starts* with the tag, sanitize_context
        // removes it entirely. The result is stripped and then re-wrapped.
        assert!(block.starts_with("<memory-context>"));
        assert!(block.ends_with("</memory-context>"));
        // Should contain exactly one opening and one closing tag (no double-wrap)
        assert_eq!(block.matches("<memory-context>").count(), 1);
        assert_eq!(block.matches("</memory-context>").count(), 1);
    }

    #[tokio::test]
    async fn mn_mgr_recall_best_effort_never_errors() {
        let mgr = MemoryManager::new("bef").await.unwrap();
        // Even with no DB / no memories, should return empty vec, not panic.
        let hits = mgr.recall_best_effort("nothing here", 5).await;
        assert!(hits.is_empty());
    }

    #[tokio::test]
    async fn mn_mgr_forget_best_effort_never_errors() {
        let mgr = MemoryManager::new("fbf").await.unwrap();
        // Forgetting a non-existent ID should not panic.
        mgr.forget_best_effort(&MemoryId::new()).await;
    }

    #[tokio::test]
    async fn mn_mgr_sync_stores_exchange() {
        let aid = format!("sync-{}", uuid::Uuid::new_v4());
        let mgr = MemoryManager::new(&aid).await.unwrap();
        let id = mgr
            .sync("What is the project status?", "The project is on track.")
            .await
            .unwrap();
        // The synced exchange should be recallable.
        let hits = mgr.recall("project status", 5).await.unwrap();
        assert!(
            hits.iter().any(|r| r.memory.id == id),
            "sync result should be found in recall, got: {:?}",
            hits
        );
    }
}
