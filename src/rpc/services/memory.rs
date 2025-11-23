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
            memory_type: req
                .memory_type
                .map(memory_type_from_proto)
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
        self.storage
            .store_memory(&memory)
            .await
            .map_err(|e| Status::from(e))?;

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
        let memory = self
            .storage
            .get_memory(memory_id)
            .await
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
        let mut memory = self
            .storage
            .get_memory(memory_id)
            .await
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
        self.storage
            .update_memory(&memory)
            .await
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
        self.storage
            .archive_memory(memory_id)
            .await
            .map_err(|e| Status::from(e))?;

        Ok(Response::new(DeleteMemoryResponse { success: true }))
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
        let memories = self
            .storage
            .list_memories(
                namespace,
                req.limit.min(1000) as usize, // Cap at 1000
                sort_by,
            )
            .await
            .map_err(|e| Status::from(e))?;

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
        use crate::rpc::conversions::namespace_from_proto;

        let req = request.into_inner();

        // Convert namespace if provided
        let namespace = match req.namespace {
            Some(ns) => Some(namespace_from_proto(ns)?),
            None => None,
        };

        // Perform hybrid search
        let results = self
            .storage
            .hybrid_search(
                &req.query,
                namespace,
                req.max_results.min(1000) as usize,
                true, // expand_graph
            )
            .await
            .map_err(|e| Status::from(e))?;

        let total_matches = results.len();

        // Convert to proto SearchResults
        let proto_results: Vec<generated::SearchResult> = results
            .into_iter()
            .map(|result| {
                generated::SearchResult {
                    memory: Some(crate::rpc::conversions::memory_note_to_proto(result.memory)),
                    score: result.score,
                    semantic_score: None, // Not available in current SearchResult
                    fts_score: None,      // Not available in current SearchResult
                    graph_score: None,    // Not available in current SearchResult
                }
            })
            .collect();

        Ok(Response::new(RecallResponse {
            results: proto_results,
            query: req.query,
            total_matches: total_matches as u32,
        }))
    }

    async fn semantic_search(
        &self,
        request: Request<SemanticSearchRequest>,
    ) -> Result<Response<SemanticSearchResponse>, Status> {
        use crate::rpc::conversions::{memory_note_to_proto, namespace_from_proto};

        let req = request.into_inner();

        // Validate embedding vector
        if req.embedding.is_empty() {
            return Err(Status::invalid_argument("Embedding vector is required"));
        }

        // Convert namespace if provided
        let namespace = match req.namespace {
            Some(ns) => Some(namespace_from_proto(ns)?),
            None => None,
        };

        // Perform vector similarity search
        let results = self
            .storage
            .vector_search(
                &req.embedding,
                req.max_results.min(1000) as usize,
                namespace,
            )
            .await
            .map_err(|e| Status::from(e))?;

        // Convert to proto SearchResults
        let proto_results: Vec<generated::SearchResult> = results
            .into_iter()
            .map(|result| {
                generated::SearchResult {
                    memory: Some(memory_note_to_proto(result.memory)),
                    score: result.score,
                    semantic_score: Some(result.score), // For semantic search, score IS semantic score
                    fts_score: None,
                    graph_score: None,
                }
            })
            .collect();

        Ok(Response::new(SemanticSearchResponse {
            results: proto_results,
        }))
    }

    async fn graph_traverse(
        &self,
        request: Request<GraphTraverseRequest>,
    ) -> Result<Response<GraphTraverseResponse>, Status> {
        use crate::rpc::conversions::memory_note_to_proto;

        let req = request.into_inner();

        // Validate seed IDs
        if req.seed_ids.is_empty() {
            return Err(Status::invalid_argument("At least one seed ID is required"));
        }

        // Parse seed IDs
        let seed_ids: Vec<MemoryId> = req
            .seed_ids
            .iter()
            .map(|id| MemoryId::from_string(id))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| Status::invalid_argument("Invalid memory ID format"))?;

        // Validate max_hops
        let max_hops = if req.max_hops == 0 {
            2 // Default
        } else {
            req.max_hops.min(5) as usize // Cap at 5
        };

        // Perform graph traversal
        let memories = self
            .storage
            .graph_traverse(
                &seed_ids, max_hops, None, // namespace filter not used in graph_traverse
            )
            .await
            .map_err(|e| Status::from(e))?;

        // Build edges from memory links
        let mut edges = Vec::new();
        for memory in &memories {
            for link in &memory.links {
                edges.push(generated::GraphEdge {
                    source_id: memory.id.to_string(),
                    target_id: link.target_id.to_string(),
                    link_type: crate::rpc::conversions::link_type_to_proto(link.link_type),
                    strength: link.strength,
                });
            }
        }

        // Convert memories to proto
        let proto_memories: Vec<generated::MemoryNote> =
            memories.into_iter().map(memory_note_to_proto).collect();

        Ok(Response::new(GraphTraverseResponse {
            memories: proto_memories,
            edges,
        }))
    }

    async fn get_context(
        &self,
        request: Request<GetContextRequest>,
    ) -> Result<Response<GetContextResponse>, Status> {
        use crate::rpc::conversions::memory_note_to_proto;
        use std::collections::HashSet;

        let req = request.into_inner();

        // Validate memory IDs
        if req.memory_ids.is_empty() {
            return Err(Status::invalid_argument(
                "At least one memory ID is required",
            ));
        }

        // Parse memory IDs
        let memory_ids: Vec<MemoryId> = req
            .memory_ids
            .iter()
            .map(|id| MemoryId::from_string(id))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| Status::invalid_argument("Invalid memory ID format"))?;

        // Fetch requested memories
        let mut memories = Vec::new();
        for id in &memory_ids {
            match self.storage.get_memory(*id).await {
                Ok(memory) => memories.push(memory),
                Err(_) => {
                    // Skip memories that don't exist (don't fail entire request)
                    continue;
                }
            }
        }

        if memories.is_empty() {
            return Err(Status::not_found("No memories found with provided IDs"));
        }

        // Optionally fetch linked memories
        let linked_memories = if req.include_links {
            let max_depth = if req.max_linked_depth == 0 {
                1
            } else {
                req.max_linked_depth.min(3) as usize // Cap at 3
            };

            // Use graph traversal to get linked memories
            let all_memories = self
                .storage
                .graph_traverse(&memory_ids, max_depth, None)
                .await
                .map_err(|e| Status::from(e))?;

            // Filter out the original memories (already in 'memories')
            let original_ids: HashSet<MemoryId> = memory_ids.iter().copied().collect();
            all_memories
                .into_iter()
                .filter(|m| !original_ids.contains(&m.id))
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };

        // Build edges from all memories (requested + linked)
        let mut edges = Vec::new();
        let all_memories_for_edges = memories.iter().chain(linked_memories.iter());

        for memory in all_memories_for_edges {
            for link in &memory.links {
                edges.push(generated::GraphEdge {
                    source_id: memory.id.to_string(),
                    target_id: link.target_id.to_string(),
                    link_type: crate::rpc::conversions::link_type_to_proto(link.link_type),
                    strength: link.strength,
                });
            }
        }

        // Convert to proto
        let proto_memories: Vec<generated::MemoryNote> =
            memories.into_iter().map(memory_note_to_proto).collect();

        let proto_linked_memories: Vec<generated::MemoryNote> = linked_memories
            .into_iter()
            .map(memory_note_to_proto)
            .collect();

        Ok(Response::new(GetContextResponse {
            memories: proto_memories,
            linked_memories: proto_linked_memories,
            edges,
        }))
    }

    type RecallStreamStream = tokio_stream::wrappers::ReceiverStream<Result<SearchResult, Status>>;

    async fn recall_stream(
        &self,
        request: Request<RecallRequest>,
    ) -> Result<Response<Self::RecallStreamStream>, Status> {
        use crate::rpc::conversions::namespace_from_proto;

        let req = request.into_inner();

        // Convert namespace if provided
        let namespace = match req.namespace {
            Some(ns) => Some(namespace_from_proto(ns)?),
            None => None,
        };

        // Perform hybrid search (same as Recall)
        let results = self
            .storage
            .hybrid_search(
                &req.query,
                namespace,
                req.max_results.min(1000) as usize,
                true, // expand_graph
            )
            .await
            .map_err(|e| Status::from(e))?;

        // Create a channel for streaming results
        let (tx, rx) = tokio::sync::mpsc::channel(100);

        // Spawn task to send results progressively
        tokio::spawn(async move {
            for result in results {
                let proto_result = generated::SearchResult {
                    memory: Some(crate::rpc::conversions::memory_note_to_proto(result.memory)),
                    score: result.score,
                    semantic_score: None,
                    fts_score: None,
                    graph_score: None,
                };

                if tx.send(Ok(proto_result)).await.is_err() {
                    // Client disconnected
                    break;
                }
            }
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(
            rx,
        )))
    }

    type ListMemoriesStreamStream =
        tokio_stream::wrappers::ReceiverStream<Result<MemoryNote, Status>>;

    async fn list_memories_stream(
        &self,
        request: Request<ListMemoriesRequest>,
    ) -> Result<Response<Self::ListMemoriesStreamStream>, Status> {
        use crate::rpc::conversions::namespace_from_proto;
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
        let memories = self
            .storage
            .list_memories(namespace, req.limit.min(1000) as usize, sort_by)
            .await
            .map_err(|e| Status::from(e))?;

        // Create a channel for streaming results
        let (tx, rx) = tokio::sync::mpsc::channel(100);

        // Spawn task to send memories progressively
        tokio::spawn(async move {
            for memory in memories {
                let proto_memory = crate::rpc::conversions::memory_note_to_proto(memory);

                if tx.send(Ok(proto_memory)).await.is_err() {
                    // Client disconnected
                    break;
                }
            }
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(
            rx,
        )))
    }

    type StoreMemoryStreamStream =
        tokio_stream::wrappers::ReceiverStream<Result<StoreMemoryProgress, Status>>;

    async fn store_memory_stream(
        &self,
        request: Request<StoreMemoryRequest>,
    ) -> Result<Response<Self::StoreMemoryStreamStream>, Status> {
        use crate::rpc::conversions::*;

        let req = request.into_inner();

        // Convert namespace
        let namespace = match req.namespace {
            Some(ns) => namespace_from_proto(ns)?,
            None => return Err(Status::invalid_argument("namespace is required")),
        };

        // Create channel for progress updates
        let (tx, rx) = tokio::sync::mpsc::channel(10);

        // Clone necessary data for spawned task
        let storage = self.storage.clone();
        let llm = self.llm.clone();
        let content = req.content.clone();
        let tags = req.tags.clone();
        let context = req.context.clone().unwrap_or_default();
        let memory_type_proto = req.memory_type;
        let importance = req.importance;
        let skip_enrichment = req.skip_llm_enrichment;

        // Spawn async task to perform storage with progress updates
        tokio::spawn(async move {
            // Stage 1: Preparation (10%)
            let _ = tx
                .send(Ok(StoreMemoryProgress {
                    stage: "preparing".to_string(),
                    percent: 10,
                    memory_id: None,
                    memory: None,
                }))
                .await;

            // Create memory ID and base memory
            let memory_id = MemoryId::new();
            let now = chrono::Utc::now();

            // Stage 2: LLM Enrichment (30-60%)
            if !skip_enrichment && llm.is_some() {
                let _ = tx
                    .send(Ok(StoreMemoryProgress {
                        stage: "enriching".to_string(),
                        percent: 30,
                        memory_id: None,
                        memory: None,
                    }))
                    .await;

                // TODO: Implement LLM enrichment when llm service is available
                // For now, skip enrichment
            }

            // Stage 3: Embedding generation (60-80%)
            let _ = tx
                .send(Ok(StoreMemoryProgress {
                    stage: "embedding".to_string(),
                    percent: 60,
                    memory_id: None,
                    memory: None,
                }))
                .await;

            // Create memory note
            let memory = InternalMemoryNote {
                id: memory_id,
                namespace,
                created_at: now,
                updated_at: now,
                content,
                summary: String::new(),
                keywords: vec![],
                tags,
                context,
                memory_type: memory_type_proto
                    .map(memory_type_from_proto)
                    .unwrap_or(InternalMemoryType::Insight),
                importance: importance.unwrap_or(5) as u8,
                confidence: 0.8,
                links: vec![],
                related_files: vec![],
                related_entities: vec![],
                access_count: 0,
                last_accessed_at: now,
                expires_at: None,
                is_archived: false,
                superseded_by: None,
                embedding: None,
                embedding_model: String::new(),
            };

            // Stage 4: Indexing (80-100%)
            let _ = tx
                .send(Ok(StoreMemoryProgress {
                    stage: "indexing".to_string(),
                    percent: 80,
                    memory_id: None,
                    memory: None,
                }))
                .await;

            // Store in backend
            match storage.store_memory(&memory).await {
                Ok(_) => {
                    // Stage 5: Complete
                    let proto_memory = memory_note_to_proto(memory);
                    let _ = tx
                        .send(Ok(StoreMemoryProgress {
                            stage: "complete".to_string(),
                            percent: 100,
                            memory_id: Some(memory_id.to_string()),
                            memory: Some(proto_memory),
                        }))
                        .await;
                }
                Err(e) => {
                    let _ = tx.send(Err(Status::from(e))).await;
                }
            }
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(
            rx,
        )))
    }
}
