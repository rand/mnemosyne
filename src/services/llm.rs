//! LLM service for memory intelligence
//!
//! Provides integration with Claude Haiku for:
//! - Note construction and enrichment
//! - Semantic link generation
//! - Consolidation decisions
//! - Memory summarization

use crate::config::ConfigManager;
use crate::error::{MnemosyneError, Result};
use crate::types::{ConsolidationDecision, LinkType, MemoryLink, MemoryNote, MemoryType};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::{debug, info, warn};

/// Structured JSON response for memory enrichment
#[derive(Debug, Deserialize, Serialize)]
struct EnrichmentResponse {
    summary: String,
    keywords: Vec<String>,
    tags: Vec<String>,
    #[serde(rename = "type")]
    memory_type: String,
    importance: u8,
}

/// Structured JSON response for link generation
#[derive(Debug, Deserialize, Serialize)]
struct LinkResponse {
    links: Vec<LinkEntry>,
}

#[derive(Debug, Deserialize, Serialize)]
struct LinkEntry {
    index: usize,
    #[serde(rename = "type")]
    link_type: String,
    strength: f32,
    reason: String,
}

/// Structured JSON response for consolidation decision
#[derive(Debug, Deserialize, Serialize)]
struct ConsolidationResponse {
    decision: String,
    reason: String,
    superseding_id: Option<String>,
}

/// Configuration for LLM service
#[derive(Debug, Clone)]
pub struct LlmConfig {
    /// Anthropic API key
    pub api_key: String,

    /// Model to use (default: claude-haiku-4-5-20251001)
    pub model: String,

    /// Max tokens for responses
    pub max_tokens: usize,

    /// Temperature for sampling
    pub temperature: f32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        // Try to get API key from ConfigManager
        let api_key = ConfigManager::new()
            .ok()
            .and_then(|cm| cm.get_api_key().ok())
            .unwrap_or_default();

        Self {
            api_key,
            model: "claude-haiku-4-5-20251001".to_string(),
            max_tokens: 1024,
            temperature: 0.7,
        }
    }
}

/// LLM service for memory intelligence
pub struct LlmService {
    config: LlmConfig,
    client: reqwest::Client,
}

/// Anthropic API message format
#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: usize,
    temperature: f32,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

/// Anthropic API response format
#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<Content>,
}

#[derive(Debug, Deserialize)]
struct Content {
    text: String,
}

impl LlmService {
    /// Create a new LLM service with custom config
    ///
    /// Note: Empty API keys are allowed during initialization to support
    /// server startup, but API calls will fail until a valid key is provided.
    pub fn new(config: LlmConfig) -> Result<Self> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))        // Total request timeout
            .connect_timeout(Duration::from_secs(5)) // Connection timeout
            .build()?; // Automatically converts reqwest::Error to MnemosyneError::Http

        Ok(Self { config, client })
    }

    /// Create with default config
    pub fn with_default() -> Result<Self> {
        Self::new(LlmConfig::default())
    }

    /// Enrich a raw memory note with LLM-generated summary, keywords, and metadata
    pub async fn enrich_memory(&self, raw_content: &str, context: &str) -> Result<MemoryNote> {
        debug!("Enriching memory with LLM");

        let prompt = format!(
            r#"You are helping construct a structured memory note for an agentic memory system.

Given this raw observation:
{}

Context: {}

Provide a structured response with:
1. A concise summary (1-2 sentences)
2. 3-5 keywords for indexing
3. 2-3 tags for categorization
4. Determine the memory type (one of: ArchitectureDecision, CodePattern, BugFix, Configuration, Constraint, Entity, Insight, Reference, Preference)
5. Assign an importance score (1-10, where 10 is critical)

Examples:

Example 1:
Raw: "Switched from SQLite to PostgreSQL for production due to concurrent write limitations. Migration completed successfully."
Context: "Database architecture discussion"
{{
  "summary": "Migration from SQLite to PostgreSQL completed to support concurrent writes in production",
  "keywords": ["PostgreSQL", "SQLite", "migration", "concurrency", "database"],
  "tags": ["architecture", "infrastructure"],
  "type": "ArchitectureDecision",
  "importance": 8
}}

Example 2:
Raw: "Fixed infinite loop in retry logic by adding max_attempts counter. Bug was causing API timeouts."
Context: "API reliability improvements"
{{
  "summary": "Added max_attempts counter to prevent infinite retry loops causing API timeouts",
  "keywords": ["retry", "bugfix", "infinite-loop", "API", "timeout"],
  "tags": ["reliability", "api"],
  "type": "BugFix",
  "importance": 7
}}

Example 3:
Raw: "User prefers dark mode for terminal interfaces"
Context: "User interface preferences"
{{
  "summary": "User preference for dark mode terminal interfaces",
  "keywords": ["dark-mode", "terminal", "UI", "preferences"],
  "tags": ["preferences", "ui"],
  "type": "Preference",
  "importance": 3
}}

Now format your response as valid JSON matching this schema:
{{
  "summary": "string (1-2 sentences)",
  "keywords": ["string array (3-5 items)"],
  "tags": ["string array (2-3 items)"],
  "type": "ArchitectureDecision|CodePattern|BugFix|Configuration|Constraint|Entity|Insight|Reference|Preference",
  "importance": number (1-10)
}}

IMPORTANT: Return ONLY valid JSON, no additional text or markdown formatting.
"#,
            raw_content, context
        );

        let response = self.call_api(&prompt).await?;

        // Parse JSON response with fallback to string parsing
        let enrichment: EnrichmentResponse = match serde_json::from_str(&response) {
            Ok(data) => data,
            Err(e) => {
                warn!(
                    "JSON parsing failed: {}, attempting fallback string parsing",
                    e
                );
                // Fallback to old string parsing for backward compatibility
                let summary = self
                    .extract_field(&response, "SUMMARY:")
                    .or_else(|_| self.extract_field(&response, "summary:"))?;
                let keywords_str = self
                    .extract_field(&response, "KEYWORDS:")
                    .or_else(|_| self.extract_field(&response, "keywords:"))?;
                let tags_str = self
                    .extract_field(&response, "TAGS:")
                    .or_else(|_| self.extract_field(&response, "tags:"))?;
                let type_str = self
                    .extract_field(&response, "TYPE:")
                    .or_else(|_| self.extract_field(&response, "type:"))?;
                let importance_str = self
                    .extract_field(&response, "IMPORTANCE:")
                    .or_else(|_| self.extract_field(&response, "importance:"))?;

                EnrichmentResponse {
                    summary,
                    keywords: keywords_str
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect(),
                    tags: tags_str
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect(),
                    memory_type: type_str.trim().to_string(),
                    importance: importance_str
                        .trim()
                        .parse::<u8>()
                        .unwrap_or(5)
                        .clamp(1, 10),
                }
            }
        };

        let memory_type = match enrichment.memory_type.as_str() {
            "ArchitectureDecision" => MemoryType::ArchitectureDecision,
            "CodePattern" => MemoryType::CodePattern,
            "BugFix" => MemoryType::BugFix,
            "Configuration" => MemoryType::Configuration,
            "Constraint" => MemoryType::Constraint,
            "Entity" => MemoryType::Entity,
            "Insight" => MemoryType::Insight,
            "Reference" => MemoryType::Reference,
            "Preference" => MemoryType::Preference,
            _ => MemoryType::Insight, // Default fallback
        };

        let importance = enrichment.importance.clamp(1, 10);

        Ok(MemoryNote {
            id: crate::types::MemoryId::new(),
            namespace: crate::types::Namespace::Global, // Caller should set this
            created_at: Utc::now(),
            updated_at: Utc::now(),
            content: raw_content.to_string(),
            summary: enrichment.summary,
            keywords: enrichment.keywords,
            tags: enrichment.tags,
            context: context.to_string(),
            memory_type,
            importance,
            confidence: 0.8, // LLM-generated has good confidence
            links: vec![],
            related_files: vec![],
            related_entities: vec![],
            access_count: 0,
            last_accessed_at: Utc::now(),
            expires_at: None,
            is_archived: false,
            superseded_by: None,
            embedding: None,
            embedding_model: self.config.model.clone(),
        })
    }

    /// Generate semantic links between a new memory and existing memories
    pub async fn generate_links(
        &self,
        new_memory: &MemoryNote,
        candidates: &[MemoryNote],
    ) -> Result<Vec<MemoryLink>> {
        debug!(
            "Generating links for memory with {} candidates",
            candidates.len()
        );

        if candidates.is_empty() {
            return Ok(vec![]);
        }

        let candidates_text: Vec<String> = candidates
            .iter()
            .enumerate()
            .map(|(i, m)| {
                format!(
                    "[{}] {} (Type: {:?}, Tags: {})",
                    i,
                    m.summary,
                    m.memory_type,
                    m.tags.join(", ")
                )
            })
            .collect();

        let prompt = format!(
            r#"You are analyzing semantic relationships between memories in an agentic memory system.

New memory:
Summary: {}
Content: {}
Type: {:?}
Tags: {}

Candidate memories:
{}

For each candidate that has a meaningful relationship, specify:
1. The candidate index
2. The relationship type (Extends, Contradicts, Implements, References, Supersedes)
3. The link strength (0.0 - 1.0, higher = stronger relationship)
4. A brief reason

Examples of good link identification:

Example 1 - Extension:
New: "Added authentication middleware to API endpoints"
Candidate [2]: "Set up JWT token generation for user sessions"
{{
  "links": [
    {{
      "index": 2,
      "type": "Extends",
      "strength": 0.9,
      "reason": "JWT implementation provides auth mechanism for the middleware"
    }}
  ]
}}

Example 2 - Multiple links:
New: "Decided to use REST API instead of GraphQL"
Candidate [5]: "GraphQL chosen for API layer due to flexible queries"
Candidate [3]: "API documentation framework selection"
{{
  "links": [
    {{
      "index": 5,
      "type": "Contradicts",
      "strength": 0.95,
      "reason": "New decision reverses previous GraphQL choice"
    }},
    {{
      "index": 3,
      "type": "References",
      "strength": 0.7,
      "reason": "Documentation framework relates to API design choice"
    }}
  ]
}}

Example 3 - No meaningful links:
New: "User prefers dark mode terminal"
Candidates are all about database architecture
{{
  "links": []
}}

Now analyze the actual memories above. Format your response as valid JSON matching this schema:
{{
  "links": [
    {{
      "index": number,
      "type": "Extends|Contradicts|Implements|References|Supersedes",
      "strength": number (0.0-1.0),
      "reason": "string"
    }}
  ]
}}

Only include meaningful links (strength >= 0.6). If no relationships exist, return {{"links": []}}.
IMPORTANT: Return ONLY valid JSON, no additional text or markdown formatting.
"#,
            new_memory.summary,
            new_memory.content,
            new_memory.memory_type,
            new_memory.tags.join(", "),
            candidates_text.join("\n")
        );

        let response = self.call_api(&prompt).await?;

        // Parse JSON response with fallback to string parsing
        let link_response: LinkResponse = match serde_json::from_str(&response) {
            Ok(data) => data,
            Err(e) => {
                warn!(
                    "JSON parsing failed: {}, attempting fallback string parsing",
                    e
                );
                // Fallback to old string parsing for backward compatibility
                if response.trim() == "NO_LINKS" {
                    return Ok(vec![]);
                }

                let mut link_entries = Vec::new();
                for line in response.lines() {
                    if let Some(link_data) = line.strip_prefix("LINK:") {
                        let parts: Vec<&str> = link_data.split(',').collect();
                        if parts.len() >= 4 {
                            if let Ok(index) = parts[0].trim().parse::<usize>() {
                                if index < candidates.len() {
                                    let link_type = parts[1].trim().to_string();
                                    let strength = parts[2]
                                        .trim()
                                        .parse::<f32>()
                                        .unwrap_or(0.5)
                                        .clamp(0.0, 1.0);
                                    let reason = parts[3..].join(",").trim().to_string();

                                    link_entries.push(LinkEntry {
                                        index,
                                        link_type,
                                        strength,
                                        reason,
                                    });
                                }
                            }
                        }
                    }
                }
                LinkResponse {
                    links: link_entries,
                }
            }
        };

        // Convert parsed entries to MemoryLinks
        let mut links = Vec::new();
        for entry in link_response.links {
            if entry.index >= candidates.len() {
                warn!(
                    "Link index {} out of bounds for {} candidates",
                    entry.index,
                    candidates.len()
                );
                continue;
            }

            let link_type = match entry.link_type.as_str() {
                "Extends" => LinkType::Extends,
                "Contradicts" => LinkType::Contradicts,
                "Implements" => LinkType::Implements,
                "References" => LinkType::References,
                "Supersedes" => LinkType::Supersedes,
                _ => LinkType::References,
            };

            links.push(MemoryLink {
                target_id: candidates[entry.index].id,
                link_type,
                strength: entry.strength.clamp(0.0, 1.0),
                reason: entry.reason,
                created_at: Utc::now(),
                last_traversed_at: None,
                user_created: false,  // LLM-generated links are system-created
            });
        }

        info!("Generated {} links", links.len());
        Ok(links)
    }

    /// Decide whether two memories should be consolidated
    pub async fn should_consolidate(
        &self,
        memory_a: &MemoryNote,
        memory_b: &MemoryNote,
    ) -> Result<ConsolidationDecision> {
        debug!(
            "Checking consolidation for memories {} and {}",
            memory_a.id, memory_b.id
        );

        let prompt = format!(
            r#"You are analyzing whether two memories should be consolidated in an agentic memory system.

Memory A (ID: {}):
Summary: {}
Content: {}
Type: {:?}
Importance: {}
Tags: {}

Memory B (ID: {}):
Summary: {}
Content: {}
Type: {:?}
Importance: {}
Tags: {}

Determine if these memories should be:
1. MERGE - Combine into one (very similar content, both valuable)
2. SUPERSEDE - One replaces the other (updated/corrected information)
3. KEEP_BOTH - Maintain separately (distinct content, both relevant)

Examples:

Example 1 - MERGE:
Memory A: "PostgreSQL migration completed successfully"
Memory B: "Switched from SQLite to PostgreSQL for production"
{{
  "decision": "MERGE",
  "reason": "Both describe the same migration event, should combine into comprehensive record",
  "superseding_id": null
}}

Example 2 - SUPERSEDE:
Memory A (ID: abc-123, Importance: 6): "API endpoint uses /api/v1/users"
Memory B (ID: def-456, Importance: 8): "API endpoint updated to /api/v2/users with new schema"
{{
  "decision": "SUPERSEDE",
  "reason": "Memory B contains updated information that makes A obsolete",
  "superseding_id": "def-456"
}}

Example 3 - KEEP_BOTH:
Memory A: "User authentication implemented with JWT"
Memory B: "Database connection pooling configured"
{{
  "decision": "KEEP_BOTH",
  "reason": "Distinct technical decisions, both remain relevant",
  "superseding_id": null
}}

Now analyze the actual memories above. Format your response as valid JSON matching this schema:
{{
  "decision": "MERGE|SUPERSEDE|KEEP_BOTH",
  "reason": "string",
  "superseding_id": "string or null"
}}

IMPORTANT: Return ONLY valid JSON, no additional text or markdown formatting.
"#,
            memory_a.id,
            memory_a.summary,
            memory_a.content,
            memory_a.memory_type,
            memory_a.importance,
            memory_a.tags.join(", "),
            memory_b.id,
            memory_b.summary,
            memory_b.content,
            memory_b.memory_type,
            memory_b.importance,
            memory_b.tags.join(", ")
        );

        let response = self.call_api(&prompt).await?;

        // Parse JSON response with fallback to string parsing
        let consolidation_response: ConsolidationResponse = match serde_json::from_str(&response) {
            Ok(data) => data,
            Err(e) => {
                warn!(
                    "JSON parsing failed: {}, attempting fallback string parsing",
                    e
                );
                // Fallback to old string parsing for backward compatibility
                let decision_str = self
                    .extract_field(&response, "DECISION:")
                    .or_else(|_| self.extract_field(&response, "decision:"))?;
                let reason = self
                    .extract_field(&response, "REASON:")
                    .or_else(|_| self.extract_field(&response, "reason:"))
                    .unwrap_or_else(|_| "No reason provided".to_string());
                let superseding_str = self
                    .extract_field(&response, "SUPERSEDING_ID:")
                    .or_else(|_| self.extract_field(&response, "superseding_id:"))
                    .ok();

                ConsolidationResponse {
                    decision: decision_str.trim().to_string(),
                    reason,
                    superseding_id: superseding_str.and_then(|s| {
                        let trimmed = s.trim();
                        if trimmed == "NONE" || trimmed.is_empty() {
                            None
                        } else {
                            Some(trimmed.to_string())
                        }
                    }),
                }
            }
        };

        let decision = match consolidation_response.decision.as_str() {
            "MERGE" => {
                // Use the more important memory as the base
                let into = if memory_a.importance >= memory_b.importance {
                    memory_a.id
                } else {
                    memory_b.id
                };

                // In a real implementation, we'd merge the content intelligently
                // For now, combine both summaries
                let content = format!(
                    "{}\n\nMerged with: {}",
                    if into == memory_a.id {
                        &memory_a.content
                    } else {
                        &memory_b.content
                    },
                    if into == memory_a.id {
                        &memory_b.summary
                    } else {
                        &memory_a.summary
                    }
                );

                ConsolidationDecision::Merge { into, content }
            }
            "SUPERSEDE" => {
                // Keep the more important/recent one
                let (kept, superseded) = if memory_a.importance >= memory_b.importance {
                    (memory_a.id, memory_b.id)
                } else {
                    (memory_b.id, memory_a.id)
                };

                ConsolidationDecision::Supersede { kept, superseded }
            }
            _ => ConsolidationDecision::KeepBoth,
        };

        Ok(decision)
    }

    /// Make an API call to Claude with a custom prompt
    ///
    /// This is a low-level method for custom LLM interactions.
    /// For common use cases, prefer specialized methods like `enrich_memory` or `should_consolidate`.
    pub async fn call_api(&self, prompt: &str) -> Result<String> {
        // Check for API key before making request
        if self.config.api_key.is_empty() {
            return Err(MnemosyneError::Config(config::ConfigError::Message(
                "ANTHROPIC_API_KEY not set".to_string(),
            )));
        }

        debug!("Calling Anthropic API");

        let request = AnthropicRequest {
            model: self.config.model.clone(),
            max_tokens: self.config.max_tokens,
            temperature: self.config.temperature,
            messages: vec![Message {
                role: "user".to_string(),
                content: prompt.to_string(),
            }],
        };

        // Always use x-api-key header (OAuth tokens work with this header)
        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                // Map reqwest errors to appropriate MnemosyneError types
                if e.is_timeout() || e.is_connect() {
                    MnemosyneError::NetworkError(format!("Network connection failed: {}", e))
                } else {
                    MnemosyneError::Http(e)
                }
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();

            // Return specific error types based on HTTP status code
            return Err(match status.as_u16() {
                401 | 403 => MnemosyneError::AuthenticationError(format!(
                    "Invalid or missing API key (status {}): {}",
                    status, error_text
                )),
                429 => MnemosyneError::RateLimitExceeded(format!(
                    "API rate limit exceeded: {}",
                    error_text
                )),
                500..=599 => MnemosyneError::NetworkError(format!(
                    "LLM service unavailable (status {}): {}",
                    status, error_text
                )),
                _ => MnemosyneError::LlmApi(format!(
                    "API request failed with status {}: {}",
                    status, error_text
                )),
            });
        }

        let api_response: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| MnemosyneError::LlmApi(format!("Failed to parse response: {}", e)))?;

        api_response
            .content
            .first()
            .map(|c| c.text.clone())
            .ok_or_else(|| MnemosyneError::LlmApi("Empty response from API".to_string()))
    }

    /// Extract a field from structured LLM response
    fn extract_field(&self, response: &str, field: &str) -> Result<String> {
        response
            .lines()
            .find(|line| line.starts_with(field))
            .and_then(|line| line.strip_prefix(field))
            .map(|s| s.trim().to_string())
            .ok_or_else(|| MnemosyneError::LlmApi(format!("Failed to extract field: {}", field)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires ANTHROPIC_API_KEY
    async fn test_enrich_memory() {
        let service = LlmService::with_default().unwrap();

        let raw_content = "Decided to use PostgreSQL for the user database because it has better ACID guarantees than MongoDB for our use case.";
        let context = "myproject - database selection";

        let memory = service.enrich_memory(raw_content, context).await.unwrap();

        assert!(!memory.summary.is_empty());
        assert!(!memory.keywords.is_empty());
        assert!(!memory.tags.is_empty());
        assert!(memory.importance >= 1 && memory.importance <= 10);
    }
}
