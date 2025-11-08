//! MemoryService implementation

use crate::rpc::generated::memory_service_server::MemoryService;
use crate::rpc::generated::*;
use tonic::{Request, Response, Status};

pub struct MemoryServiceImpl {
    // TODO: Add storage backend
}

impl MemoryServiceImpl {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for MemoryServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[tonic::async_trait]
impl MemoryService for MemoryServiceImpl {
    async fn store_memory(
        &self,
        _request: Request<StoreMemoryRequest>,
    ) -> Result<Response<StoreMemoryResponse>, Status> {
        Err(Status::unimplemented("StoreMemory not yet implemented"))
    }

    async fn get_memory(
        &self,
        _request: Request<GetMemoryRequest>,
    ) -> Result<Response<GetMemoryResponse>, Status> {
        Err(Status::unimplemented("GetMemory not yet implemented"))
    }

    async fn update_memory(
        &self,
        _request: Request<UpdateMemoryRequest>,
    ) -> Result<Response<UpdateMemoryResponse>, Status> {
        Err(Status::unimplemented("UpdateMemory not yet implemented"))
    }

    async fn delete_memory(
        &self,
        _request: Request<DeleteMemoryRequest>,
    ) -> Result<Response<DeleteMemoryResponse>, Status> {
        Err(Status::unimplemented("DeleteMemory not yet implemented"))
    }

    async fn list_memories(
        &self,
        _request: Request<ListMemoriesRequest>,
    ) -> Result<Response<ListMemoriesResponse>, Status> {
        Err(Status::unimplemented("ListMemories not yet implemented"))
    }

    async fn recall(
        &self,
        _request: Request<RecallRequest>,
    ) -> Result<Response<RecallResponse>, Status> {
        Err(Status::unimplemented("Recall not yet implemented"))
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
