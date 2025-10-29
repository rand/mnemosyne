//! Output parsing for agent detection and semantic highlighting

use regex::Regex;
use std::sync::OnceLock;

/// Agent marker types detected in output
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentMarker {
    /// Orchestrator agent
    Orchestrator,
    /// Optimizer agent
    Optimizer,
    /// Reviewer agent
    Reviewer,
    /// Executor agent
    Executor,
    /// Sub-agent (spawned by executor)
    SubAgent,
    /// Unknown/generic agent
    Unknown,
}

impl AgentMarker {
    /// Parse agent marker from text
    pub fn from_text(text: &str) -> Option<Self> {
        let lower = text.to_lowercase();
        if lower.contains("orchestrator") {
            Some(Self::Orchestrator)
        } else if lower.contains("optimizer") {
            Some(Self::Optimizer)
        } else if lower.contains("reviewer") {
            Some(Self::Reviewer)
        } else if lower.contains("executor") {
            Some(Self::Executor)
        } else if lower.contains("sub-agent") || lower.contains("subagent") {
            Some(Self::SubAgent)
        } else if lower.contains("agent") {
            Some(Self::Unknown)
        } else {
            None
        }
    }

    /// Get display name
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Orchestrator => "Orchestrator",
            Self::Optimizer => "Optimizer",
            Self::Reviewer => "Reviewer",
            Self::Executor => "Executor",
            Self::SubAgent => "Sub-Agent",
            Self::Unknown => "Agent",
        }
    }

    /// Get color for agent (as RGB)
    pub fn color(&self) -> (u8, u8, u8) {
        match self {
            Self::Orchestrator => (255, 200, 100), // Orange
            Self::Optimizer => (100, 200, 255),    // Blue
            Self::Reviewer => (255, 100, 100),     // Red
            Self::Executor => (100, 255, 100),     // Green
            Self::SubAgent => (200, 100, 255),     // Purple
            Self::Unknown => (150, 150, 150),      // Gray
        }
    }
}

/// Parsed output chunk with metadata
#[derive(Debug, Clone)]
pub struct ParsedChunk {
    /// Original text
    pub text: String,
    /// Detected agent marker (if any)
    pub agent: Option<AgentMarker>,
    /// Whether this is an error message
    pub is_error: bool,
    /// Whether this is a tool use block
    pub is_tool_use: bool,
}

/// Output parser for detecting patterns in Claude Code output
pub struct OutputParser {
    /// Buffer for incomplete lines
    buffer: String,
}

impl OutputParser {
    /// Create new parser
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
        }
    }

    /// Parse output chunk
    pub fn parse(&mut self, data: &[u8]) -> Vec<ParsedChunk> {
        let text = String::from_utf8_lossy(data);
        self.buffer.push_str(&text);

        let mut chunks = Vec::new();
        let mut lines: Vec<String> = self.buffer.lines().map(String::from).collect();

        // Keep incomplete line in buffer
        if !self.buffer.ends_with('\n') {
            if let Some(last) = lines.pop() {
                self.buffer = last;
            } else {
                self.buffer.clear();
            }
        } else {
            self.buffer.clear();
        }

        // Parse complete lines
        for line in lines {
            chunks.push(self.parse_line(&line));
        }

        chunks
    }

    /// Parse a single line
    fn parse_line(&self, line: &str) -> ParsedChunk {
        ParsedChunk {
            agent: AgentMarker::from_text(line),
            is_error: Self::is_error_line(line),
            is_tool_use: Self::is_tool_use_line(line),
            text: line.to_string(),
        }
    }

    /// Check if line contains error
    fn is_error_line(line: &str) -> bool {
        static ERROR_REGEX: OnceLock<Regex> = OnceLock::new();
        let regex = ERROR_REGEX.get_or_init(|| {
            Regex::new(r"(?i)(error|fail|exception|panic|warning)").unwrap()
        });
        regex.is_match(line)
    }

    /// Check if line is tool use
    fn is_tool_use_line(line: &str) -> bool {
        static TOOL_REGEX: OnceLock<Regex> = OnceLock::new();
        let regex = TOOL_REGEX.get_or_init(|| {
            Regex::new(r"<function_calls>|<invoke|Tool").unwrap()
        });
        regex.is_match(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_marker_detection() {
        assert_eq!(AgentMarker::from_text("orchestrator running"), Some(AgentMarker::Orchestrator));
        assert_eq!(AgentMarker::from_text("optimizer active"), Some(AgentMarker::Optimizer));
        assert_eq!(AgentMarker::from_text("no agent here"), None);
    }

    #[test]
    fn test_parser_creation() {
        let parser = OutputParser::new();
        assert_eq!(parser.buffer.len(), 0);
    }
}