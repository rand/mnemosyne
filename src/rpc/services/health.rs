//! HealthService implementation

use crate::rpc::generated::health_service_server::HealthService;
use crate::rpc::generated::*;
use std::collections::HashMap;
use tonic::{Request, Response, Status};

pub struct HealthServiceImpl {
    version: String,
    start_time: std::time::Instant,
}

impl HealthServiceImpl {
    pub fn new() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            start_time: std::time::Instant::now(),
        }
    }
}

impl Default for HealthServiceImpl {
    fn default() -> Self {
        Self::new()
    }
}

#[tonic::async_trait]
impl HealthService for HealthServiceImpl {
    async fn health_check(
        &self,
        _request: Request<HealthCheckRequest>,
    ) -> Result<Response<HealthCheckResponse>, Status> {
        let mut components = HashMap::new();
        components.insert("rpc_server".to_string(), "healthy".to_string());
        components.insert("storage".to_string(), "healthy".to_string());

        let response = HealthCheckResponse {
            healthy: true,
            version: self.version.clone(),
            components,
        };

        Ok(Response::new(response))
    }

    async fn get_stats(
        &self,
        _request: Request<GetStatsRequest>,
    ) -> Result<Response<GetStatsResponse>, Status> {
        // TODO: Implement actual stats from storage
        let stats = Stats {
            total_memories: 0,
            total_links: 0,
            namespaces_count: 0,
            memories_by_type: HashMap::new(),
        };

        Ok(Response::new(GetStatsResponse { stats: Some(stats) }))
    }

    async fn get_metrics(
        &self,
        _request: Request<GetMetricsRequest>,
    ) -> Result<Response<GetMetricsResponse>, Status> {
        let uptime = self.start_time.elapsed().as_secs();

        let metrics = Metrics {
            uptime_seconds: uptime,
            requests_total: 0, // TODO: Implement request counter
            requests_errors: 0,
            request_duration_avg_ms: 0.0,
            storage_ops_total: 0,
            memory_usage_bytes: 0,
        };

        Ok(Response::new(GetMetricsResponse {
            metrics: Some(metrics),
        }))
    }

    async fn get_memory_usage(
        &self,
        _request: Request<GetMemoryUsageRequest>,
    ) -> Result<Response<GetMemoryUsageResponse>, Status> {
        // TODO: Implement actual memory usage tracking
        Ok(Response::new(GetMemoryUsageResponse {
            rss_bytes: 0,
            heap_bytes: 0,
            db_size_bytes: 0,
            cache_bytes: 0,
        }))
    }

    type StreamMetricsStream =
        tokio_stream::wrappers::ReceiverStream<Result<MetricsSnapshot, Status>>;

    async fn stream_metrics(
        &self,
        request: Request<StreamMetricsRequest>,
    ) -> Result<Response<Self::StreamMetricsStream>, Status> {
        let interval_ms = request.into_inner().interval_ms.max(100); // Minimum 100ms
        let (tx, rx) = tokio::sync::mpsc::channel(10);

        let start_time = self.start_time;

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_millis(interval_ms as u64));

            loop {
                interval.tick().await;

                let uptime = start_time.elapsed().as_secs();
                let metrics = Metrics {
                    uptime_seconds: uptime,
                    requests_total: 0,
                    requests_errors: 0,
                    request_duration_avg_ms: 0.0,
                    storage_ops_total: 0,
                    memory_usage_bytes: 0,
                };

                let snapshot = MetricsSnapshot {
                    timestamp: chrono::Utc::now().timestamp() as u64,
                    metrics: Some(metrics),
                };

                if tx.send(Ok(snapshot)).await.is_err() {
                    break; // Client disconnected
                }
            }
        });

        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(
            rx,
        )))
    }

    async fn get_version(
        &self,
        _request: Request<GetVersionRequest>,
    ) -> Result<Response<GetVersionResponse>, Status> {
        Ok(Response::new(GetVersionResponse {
            version: self.version.clone(),
            api_version: "v1".to_string(),
            features: vec!["rpc".to_string()],
            build_time: option_env!("BUILD_TIME").unwrap_or("unknown").to_string(),
            git_commit: option_env!("GIT_COMMIT").unwrap_or("dev").to_string(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_check() {
        let service = HealthServiceImpl::new();
        let request = Request::new(HealthCheckRequest {});
        let response = service.health_check(request).await.unwrap();
        assert!(response.into_inner().healthy);
    }

    #[tokio::test]
    async fn test_get_version() {
        let service = HealthServiceImpl::new();
        let request = Request::new(GetVersionRequest {});
        let response = service.get_version(request).await.unwrap();
        let version_response = response.into_inner();
        assert!(!version_response.version.is_empty());
        assert_eq!(version_response.api_version, "v1");
    }
}
