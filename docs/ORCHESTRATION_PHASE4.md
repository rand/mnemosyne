# Phase 4: Enhanced Orchestration Coordination

This document details the deferred Phase 4 enhancements for the multi-agent orchestration system.

## Status: DEFERRED (Not Blocking MVP)

Phase 4 features are enhancements to the fully-functional orchestration system completed in Phases 1-3. The current system provides all essential orchestration capabilities. These enhancements add scalability, observability, and advanced coordination for production workloads.

**When to implement**: After production validation of core orchestration features and when specific scaling/coordination needs arise.

**Estimated effort**: 12-16 hours total

---

## 4.1: Dynamic Agent Scaling (4-5 hours)

### Goal
Automatically scale agent pools based on workload, spawning/terminating agents dynamically.

### Current State
- Fixed pool: 4 agents (Orchestrator, Optimizer, Reviewer, Executor)
- Single instance of each agent type
- Manual scaling only (restart with different config)

### Enhancements Needed

#### Agent Pool Management
```rust
// src/orchestration/scaling/pool.rs

pub struct AgentPool {
    /// Agent type being managed
    agent_type: AgentRole,

    /// Minimum pool size
    min_size: usize,

    /// Maximum pool size
    max_size: usize,

    /// Active agents
    active_agents: Vec<ActorRef<AgentMessage>>,

    /// Idle agents (ready for work)
    idle_agents: VecDeque<ActorRef<AgentMessage>>,

    /// Scaling policy
    policy: ScalingPolicy,
}

pub enum ScalingPolicy {
    /// Scale based on work queue depth
    QueueDepth {
        scale_up_threshold: usize,
        scale_down_threshold: usize,
    },

    /// Scale based on CPU/memory usage
    ResourceBased {
        cpu_threshold: f32,
        memory_threshold: f32,
    },

    /// Scale based on work completion rate
    ThroughputBased {
        target_items_per_minute: usize,
    },
}

impl AgentPool {
    /// Spawn new agent if needed
    async fn scale_up(&mut self) -> Result<()>;

    /// Terminate idle agent if possible
    async fn scale_down(&mut self) -> Result<()>;

    /// Get next available agent from pool
    fn get_agent(&mut self) -> Option<ActorRef<AgentMessage>>;

    /// Return agent to idle pool
    fn return_agent(&mut self, agent: ActorRef<AgentMessage>);
}
```

#### Load Balancing
```rust
// src/orchestration/scaling/load_balancer.rs

pub struct LoadBalancer {
    /// Work assignment strategy
    strategy: LoadBalancingStrategy,

    /// Agent load tracking
    agent_loads: HashMap<ActorRef<AgentMessage>, WorkLoad>,
}

pub enum LoadBalancingStrategy {
    /// Round-robin assignment
    RoundRobin,

    /// Assign to least-loaded agent
    LeastLoaded,

    /// Assign based on agent specialization
    SpecializationBased,

    /// Work stealing from overloaded agents
    WorkStealing,
}

pub struct WorkLoad {
    /// Current work items assigned
    current_items: usize,

    /// Average completion time
    avg_completion_ms: u64,

    /// Success rate
    success_rate: f32,
}
```

#### Resource Monitoring
```rust
// src/orchestration/scaling/monitor.rs

pub struct ResourceMonitor {
    /// CPU usage per agent
    cpu_usage: HashMap<AgentRole, f32>,

    /// Memory usage per agent
    memory_usage: HashMap<AgentRole, usize>,

    /// Work queue depth
    queue_depth: usize,

    /// Monitoring interval
    interval: Duration,
}

impl ResourceMonitor {
    /// Collect current resource metrics
    async fn collect_metrics(&mut self) -> Metrics;

    /// Determine if scaling is needed
    fn should_scale(&self) -> ScalingDecision;

    /// Get agent utilization percentage
    fn agent_utilization(&self, agent: &AgentRole) -> f32;
}
```

### Testing Requirements
- [ ] Scale up under load (queue depth > threshold)
- [ ] Scale down when idle (agents sitting idle for N minutes)
- [ ] Load balancing across agent pool
- [ ] Resource limits respected (max pool size)
- [ ] Graceful agent termination (finish current work)

### Configuration
```toml
[orchestration.scaling]
enabled = true

[orchestration.scaling.executor]
min_pool_size = 1
max_pool_size = 8
scale_up_threshold = 10  # work items in queue
scale_down_threshold = 2
idle_timeout_mins = 5

[orchestration.scaling.optimizer]
min_pool_size = 1
max_pool_size = 3
```

---

## 4.2: Cross-Session Coordination (3-4 hours)

### Goal
Persist work state across orchestration sessions, enabling resumption after restarts.

### Current State
- Event sourcing enables crash recovery within a session
- Work queue is in-memory only
- Session restart loses pending work

### Enhancements Needed

#### Persistent Work Queue
```rust
// src/orchestration/persistence/work_queue.rs

pub struct PersistentWorkQueue {
    /// In-memory work queue
    queue: WorkQueue,

    /// Storage backend
    storage: Arc<dyn StorageBackend>,

    /// Namespace for this session
    namespace: Namespace,
}

impl PersistentWorkQueue {
    /// Save work queue state to storage
    async fn persist(&self) -> Result<()>;

    /// Load work queue from storage
    async fn restore(&mut self) -> Result<()>;

    /// Checkpoint every N operations
    async fn checkpoint(&self) -> Result<()>;
}
```

#### Session Handoff Protocol
```rust
// src/orchestration/persistence/session.rs

pub struct SessionManager {
    /// Current session ID
    current_session: String,

    /// Previous session ID (if resuming)
    previous_session: Option<String>,
}

impl SessionManager {
    /// Start new session
    async fn start_session(&mut self) -> Result<String>;

    /// Resume from previous session
    async fn resume_session(&mut self, session_id: &str) -> Result<()>;

    /// Gracefully end session
    async fn end_session(&mut self) -> Result<()>;

    /// Check for orphaned sessions (crashed without cleanup)
    async fn detect_orphaned_sessions(&self) -> Result<Vec<String>>;

    /// Recover work from orphaned session
    async fn recover_orphaned_work(&self, session_id: &str) -> Result<Vec<WorkItem>>;
}
```

#### State Reconciliation
```rust
// src/orchestration/persistence/reconciliation.rs

pub struct StateReconciliator {
    /// Event log
    events: Arc<EventReplay>,
}

impl StateReconciliator {
    /// Reconcile in-memory state with persisted events
    async fn reconcile(&self, current_state: &WorkQueue) -> Result<WorkQueue>;

    /// Detect and resolve state conflicts
    async fn resolve_conflicts(&self, local: &WorkQueue, remote: &WorkQueue) -> Result<WorkQueue>;

    /// Verify state consistency
    fn verify_consistency(&self, state: &WorkQueue) -> Result<()>;
}
```

### Testing Requirements
- [ ] Work queue persists across restarts
- [ ] Resume incomplete work items
- [ ] Detect duplicate work (avoid re-running completed items)
- [ ] Handle session crashes (orphaned work recovery)
- [ ] State reconciliation after network partition

### Configuration
```toml
[orchestration.persistence]
enabled = true
checkpoint_interval_secs = 60
auto_resume = true
orphan_timeout_mins = 30
```

---

## 4.3: Distributed P2P Orchestration (4-5 hours)

### Goal
Extend orchestration across multiple machines using Iroh P2P networking.

### Current State
- Iroh network layer scaffolded (`src/orchestration/network/`)
- Single-machine operation only
- No cross-machine work distribution

### Enhancements Needed

#### Distributed Work Queue
```rust
// src/orchestration/distributed/work_queue.rs

pub struct DistributedWorkQueue {
    /// Local work queue
    local_queue: WorkQueue,

    /// Remote peers
    peers: HashMap<NodeId, PeerInfo>,

    /// Work distribution strategy
    strategy: DistributionStrategy,
}

pub enum DistributionStrategy {
    /// Partition work by namespace
    NamespacePartitioning,

    /// Distribute based on agent specialization
    SpecializationBased,

    /// Work stealing from overloaded peers
    WorkStealing {
        steal_threshold: usize,
    },
}

impl DistributedWorkQueue {
    /// Submit work to distributed queue
    async fn submit_distributed(&mut self, work: WorkItem) -> Result<()>;

    /// Pull work from remote peer
    async fn steal_work(&mut self, peer: &NodeId) -> Result<Option<WorkItem>>;

    /// Broadcast work completion
    async fn broadcast_completion(&self, work_id: &WorkItemId) -> Result<()>;
}
```

#### Consensus Protocol
```rust
// src/orchestration/distributed/consensus.rs

pub struct ConsensusManager {
    /// Current leader
    leader: Option<NodeId>,

    /// Leader election state
    election_state: ElectionState,
}

pub enum ElectionState {
    Follower,
    Candidate { votes_received: usize },
    Leader { term: u64 },
}

impl ConsensusManager {
    /// Start leader election
    async fn start_election(&mut self) -> Result<()>;

    /// Handle election vote request
    async fn handle_vote_request(&mut self, candidate: NodeId) -> Result<bool>;

    /// Heartbeat to maintain leadership
    async fn send_heartbeat(&self) -> Result<()>;

    /// Determine current leader
    fn current_leader(&self) -> Option<NodeId>;
}
```

#### Network Partition Handling
```rust
// src/orchestration/distributed/partition.rs

pub struct PartitionDetector {
    /// Last heartbeat from each peer
    last_heartbeat: HashMap<NodeId, Instant>,

    /// Partition detection threshold
    timeout: Duration,
}

impl PartitionDetector {
    /// Detect network partition
    fn detect_partition(&self) -> Vec<NodeId>;

    /// Handle partition (pause/resume work)
    async fn handle_partition(&mut self, isolated_peers: Vec<NodeId>) -> Result<()>;

    /// Reconcile state after partition heals
    async fn reconcile_after_partition(&mut self) -> Result<()>;
}
```

#### Work Stealing
```rust
// src/orchestration/distributed/stealing.rs

pub struct WorkStealingCoordinator {
    /// Threshold for stealing work
    steal_threshold: usize,

    /// Backoff strategy
    backoff: ExponentialBackoff,
}

impl WorkStealingCoordinator {
    /// Attempt to steal work from overloaded peer
    async fn try_steal(&mut self, from_peer: &NodeId) -> Result<Option<WorkItem>>;

    /// Advertise available work
    async fn advertise_work(&self, work_count: usize) -> Result<()>;

    /// Handle steal request from peer
    async fn handle_steal_request(&mut self, from_peer: &NodeId) -> Result<Option<WorkItem>>;
}
```

### Testing Requirements
- [ ] Multi-node work distribution
- [ ] Leader election (1 Orchestrator across cluster)
- [ ] Work stealing under load imbalance
- [ ] Network partition detection and recovery
- [ ] State reconciliation after partition heals
- [ ] Graceful node join/leave

### Configuration
```toml
[orchestration.distributed]
enabled = false  # Single-machine by default
max_peers = 10
heartbeat_interval_secs = 5
partition_timeout_secs = 30
work_stealing_threshold = 20
```

---

## 4.4: Observability & Monitoring (3-4 hours)

### Goal
Real-time visibility into orchestration operations for debugging and optimization.

### Current State
- Event sourcing provides audit trail
- No real-time dashboards
- Limited metrics collection
- Tracing via `tracing` crate only

### Enhancements Needed

#### Metrics Collection
```rust
// src/orchestration/observability/metrics.rs

pub struct MetricsCollector {
    /// Work queue metrics
    queue_metrics: QueueMetrics,

    /// Agent metrics
    agent_metrics: HashMap<AgentRole, AgentMetrics>,

    /// Performance metrics
    performance: PerformanceMetrics,
}

pub struct QueueMetrics {
    /// Total work items submitted
    pub total_submitted: u64,

    /// Total completed
    pub total_completed: u64,

    /// Total failed
    pub total_failed: u64,

    /// Current queue depth
    pub current_depth: usize,

    /// Average wait time
    pub avg_wait_time_ms: u64,
}

pub struct AgentMetrics {
    /// Work items handled
    pub items_handled: u64,

    /// Success rate
    pub success_rate: f32,

    /// Average processing time
    pub avg_processing_time_ms: u64,

    /// Current state
    pub current_state: AgentState,
}

pub struct PerformanceMetrics {
    /// Event sourcing latency
    pub event_persist_latency_ms: f32,

    /// Message passing latency
    pub message_latency_ms: f32,

    /// Memory usage
    pub memory_usage_bytes: usize,
}

impl MetricsCollector {
    /// Export metrics in Prometheus format
    fn export_prometheus(&self) -> String;

    /// Export metrics as JSON
    fn export_json(&self) -> serde_json::Value;

    /// Reset metrics counters
    fn reset(&mut self);
}
```

#### Real-Time Dashboard
```rust
// src/orchestration/observability/dashboard.rs

pub struct OrchestrationDashboard {
    /// Metrics collector
    metrics: Arc<MetricsCollector>,

    /// Update interval
    refresh_rate: Duration,
}

impl OrchestrationDashboard {
    /// Render terminal dashboard (using ratatui)
    fn render_tui(&self) -> Result<()>;

    /// Start web dashboard server
    async fn start_web_server(&self, port: u16) -> Result<()>;

    /// Export dashboard data as JSON
    fn export_snapshot(&self) -> Result<DashboardSnapshot>;
}

pub struct DashboardSnapshot {
    pub timestamp: DateTime<Utc>,
    pub queue_status: QueueStatus,
    pub agent_status: HashMap<AgentRole, AgentStatus>,
    pub recent_events: Vec<AgentEvent>,
    pub performance: PerformanceSnapshot,
}
```

#### Tracing & Debugging
```rust
// src/orchestration/observability/tracing.rs

pub struct OrchestrationTracer {
    /// Trace context
    context: TraceContext,

    /// Span tree
    spans: HashMap<WorkItemId, Span>,
}

impl OrchestrationTracer {
    /// Start tracing a work item
    fn start_trace(&mut self, work_id: &WorkItemId) -> Span;

    /// Record event in trace
    fn record_event(&mut self, work_id: &WorkItemId, event: &AgentEvent);

    /// Export trace as JSON (OpenTelemetry format)
    fn export_trace(&self, work_id: &WorkItemId) -> Result<String>;

    /// Visualize trace timeline
    fn visualize_timeline(&self, work_id: &WorkItemId) -> Result<String>;
}
```

#### Performance Profiling
```rust
// src/orchestration/observability/profiler.rs

pub struct OrchestrationProfiler {
    /// Profiling sessions
    sessions: HashMap<String, ProfilingSession>,
}

pub struct ProfilingSession {
    pub id: String,
    pub started_at: DateTime<Utc>,
    pub samples: Vec<ProfileSample>,
}

pub struct ProfileSample {
    pub timestamp: DateTime<Utc>,
    pub cpu_usage: f32,
    pub memory_usage: usize,
    pub active_agents: usize,
    pub queue_depth: usize,
    pub message_rate: f32,
}

impl OrchestrationProfiler {
    /// Start profiling session
    fn start_profiling(&mut self, name: &str) -> String;

    /// Stop profiling and generate report
    fn stop_profiling(&mut self, session_id: &str) -> Result<ProfilingReport>;

    /// Identify performance bottlenecks
    fn analyze_bottlenecks(&self, session_id: &str) -> Vec<Bottleneck>;
}
```

### Testing Requirements
- [ ] Metrics collection accuracy
- [ ] Dashboard rendering (TUI and web)
- [ ] Trace export compatibility (OpenTelemetry)
- [ ] Profiling overhead < 5%
- [ ] Real-time updates (< 1s latency)

### Configuration
```toml
[orchestration.observability]
enabled = true
metrics_export_interval_secs = 10
dashboard_enabled = true
dashboard_type = "tui"  # or "web"
dashboard_port = 8080
tracing_enabled = true
profiling_enabled = false  # Enable on-demand
```

### Dashboard Output Example
```
┌─────────────────────────────────────────────────────────────────┐
│ Mnemosyne Orchestration Dashboard                              │
├─────────────────────────────────────────────────────────────────┤
│ Queue Status:                                                   │
│   Total Submitted: 142                                          │
│   Completed: 128 (90.1%)                                        │
│   Failed: 3 (2.1%)                                              │
│   Pending: 11                                                   │
│   Avg Wait: 245ms                                               │
├─────────────────────────────────────────────────────────────────┤
│ Agent Status:                                                   │
│   Orchestrator: ACTIVE  │  Work Items: 142                     │
│   Optimizer:    ACTIVE  │  Avg Time: 123ms                     │
│   Reviewer:     ACTIVE  │  Success Rate: 97.8%                 │
│   Executor:     ACTIVE  │  Utilization: 78%                    │
├─────────────────────────────────────────────────────────────────┤
│ Recent Events (last 5):                                         │
│   [14:32:45] WorkItemCompleted: item-abc123 (142ms)            │
│   [14:32:44] PhaseTransition: PromptToSpec → SpecToFullSpec    │
│   [14:32:43] WorkItemStarted: item-xyz789                      │
│   [14:32:41] QualityGateApproved: item-def456                  │
│   [14:32:40] WorkItemAssigned: item-ghi789 (priority: 5)       │
└─────────────────────────────────────────────────────────────────┘
```

---

## Implementation Priority

When Phase 4 is scheduled, recommended implementation order:

1. **Observability (4.4)** - Most valuable for production debugging
2. **Cross-Session Coordination (4.2)** - Enables long-running workflows
3. **Dynamic Scaling (4.1)** - Handles increased load
4. **Distributed P2P (4.3)** - Complex, only needed for multi-machine setups

---

## Integration Points

### With Existing Systems

**Evolution Integration:**
- Metrics feed into evaluation system
- Performance data guides optimization priorities

**MCP Integration:**
- Dashboard accessible via MCP tool: `mnemosyne.orchestrate.status`
- Metrics exported for Claude Code consumption

**Event Sourcing:**
- All Phase 4 features leverage existing event log
- Cross-session coordination builds on event replay

---

## Success Metrics

Phase 4 completion is validated when:
- [ ] Dynamic scaling reduces resource waste by 40%+
- [ ] Cross-session coordination enables 24hr+ workflows
- [ ] Dashboard provides < 1s latency visibility
- [ ] Distributed mode scales to 10+ nodes
- [ ] Test coverage maintained at 80%+

---

## References

- Erlang OTP supervision: https://www.erlang.org/doc/design_principles/sup_princ.html
- Raft consensus: https://raft.github.io/
- OpenTelemetry tracing: https://opentelemetry.io/
- Prometheus metrics: https://prometheus.io/docs/introduction/overview/
