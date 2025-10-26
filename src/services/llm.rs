//! LLM service for memory intelligence
//!
//! Provides integration with Claude Haiku for:
//! - Note construction and enrichment
//! - Semantic link generation
//! - Consolidation decisions
//! - Memory summarization

use crate::error::{MnemosyneError, Result};
use crate::types::{ConsolidationDecision, LinkType, MemoryLink, MemoryNote, MemoryType};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::env;
use tracing::{debug, info};

/// Configuration for LLM service
#[derive(Debug, Clone)]
pub struct LlmConfig {
    /// Anthropic API key
    pub api_key: String,

    /// Model to use (default: claude-3-5-haiku-20241022)
    pub model: String,

    /// Max tokens for responses
    pub max_tokens: usize,

    /// Temperature for sampling
    pub temperature: f32,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            api_key: env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
            model: "claude-3-5-haiku-20241022".to_string(),
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
    pub fn new(config: LlmConfig) -> Result<Self> {
        if config.api_key.is_empty() {
            return Err(MnemosyneError::Config(
                config::ConfigError::Message("ANTHROPIC_API_KEY not set".to_string()),
            ));
        }

        Ok(Self {
            config,
            client: reqwest::Client::new(),
        })
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

Format your response EXACTLY as:
SUMMARY: <summary>
KEYWORDS: <keyword1>, <keyword2>, ...
TAGS: <tag1>, <tag2>, ...
TYPE: <memory_type>
IMPORTANCE: <score>
"#,
            raw_content, context
        );

        let response = self.call_api(&prompt).await?;

        // Parse the structured response
        let summary = self.extract_field(&response, "SUMMARY:")?;
        let keywords_str = self.extract_field(&response, "KEYWORDS:")?;
        let tags_str = self.extract_field(&response, "TAGS:")?;
        let type_str = self.extract_field(&response, "TYPE:")?;
        let importance_str = self.extract_field(&response, "IMPORTANCE:")?;

        let keywords: Vec<String> = keywords_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let tags: Vec<String> = tags_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let memory_type = match type_str.trim() {
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

        let importance = importance_str
            .trim()
            .parse::<u8>()
            .unwrap_or(5)
            .clamp(1, 10);

        Ok(MemoryNote {
            id: crate::types::MemoryId::new(),
            namespace: crate::types::Namespace::Global, // Caller should set this
            created_at: Utc::now(),
            updated_at: Utc::now(),
            content: raw_content.to_string(),
            summary,
            keywords,
            tags,
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
3. The link strength (0.0 - 1.0)
4. A brief reason

Format EXACTLY as (one per line):
LINK: <index>, <type>, <strength>, <reason>

Only include meaningful links. If no relationships exist, respond with:
NO_LINKS
"#,
            new_memory.summary,
            new_memory.content,
            new_memory.memory_type,
            new_memory.tags.join(", "),
            candidates_text.join("\n")
        );

        let response = self.call_api(&prompt).await?;

        if response.trim() == "NO_LINKS" {
            return Ok(vec![]);
        }

        let mut links = Vec::new();

        for line in response.lines() {
            if let Some(link_data) = line.strip_prefix("LINK:") {
                let parts: Vec<&str> = link_data.split(',').collect();
                if parts.len() >= 4 {
                    if let Ok(index) = parts[0].trim().parse::<usize>() {
                        if index < candidates.len() {
                            let link_type = match parts[1].trim() {
                                "Extends" => LinkType::Extends,
                                "Contradicts" => LinkType::Contradicts,
                                "Implements" => LinkType::Implements,
                                "References" => LinkType::References,
                                "Supersedes" => LinkType::Supersedes,
                                _ => LinkType::References,
                            };

                            let strength = parts[2].trim().parse::<f32>().unwrap_or(0.5).clamp(0.0, 1.0);
                            let reason = parts[3..].join(",").trim().to_string();

                            links.push(MemoryLink {
                                target_id: candidates[index].id,
                                link_type,
                                strength,
                                reason,
                                created_at: Utc::now(),
                            });
                        }
                    }
                }
            }
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

Memory A:
Summary: {}
Content: {}
Type: {:?}
Importance: {}
Tags: {}

Memory B:
Summary: {}
Content: {}
Type: {:?}
Importance: {}
Tags: {}

Determine if these memories should be:
1. MERGE - Combine into one (very similar content)
2. SUPERSEDE - One replaces the other (updated information)
3. KEEP_BOTH - Maintain separately (distinct content)

Format EXACTLY as:
DECISION: <MERGE|SUPERSEDE|KEEP_BOTH>
REASON: <brief explanation>
SUPERSEDING_ID: <memory_id if SUPERSEDE, otherwise NONE>
"#,
            memory_a.summary,
            memory_a.content,
            memory_a.memory_type,
            memory_a.importance,
            memory_a.tags.join(", "),
            memory_b.summary,
            memory_b.content,
            memory_b.memory_type,
            memory_b.importance,
            memory_b.tags.join(", ")
        );

        let response = self.call_api(&prompt).await?;

        let decision_str = self.extract_field(&response, "DECISION:")?;
        let _reason = self.extract_field(&response, "REASON:")?;
        let _superseding_str = self.extract_field(&response, "SUPERSEDING_ID:")?;

        let decision = match decision_str.trim() {
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

    /// Make an API call to Claude
    async fn call_api(&self, prompt: &str) -> Result<String> {
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

        let response = self
            .client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| MnemosyneError::Http(e))?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            return Err(MnemosyneError::LlmApi(format!(
                "API request failed with status {}: {}",
                status, error_text
            )));
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
            .ok_or_else(|| {
                MnemosyneError::LlmApi(format!("Failed to extract field: {}", field))
            })
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
