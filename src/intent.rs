//! Query intent analysis and typed query planning (design borrowed from
//! OpenViking's IntentAnalyzer).
//!
//! Before retrieval, a lightweight planner rewrites the raw input into 0–5
//! [`TypedQuery`]s:
//!
//! - **0 queries**: chit-chat / greetings that don't need retrieval at all
//!   (cheap token win on session-start paths).
//! - **1+ queries**: complex inputs decompose into typed queries with style
//!   hints — verb-first for tasks/skills, noun phrases for references,
//!   "user's X" phrasing for preferences.
//!
//! This implementation is fully heuristic so it works offline with no LLM;
//! an LLM-backed planner can layer on top later using the same output type.

use serde::{Deserialize, Serialize};

/// What kind of context a query is after
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextType {
    /// User/project memories ("what do we know about X")
    Memory,
    /// Reference material ("docs/templates for X")
    Resource,
    /// How-to / procedural knowledge ("how to do X")
    Skill,
}

/// One planned, typed retrieval query
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypedQuery {
    /// Rewritten query text
    pub query: String,
    /// What kind of context this targets
    pub context_type: ContextType,
    /// Short description of the query purpose
    pub intent: String,
    /// Priority 1 (highest) – 5 (lowest)
    pub priority: u8,
}

/// Result of intent analysis over an input
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct QueryPlan {
    /// Empty means "no retrieval needed" (chit-chat etc.)
    pub queries: Vec<TypedQuery>,
    /// Why zero queries were produced (when applicable)
    pub skip_reason: Option<String>,
}

impl QueryPlan {
    /// Whether the caller should skip retrieval entirely
    pub fn should_skip_retrieval(&self) -> bool {
        self.queries.is_empty()
    }
}

/// Phrases that indicate conversational filler not worth a retrieval round-trip
const CHIT_CHAT_PATTERNS: &[&str] = &[
    "hi",
    "hey",
    "hello",
    "yo",
    "thanks",
    "thank you",
    "thx",
    "ok",
    "okay",
    "cool",
    "nice",
    "great",
    "got it",
    "sounds good",
    "bye",
    "goodbye",
    "good morning",
    "good evening",
    "how are you",
];

/// Verb-first openers that signal procedural / skill intent
const SKILL_VERBS: &[&str] = &[
    "how to",
    "how do i",
    "how can i",
    "create",
    "build",
    "implement",
    "extract",
    "generate",
    "write",
    "set up",
    "setup",
    "configure",
    "install",
    "deploy",
    "run",
    "debug",
    "fix",
    "refactor",
    "migrate",
    "test",
    "search",
    "find all",
    "list all",
];

/// Possessive/first-person markers that signal personal memory
const MEMORY_MARKERS: &[&str] = &[
    "my ",
    "our ",
    "i prefer",
    "i like",
    "i always",
    "i never",
    "we prefer",
    "we decided",
    "user's",
    "users'",
    "my own",
    "remember",
];

/// Noun-phrase markers that signal reference/resource lookup
const RESOURCE_MARKERS: &[&str] = &[
    "document",
    "doc for",
    "template",
    "spec",
    "reference",
    "example of",
    "architecture of",
    "api",
    "guide",
    "where is",
    "what is the",
];

fn is_chit_chat(input: &str) -> Option<String> {
    let normalized = input.trim().to_lowercase();
    let stripped = normalized.trim_end_matches(|c: char| "!?., ".contains(c));
    if stripped.len() <= 40 && CHIT_CHAT_PATTERNS.iter().any(|p| stripped == *p || stripped.starts_with(p)) {
        Some(stripped.to_string())
    } else {
        None
    }
}

fn classify(text_lower: &str) -> ContextType {
    if MEMORY_MARKERS.iter().any(|m| text_lower.contains(m)) {
        return ContextType::Memory;
    }
    if SKILL_VERBS.iter().any(|v| text_lower.starts_with(v)) {
        return ContextType::Skill;
    }
    if RESOURCE_MARKERS.iter().any(|r| text_lower.contains(r)) {
        return ContextType::Resource;
    }
    // Default heuristic: questions about "how" are skills; bare noun phrases
    // are resources; everything else leans memory since that's our core use.
    if text_lower.starts_with("how ") {
        ContextType::Skill
    } else {
        ContextType::Memory
    }
}

/// Rewrite a raw input into a typed query with style hints applied.
fn rewrite(raw: &str, context_type: ContextType) -> String {
    let trimmed = raw.trim();
    match context_type {
        ContextType::Skill => {
            // Verb-first imperative
            let lower = trimmed.to_lowercase();
            if let Some(rest) = lower.strip_prefix("how do i ") {
                capitalize(rest)
            } else if let Some(rest) = lower.strip_prefix("how can i ") {
                capitalize(rest)
            } else if let Some(rest) = lower.strip_prefix("how to ") {
                capitalize(rest)
            } else {
                trimmed.to_string()
            }
        }
        ContextType::Memory => {
            // "User's X" phrasing
            format!("User preferences and context: {}", trimmed)
        }
        ContextType::Resource => trimmed.to_string(),
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Plan typed queries from a raw user/agent input.
///
/// Returns a plan with zero queries when the input is chit-chat.
pub fn plan_queries(input: &str) -> QueryPlan {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return QueryPlan {
            queries: Vec::new(),
            skip_reason: Some("empty input".to_string()),
        };
    }

    if let Some(matched) = is_chit_chat(trimmed) {
        return QueryPlan {
            queries: Vec::new(),
            skip_reason: Some(format!("chit-chat: {:?}", matched)),
        };
    }

    let mut queries = Vec::new();
    let lower = trimmed.to_lowercase();

    // Primary query
    let primary_type = classify(&lower);
    queries.push(TypedQuery {
        query: rewrite(trimmed, primary_type),
        context_type: primary_type,
        intent: describe_intent(primary_type).to_string(),
        priority: 1,
    });

    // Secondary queries for compound asks ("and also", "then") — cap at 5 total
    for part in split_compound(trimmed) {
        if queries.len() >= 5 {
            break;
        }
        let part_lower = part.to_lowercase();
        if part_lower.len() < 4 {
            continue;
        }
        let t = classify(&part_lower);
        if t != primary_type || normalize_whitespace(&part).to_lowercase()
            != normalize_whitespace(trimmed).to_lowercase()
        {
            queries.push(TypedQuery {
                query: rewrite(&part, t),
                context_type: t,
                intent: describe_intent(t).to_string(),
                priority: 2,
            });
        }
    }

    QueryPlan {
        queries,
        skip_reason: None,
    }
}

fn describe_intent(t: ContextType) -> &'static str {
    match t {
        ContextType::Memory => "recall stored project/user knowledge",
        ContextType::Resource => "locate reference material or docs",
        ContextType::Skill => "find procedural how-to knowledge",
    }
}

/// Split on compound connectors without breaking quoted text too badly.
fn split_compound(input: &str) -> Vec<String> {
    let mut parts = vec![input.to_string()];
    for sep in [" and also ", " then ", "; ", " plus "] {
        parts = parts
            .iter()
            .flat_map(|p| p.split(sep).map(|s| s.to_string()).collect::<Vec<_>>())
            .collect();
    }
    parts
        .into_iter()
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chit_chat_produces_zero_queries() {
        for msg in ["hi", "Hello!", "thanks", "OK.", "How are you?", "hey"] {
            let plan = plan_queries(msg);
            assert!(plan.should_skip_retrieval(), "expected skip for {:?}", msg);
            assert!(plan.skip_reason.is_some());
        }
    }

    #[test]
    fn test_empty_input_skips() {
        assert!(plan_queries("").should_skip_retrieval());
        assert!(plan_queries("   ").should_skip_retrieval());
    }

    #[test]
    fn test_skill_query_is_verb_first() {
        let plan = plan_queries("How do I configure the embedding service?");
        assert!(!plan.should_skip_retrieval());
        let q = &plan.queries[0];
        assert_eq!(q.context_type, ContextType::Skill);
        assert_eq!(q.query, "Configure the embedding service?");
    }

    #[test]
    fn test_memory_markers_detected() {
        let plan = plan_queries("What are my coding preferences?");
        assert_eq!(plan.queries[0].context_type, ContextType::Memory);
    }

    #[test]
    fn test_resource_markers_detected() {
        let plan = plan_queries("Find the template for RFC documents");
        assert_eq!(plan.queries[0].context_type, ContextType::Resource);
    }

    #[test]
    fn test_compound_input_decomposes() {
        let plan = plan_queries("Extract PDF tables; then find my notes about deployment");
        assert!(plan.queries.len() >= 2);
        assert_eq!(plan.queries[0].priority, 1);
        assert_eq!(plan.queries[1].priority, 2);
    }

    #[test]
    fn test_max_five_queries() {
        let long = "a and also b and also c and also d and also e and also f and also g";
        let plan = plan_queries(long);
        assert!(plan.queries.len() <= 5);
    }

    #[test]
    fn test_priorities_valid_range() {
        let plan = plan_queries("create an RFC document");
        for q in &plan.queries {
            assert!((1..=5).contains(&q.priority));
        }
    }

    #[test]
    fn test_plan_serializes() {
        let plan = plan_queries("my preferences");
        let json = serde_json::to_string(&plan).unwrap();
        assert!(json.contains("context_type"));
    }

    #[test]
    fn test_capitalize_helper() {
        assert_eq!(capitalize("hello world"), "Hello world");
        assert_eq!(capitalize(""), "");
    }
}
