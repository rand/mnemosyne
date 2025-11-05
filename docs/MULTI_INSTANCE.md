# Multi-Instance Support

Mnemosyne supports running multiple instances in parallel without conflicts. This enables concurrent workflows, parallel agent coordination, and isolated development sessions.

## Architecture

### Cross-Process Coordination

Multiple instances share state through file-based coordination in `.mnemosyne/`:

```
.mnemosyne/
├── branch_registry.json       # Git branch assignments
├── process_registry.json      # Active instances with PIDs
└── coordination_queue/        # Inter-instance messages
    ├── msg-001.json
    └── msg-002.json
```

### Instance Isolation

Each instance has:
- **Unique Instance ID**: 8-character UUID (e.g., `a1b2c3d4`)
- **Dedicated API Port**: Dynamically allocated from 3000-3010
- **Process Registration**: PID tracking with heartbeat monitoring
- **Shared Database**: Concurrent read/write via LibSQL
- **Git Worktree**: Isolated working directory for branch-specific work (if in git repo)

### Git Worktree Isolation

When running in a git repository, Mnemosyne automatically creates isolated worktrees for true branch isolation:

#### How It Works

```
repo/                          # Main worktree
├── .git/
├── .mnemosyne/
│   └── worktrees/            # Agent worktrees
│       ├── a1b2c3d4/         # Instance 1 worktree
│       └── e5f6g7h8/         # Instance 2 worktree
└── src/
```

**Benefits:**
- **True Physical Isolation**: Each instance has its own working directory and HEAD pointer
- **Independent Branches**: Instances can work on different branches without conflicts
- **Shared Object Database**: Efficient storage - git objects shared between worktrees
- **Automatic Cleanup**: Stale worktrees removed by `mnemosyne doctor --fix`

**Automatic Behavior:**
1. On startup, if in a git repo:
   - If in main working directory (not already in a worktree), skip worktree creation
   - If in a worktree, create dedicated worktree for this instance
2. Change to worktree directory for all operations (if worktree created)
3. On shutdown, cleanup worktree (if created)
4. Track worktree path in process registration (if applicable)

**Note**: When launching from the main working directory on the current branch, worktree isolation is skipped to avoid the error "Branch 'X' is already used by another worktree". Worktree isolation is primarily useful when multiple instances need to work on different branches simultaneously.

#### Cleanup Stale Worktrees

If an instance crashes, its worktree may remain. Use doctor command:

```bash
# Check for stale worktrees
mnemosyne doctor

# Cleanup all stale worktrees
mnemosyne doctor --fix
```

#### Manual Worktree Management

```bash
# List all worktrees
git worktree list

# Remove specific worktree
git worktree remove .mnemosyne/worktrees/<instance-id>

# Prune stale worktree references
git worktree prune
```

### Dynamic Port Allocation

When starting, the API server:
1. Tries configured port (default: 3000)
2. If unavailable, tries ports 3001-3010
3. Logs selected port with instance ID
4. If all ports busy, continues without API (core functionality unaffected)

## Usage

### Running Multiple Instances

**Terminal 1:**
```bash
mnemosyne serve --with-api
# API server [a1b2c3d4] listening on http://127.0.0.1:3000
```

**Terminal 2:**
```bash
mnemosyne serve --with-api
# Port 3000 in use, trying alternative ports...
# API server [e5f6g7h8] listening on http://127.0.0.1:3001
```

### Monitoring Instances

Each instance exposes health endpoint with instance ID:

```bash
curl http://127.0.0.1:3000/health
{
  "status": "ok",
  "version": "2.1.0",
  "instance_id": "a1b2c3d4",
  "subscribers": 0
}

curl http://127.0.0.1:3001/health
{
  "status": "ok",
  "version": "2.1.0",
  "instance_id": "e5f6g7h8",
  "subscribers": 0
}
```

### Dashboard Connections

Connect multiple dashboards to different instances:

```bash
# Terminal 1: Dashboard for instance 1
mnemosyne-dash --api http://127.0.0.1:3000

# Terminal 2: Dashboard for instance 2
mnemosyne-dash --api http://127.0.0.1:3001
```

## Event Broadcasting

### Process-Local Events

Each instance has its own in-memory event broadcaster:
- Events are **not** shared between instances by default
- Each dashboard sees only its connected instance's events
- Use cross-process coordination for inter-instance communication

### Cross-Instance Coordination

For coordinated workflows, instances use file-based message queue:

```rust
// Send coordination message
let coordinator = CrossProcessCoordinator::new(".mnemosyne", agent_id)?;
coordinator.send_message(CoordinationMessage {
    id: "msg-001".to_string(),
    from_agent: agent1_id,
    to_agent: Some(agent2_id),
    message_type: MessageType::JoinRequest,
    timestamp: Utc::now(),
    payload: json!({"branch": "main"}),
})?;

// Receive messages
let messages = coordinator.receive_messages()?;
```

## Database Concurrency

### Shared Database Access

All instances share the same LibSQL database:
- **Readers**: Unlimited concurrent reads
- **Writers**: Serialized with SQLite locking
- **Transactions**: ACID guarantees maintained
- **Conflicts**: Automatic retry with exponential backoff

### Isolation Levels

- **Memory Operations**: SERIALIZABLE (default SQLite)
- **Graph Updates**: EXCLUSIVE lock during write
- **Embeddings**: Concurrent inserts with row-level locking

## Use Cases

### 1. Parallel Agent Workflows

Run multiple agents in separate terminals:

```bash
# Terminal 1: Main orchestrator
mnemosyne orchestrate --plan=feature.json

# Terminal 2: Background optimization
mnemosyne optimize --context

# Terminal 3: Continuous review
mnemosyne review --watch
```

### 2. Development + Production

Separate dev/prod instances:

```bash
# Development
DATABASE_URL=dev.db mnemosyne serve --with-api

# Production
DATABASE_URL=prod.db mnemosyne serve --with-api --api-addr 127.0.0.1:4000
```

### 3. Isolated Namespaces

Test different namespaces concurrently:

```bash
# Project-specific instance
mnemosyne remember -n "project:myapp" -c "..." &

# Global knowledge base
mnemosyne remember -n "global" -c "..." &
```

## Limitations

### API Server Capacity

- Maximum 11 instances with API enabled (ports 3000-3010)
- Additional instances run without API server (core functions work)
- Dashboard monitoring requires API server

### Database Contention

- High write concurrency may cause lock contention
- Recommend staggered writes or batch operations
- Read operations scale linearly

### Cross-Process Latency

- File-based coordination has ~100ms latency
- Polling interval: 2 seconds (configurable)
- Use in-process events for real-time coordination

## Troubleshooting

### "Address already in use" Warning

**Expected behavior**: Second instance tries alternative ports automatically.

```bash
# Instance 1
2025-11-05T04:54:00Z INFO API server [a1b2c3d4] listening on http://127.0.0.1:3000

# Instance 2
2025-11-05T04:54:16Z DEBUG Port 3000 in use, trying alternative ports...
2025-11-05T04:54:16Z INFO API server [e5f6g7h8] listening on http://127.0.0.1:3001
```

### All Ports Exhausted

```bash
WARN All ports (3000–3010) are in use. API server unavailable for instance i9j0k1l2.
     Core functionality not affected.
```

**Solution**: Stop unused instances or disable API server (`--with-api=false`)

### Database Lock Errors

```
Error: database is locked
```

**Solution**:
- Reduce concurrent writes
- Use batch operations
- Increase `busy_timeout` in database config

### Stale Process Registry

```bash
# Clean up dead processes
mnemosyne doctor --cleanup-stale
```

## Performance Characteristics

### Memory Usage

Per instance (approximate):
- Base: ~50 MB
- API server: +10 MB
- Event buffer: +capacity × 1 KB
- State manager: +5 MB

### CPU Usage

- Idle: <1%
- Active coordination: 5-10%
- Heavy writes: 20-30%
- Event streaming: 2-5%

### Disk I/O

- Database writes: ~100 KB/s (typical)
- Coordination queue: ~10 KB/s
- Process heartbeats: ~1 KB/s

## Best Practices

1. **Limit Concurrent Writers**: Batch operations when possible
2. **Use Namespaces**: Isolate unrelated workflows
3. **Monitor Health**: Check `/health` endpoints regularly
4. **Clean Up**: Stop instances when finished
5. **Stagger Starts**: Wait 100ms between instance launches
6. **Set Unique Ports**: Use `--api-addr` for predictable ports
7. **Share Secrets**: Use same `MNEMOSYNE_SHARED_SECRET` for coordination

## Configuration

### Environment Variables

```bash
# Shared secret for cross-process coordination
export MNEMOSYNE_SHARED_SECRET="your-secret-key"

# Database path (default: ~/.local/share/mnemosyne/mnemosyne.db)
export MNEMOSYNE_DB_PATH="/path/to/db"

# API server address
export MNEMOSYNE_API_ADDR="127.0.0.1:5000"
```

### Command-Line Options

```bash
mnemosyne serve \
  --with-api \                      # Enable API server
  --api-addr 127.0.0.1:3000 \      # Set API address
  --api-capacity 1000              # Event buffer size
```

## Implementation Details

### Instance ID Generation

```rust
let instance_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
```

### Port Selection Algorithm

```rust
// Try configured port first
match TcpListener::bind(config.addr).await {
    Ok(listener) => /* use this port */,
    Err(_) => {
        // Try ports +1 through +10
        for offset in 1..=10 {
            let alt_port = base_port + offset;
            match TcpListener::bind(alt_addr).await {
                Ok(listener) => /* use this port */,
                Err(_) => continue,
            }
        }
    }
}
```

### Process Liveness Detection

```rust
// Register with heartbeat
coordinator.register()?;

// Send periodic heartbeats (every 2s)
loop {
    tokio::time::sleep(Duration::from_secs(2)).await;
    coordinator.heartbeat()?;
}

// Cleanup stale processes (>30s since last heartbeat)
coordinator.cleanup_stale_processes()?;
```

## Security Considerations

### PID Spoofing Protection

Process registrations use HMAC signatures to prevent PID spoofing:

```rust
let signature = HMAC-SHA256(
    secret,
    agent_id || pid || registered_at
);
```

### File Permissions

Coordination files restricted to owner (Unix):
```bash
chmod 0700 .mnemosyne/
chmod 0600 .mnemosyne/*.json
```

### Message Size Limits

- Max message size: 1 KB
- Prevents DoS via oversized messages
- Enforced at send and receive

## See Also

- [Cross-Process Coordination](./CROSS_PROCESS.md)
- [Agent Identity System](./AGENT_IDENTITY.md)
- [Event Broadcasting](./EVENTS.md)
- [Database Architecture](./DATABASE.md)
