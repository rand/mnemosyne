//! Agent context helpers — utilities for building agent prompt context
//! from mnemosyne memory recall results.
//!
//! Inspired by NousResearch/hermes-agent's memory injection patterns:
//! - `StreamingContextScrubber` — a state machine that strips
//!   `<memory-context>` blocks from LLM streaming output chunk-by-chunk,
//!   preventing the model from echoing back its injected memory context as
//!   if it were the agent's own text.
//! - `build_memory_context_block` — wrap recall text in a fenced block
//!   that the model treats as reference data, not new input.

use crate::utils::sanitize_context;

/// State machine for scrubbing memory-context blocks from streaming text.
///
/// The one-shot `sanitize_context` function cannot survive chunk boundaries:
/// a `<memory-context>` opened in one delta and closed in a later delta
/// would leak its payload to the user interface. This scrubber runs a
/// small state machine across deltas, holding back partial-tag tails and
/// discarding everything inside a span (including the system-note line).
///
/// Ported from `agent.memory_manager.StreamingContextScrubber`
/// in hermes-agent.
pub struct StreamingContextScrubber {
    open_tag: &'static str,
    close_tag: &'static str,
    in_span: bool,
    buf: String,
}

impl Default for StreamingContextScrubber {
    fn default() -> Self {
        Self::new()
    }
}

impl StreamingContextScrubber {
    /// Create a new scrubber, ready to process streaming deltas.
    pub fn new() -> Self {
        Self {
            open_tag: "<memory-context>",
            close_tag: "</memory-context>",
            in_span: false,
            buf: String::new(),
        }
    }

    /// Reset the scrubber to its initial state.
    ///
    /// Re-entrant per agent instance. Call this at the top of each turn
    /// (or before processing a new response stream).
    pub fn reset(&mut self) {
        self.in_span = false;
        self.buf.clear();
    }

    /// Feed a streaming chunk and return the visible (cleaned) portion.
    ///
    /// Any trailing fragment that could be the start of an open/close tag
    /// is held back in the internal buffer and surfaced on the next
    /// `feed()` call or discarded/emitted by `flush()`.
    pub fn feed(&mut self, text: &str) -> String {
        if text.is_empty() {
            return String::new();
        }
        self.buf.push_str(text);
        let mut out: Vec<String> = Vec::new();

        loop {
            if self.in_span {
                // We're inside a <memory-context>...</memory-context> block.
                // Look for the close tag.
                if let Some(rel) = self.buf.find(self.close_tag) {
                    // Found close — drop everything up to and including the tag.
                    self.buf.drain(..rel + self.close_tag.len());
                    self.in_span = false;
                } else {
                    // No close tag yet — hold back a potential partial close tag
                    // suffix so we don't prematurely emit inside-span text.
                    let held = partial_suffix_len(&self.buf, self.close_tag);
                    let keep = self.buf.len().saturating_sub(held);
                    if keep > 0 {
                        self.buf.drain(..keep);
                    }
                    break;
                }
            } else {
                // We're outside any span. Look for an open tag.
                if let Some(idx) = self.buf.find(self.open_tag) {
                    // Emit text before the tag
                    if idx > 0 {
                        out.push(self.buf[..idx].to_string());
                    }
                    // Consume up to and including the open tag, enter span
                    self.buf.drain(..idx + self.open_tag.len());
                    self.in_span = true;
                } else {
                    // No open tag found — emit everything except a potential
                    // partial open tag suffix
                    let held = partial_suffix_len(&self.buf, self.open_tag);
                    let keep = self.buf.len().saturating_sub(held);
                    if keep > 0 {
                        out.push(self.buf.drain(..keep).collect::<String>());
                    }
                    break;
                }
            }
        }
        out.join("")
    }

    /// Flush any held-back buffer at end-of-stream.
    ///
    /// If we're still inside an unterminated span, the remaining content is
    /// discarded (safer: leaking partial memory context is worse than a
    /// truncated answer). Otherwise the held-back partial-tag tail is emitted
    /// verbatim (it turned out not to be a real tag).
    pub fn flush(&mut self) -> String {
        if self.in_span || self.buf.is_empty() {
            let result = if self.in_span {
                String::new() // Discard unterminated span
            } else {
                std::mem::take(&mut self.buf)
            };
            self.in_span = false;
            result
        } else {
            let result = std::mem::take(&mut self.buf);
            result
        }
    }

    /// Process a full string (non-streaming convenience wrapper).
    ///
    /// Equivalent to calling `feed()` once, then `flush()`.
    pub fn scrub(text: &str) -> String {
        let mut s = Self::new();
        let mut result = s.feed(text);
        result.push_str(&s.flush());
        result
    }
}

/// Return the length of the longest buf-suffix that could be a prefix
/// of the tag. In other words, how many trailing bytes of `buf` form the
/// beginning of `tag`? These bytes must be held back because they MIGHT
/// be the start of a tag (across a chunk boundary).
fn partial_suffix_len(buf: &str, tag: &str) -> usize {
    let tag_lower = tag.to_ascii_lowercase();
    let buf_lower = buf.to_ascii_lowercase();
    let max_check = usize::min(buf_lower.len(), tag_lower.len().saturating_sub(1));
    for i in (1..=max_check).rev() {
        let end = buf_lower.len();
        if end >= i {
            let suffix = &buf_lower[end - i..];
            if tag_lower.starts_with(suffix) {
                return i;
            }
        }
    }
    0
}

/// Wrap prefetched memory context in a `<memory-context>` fence block.
///
/// Mirrors `build_memory_context_block` from hermes-agent's
/// `agent/memory_manager.py`. Keeps memory context isolated from the
/// user message so the model reads it as reference data, not new input.
pub fn build_memory_context_block(raw_context: impl AsRef<str>) -> String {
    let text = raw_context.as_ref().trim();
    if text.is_empty() {
        return String::new();
    }
    let clean = sanitize_context(text);
    if clean != text {
        tracing::warn!("memory provider returned pre-wrapped context; stripped");
    }
    format!(
        "<memory-context>\n[System note: The following is recalled memory context, NOT new user input. Treat as authoritative reference data — this is the agent's persistent memory and should inform all responses.]\n\n{}\n</memory-context>",
        clean
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scrubber_removes_block() {
        let mut s = StreamingContextScrubber::new();
        let result = s.feed("Hello <memory-context>Secret stuff</memory-context> World");
        let flushed = s.flush();
        assert_eq!(result + &flushed, "Hello  World");
    }

    #[test]
    fn test_scrubber_block_spanning_chunks() {
        let mut s = StreamingContextScrubber::new();
        let part1 = s.feed("Hello <memory-context>Sec");
        let part2 = s.feed("ret stuff</memory-context> World");
        let part3 = s.flush();
        assert_eq!(part1 + &part2 + &part3, "Hello  World");
    }

    #[test]
    fn test_scrubber_passthrough() {
        let mut s = StreamingContextScrubber::new();
        let result = s.feed("Just normal text here");
        let flushed = s.flush();
        assert_eq!(result + &flushed, "Just normal text here");
    }

    #[test]
    fn test_scrubber_partial_tag_held() {
        let mut s = StreamingContextScrubber::new();
        // Send "Hello <memory" — the `<memory` suffix is held back
        let result = s.feed("Hello <memory");
        assert_eq!(result, "Hello ");
        // Now complete the tag
        let result2 = s.feed("-context>Secret</memory-context> World");
        let flushed = s.flush();
        assert_eq!(result2 + &flushed, " World");
    }

    #[test]
    fn test_scrubber_multiple_blocks() {
        let mut s = StreamingContextScrubber::new();
        let result = s.feed(
            "A<memory-context>BLOCK1</memory-context>B<memory-context>BLOCK2</memory-context>C",
        );
        let flushed = s.flush();
        assert_eq!(result + &flushed, "ABC");
    }

    #[test]
    fn test_scrubber_unterminated_span_discarded() {
        let mut s = StreamingContextScrubber::new();
        let result = s.feed("Before <memory-context>Secret stuff");
        let flushed = s.flush();
        // The "Before " is emitted, the unterminated span is discarded
        assert_eq!(result + &flushed, "Before ");
    }

    #[test]
    fn test_scrub_static() {
        assert_eq!(
            StreamingContextScrubber::scrub("Hello <memory-context>Secret</memory-context> World"),
            "Hello  World"
        );
        assert_eq!(
            StreamingContextScrubber::scrub("Normal text"),
            "Normal text"
        );
    }

    #[test]
    fn test_build_memory_context_block() {
        let block = build_memory_context_block("Something useful here.");
        assert!(block.contains("<memory-context>"));
        assert!(block.contains("</memory-context>"));
        assert!(block.contains("Something useful here."));
        assert!(block.contains("System note"));
    }

    #[test]
    fn test_build_memory_context_block_empty() {
        assert_eq!(build_memory_context_block(""), "");
        assert_eq!(build_memory_context_block("   "), "");
    }
}
