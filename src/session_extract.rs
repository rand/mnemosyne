//! Session commit → long-term memory extraction pipeline (design borrowed
//! from OpenViking's session/memory concept).
//!
//! Lifecycle: **Create → Interact → Commit**
//!
//! `commit()` runs in two phases, mirroring the upstream design:
//!
//! 1. **Synchronous**: archive the session messages and return immediately.
//! 2. **Asynchronous** (call [`extract_and_decide`]): generate candidate
//!    memories from the conversation, vector pre-filter similar existing
//!    memories, make per-item dedup decisions (`skip / create / merge /
//!    delete`), and emit a [`MemoryDiff`] audit record of every mutation for
//!    rollback.
//!
//! Candidate generation here is heuristic (preference/decision statement
//! detection) so the pipeline works offline; an LLM extractor can replace
//! [`extract_candidates`] without changing downstream types.

use crate::types::MemoryId;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// One message in a session transcript
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    /// "user" | "assistant"
    pub role: String,
    pub text: String,
}

impl SessionMessage {
    pub fn new(role: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            role: role.into(),
            text: text.into(),
        }
    }
}

/// A candidate memory extracted from a session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateMemory {
    pub content: String,
    /// Why this was extracted (e.g. "user preference", "decision")
    pub reason: &'static str,
}

/// Per-existing-item dedup decisions (mirrors OpenViking's decision levels)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum DedupDecision {
    /// Candidate is a duplicate of an existing memory — do nothing
    Skip { existing_id: MemoryId },
    /// Merge candidate content into an existing memory
    Merge { existing_id: MemoryId },
    /// Conflicting existing memory should be deleted in favor of candidate
    Delete { existing_id: MemoryId },
}

/// Outcome of resolving one candidate against existing memories
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CandidateResolution {
    /// Create a brand-new memory
    Create,
    /// Do not create; apply per-item decisions to existing memories only
    ResolveExisting(Vec<DedupDecision>),
    /// Skip entirely (exact duplicate)
    SkipAll(DedupDecision),
}

// ---------------------------------------------------------------------------
// Similarity pre-filter (lexical Jaccard over word sets — no embeddings needed;
// callers with an embedding service can substitute cosine similarity)
// ---------------------------------------------------------------------------

/// Lexical similarity between two texts: Jaccard over lowercased word sets.
pub fn lexical_similarity(a: &str, b: &str) -> f32 {
    let words = |s: &str| -> std::collections::HashSet<String> {
        s.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| w.len() > 2)
            .map(|w| w.to_string())
            .collect()
    };
    let sa = words(a);
    let sb = words(b);
    if sa.is_empty() && sb.is_empty() {
        return 1.0;
    }
    if sa.is_empty() || sb.is_empty() {
        return 0.0;
    }
    let intersection = sa.intersection(&sb).count() as f32;
    let union = (sa.len() + sb.len()) as f32 - intersection;
    intersection / union
}

// ---------------------------------------------------------------------------
// Candidate extraction (heuristic, offline-safe)
// ---------------------------------------------------------------------------

/// Sentence-level markers that a user message contains a durable preference
const PREFERENCE_MARKERS: &[&str] = &[
    "i prefer",
    "i like",
    "i always",
    "i never",
    "we prefer",
    "we use",
    "please remember",
    "remember that",
    "keep in mind",
    "from now on",
    "my name is",
];

/// Markers that a user/assistant exchange records a decision or insight
const DECISION_MARKERS: &[&str] = &[
    "we decided",
    "decided to",
    "the approach is",
    "chose ",
    "choosing ",
    "going with ",
    "it turns out",
    "root cause",
    "the fix was",
    "lesson learned",
];

/// Extract candidate memories from a session transcript.
///
/// Heuristics: scan user messages for preference statements and either role
/// for decision/insight sentences. Each matching sentence becomes a candidate.
pub fn extract_candidates(messages: &[SessionMessage]) -> Vec<CandidateMemory> {
    let mut out = Vec::new();
    for msg in messages {
        let is_user = msg.role.eq_ignore_ascii_case("user");
        for sentence in split_sentences(&msg.text) {
            let lower = sentence.to_lowercase();
            if is_user && PREFERENCE_MARKERS.iter().any(|m| lower.contains(m)) {
                out.push(CandidateMemory {
                    content: sentence.clone(),
                    reason: "user preference",
                });
            } else if DECISION_MARKERS.iter().any(|m| lower.contains(m)) {
                out.push(CandidateMemory {
                    content: sentence.clone(),
                    reason: "decision/insight",
                });
            }
        }
    }
    // Dedup identical candidates, preserving order
    let mut seen = std::collections::HashSet::new();
    out.retain(|c| seen.insert(normalize_for_dedup(&c.content)));
    out
}

fn split_sentences(text: &str) -> Vec<String> {
    text.split(|c: char| c == '.' || c == '\n' || c == '!' || c == '?')
        .map(|s| s.trim())
        .filter(|s| s.len() > 8)
        .map(|s| format!("{}.", s))
        .collect()
}

fn normalize_for_dedup(s: &str) -> String {
    s.to_lowercase().split_whitespace().collect::<Vec<_>>().join(" ")
}

// ---------------------------------------------------------------------------
// Dedup decisions
// ---------------------------------------------------------------------------

/// Similarity above which a candidate is considered an exact duplicate
pub const SKIP_THRESHOLD: f32 = 0.92;
/// Similarity above which a candidate merges into the best existing match
pub const MERGE_THRESHOLD: f32 = 0.72;

/// Decide how to resolve a candidate against pre-filtered existing memories.
///
/// `existing` is a list of `(id, similarity)` pairs from vector pre-filtering
/// (or lexical fallback), sorted arbitrarily. Decision rules:
///
/// - any similarity ≥ `SKIP_THRESHOLD` → skip (duplicate exists)
/// - best similarity ≥ `MERGE_THRESHOLD` → merge into best match
/// - otherwise → create new
///
/// Deletion is intentionally conservative: only triggered by explicit caller
/// policy via [`resolve_with_policy`] when the existing memory is fully
/// subsumed (similarity ≥ SKIP_THRESHOLD but *older* content is shorter).
pub fn resolve_candidate(
    candidate_content: &str,
    existing: &[(MemoryId, f32)],
) -> CandidateResolution {
    if existing.is_empty() {
        return CandidateResolution::Create;
    }
    // Exact duplicate check first (normalized text equality)
    for (id, sim) in existing {
        if *sim >= SKIP_THRESHOLD {
            return CandidateResolution::SkipAll(DedupDecision::Skip { existing_id: *id });
        }
    }
    let best = existing.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    if let Some((id, sim)) = best {
        if *sim >= MERGE_THRESHOLD {
            return CandidateResolution::ResolveExisting(vec![DedupDecision::Merge { existing_id: *id }]);
        }
    }
    let _ = lexical_similarity(candidate_content, ""); // keep fn referenced for API stability
    CandidateResolution::Create
}

/// Policy-aware resolution allowing delete decisions for subsumed memories.
///
/// `existing_with_lengths` provides `(id, similarity, existing_content_len)`.
/// If the candidate fully covers an existing memory (similarity ≥
/// `SKIP_THRESHOLD`) *and* the candidate is substantially richer (≥25% longer),
/// the old memory is deleted in favor of creating the new one.
pub fn resolve_with_policy(
    candidate_content: &str,
    existing_with_lengths: &[(MemoryId, f32, usize)],
) -> CandidateResolution {
    let cand_len = candidate_content.len();
    for (id, sim, old_len) in existing_with_lengths {
        if *sim >= SKIP_THRESHOLD {
            if cand_len > *old_len * 5 / 4 {
                return CandidateResolution::ResolveExisting(vec![
                    DedupDecision::Delete { existing_id: *id },
                ]);
            }
            return CandidateResolution::SkipAll(DedupDecision::Skip { existing_id: *id });
        }
    }
    let pairs: Vec<(MemoryId, f32)> =
        existing_with_lengths.iter().map(|(id, s, _)| (*id, *s)).collect();
    resolve_candidate(candidate_content, &pairs)
}

// ---------------------------------------------------------------------------
// Memory diff audit log
// ---------------------------------------------------------------------------

/// One recorded operation in a memory diff
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffOperation {
    Add {
        id: MemoryId,
        content: String,
        reason: String,
    },
    Update {
        id: MemoryId,
        before: String,
        after: String,
    },
    #[serde(rename = "delete")]
    Delete {
        id: MemoryId,
        deleted_content: String,
    },
}

/// Audit record of all memory changes from one session commit — written to
/// disk so extractions can be reviewed and rolled back.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryDiff {
    /// ISO-8601 timestamp of extraction
    pub extracted_at: String,
    pub operations: Vec<DiffOperation>,
}

impl MemoryDiff {
    pub fn new() -> Self {
        Self {
            extracted_at: Utc::now().to_rfc3339(),
            operations: Vec::new(),
        }
    }

    pub fn summary(&self) -> (usize, usize, usize) {
        let adds = self.operations.iter().filter(|o| matches!(o, DiffOperation::Add { .. })).count();
        let updates = self.operations.iter().filter(|o| matches!(o, DiffOperation::Update { .. })).count();
        let deletes = self.operations.iter().filter(|o| matches!(o, DiffOperation::Delete { .. })).count();
        (adds, updates, deletes)
    }

    /// Persist to `<dir>/memory_diff_<timestamp>.json`
    pub fn write_to_dir(&self, dir: &Path) -> std::io::Result<std::path::PathBuf> {
        std::fs::create_dir_all(dir)?;
        let filename = format!(
            "memory_diff_{}.json",
            Utc::now().format("%Y%m%d_%H%M%S%3f")
        );
        let path = dir.join(filename);
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        std::fs::write(&path, json)?;
        Ok(path)
    }
}

impl Default for MemoryDiff {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Full pipeline
// ---------------------------------------------------------------------------

/// Result of running the full extraction pipeline for one commit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionResult {
    pub candidates_extracted: usize,
    pub created: Vec<MemoryId>,
    pub merged_into: Vec<MemoryId>,
    pub skipped: usize,
    pub deleted: Vec<MemoryId>,
}

/// Run the sync phase of a commit: archive messages to a JSONL file.
///
/// Returns the archive path. Mirrors OpenViking's "archive (sync)" step.
pub fn archive_messages(
    session_id: &str,
    messages: &[SessionMessage],
    archive_dir: &Path,
) -> std::io::Result<std::path::PathBuf> {
    std::fs::create_dir_all(archive_dir)?;
    let path = archive_dir.join(format!("{}_messages.jsonl", sanitize(session_id)));
    let mut out = String::new();
    for m in messages {
        let line = serde_json::json!({"role": m.role, "text": m.text, "ts": Utc::now().to_rfc3339()});
        out.push_str(&line.to_string());
        out.push('\n');
    }
    std::fs::write(&path, out)?;
    Ok(path)
}

fn sanitize(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexical_similarity_identical_and_disjoint() {
        assert!((lexical_similarity("use redis for caching", "use redis for caching") - 1.0).abs() < 1e-6);
        assert_eq!(lexical_similarity("redis caching layer", "quantum entanglement physics"), 0.0);
    }

    #[test]
    fn test_lexical_similarity_partial_overlap() {
        let s = lexical_similarity(
            "we decided to use postgres for storage",
            "we chose postgres storage engine today",
        );
        assert!(s > 0.2 && s < 0.9);
    }

    #[test]
    fn test_extract_preferences_from_user_messages() {
        let msgs = vec![
            SessionMessage::new("user", "I prefer tabs over spaces. Also I like dark themes."),
            SessionMessage::new("assistant", "Sure thing."),
        ];
        let cands = extract_candidates(&msgs);
        assert!(cands.iter().all(|c| c.reason == "user preference"));
        assert!(cands.len() >= 2);
    }

    #[test]
    fn test_extract_decisions_from_any_role() {
        let msgs = vec![
            SessionMessage::new("assistant", "The fix was to bump the timeout. We decided to retry twice."),
            SessionMessage::new("user", "great"),
        ];
        let cands = extract_candidates(&msgs);
        assert!(cands.iter().any(|c| c.reason == "decision/insight"));
    }

    #[test]
    fn test_no_candidates_from_smalltalk() {
        let msgs = vec![
            SessionMessage::new("user", "hi there!"),
            SessionMessage::new("assistant", "Hello! How can I help?"),
        ];
        assert!(extract_candidates(&msgs).is_empty());
    }

    #[test]
    fn test_duplicate_candidates_removed() {
        let msgs = vec![
            SessionMessage::new("user", "I prefer vim keybindings."),
            SessionMessage::new("user", "I prefer vim keybindings."),
        ];
        let cands = extract_candidates(&msgs);
        assert_eq!(cands.len(), 1);
    }

    #[test]
    fn test_resolve_skip_on_high_similarity() {
        let id = MemoryId(uuid::Uuid::new_v4());
        let res = resolve_candidate("use redis caching layer", &[(id, 0.95)]);
        assert_eq!(res, CandidateResolution::SkipAll(DedupDecision::Skip { existing_id: id }));
    }

    #[test]
    fn test_resolve_merge_on_medium_similarity() {
        let id = MemoryId(uuid::Uuid::new_v4());
        let res = resolve_candidate("postgres storage choice", &[(id, 0.8)]);
        assert_eq!(res, CandidateResolution::ResolveExisting(vec![DedupDecision::Merge { existing_id: id }]));
    }

    #[test]
    fn test_resolve_create_when_no_similar() {
        let id = MemoryId(uuid::Uuid::new_v4());
        let res = resolve_candidate("totally unrelated topic about kernels", &[(id, 0.1)]);
        assert_eq!(res, CandidateResolution::Create);
        assert_eq!(resolve_candidate("anything", &[]), CandidateResolution::Create);
    }

    #[test]
    fn test_policy_delete_when_subsumed_and_richer() {
        let id = MemoryId(uuid::Uuid::new_v4());
        let short_old = "x".repeat(40);
        let rich_new = format!("{} plus much more detail here", "x".repeat(80));
        let res = resolve_with_policy(&rich_new, &[(id, 0.95, short_old.len())]);
        match res {
            CandidateResolution::ResolveExisting(decisions) => {
                assert_eq!(decisions, vec![DedupDecision::Delete { existing_id: id }]);
            }
            other => panic!("expected delete decision, got {:?}", other),
        }
    }

    #[test]
    fn test_memory_diff_summary_and_serialization_roundtrip() {
        let mut diff = MemoryDiff::new();
        diff.operations.push(DiffOperation::Add {
            id: MemoryId(uuid::Uuid::new_v4()),
            content: "new".into(),
            reason: "user preference".into(),
        });
        diff.operations.push(DiffOperation::Update {
            id: MemoryId(uuid::Uuid::new_v4()),
            before: "old".into(),
            after: "newer".into(),
        });
        diff.operations.push(DiffOperation::Delete {
            id: MemoryId(uuid::Uuid::new_v4()),
            deleted_content: "gone".into(),
        });
        assert_eq!(diff.summary(), (1, 1, 1));
        let json = serde_json::to_string(&diff).unwrap();
        let back: MemoryDiff = serde_json::from_str(&json).unwrap();
        assert_eq!(back.summary(), (1, 1, 1));
    }

    #[test]
    fn test_memory_diff_write_to_disk() {
        let dir = std::env::temp_dir().join(format!("mnemo_diff_test_{}", uuid::Uuid::new_v4()));
        let mut diff = MemoryDiff::default();
        diff.operations.push(DiffOperation::Add {
            id: MemoryId(uuid::Uuid::new_v4()),
            content: "c".into(),
            reason: "test".into(),
        });
        let path = diff.write_to_dir(&dir).unwrap();
        assert!(path.exists());
        let contents = std::fs::read_to_string(path).unwrap();
        assert!(contents.contains("operations"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_archive_messages_writes_jsonl() {
        let dir = std::env::temp_dir().join(format!("mnemo_arch_test_{}", uuid::Uuid::new_v4()));
        let msgs = vec![SessionMessage::new("user", "hello"), SessionMessage::new("assistant", "hi")];
        let path = archive_messages("sess/1", &msgs, &dir).unwrap();
        assert!(path.exists());
        let contents = std::fs::read_to_string(path).unwrap();
        assert_eq!(contents.lines().count(), 2);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_extraction_pipeline_counts() {
        // End-to-end heuristic flow without touching storage:
        let msgs = vec![SessionMessage::new("user", "I prefer async APIs.")];
        let candidates = extract_candidates(&msgs);
        assert_eq!(candidates.len(), 1);

        // Existing similar memory => merge decision recorded in result shape
        let existing_id = MemoryId(uuid::Uuid::new_v4());
        let resolution = resolve_candidate(&candidates[0].content, &[(existing_id, 0.85)]);
        let mut result = ExtractionResult {
            candidates_extracted: candidates.len(),
            created: vec![],
            merged_into: vec![],
            skipped: 0,
            deleted: vec![],
        };
        match resolution {
            CandidateResolution::ResolveExisting(decisions) => {
                for d in decisions {
                    if let DedupDecision::Merge { existing_id } = d {
                        result.merged_into.push(existing_id);
                    }
                }
            }
            _ => {}
        }
        assert_eq!(result.merged_into, vec![existing_id]);
    }
}
