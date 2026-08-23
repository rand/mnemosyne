//! Hierarchical memory organization (inspired by OpenViking's context database design)
//!
//! This module implements a topic-tree over memories with tiered summaries:
//!
//! - **L0 (Abstract)**: one-sentence summary (<=256 chars) used for quick relevance checks
//! - **L1 (Overview)**: structured overview (<=4000 chars) for navigation and planning
//! - **L2 (Detail)**: full memory content, loaded on demand
//!
//! Memories are organized into directories derived deterministically from their
//! namespace, type, and primary tag (e.g. `project:myapp/decisions/caching`).
//! Each directory carries its own L0/L1 sidecars aggregated bottom-up from its
//! children, plus freshness metadata (`total_entries`, `sampled_entries`,
//! `pending_child_changes`) so stale summaries are detectable.
//!
//! The [`HierarchicalRetriever`] implements directory-recursive retrieval with
//! score propagation (`final = alpha * child + (1 - alpha) * parent`), a priority
//! queue over directories, convergence detection, and a replayable retrieval
//! trajectory for observability.

use crate::types::{MemoryNote, MemoryType, Namespace};
use serde::{Deserialize, Serialize};
use std::collections::{BinaryHeap, BTreeMap, HashMap};

/// Maximum characters for an L0 abstract body
pub const ABSTRACT_MAX_CHARS: usize = 256;
/// Maximum characters for an L1 overview body
pub const OVERVIEW_MAX_CHARS: usize = 4000;
/// Default number of direct children sampled when generating a directory overview
pub const OVERVIEW_SAMPLE_LIMIT: usize = 32;
/// Default weight of a child's own score in propagation (1.0 = ignore parent score)
pub const DEFAULT_SCORE_PROPAGATION_ALPHA: f32 = 1.0;
/// Stop recursing after this many rounds with unchanged top-k
pub const MAX_CONVERGENCE_ROUNDS: usize = 3;
/// Number of global-search candidate directories to start from
pub const GLOBAL_SEARCH_TOPK: usize = 10;
/// A directory must beat its best child by this ratio to win as a result
pub const DIRECTORY_DOMINANCE_RATIO: f32 = 1.2;

// ---------------------------------------------------------------------------
// Topic paths
// ---------------------------------------------------------------------------

/// Canonical directory segment for each memory type
pub fn type_segment(memory_type: &MemoryType) -> &'static str {
    match memory_type {
        MemoryType::ArchitectureDecision => "decisions",
        MemoryType::CodePattern => "patterns",
        MemoryType::BugFix => "bugfixes",
        MemoryType::Configuration => "config",
        MemoryType::Constraint => "constraints",
        MemoryType::Entity => "entities",
        MemoryType::Insight => "insights",
        MemoryType::Reference => "references",
        MemoryType::Preference => "preferences",
        MemoryType::Task => "tasks",
        MemoryType::AgentEvent => "events",
        MemoryType::Constitution => "constitution",
        MemoryType::FeatureSpec => "specs",
        MemoryType::ImplementationPlan => "plans",
        MemoryType::TaskBreakdown => "breakdowns",
        MemoryType::QualityChecklist => "checklists",
        MemoryType::Clarification => "clarifications",
    }
}

/// Derive a deterministic topic path for a memory.
///
/// Format: `<namespace>/<type-segment>[/<primary-tag>]`
/// Example: `project:mnemosyne/decisions/caching`
///
/// Tags are slugified (lowercase, non-alphanumeric collapsed to `-`) and the
/// first tag alphabetically is used so the path is stable regardless of tag order.
pub fn topic_path_for(note: &MemoryNote) -> String {
    let ns = note.namespace.to_string();
    let mut path = format!("{}/{}", ns, type_segment(&note.memory_type));
    if let Some(primary) = note.tags.iter().min() {
        let slug = slugify(primary);
        if !slug.is_empty() {
            path.push('/');
            path.push_str(&slug);
        }
    }
    path
}

/// Slugify a string for use as a path segment
pub fn slugify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_dash = true; // suppress leading dashes
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}


/// Truncate a string to at most `max` characters **including** any ellipsis
/// suffix appended by `truncate_at_char_boundary`.
fn truncate_total(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    crate::utils::string::truncate_at_char_boundary(s, max.saturating_sub(3))
}

// ---------------------------------------------------------------------------
// Tiered content (L0/L1)
// ---------------------------------------------------------------------------

/// Generate the L0 abstract for a single memory (quick relevance check).
///
/// Uses the existing summary when available, otherwise the first sentences of
/// the content. Truncated to [`ABSTRACT_MAX_CHARS`] at a char boundary.
pub fn l0_abstract_for(note: &MemoryNote) -> String {
    let base = if note.summary.trim().is_empty() {
        first_sentences(&note.content, 2)
    } else {
        note.summary.clone()
    };
    truncate_total(base.trim(), ABSTRACT_MAX_CHARS)
}

/// Generate the L1 overview for a single memory (navigation + planning).
///
/// Structured Markdown: one-line brief, context, keywords, tags, relations.
pub fn l1_overview_for(note: &MemoryNote) -> String {
    let mut md = String::new();
    md.push_str(&format!(
        "# {}\n\n",
        truncate_total(&first_sentences(&note.content, 1), 120)
    ));
    md.push_str(&format!("**Type**: {:?}\n", note.memory_type));
    md.push_str(&format!("**Importance**: {}\n", note.importance));
    if !note.context.trim().is_empty() {
        md.push_str(&format!("\n## Context\n{}\n", note.context.trim()));
    }
    if !note.keywords.is_empty() {
        md.push_str("\n## Keywords\n");
        md.push_str(&note.keywords.join(", "));
        md.push('\n');
    }
    if !note.tags.is_empty() {
        md.push_str("\n## Tags\n");
        md.push_str(&note.tags.join(", "));
        md.push('\n');
    }
    if !note.links.is_empty() {
        md.push_str(&format!(
            "\n## Relations\n{} linked memories\n",
            note.links.len()
        ));
    }
    truncate_total(md.trim(), OVERVIEW_MAX_CHARS)
}

/// Extract up to `n` sentences (very small heuristic splitter).
fn first_sentences(text: &str, n: usize) -> String {
    let mut out = String::new();
    let mut count = 0usize;
    for sentence in text.split(|c| c == '.' || c == '\n' || c == '?' || c == '!') {
        let s = sentence.trim();
        if s.is_empty() {
            continue;
        }
        if count > 0 {
            out.push_str(". ");
        }
        out.push_str(s);
        count += 1;
        if count >= n {
            break;
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Freshness metadata + stable sampling
// ---------------------------------------------------------------------------

/// Coverage/freshness metadata for a generated directory summary.
///
/// Mirrors OpenViking's sidecar freshness fields: counts cover *direct*
/// children only, `pending_child_changes > 0` means the body is readable but
/// known to lag behind lower-level changes.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Freshness {
    /// Total direct entries contributing to directory semantics
    pub total_entries: usize,
    /// Direct entries sampled for this summary
    pub sampled_entries: usize,
    /// Direct entries not sampled (sampled + unsampled == total)
    pub unsampled_entries: usize,
    /// Known changed direct entries not yet reflected in the current body
    pub pending_child_changes: usize,
}

impl Freshness {
    fn compute(total_children: usize, sampled: usize, pending: usize) -> Self {
        let sampled = sampled.min(total_children);
        Self {
            total_entries: total_children,
            sampled_entries: sampled,
            unsampled_entries: total_children - sampled,
            pending_child_changes: pending,
        }
    }

    /// Whether this summary covers all direct children
    pub fn is_complete(&self) -> bool {
        self.unsampled_entries == 0 && self.pending_child_changes == 0
    }
}

/// Deterministic, order-preserving sampling of up to `limit` items.
///
/// When items exceed the limit, picks evenly spaced indices so repeated calls
/// on an unchanged collection choose the same sample (no noisy rewrites).
pub fn stable_sample<T>(items: &[T], limit: usize) -> Vec<&T> {
    if items.len() <= limit || limit == 0 {
        return items.iter().collect();
    }
    let step = items.len() as f64 / limit as f64;
    (0..limit)
        .map(|i| &items[((i as f64 * step).floor() as usize).min(items.len() - 1)])
        .collect()
}

// ---------------------------------------------------------------------------
// Directory tree
// ---------------------------------------------------------------------------

/// A node in the topic tree: either a leaf memory or a directory with sidecars.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeNode {
    /// Full path (e.g. `project:x/decisions/caching`)
    pub path: String,
    /// Path of the parent directory (empty for roots)
    pub parent: String,
    /// Final path segment
    pub name: String,
    /// True for individual memories, false for aggregate directories
    pub is_leaf: bool,
    /// L0 abstract body
    pub abstract_text: String,
    /// L1 overview body (directories only; leaves carry their own overview)
    pub overview_text: String,
    /// Freshness metadata (directories only)
    pub freshness: Option<Freshness>,
}

/// Build the topic tree (list of all nodes, parents before children) from notes.
///
/// Each unique topic path becomes a leaf; every ancestor prefix becomes a
/// directory whose L0/L1 are aggregated bottom-up from child L0 bodies using
/// stable sampling, exactly like OpenViking's SemanticProcessor flow:
/// `file summaries -> leaf L1 -> leaf L0 -> parent directories`.
pub fn build_tree(notes: &[&MemoryNote]) -> Vec<TreeNode> {
    // Group notes by topic path
    let mut by_path: BTreeMap<String, Vec<&MemoryNote>> = BTreeMap::new();
    for note in notes {
        by_path
            .entry(topic_path_for(note))
            .or_default()
            .push(note);
    }

    // Collect all ancestor directories
    let mut dirs: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for path in by_path.keys() {
        let mut acc = String::new();
        for seg in path.split('/') {
            if !acc.is_empty() {
                acc.push('/');
            }
            acc.push_str(seg);
            dirs.insert(acc.clone());
        }
    }

    // Build leaf nodes
    let mut nodes: Vec<TreeNode> = Vec::new();
    let mut dir_children: HashMap<String, Vec<(bool, String)>> = HashMap::new(); // dir -> [(is_leaf, l0)]
    for (path, group) in &by_path {
        let note = group[0];
        let (parent, name) = split_parent(path);
        dir_children.entry(parent.clone()).or_default().push((
            true,
            l0_abstract_for(note),
        ));
        nodes.push(TreeNode {
            path: path.clone(),
            parent,
            name,
            is_leaf: true,
            abstract_text: l0_abstract_for(note),
            overview_text: l1_overview_for(note),
            freshness: None,
        });
    }

    // Build directory nodes deepest-first so parent aggregation sees child L0s
    let sorted_dirs: Vec<&String> = dirs.iter().collect();
    let mut dir_l0: HashMap<String, String> = HashMap::new();
    for dir in sorted_dirs.into_iter().rev() {
        let dir = dir.clone();
        let (parent, name) = split_parent(&dir);
        // Children L0s: leaf notes under this exact path + subdirectory L0s
        let mut child_l0s: Vec<String> = Vec::new();
        if let Some(leaf_l0s) = dir_children.get(&dir) {
            child_l0s.extend(leaf_l0s.iter().map(|(_, l0)| l0.clone()));
        } else {
            // pure pass-through directory (only subdirs)
        }
        for (sub_path, sub_l0) in &dir_l0 {
            if parent_of(sub_path).as_deref() == Some(dir.as_str()) {
                child_l0s.push(sub_l0.clone());
            }
        }
        let total = child_l0s.len();
        let sampled = stable_sample(&child_l0s, OVERVIEW_SAMPLE_LIMIT);
        let sampled_strs: Vec<&str> = sampled.iter().map(|s| s.as_str()).collect();
        let l1 = directory_overview(&dir, &sampled_strs);
        let l0 = directory_abstract(&l1);
        // Track changed-but-unsampled children via pending count
        let freshness = Freshness::compute(total, sampled.len(), 0);
        dir_l0.insert(dir.clone(), l0.clone());
        dir_children.entry(parent.clone()).or_default().push((false, l0.clone()));
        nodes.push(TreeNode {
            path: dir,
            parent,
            name,
            is_leaf: false,
            abstract_text: l0,
            overview_text: l1,
            freshness: Some(freshness),
        });
    }

    // Order: directories breadth-first-ish by depth then leaves — simple sort by depth then path
    nodes.sort_by(|a, b| {
        a.path.split('/').count()
            .cmp(&b.path.split('/').count())
            .then_with(|| a.path.cmp(&b.path))
    });
    nodes
}

fn split_parent(path: &str) -> (String, String) {
    match path.rfind('/') {
        Some(i) => (path[..i].to_string(), path[i + 1..].to_string()),
        None => (String::new(), path.to_string()),
    }
}

fn parent_of(path: &str) -> Option<String> {
    path.rfind('/').map(|i| path[..i].to_string())
}

/// Aggregate sampled child L0 bodies into a directory L1 overview.
fn directory_overview(dir_path: &str, sampled_l0s: &[&str]) -> String {
    let mut md = format!("# {}\n\n", dir_path);
    md.push_str(&format!(
        "Directory covering {} entr{}.\n",
        sampled_l0s.len(),
        if sampled_l0s.len() == 1 { "y" } else { "ies" }
    ));
    md.push_str("\n## Quick Navigation\n");
    for l0 in sampled_l0s {
        md.push_str(&format!("- {}\n", l0));
    }
    truncate_total(md.trim(), OVERVIEW_MAX_CHARS)
}

/// Extract L0 from a directory L1 body: text between the H1 title and the
/// first `##` heading (the brief description paragraph), truncated to L0 size.
fn directory_abstract(l1_body: &str) -> String {
    let after_title = l1_body
        .strip_prefix("# ")
        .and_then(|rest| rest.find('\n').map(|i| &rest[i + 1..]))
        .unwrap_or(l1_body);
    let brief = after_title
        .split("##")
        .next()
        .unwrap_or(after_title)
        .trim();
    truncate_total(brief, ABSTRACT_MAX_CHARS)
}

// ---------------------------------------------------------------------------
// Hierarchical retriever
// ---------------------------------------------------------------------------

/// Configuration for the hierarchical retriever
#[derive(Debug, Clone)]
pub struct RetrieverConfig {
    /// Weight of a node's own score in propagation (1.0 = only own score)
    pub score_propagation_alpha: f32,
    /// Global search candidate directories
    pub global_search_topk: usize,
    /// Convergence rounds with unchanged top-k before stopping
    pub max_convergence_rounds: usize,
    /// Minimum final score to include a node as a result
    pub score_threshold: f32,
}

impl Default for RetrieverConfig {
    fn default() -> Self {
        Self {
            score_propagation_alpha: DEFAULT_SCORE_PROPAGATION_ALPHA,
            global_search_topk: GLOBAL_SEARCH_TOPK,
            max_convergence_rounds: MAX_CONVERGENCE_ROUNDS,
            score_threshold: 0.05,
        }
    }
}

/// One step in a retrieval trajectory (for observability/debugging)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryStep {
    pub uri: String,
    pub parent_score: f32,
    pub own_score: f32,
    pub final_score: f32,
    pub action: &'static str, // "global_search" | "recurse" | "collect"
    pub round: usize,
}

/// Observable trace of a full hierarchical retrieval
#[derive(Debug, Clone, Default, Serialize)]
pub struct RetrievalTrajectory {
    pub steps: Vec<TrajectoryStep>,
    pub rounds_executed: usize,
    pub converged: bool,
}

impl RetrievalTrajectory {
    /// Serialize to pretty JSON for `--trace` output
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string())
    }
}

/// A scored match returned by the retriever
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchedNode {
    pub uri: String,
    pub is_leaf: bool,
    pub abstract_text: String,
    pub score: f32,
}

/// Internal priority-queue entry (min-heap behavior via Reverse ordering on
/// score so we always pop the *lowest*? No — we want highest first).
#[derive(PartialEq, Debug)]
struct QueueEntry {
    score: f32,
    path: String,
}

impl Eq for QueueEntry {}

impl Ord for QueueEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // BinaryHeap is a max-heap; compare by score, tie-break by path for determinism
        self.score
            .partial_cmp(&other.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| other.path.cmp(&self.path))
    }
}

impl PartialOrd for QueueEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Directory-recursive retriever over a built tree.
///
/// Algorithm (mirrors OpenViking's HierarchicalRetriever):
/// 1. Normalize per-node scores into [0, 1].
/// 2. Global vector-style search locates the top-k starting directories.
/// 3. Priority-queue recursion: pop best directory, score children, blend
///    `final = alpha*own + (1-alpha)*parent`, push directories back.
/// 4. Stop early when top-k results are unchanged for `max_convergence_rounds`.
#[derive(Debug, Clone)]
pub struct HierarchicalRetriever {
    config: RetrieverConfig,
}

impl HierarchicalRetriever {
    pub fn new(config: RetrieverConfig) -> Self {
        Self { config }
    }

    /// Run hierarchical retrieval.
    ///
    /// `node_scores` maps node paths to raw scores from the underlying scorer
    /// (any monotonic scale — they are normalized internally).
    pub fn retrieve(
        &self,
        tree: &[TreeNode],
        node_scores: &HashMap<String, f32>,
    ) -> (Vec<MatchedNode>, RetrievalTrajectory) {
        let mut trajectory = RetrievalTrajectory::default();

        // Normalize scores into [0,1]
        let max_score = node_scores.values().cloned().fold(0.0f32, f32::max);
        if max_score <= 0.0 {
            return (Vec::new(), trajectory);
        }
        let normalized: HashMap<String, f32> = node_scores
            .iter()
            .map(|(k, v)| (k.clone(), v / max_score))
            .collect();

        // Index nodes
        let by_path: HashMap<&str, &TreeNode> = tree
            .iter()
            .map(|n| (n.path.as_str(), n))
            .collect();
        let mut children_of: HashMap<&str, Vec<&TreeNode>> = HashMap::new();
        for node in tree {
            if !node.parent.is_empty() {
                children_of.entry(node.parent.as_str()).or_default().push(node);
            }
        }

        // Step 1-3: global search locates the top-k starting directories
        // (roots ranked by their best descendant score). This handles both
        // deep trees and degenerate leaf-only trees uniformly.
        let roots: Vec<&TreeNode> = tree
            .iter()
            .filter(|n| n.parent.is_empty() || !by_path.contains_key(n.parent.as_str()))
            .collect();
        let mut queue: BinaryHeap<QueueEntry> = BinaryHeap::new();
        let mut collected: Vec<MatchedNode> = Vec::new();

        let mut scored_roots: Vec<(f32, &TreeNode)> = roots
            .into_iter()
            .map(|d| (best_descendant_score(d, &normalized, &children_of), d))
            .collect();
        scored_roots.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

        for (score, dir) in scored_roots.into_iter().take(self.config.global_search_topk) {
            trajectory.steps.push(TrajectoryStep {
                uri: dir.path.clone(),
                parent_score: 0.0,
                own_score: score,
                final_score: score,
                action: "global_search",
                round: 0,
            });
            if score > 0.0 {
                queue.push(QueueEntry {
                    score,
                    path: dir.path.clone(),
                });
            }
        }

        // Step 4: recursive search with convergence detection
        let alpha = self.config.score_propagation_alpha.clamp(0.0, 1.0);
        let mut round = 0usize;
        let mut unchanged_rounds = 0usize;
        let mut last_topk: Vec<String> = Vec::new();

        while let Some(entry) = queue.pop() {
            round += 1;
            let node = match by_path.get(entry.path.as_str()) {
                Some(n) => *n,
                None => continue,
            };

            let children = children_of.get(node.path.as_str()).cloned().unwrap_or_default();
            if children.is_empty() {
                // Leaf reached: collect
                if entry.score >= self.config.score_threshold {
                    collected.push(MatchedNode {
                        uri: node.path.clone(),
                        is_leaf: true,
                        abstract_text: node.abstract_text.clone(),
                        score: entry.score,
                    });
                    trajectory.steps.push(TrajectoryStep {
                        uri: node.path.clone(),
                        parent_score: entry.score,
                        own_score: normalized.get(node.path.as_str()).copied().unwrap_or(0.0),
                        final_score: entry.score,
                        action: "collect",
                        round,
                    });
                }
            } else {
                for child in &children {
                    let own = normalized.get(child.path.as_str()).copied().unwrap_or(0.0);
                    let propagated = alpha * own + (1.0 - alpha) * entry.score;
                    trajectory.steps.push(TrajectoryStep {
                        uri: child.path.clone(),
                        parent_score: entry.score,
                        own_score: own,
                        final_score: propagated,
                        action: "recurse",
                        round,
                    });
                    if propagated < self.config.score_threshold && child.is_leaf {
                        continue;
                    }
                    if child.is_leaf {
                        if propagated >= self.config.score_threshold {
                            collected.push(MatchedNode {
                                uri: child.path.clone(),
                                is_leaf: true,
                                abstract_text: child.abstract_text.clone(),
                                score: propagated,
                            });
                            trajectory.steps.push(TrajectoryStep {
                                uri: child.path.clone(),
                                parent_score: entry.score,
                                own_score: own,
                                final_score: propagated,
                                action: "collect",
                                round,
                            });
                        }
                    } else {
                        queue.push(QueueEntry {
                            score: propagated,
                            path: child.path.clone(),
                        });
                    }
                }
                // Directory dominance: if this directory dominates its best
                // child, collect it as a contextual result too.
                let best_child = children
                    .iter()
                    .map(|c| {
                        alpha * normalized.get(c.path.as_str()).copied().unwrap_or(0.0)
                            + (1.0 - alpha) * entry.score
                    })
                    .fold(0.0f32, f32::max);
                if entry.score > best_child * DIRECTORY_DOMINANCE_RATIO
                    && entry.score >= self.config.score_threshold
                {
                    collected.push(MatchedNode {
                        uri: node.path.clone(),
                        is_leaf: false,
                        abstract_text: node.abstract_text.clone(),
                        score: entry.score,
                    });
                }
            }

            // Convergence check every round on the current collected top-k
            collected.sort_by(|a, b| {
                b.score
                    .partial_cmp(&a.score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let current_topk: Vec<String> =
                collected.iter().take(self.config.global_search_topk).map(|m| m.uri.clone()).collect();
            if current_topk == last_topk {
                unchanged_rounds += 1;
                if unchanged_rounds >= self.config.max_convergence_rounds {
                    break;
                }
            } else {
                unchanged_rounds = 0;
                last_topk = current_topk;
            }
        }

        collected.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        trajectory.rounds_executed = round;
        trajectory.converged = unchanged_rounds >= self.config.max_convergence_rounds;
        (collected, trajectory)
    }
}

/// Best normalized score among a directory's descendants (including itself).
fn best_descendant_score(
    dir: &TreeNode,
    normalized: &HashMap<String, f32>,
    children_of: &HashMap<&str, Vec<&TreeNode>>,
) -> f32 {
    let mut best = normalized.get(dir.path.as_str()).copied().unwrap_or(0.0);
    let mut stack = vec![dir];
    let mut visited: std::collections::HashSet<&str> = std::collections::HashSet::new();
    while let Some(n) = stack.pop() {
        if !visited.insert(n.path.as_str()) {
            continue;
        }
        if let Some(children) = children_of.get(n.path.as_str()) {
            for c in children {
                best = best.max(normalized.get(c.path.as_str()).copied().unwrap_or(0.0));
                stack.push(c);
            }
        }
    }
    best
}

// ---------------------------------------------------------------------------
// Integration helpers (bridge between storage results and the retriever)
// ---------------------------------------------------------------------------

/// Rerank hybrid-search results through the topic-tree hierarchy.
///
/// Takes notes and their raw retrieval scores (any monotonic scale), builds
/// the topic tree, optionally blends hotness, runs [`HierarchicalRetriever`],
/// and maps matched paths back to original indices.
///
/// Returns `(matches, trajectory)` where each match is
/// `(original_index, propagated_score)` sorted best-first.
pub fn rerank_results(
    notes: &[&MemoryNote],
    scores: &[f32],
    config: RetrieverConfig,
    blend_hotness: bool,
) -> (Vec<(usize, f32)>, RetrievalTrajectory) {
    assert_eq!(notes.len(), scores.len(), "notes/scores length mismatch");

    // Aggregate max raw score per topic path (multiple notes can share one)
    let mut path_scores: HashMap<String, f32> = HashMap::new();
    let mut path_indices: HashMap<String, Vec<usize>> = HashMap::new();
    for (i, note) in notes.iter().enumerate() {
        let path = topic_path_for(note);
        let entry = path_scores.entry(path.clone()).or_insert(0.0);
        *entry = entry.max(scores[i]);
        path_indices.entry(path).or_default().push(i);
    }

    // Blend hotness into raw scores before normalization:
    // 80% retrieval relevance + up to 20% hotness of the best note on the path.
    let now = chrono::Utc::now();
    let mut blended: HashMap<String, f32> = path_scores.clone();
    if blend_hotness {
        for (i, note) in notes.iter().enumerate() {
            let path = topic_path_for(note);
            let hot = crate::utils::hotness::hotness_score(
                note.access_count,
                Some(note.last_accessed_at),
                now,
                7.0,
            ) as f32;
            if let Some(s) = blended.get_mut(&path) {
                let contribution = hot * scores[i].max(1e-6) * 0.2;
                *s += contribution;
            }
        }
    }

    let tree = build_tree(notes);
    let retriever = HierarchicalRetriever::new(config);
    let (matched, trajectory) = retriever.retrieve(&tree, &blended);

    // Map matched paths back to original result indices (best original first)
    let mut out: Vec<(usize, f32)> = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for m in matched {
        if let Some(idxs) = path_indices.get(&m.uri) {
            let mut order: Vec<usize> = idxs.clone();
            order.sort_by(|a, b| {
                scores[*b]
                    .partial_cmp(&scores[*a])
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            for idx in order {
                if seen.insert(idx) {
                    out.push((idx, m.score));
                }
            }
        }
    }
    // Append any notes whose paths never surfaced (kept, tail-ranked)
    for i in 0..notes.len() {
        if seen.insert(i) {
            out.push((i, scores[i] * 0.5));
        }
    }
    (out, trajectory)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MemoryNote, MemoryId};
    use chrono::Utc;
    use uuid::Uuid;

    fn note(ns: Namespace, mtype: MemoryType, tags: &[&str], content: &str) -> MemoryNote {
        MemoryNote {
            id: MemoryId(Uuid::new_v4()),
            namespace: ns,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            content: content.to_string(),
            summary: format!("Summary of {}", content),
            keywords: vec![],
            tags: tags.iter().map(|s| s.to_string()).collect(),
            context: String::new(),
            memory_type: mtype,
            importance: 7,
            confidence: 1.0,
            links: vec![],
            related_files: vec![],
            related_entities: vec![],
            access_count: 0,
            last_accessed_at: Utc::now(),
            expires_at: None,
            is_archived: false,
            superseded_by: None,
            embedding: None,
            embedding_model: "test".to_string(),
        }
    }

    #[test]
    fn test_topic_path_deterministic() {
        let n1 = note(
            Namespace::Project { name: "app".into() },
            MemoryType::ArchitectureDecision,
            &["Caching Layer", "redis"],
            "content",
        );
        let n2 = note(
            Namespace::Project { name: "app".into() },
            MemoryType::ArchitectureDecision,
            &["redis", "Caching Layer"],
            "content",
        );
        assert_eq!(topic_path_for(&n1), topic_path_for(&n2));
        assert_eq!(
            topic_path_for(&n1),
            "project:app/decisions/caching-layer"
        );
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World!"), "hello-world");
        assert_eq!(slugify("  --weird___name--  "), "weird-name");
        assert_eq!(slugify(""), "");
    }

    #[test]
    fn test_l0_truncated_and_prefers_summary() {
        let mut n = note(Namespace::Global, MemoryType::Insight, &[], "body");
        n.summary = "Short.".to_string();
        assert_eq!(l0_abstract_for(&n), "Short.");
        n.summary = "x".repeat(500);
        assert!(l0_abstract_for(&n).chars().count() <= ABSTRACT_MAX_CHARS);
    }

    #[test]
    fn test_stable_sampling_is_deterministic_and_ordered() {
        let items: Vec<usize> = (0..100).collect();
        let s1 = stable_sample(&items, 10);
        let s2 = stable_sample(&items, 10);
        assert_eq!(s1, s2);
        assert_eq!(s1.len(), 10);
        // order preserving
        let vals: Vec<usize> = s1.into_iter().copied().collect();
        let mut sorted = vals.clone();
        sorted.sort();
        assert_eq!(vals, sorted);
        // under limit returns everything
        let small: Vec<usize> = vec![1, 2, 3];
        assert_eq!(stable_sample(&small, 10).len(), 3);
    }

    #[test]
    fn test_freshness_compute() {
        let f = Freshness::compute(50, 32, 0);
        assert_eq!(f.total_entries, 50);
        assert_eq!(f.sampled_entries, 32);
        assert_eq!(f.unsampled_entries, 18);
        assert!(!f.is_complete());
        assert!(Freshness::compute(5, 5, 0).is_complete());
    }

    #[test]
    fn test_build_tree_creates_directories_and_sidecars() {
        let ns = Namespace::Project { name: "app".into() };
        let notes = vec![
            note(ns.clone(), MemoryType::ArchitectureDecision, &["caching"], "Use Redis for caching"),
            note(ns.clone(), MemoryType::ArchitectureDecision, &["auth"], "Use JWT tokens"),
            note(ns.clone(), MemoryType::CodePattern, &["errors"], "Error handling pattern"),
        ];
        let refs: Vec<&MemoryNote> = notes.iter().collect();
        let tree = build_tree(&refs);

        // Roots exist: project:app, project:app/decisions, project:app/patterns
        assert!(tree.iter().any(|n| n.path == "project:app" && !n.is_leaf));
        assert!(tree.iter().any(|n| n.path == "project:app/decisions" && !n.is_leaf));

        // Directory abstract extracted from L1 brief paragraph
        let root = tree.iter().find(|n| n.path == "project:app").unwrap();
        assert!(!root.abstract_text.is_empty());
        assert!(root.abstract_text.chars().count() <= ABSTRACT_MAX_CHARS);

        // Leaves present
        assert!(tree.iter().any(|n| n.is_leaf && n.path.contains("decisions/caching")));
    }

    #[test]
    fn test_directory_abstract_extraction() {
        let l1 = "# project:x\n\nBrief line here.\n\n## Quick Navigation\n- a\n";
        assert_eq!(directory_abstract(l1), "Brief line here.");
    }

    fn test_tree() -> Vec<TreeNode> {
        // project:app
        //   decisions/
        //     caching   (score 1.0)
        //     auth      (score 0.6)
        //   patterns/
        //     errors    (score 0.05)
        let mk = |path: &str, parent: &str, is_leaf: bool| TreeNode {
            path: path.into(),
            parent: parent.into(),
            name: path.rsplit('/').next().unwrap().into(),
            is_leaf,
            abstract_text: "abs".into(),
            overview_text: "ov".into(),
            freshness: None,
        };
        vec![
            mk("project:app", "", false),
            mk("project:app/decisions", "project:app", false),
            mk("project:app/patterns", "project:app", false),
            mk("project:app/decisions/caching", "project:app/decisions", true),
            mk("project:app/decisions/auth", "project:app/decisions", true),
            mk("project:app/patterns/errors", "project:app/patterns", true),
        ]
    }

    fn scores() -> HashMap<String, f32> {
        HashMap::from([
            ("project:app".to_string(), 0.8),
            ("project:app/decisions".to_string(), 0.9),
            ("project:app/patterns".to_string(), 0.05),
            ("project:app/decisions/caching".to_string(), 1.0),
            ("project:app/decisions/auth".to_string(), 0.6),
            ("project:app/patterns/errors".to_string(), 0.05),
        ])
    }

    #[test]
    fn test_hierarchical_retriever_ranks_and_collects() {
        let r = HierarchicalRetriever::new(RetrieverConfig::default());
        let (results, traj) = r.retrieve(&test_tree(), &scores());
        assert!(!results.is_empty());
        // Top hit is the strongest leaf
        assert_eq!(results[0].uri, "project:app/decisions/caching");
        // Results sorted descending
        assert!(results.windows(2).all(|w| w[0].score >= w[1].score));
        // Trajectory recorded global search + recursion
        assert!(traj.steps.iter().any(|s| s.action == "global_search"));
        assert!(traj.steps.iter().any(|s| s.action == "recurse"));
        assert!(traj.rounds_executed > 0);
    }

    #[test]
    fn test_score_propagation_alpha_zero_blends_parent() {
        let cfg = RetrieverConfig {
            score_propagation_alpha: 0.0,
            ..Default::default()
        };
        let r = HierarchicalRetriever::new(cfg);
        let (results, _) = r.retrieve(&test_tree(), &scores());
        // With alpha=0 every leaf inherits its parent's score; both decisions
        // leaves get the same propagated score (the dir's global score).
        let caching = results.iter().find(|m| m.uri.ends_with("caching")).unwrap();
        let auth = results.iter().find(|m| m.uri.ends_with("auth")).unwrap();
        assert!((caching.score - auth.score).abs() < 1e-5);
    }

    #[test]
    fn test_empty_scores_returns_empty() {
        let r = HierarchicalRetriever::new(RetrieverConfig::default());
        let (results, traj) = r.retrieve(&test_tree(), &HashMap::new());
        assert!(results.is_empty());
        assert!(traj.steps.is_empty());
    }

    #[test]
    fn test_trajectory_serializes() {
        let r = HierarchicalRetriever::new(RetrieverConfig::default());
        let (_, traj) = r.retrieve(&test_tree(), &scores());
        let json = traj.to_json();
        assert!(json.contains("global_search") || json.contains("recurse"));
    }

    #[test]
    fn test_rerank_results_maps_back_and_preserves_all() {
        let ns = Namespace::Project { name: "app".into() };
        let notes = vec![
            note(ns.clone(), MemoryType::ArchitectureDecision, &["caching"], "Redis cache decision"),
            note(ns.clone(), MemoryType::ArchitectureDecision, &["auth"], "JWT auth decision"),
            note(ns.clone(), MemoryType::CodePattern, &["errors"], "Error pattern"),
        ];
        let refs: Vec<&MemoryNote> = notes.iter().collect();
        let raw: Vec<f32> = vec![0.9, 0.5, 0.1];

        let (ranked, traj) = rerank_results(
            &refs,
            &raw,
            RetrieverConfig::default(),
            true,
        );

        // All original indices present exactly once
        let mut idxs: Vec<usize> = ranked.iter().map(|(i, _)| *i).collect();
        idxs.sort();
        assert_eq!(idxs, vec![0, 1, 2]);
        // Best note stays first
        assert_eq!(ranked[0].0, 0);
        // Trajectory produced
        assert!(!traj.steps.is_empty());
    }

    #[test]
    fn test_rerank_results_hotness_boosts_accessed() {
        use chrono::Duration;
        let ns = Namespace::Project { name: "app".into() };
        let mut hot = note(ns.clone(), MemoryType::Insight, &["x"], "hot insight");
        hot.access_count = 100;
        hot.last_accessed_at = Utc::now();
        let mut cold = note(ns.clone(), MemoryType::Insight, &["y"], "cold insight");
        cold.access_count = 1;
        cold.last_accessed_at = Utc::now() - Duration::days(60);

        // Equal raw scores; different paths so no aggregation interference.
        // (Different tags => different topic dirs under same type segment.)
        let refs = vec![&cold, &hot];
        let (ranked, _) = rerank_results(
            &refs,
            &[0.5, 0.5],
            RetrieverConfig::default(),
            true,
        );
        assert_eq!(ranked[0].0, 1, "accessed-hot memory should rank first");
    }
}
