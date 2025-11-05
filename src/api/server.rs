//! HTTP API server with SSE support

use super::{
    events::{Event, EventBroadcaster},
    state::{AgentInfo, ContextFile, StateManager},
};
use axum::{
    extract::State,
    http::StatusCode,
    response::{
        sse::{Event as SseEvent, KeepAlive},
        IntoResponse, Sse,
    },
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::{convert::Infallible, net::SocketAddr, sync::Arc};
use tokio_stream::{wrappers::BroadcastStream, StreamExt as _};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{debug, info};

/// API server configuration
#[derive(Debug, Clone)]
pub struct ApiServerConfig {
    /// Server address
    pub addr: SocketAddr,
    /// Event channel capacity
    pub event_capacity: usize,
}

impl Default for ApiServerConfig {
    fn default() -> Self {
        Self {
            addr: ([127, 0, 0, 1], 3000).into(),
            event_capacity: 1000,
        }
    }
}

/// API server state
#[derive(Clone)]
struct AppState {
    /// Event broadcaster
    events: EventBroadcaster,
    /// State manager
    state: Arc<StateManager>,
    /// Instance ID
    instance_id: String,
}

/// API server
pub struct ApiServer {
    config: ApiServerConfig,
    events: EventBroadcaster,
    state: Arc<StateManager>,
    instance_id: String,
}

impl ApiServer {
    /// Create new API server
    pub fn new(config: ApiServerConfig) -> Self {
        let events = EventBroadcaster::new(config.event_capacity);
        let state = Arc::new(StateManager::new());
        let instance_id = uuid::Uuid::new_v4().to_string()[..8].to_string();

        Self {
            config,
            events,
            state,
            instance_id,
        }
    }

    /// Get event broadcaster
    pub fn broadcaster(&self) -> &EventBroadcaster {
        &self.events
    }

    /// Get state manager
    pub fn state_manager(&self) -> &Arc<StateManager> {
        &self.state
    }

    /// Get instance ID
    pub fn instance_id(&self) -> &str {
        &self.instance_id
    }

    /// Build router
    fn build_router(state: AppState) -> Router {
        Router::new()
            // Event streaming
            .route("/events", get(events_handler))
            // State endpoints
            .route("/state/agents", get(list_agents_handler))
            .route("/state/agents", post(update_agent_handler))
            .route("/state/context-files", get(list_context_files_handler))
            .route("/state/context-files", post(update_context_file_handler))
            .route("/state/stats", get(stats_handler))
            // Health check
            .route("/health", get(health_handler))
            // State
            .with_state(state)
            // Middleware
            .layer(CorsLayer::permissive())
            .layer(TraceLayer::new_for_http())
    }

    /// Start serving with dynamic port allocation
    ///
    /// Tries the configured address first, then attempts alternative ports
    /// if the primary port is unavailable (e.g., when multiple instances are running).
    pub async fn serve(self) -> anyhow::Result<()> {
        let state = AppState {
            events: self.events.clone(),
            state: self.state.clone(),
            instance_id: self.instance_id.clone(),
        };

        let router = Self::build_router(state);

        // Try configured address first
        match tokio::net::TcpListener::bind(self.config.addr).await {
            Ok(listener) => {
                info!(
                    "API server [{}] listening on http://{}",
                    self.instance_id, self.config.addr
                );
                info!(
                    "Dashboard: mnemosyne-dash --api http://{}",
                    self.config.addr
                );
                axum::serve(listener, router).await?;
                return Ok(());
            }
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
                debug!(
                    "Port {} in use, trying alternative ports...",
                    self.config.addr.port()
                );
            }
            Err(e) => return Err(e.into()),
        }

        // Try alternative ports (3001-3010)
        let base_port = self.config.addr.port();
        for offset in 1..=10 {
            let alt_port = base_port + offset;
            let alt_addr = SocketAddr::new(self.config.addr.ip(), alt_port);

            match tokio::net::TcpListener::bind(alt_addr).await {
                Ok(listener) => {
                    info!(
                        "API server [{}] listening on http://{}",
                        self.instance_id, alt_addr
                    );
                    info!(
                        "Dashboard: mnemosyne-dash --api http://{}",
                        alt_addr
                    );
                    axum::serve(listener, router).await?;
                    return Ok(());
                }
                Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
                    continue;
                }
                Err(e) => return Err(e.into()),
            }
        }

        Err(anyhow::anyhow!(
            "All ports ({}–{}) are in use. API server unavailable for instance {}. \
             Core functionality not affected.",
            base_port,
            base_port + 10,
            self.instance_id
        ))
    }
}

/// SSE events handler
async fn events_handler(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<SseEvent, Infallible>>> {
    debug!("New SSE client connected");

    let rx = state.events.subscribe();
    let stream = BroadcastStream::new(rx);

    let event_stream = stream.filter_map(|result| match result {
        Ok(event) => {
            // Convert Event to SSE Event
            let data = serde_json::to_string(&event).ok()?;
            Some(Ok(SseEvent::default().data(data).id(event.id)))
        }
        Err(_) => None, // Skip lagged messages
    });

    Sse::new(event_stream).keep_alive(KeepAlive::default())
}

/// List agents handler
async fn list_agents_handler(State(state): State<AppState>) -> Json<Vec<AgentInfo>> {
    let agents = state.state.list_agents().await;
    Json(agents)
}

/// Update agent handler
#[derive(Debug, Deserialize)]
struct UpdateAgentRequest {
    agent: AgentInfo,
}

async fn update_agent_handler(
    State(state): State<AppState>,
    Json(req): Json<UpdateAgentRequest>,
) -> StatusCode {
    let agent_id = req.agent.id.clone();
    state.state.update_agent(req.agent).await;

    // Broadcast agent updated event
    let event = Event::agent_started(agent_id);
    let _ = state.events.broadcast(event);

    StatusCode::OK
}

/// List context files handler
async fn list_context_files_handler(State(state): State<AppState>) -> Json<Vec<ContextFile>> {
    let files = state.state.list_context_files().await;
    Json(files)
}

/// Update context file handler
#[derive(Debug, Deserialize)]
struct UpdateContextFileRequest {
    file: ContextFile,
}

async fn update_context_file_handler(
    State(state): State<AppState>,
    Json(req): Json<UpdateContextFileRequest>,
) -> StatusCode {
    state.state.update_context_file(req.file.clone()).await;

    // Broadcast context modified event
    let event = Event::context_modified(req.file.path);
    let _ = state.events.broadcast(event);

    StatusCode::OK
}

/// Stats handler
async fn stats_handler(State(state): State<AppState>) -> impl IntoResponse {
    let stats = state.state.stats().await;
    Json(stats)
}

/// Health check handler
#[derive(Debug, Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    instance_id: String,
    subscribers: usize,
}

async fn health_handler(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        instance_id: state.instance_id.clone(),
        subscribers: state.events.subscriber_count(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_creation() {
        let config = ApiServerConfig::default();
        let server = ApiServer::new(config);
        assert_eq!(server.broadcaster().subscriber_count(), 0);
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let state = AppState {
            events: EventBroadcaster::default(),
            state: Arc::new(StateManager::new()),
            instance_id: "test-instance".to_string(),
        };

        let response = health_handler(State(state)).await;
        assert_eq!(response.0.status, "ok");
        assert_eq!(response.0.instance_id, "test-instance");
    }
}
