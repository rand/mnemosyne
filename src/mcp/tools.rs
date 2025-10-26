//! MCP tool implementations
//!
//! Provides 8 core memory tools organized around the OODA loop:
//! - OBSERVE: recall, list
//! - ORIENT: graph, context
//! - DECIDE: remember, consolidate
//! - ACT: update, delete

use crate::error::Result;
use crate::services::LlmService;
use crate::storage::StorageBackend;
use crate::types::{MemoryId, Namespace};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tracing::{debug, warn};

/// Tool schema definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Tool name (e.g., "mnemosyne.recall")
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// JSON Schema for input parameters
    pub input_schema: Value,
}

/// Tool handler that dispatches to appropriate implementation
pub struct ToolHandler {
    storage: Arc<dyn StorageBackend>,
    llm: Arc<LlmService>,
}

impl ToolHandler {
    /// Create a new tool handler
    pub fn new(storage: Arc<dyn StorageBackend>, llm: Arc<LlmService>) -> Self {
        Self { storage, llm }
    }

    /// Get list of all available tools
    pub fn list_tools(&self) -> Vec<Tool> {
        vec![
            // OBSERVE tools
            Tool {
                name: "mnemosyne.recall".to_string(),
                description: "Search memories by semantic query, keywords, or tags. Returns ranked results with relevance scores.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query (semantic or keyword)"
                        },
                        "namespace": {
                            "type": "string",
                            "description": "Optional namespace filter (e.g., 'project:myapp')"
                        },
                        "max_results": {
                            "type": "integer",
                            "description": "Maximum number of results",
                            "default": 10
                        },
                        "min_importance": {
                            "type": "integer",
                            "description": "Minimum importance threshold (1-10)"
                        }
                    },
                    "required": ["query"]
                }),
            },
            Tool {
                name: "mnemosyne.list".to_string(),
                description: "List recent memories in a namespace. Useful for browsing memory history.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "namespace": {
                            "type": "string",
                            "description": "Namespace to list (e.g., 'project:myapp', 'global')"
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Maximum number of memories to return",
                            "default": 20
                        }
                    }
                }),
            },
            // ORIENT tools
            Tool {
                name: "mnemosyne.graph".to_string(),
                description: "Get memory graph starting from seed memory IDs. Traverses semantic links to build context.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "seed_ids": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Starting memory IDs for graph traversal"
                        },
                        "max_hops": {
                            "type": "integer",
                            "description": "Maximum link hops from seed nodes",
                            "default": 2
                        }
                    },
                    "required": ["seed_ids"]
                }),
            },
            Tool {
                name: "mnemosyne.context".to_string(),
                description: "Get full context for specific memory IDs, including linked memories and metadata.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "memory_ids": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Memory IDs to retrieve"
                        },
                        "include_links": {
                            "type": "boolean",
                            "description": "Whether to include linked memories",
                            "default": true
                        }
                    },
                    "required": ["memory_ids"]
                }),
            },
            // DECIDE tools
            Tool {
                name: "mnemosyne.remember".to_string(),
                description: "Store a new memory with LLM enrichment. Automatically generates summary, keywords, tags, and semantic links.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "description": "Memory content to store"
                        },
                        "namespace": {
                            "type": "string",
                            "description": "Namespace (e.g., 'project:myapp', 'global')"
                        },
                        "importance": {
                            "type": "integer",
                            "description": "Importance level (1-10), if not provided LLM will determine"
                        },
                        "context": {
                            "type": "string",
                            "description": "Additional context about when/why this is relevant"
                        }
                    },
                    "required": ["content", "namespace"]
                }),
            },
            Tool {
                name: "mnemosyne.consolidate".to_string(),
                description: "Analyze and merge/supersede similar memories to prevent duplication.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "memory_ids": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Memory IDs to consider for consolidation"
                        },
                        "namespace": {
                            "type": "string",
                            "description": "Optional namespace to search for candidates"
                        }
                    }
                }),
            },
            // ACT tools
            Tool {
                name: "mnemosyne.update".to_string(),
                description: "Update an existing memory's content, importance, or tags.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "memory_id": {
                            "type": "string",
                            "description": "Memory ID to update"
                        },
                        "content": {
                            "type": "string",
                            "description": "New content (triggers re-embedding)"
                        },
                        "importance": {
                            "type": "integer",
                            "description": "New importance level (1-10)"
                        },
                        "tags": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "New tags (replaces existing)"
                        },
                        "add_tags": {
                            "type": "array",
                            "items": {"type": "string"},
                            "description": "Additional tags (appends to existing)"
                        }
                    },
                    "required": ["memory_id"]
                }),
            },
            Tool {
                name: "mnemosyne.delete".to_string(),
                description: "Archive (soft delete) a memory. Does not permanently delete, can be restored.".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "memory_id": {
                            "type": "string",
                            "description": "Memory ID to archive"
                        }
                    },
                    "required": ["memory_id"]
                }),
            },
        ]
    }

    /// Execute a tool call
    pub async fn execute(&self, tool_name: &str, params: Value) -> Result<Value> {
        debug!("Executing tool: {}", tool_name);

        match tool_name {
            "mnemosyne.recall" => self.recall(params).await,
            "mnemosyne.list" => self.list(params).await,
            "mnemosyne.graph" => self.graph(params).await,
            "mnemosyne.context" => self.context(params).await,
            "mnemosyne.remember" => self.remember(params).await,
            "mnemosyne.consolidate" => self.consolidate(params).await,
            "mnemosyne.update" => self.update(params).await,
            "mnemosyne.delete" => self.delete(params).await,
            _ => {
                warn!("Unknown tool: {}", tool_name);
                Ok(serde_json::json!({
                    "error": format!("Unknown tool: {}", tool_name)
                }))
            }
        }
    }

    // === OBSERVE Tools ===

    async fn recall(&self, params: Value) -> Result<Value> {
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct RecallParams {
            query: String,
            namespace: Option<String>,
            max_results: Option<usize>,
            min_importance: Option<u8>,
        }

        let params: RecallParams = serde_json::from_value(params)?;

        // TODO: Parse namespace from string
        // TODO: Implement hybrid search (vector + keyword + graph)

        // For now, return placeholder
        Ok(serde_json::json!({
            "results": [],
            "query": params.query,
            "message": "Recall implementation pending (Phase 5: Hybrid Search)"
        }))
    }

    async fn list(&self, params: Value) -> Result<Value> {
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct ListParams {
            namespace: Option<String>,
            limit: Option<usize>,
        }

        let _params: ListParams = serde_json::from_value(params)?;

        // TODO: Implement list
        Ok(serde_json::json!({
            "memories": [],
            "message": "List implementation pending"
        }))
    }

    // === ORIENT Tools ===

    async fn graph(&self, params: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct GraphParams {
            seed_ids: Vec<String>,
            max_hops: Option<usize>,
        }

        let params: GraphParams = serde_json::from_value(params)?;

        // Parse seed IDs
        let seed_ids: Result<Vec<MemoryId>> = params
            .seed_ids
            .iter()
            .map(|s| MemoryId::from_string(s).map_err(|e| crate::error::MnemosyneError::InvalidId(e.to_string())))
            .collect();

        let seed_ids = seed_ids?;
        let max_hops = params.max_hops.unwrap_or(2);

        // Call storage graph traversal
        let memories = self.storage.graph_traverse(&seed_ids, max_hops).await?;

        Ok(serde_json::json!({
            "memories": memories,
            "count": memories.len()
        }))
    }

    async fn context(&self, params: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct ContextParams {
            memory_ids: Vec<String>,
            include_links: Option<bool>,
        }

        let params: ContextParams = serde_json::from_value(params)?;

        // Parse memory IDs
        let memory_ids: Result<Vec<MemoryId>> = params
            .memory_ids
            .iter()
            .map(|s| MemoryId::from_string(s).map_err(|e| crate::error::MnemosyneError::InvalidId(e.to_string())))
            .collect();

        let memory_ids = memory_ids?;
        let include_links = params.include_links.unwrap_or(true);

        // Fetch memories
        let mut memories = Vec::new();
        for id in memory_ids {
            match self.storage.get_memory(id).await {
                Ok(memory) => memories.push(memory),
                Err(e) => warn!("Failed to get memory {}: {}", id, e),
            }
        }

        // Optionally fetch linked memories
        if include_links {
            // TODO: Fetch linked memories
        }

        Ok(serde_json::json!({
            "memories": memories,
            "count": memories.len()
        }))
    }

    // === DECIDE Tools ===

    async fn remember(&self, params: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct RememberParams {
            content: String,
            namespace: String,
            importance: Option<u8>,
            context: Option<String>,
        }

        let params: RememberParams = serde_json::from_value(params)?;

        // Parse namespace
        let namespace = self.parse_namespace(&params.namespace)?;

        // Enrich with LLM
        let context = params.context.unwrap_or_else(|| "User-provided memory".to_string());
        let mut memory = self.llm.enrich_memory(&params.content, &context).await?;

        // Override with user-provided values
        memory.namespace = namespace;
        if let Some(importance) = params.importance {
            memory.importance = importance.clamp(1, 10);
        }

        // Store memory
        self.storage.store_memory(&memory).await?;

        Ok(serde_json::json!({
            "memory_id": memory.id.to_string(),
            "summary": memory.summary,
            "importance": memory.importance,
            "tags": memory.tags
        }))
    }

    async fn consolidate(&self, params: Value) -> Result<Value> {
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct ConsolidateParams {
            memory_ids: Option<Vec<String>>,
            namespace: Option<String>,
        }

        let _params: ConsolidateParams = serde_json::from_value(params)?;

        // TODO: Implement consolidation
        Ok(serde_json::json!({
            "message": "Consolidation implementation pending (Phase 5)"
        }))
    }

    // === ACT Tools ===

    async fn update(&self, params: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct UpdateParams {
            memory_id: String,
            content: Option<String>,
            importance: Option<u8>,
            tags: Option<Vec<String>>,
            add_tags: Option<Vec<String>>,
        }

        let params: UpdateParams = serde_json::from_value(params)?;

        // Parse memory ID
        let memory_id = MemoryId::from_string(&params.memory_id)
            .map_err(|e| crate::error::MnemosyneError::InvalidId(e.to_string()))?;

        // Get existing memory
        let mut memory = self.storage.get_memory(memory_id).await?;

        // Apply updates
        if let Some(content) = params.content {
            memory.content = content;
            // TODO: Re-generate embedding
        }

        if let Some(importance) = params.importance {
            memory.importance = importance.clamp(1, 10);
        }

        if let Some(tags) = params.tags {
            memory.tags = tags;
        } else if let Some(add_tags) = params.add_tags {
            for tag in add_tags {
                if !memory.tags.contains(&tag) {
                    memory.tags.push(tag);
                }
            }
        }

        memory.updated_at = chrono::Utc::now();

        // Update storage
        self.storage.update_memory(&memory).await?;

        Ok(serde_json::json!({
            "memory_id": memory.id.to_string(),
            "updated": true
        }))
    }

    async fn delete(&self, params: Value) -> Result<Value> {
        #[derive(Deserialize)]
        struct DeleteParams {
            memory_id: String,
        }

        let params: DeleteParams = serde_json::from_value(params)?;

        // Parse memory ID
        let memory_id = MemoryId::from_string(&params.memory_id)
            .map_err(|e| crate::error::MnemosyneError::InvalidId(e.to_string()))?;

        // Archive (soft delete)
        self.storage.archive_memory(memory_id).await?;

        Ok(serde_json::json!({
            "memory_id": memory_id.to_string(),
            "archived": true
        }))
    }

    // === Helper Methods ===

    fn parse_namespace(&self, namespace_str: &str) -> Result<Namespace> {
        let parts: Vec<&str> = namespace_str.split(':').collect();

        match parts.as_slice() {
            ["global"] => Ok(Namespace::Global),
            ["project", name] => Ok(Namespace::Project {
                name: name.to_string(),
            }),
            ["session", project, session_id] => Ok(Namespace::Session {
                project: project.to_string(),
                session_id: session_id.to_string(),
            }),
            _ => Err(crate::error::MnemosyneError::InvalidNamespace(
                namespace_str.to_string(),
            )),
        }
    }
}
