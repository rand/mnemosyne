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
        // '→' is 3 bytes in UTF-8
        let text = "Phase 4.1→4.2 complete";
        let result = truncate_at_char_boundary(text, 10);
        assert_eq!(result, "Phase 4.1→...");

        // Verify we didn't panic and the result is valid UTF-8
        assert!(result.is_char_boundary(0));
        assert!(result.is_char_boundary(result.len()));
    }

    #[test]
    fn test_truncate_emoji() {
        // Emojis are 4 bytes each in UTF-8
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
        // Japanese characters are typically 3 bytes each
        let text = "こんにちは世界";
        assert_eq!(truncate_at_char_boundary(text, 3), "こんに...");
    }

    #[test]
    fn test_original_crash_case() {
        // This is the exact text that caused the original crash
        let text = "Phase 4.1-4.2 complete (commit 5a728f4): Executor and Reviewer agents integrated with PyO3 bridge. Both inherit from AgentExecutionMixin and implement _execute_work_item(). Executor converts WorkItem→work_plan→execute_work_plan()→WorkResult. Reviewer validates";

        // This used to panic when slicing at byte 200 fell inside the '→' character
        let result = truncate_at_char_boundary(text, 200);

        // Should not panic and should be valid UTF-8
        assert!(result.len() <= text.len());
        assert!(result.ends_with("..."));
    }
}
