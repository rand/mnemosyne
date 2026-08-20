//! String utility functions for safe UTF-8 text manipulation

/// Safely truncate a string at a character boundary, adding ellipsis if truncated.
///
/// Unlike naive byte slicing (`&s[..n]`), this function ensures we don't slice
/// in the middle of a multi-byte UTF-8 character, which would cause a panic.
///
/// # Arguments
/// * `s` - The string to truncate
/// * `max_chars` - Maximum number of UTF-8 characters (not bytes) to keep
///
/// # Returns
/// A new String that is either the original string (if <= max_chars) or
/// truncated at the nearest character boundary with "..." appended.
///
/// # Examples
/// ```
/// use mnemosyne::utils::string::truncate_at_char_boundary;
///
/// // ASCII text
/// assert_eq!(truncate_at_char_boundary("hello world", 5), "hello...");
/// assert_eq!(truncate_at_char_boundary("hello", 10), "hello");
///
/// // Multi-byte UTF-8 characters
/// assert_eq!(truncate_at_char_boundary("hello→world", 6), "hello→...");
/// assert_eq!(truncate_at_char_boundary("🎉🎊🎈", 2), "🎉🎊...");
/// ```
pub fn truncate_at_char_boundary(s: &str, max_chars: usize) -> String {
    let char_count = s.chars().count();

    if char_count <= max_chars {
        s.to_string()
    } else {
        // Take exactly max_chars characters and append ellipsis
        let truncated: String = s.chars().take(max_chars).collect();
        format!("{}...", truncated)
    }
}

// ─── Agent memory helpers (inspired by NousResearch/hermes-agent agent/memory_manager.py) ───

/// Trivial user prompts that carry no signal worth recalling.
///
/// Mirrors the `MemoryManager` logic in hermes-agent so the agent runtime
/// can skip expensive prefetch for no-ops like "hi", "thanks", "ok".
static TRIVIAL_PROMPTS: &[&str] = &[
    "hi",
    "hello",
    "hey",
    "hi!",
    "hello!",
    "hey!",
    "thanks",
    "thanks!",
    "thank you",
    "thank you!",
    "ok",
    "ok!",
    "okay",
    "k",
    "kk",
    "got it",
    "nice",
    "cool",
    "great",
    "lol",
    "haha",
];

/// Return `true` when `text` is a trivial greeting/acknowledgement that
/// would produce no useful memory write or recall.
#[inline(always)]
pub fn is_trivial_prompt(text: &str) -> bool {
    let normalized = text.trim().to_lowercase();
    // Fast path: single-word prompts
    if let Some(first) = normalized.split_whitespace().next() {
        if TRIVIAL_PROMPTS.contains(&first) {
            // Make sure there isn't a second word that changes meaning
            let rest = normalized.strip_prefix(first).unwrap_or("").trim();
            if rest.is_empty() || rest == "!" || rest == "!!" {
                return true;
            }
        }
    }
    // Full-phrase trivial prompts
    TRIVIAL_PROMPTS.contains(&normalized.as_str())
}

/// Strip `<memory-context>...</memory-context>` fence blocks and the injected
/// `[System note: ...]` header that wraps prefetched memory context when it
/// is embedded into a user message.
///
/// Use this on any model output that may contain leaked memory context before
/// it is displayed to the user or re-submitted as input on a subsequent turn.
///
/// Ported from `agent.memory_manager.sanitize_context` in hermes-agent.
pub fn sanitize_context(text: &str) -> String {
    // Refs: we either strip one of two things:
    //   1. An injected block   <memory-context>\n[System note: ...]...\n</memory-context>
    //   2. A stray notice      [System note: ...]
    // Both appear as plain text when a model echoes the raw context back.

    // 1) Strip <memory-context>...</memory-context> blocks (may be multiple).
    //    We loop so adjacent blocks are all removed.
    let mut out = text.to_string();
    while let Some(idx) = out.find("<memory-context>") {
        if let Some(rel) = out[idx..].find("</memory-context>") {
            let close_pos = idx + rel;
            let end = close_pos + "</memory-context>".len();
            out = format!("{}{}", &out[..idx], &out[end..]);
        } else {
            break; // No matching close — stop to avoid stripping too much
        }
    }

    // 2) Strip `[System note: ...]` injection markers.
    //    The note is introduced by `[System note: ` and terminated by `].`
    //    (full stop after closing bracket).  After `].` we consume any
    //    trailing whitespace (newline, space, etc.).
    let start_marker = "[System note: ";
    while let Some(start) = out.find(start_marker) {
        let tail = &out[start..];
        match tail.find("].") {
            Some(rel) => {
                // Position of the `.` after `]`.
                let dot_pos = start + rel + 1;
                let rest = &out[dot_pos..];
                let ws = rest.chars().take_while(|c| c.is_whitespace()).count();
                let end = dot_pos + ws;
                out = format!("{}{}", &out[..start], &out[end..]);
            }
            None => {
                // No closing `].` — strip from marker to end of string.
                out.truncate(start);
                break;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_ascii_short() {
        assert_eq!(truncate_at_char_boundary("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_ascii_exact() {
        assert_eq!(truncate_at_char_boundary("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_ascii_long() {
        assert_eq!(truncate_at_char_boundary("hello world", 5), "hello...");
    }

    #[test]
    fn test_truncate_empty() {
        assert_eq!(truncate_at_char_boundary("", 5), "");
    }

    #[test]
    fn test_truncate_multibyte_arrow() {
        let text = "Phase 4.1→4.2 complete";
        let result = truncate_at_char_boundary(text, 10);
        assert_eq!(result, "Phase 4.1→...");
        assert!(result.is_char_boundary(0));
        assert!(result.is_char_boundary(result.len()));
    }

    #[test]
    fn test_truncate_emoji() {
        let text = "🎉🎊🎈🎁🎀";
        assert_eq!(truncate_at_char_boundary(text, 2), "🎉🎊...");
        assert_eq!(truncate_at_char_boundary(text, 5), "🎉🎊🎈🎁🎀");
    }

    #[test]
    fn test_truncate_mixed_content() {
        let text = "commit 5a728f4: Executor→Reviewer";
        let result = truncate_at_char_boundary(text, 20);
        assert_eq!(result, "commit 5a728f4: Exec...");
    }

    #[test]
    fn test_truncate_japanese() {
        let text = "こんにちは世界";
        assert_eq!(truncate_at_char_boundary(text, 3), "こんに...");
    }

    #[test]
    fn test_original_crash_case() {
        let text = "Phase 4.1-4.2 complete (commit 5a728f4): Executor and Reviewer agents integrated with PyO3 bridge. Both inherit from AgentExecutionMixin and implement _execute_work_item(). Executor converts WorkItem→work_plan→execute_work_plan()→WorkResult. Reviewer validates";
        let result = truncate_at_char_boundary(text, 200);
        assert!(result.len() <= text.len());
        assert!(result.ends_with("..."));
    }

    // ── is_trivial_prompt tests ──

    #[test]
    fn test_trivial_hi() {
        assert!(is_trivial_prompt("hi"));
        assert!(is_trivial_prompt("hi!"));
        assert!(is_trivial_prompt("hi  "));
    }

    #[test]
    fn test_nontrivial_hi() {
        assert!(!is_trivial_prompt("hi how are you"));
        assert!(!is_trivial_prompt("hi, remember this"));
    }

    #[test]
    fn test_trivial_thanks() {
        assert!(is_trivial_prompt("thanks"));
        assert!(is_trivial_prompt("Thanks!"));
        assert!(is_trivial_prompt("thank you"));
        assert!(is_trivial_prompt("thank you!"));
    }

    #[test]
    fn test_trivial_ok() {
        assert!(is_trivial_prompt("ok"));
        assert!(is_trivial_prompt("ok!"));
        assert!(is_trivial_prompt("OK"));
        assert!(is_trivial_prompt("okay"));
    }

    #[test]
    fn test_nontrivial_sentences() {
        assert!(!is_trivial_prompt("save this to memory"));
        assert!(!is_trivial_prompt("what is the project status?"));
    }

    // ── sanitize_context tests ──

    #[test]
    fn test_no_memory_context_passthrough() {
        let input = "Hello, how are you?";
        assert_eq!(sanitize_context(input), input);
    }

    #[test]
    fn test_strip_fenced_block() {
        let input = "User asked: foo bar\n<memory-context>\n[System note: ...]\nSecret stuff\n</memory-context>";
        let result = sanitize_context(input);
        assert!(!result.contains("<memory-context>"));
        assert!(!result.contains("Secret stuff"));
    }

    #[test]
    fn test_strip_system_note_line() {
        let input = "[System note: The following is recalled memory context, NOT new user input.] Some content here.";
        let result = sanitize_context(input);
        assert!(!result.contains("System note"));
    }

    #[test]
    fn test_empty_input() {
        assert_eq!(sanitize_context(""), "");
        assert_eq!(sanitize_context("   "), "   ");
    }

    #[test]
    fn test_multiple_blocks() {
        let input =
            "A<memory-context>BLOCK1</memory-context>B<memory-context>BLOCK2</memory-context>C";
        let result = sanitize_context(input);
        assert_eq!(result, "ABC");
    }
}
