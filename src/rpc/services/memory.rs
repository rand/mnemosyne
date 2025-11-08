//! MemoryService implementation

use crate::rpc::generated::memory_service_server::MemoryService;
use crate::rpc::generated::{self as generated, *};
use crate::services::LlmService;
use crate::storage::StorageBackend;
use crate::types::{MemoryId, MemoryNote as InternalMemoryNote, MemoryType as InternalMemoryType};
use std::sync::Arc;
use tonic::{Request, Response, Status};

pub struct MemoryServiceImpl {
    storage: Arc<dyn StorageBackend>,
    llm: Option<Arc<LlmService>>,
}

impl MemoryServiceImpl {
    pub fn new(storage: Arc<dyn StorageBackend>, llm: Option<Arc<LlmService>>) -> Self {
        Self { storage, llm }
    }
}

#[tonic::async_trait]
impl MemoryService for MemoryServiceImpl {
    async fn store_memory(
        &self,
        request: Request<StoreMemoryRequest>,
    ) -> Result<Response<StoreMemoryResponse>, Status> {
        use crate::rpc::conversions::*;

        let req = request.into_inner();

        // Convert namespace
        let namespace = match req.namespace {
            Some(ns) => namespace_from_proto(ns)?,
            None => return Err(Status::invalid_argument("namespace is required")),
        };

        // Create memory note
        let memory_id = MemoryId::new();
        let now = chrono::Utc::now();

        let memory = InternalMemoryNote {
            id: memory_id,
            namespace,
            created_at: now,
            updated_at: now,
            content: req.content,
            summary: String::new(), // Will be filled by LLM if available
            keywords: vec![],       // Will be filled by LLM if available
            tags: req.tags,
            context: req.context.unwrap_or_default(),
            memory_type: req.memory_type.map(memory_type_from_proto)
                .unwrap_or(InternalMemoryType::Insight),
            importance: req.importance.unwrap_or(5) as u8,
            confidence: 0.8, // Default confidence
            links: vec![],
            related_files: vec![],
            related_entities: vec![],
            access_count: 0,
            last_accessed_at: now,
            expires_at: None,
            is_archived: false,
            superseded_by: None,
            embedding: None, // Will be filled later if needed
            embedding_model: String::new(),
        };

        // TODO: LLM enrichment if skip_llm_enrichment is false and llm is available
        // For now, just store as-is

        // Store in backend
        self.storage.store_memory(&memory).await.map_err(|e| Status::from(e))?;

        Ok(Response::new(StoreMemoryResponse {
            memory_id: memory_id.to_string(),
            memory: Some(memory_note_to_proto(memory)),
        }))
    }

    async fn get_memory(
        &self,
        request: Request<GetMemoryRequest>,
    ) -> Result<Response<GetMemoryResponse>, Status> {
        use crate::rpc::conversions::memory_note_to_proto;

        let req = request.into_inner();

        // Parse memory ID
        let memory_id = MemoryId::from_string(&req.memory_id)
            .map_err(|_| Status::invalid_argument("Invalid memory ID format"))?;

        // Get from storage
        let memory = self.storage.get_memory(memory_id).await
            .map_err(|e| Status::from(e))?;

        // Increment access counter
        let _ = self.storage.increment_access(memory_id).await;

        Ok(Response::new(GetMemoryResponse {
            memory: Some(memory_note_to_proto(memory)),
        }))
    }

    async fn update_memory(
        &self,
        request: Request<UpdateMemoryRequest>,
    ) -> Result<Response<UpdateMemoryResponse>, Status> {
        use crate::rpc::conversions::memory_note_to_proto;

        let req = request.into_inner();

        // Parse memory ID
        let memory_id = MemoryId::from_string(&req.memory_id)
            .map_err(|_| Status::invalid_argument("Invalid memory ID format"))?;

        // Get existing memory
        let mut memory = self.storage.get_memory(memory_id).await
            .map_err(|e| Status::from(e))?;

        // Update fields
        if let Some(content) = req.content {
            memory.content = content;
            memory.updated_at = chrono::Utc::now();
        }

        if let Some(importance) = req.importance {
            memory.importance = importance as u8;
        }

        // Handle tag operations
        if !req.tags.is_empty() {
            memory.tags = req.tags;
        }
        if !req.add_tags.is_empty() {
            for tag in req.add_tags {
                if !memory.tags.contains(&tag) {
                    memory.tags.push(tag);
                }
            }
        }
        for tag in &req.remove_tags {
            memory.tags.retain(|t| t != tag);
        }

        // Store updated memory
        self.storage.update_memory(&memory).await
            .map_err(|e| Status::from(e))?;

        Ok(Response::new(UpdateMemoryResponse {
            memory: Some(memory_note_to_proto(memory)),
        }))
    }

    async fn delete_memory(
        &self,
        request: Request<DeleteMemoryRequest>,
    ) -> Result<Response<DeleteMemoryResponse>, Status> {
        let req = request.into_inner();

        // Parse memory ID
        let memory_id = MemoryId::from_string(&req.memory_id)
            .map_err(|_| Status::invalid_argument("Invalid memory ID format"))?;

        // Archive (soft delete) the memory
        self.storage.archive_memory(memory_id).await
            .map_err(|e| Status::from(e))?;

        Ok(Response::new(DeleteMemoryResponse {
            success: true,
        }))
    }

    async fn list_memories(
        &self,
        request: Request<ListMemoriesRequest>,
    ) -> Result<Response<ListMemoriesResponse>, Status> {
        use crate::rpc::conversions::{memory_note_to_proto, namespace_from_proto};
        use crate::storage::MemorySortOrder;

        let req = request.into_inner();

        // Convert namespace if provided
        let namespace = match req.namespace {
            Some(ns) => Some(namespace_from_proto(ns)?),
            None => None,
        };

        // Convert sort order
        let sort_by = match req.sort_by.as_str() {
            "importance" => MemorySortOrder::Importance,
            "access_count" => MemorySortOrder::AccessCount,
            _ => MemorySortOrder::Recent,
        };

        // List memories
        let memories = self.storage.list_memories(
            namespace,
            req.limit.min(1000) as usize, // Cap at 1000
            sort_by,
        ).await.map_err(|e| Status::from(e))?;

        let total_count = memories.len();

        Ok(Response::new(ListMemoriesResponse {
            memories: memories.into_iter().map(memory_note_to_proto).collect(),
            total_count: total_count as u32,
            has_more: false, // TODO: Implement pagination
        }))
    }

    async fn recall(
        &self,
        request: Request<RecallRequest>,
    ) -> Result<Response<RecallResponse>, Status> {
        use crate::rpc::conversions::{namespace_from_proto};

        let req = request.into_inner();

        // Convert namespace if provided
        let namespace = match req.namespace {
            Some(ns) => Some(namespace_from_proto(ns)?),
            None => None,
        };

        // Perform hybrid search
        let results = self.storage.hybrid_search(
            &req.query,
            namespace,
            req.max_results.min(1000) as usize,
            true, // expand_graph
        ).await.map_err(|e| Status::from(e))?;

        let total_matches = results.len();

        // Convert to proto SearchResults
        let proto_results: Vec<generated::SearchResult> = results.into_iter().map(|result| {
            generated::SearchResult {
                memory: Some(crate::rpc::conversions::memory_note_to_proto(result.memory)),
                score: result.score,
                semantic_score: None, // Not available in current SearchResult
                fts_score: None,      // Not available in current SearchResult
                graph_score: None,    // Not available in current SearchResult
            }
        }).collect();

        Ok(Response::new(RecallResponse {
            results: proto_results,
            query: req.query,
            total_matches: total_matches as u32,
        }))
    }

    async fn semantic_search(
        &self,
        _request: Request<SemanticSearchRequest>,
    ) -> Result<Response<SemanticSearchResponse>, Status> {
        Err(Status::unimplemented("SemanticSearch not yet implemented"))
    }

    async fn graph_traverse(
        &self,
        _request: Request<GraphTraverseRequest>,
    ) -> Result<Response<GraphTraverseResponse>, Status> {
        Err(Status::unimplemented("GraphTraverse not yet implemented"))
    }

    async fn get_context(
        &self,
        _request: Request<GetContextRequest>,
    ) -> Result<Response<GetContextResponse>, Status> {
        Err(Status::unimplemented("GetContext not yet implemented"))
    }

    type RecallStreamStream = tokio_stream::wrappers::ReceiverStream<Result<SearchResult, Status>>;

    async fn recall_stream(
        &self,
        _request: Request<RecallRequest>,
    ) -> Result<Response<Self::RecallStreamStream>, Status> {
        Err(Status::unimplemented("RecallStream not yet implemented"))
    }

    type ListMemoriesStreamStream = tokio_stream::wrappers::ReceiverStream<Result<MemoryNote, Status>>;

    async fn list_memories_stream(
        &self,
        _request: Request<ListMemoriesRequest>,
    ) -> Result<Response<Self::ListMemoriesStreamStream>, Status> {
        Err(Status::unimplemented("ListMemoriesStream not yet implemented"))
    }

    type StoreMemoryStreamStream = tokio_stream::wrappers::ReceiverStream<Result<StoreMemoryProgress, Status>>;

    async fn store_memory_stream(
        &self,
        _request: Request<StoreMemoryRequest>,
    ) -> Result<Response<Self::StoreMemoryStreamStream>, Status> {
        Err(Status::unimplemented("StoreMemoryStream not yet implemented"))
    }
}
