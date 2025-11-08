# RPC Server Quick Start Guide

**Last Updated**: 2025-11-08

Get up and running with Mnemosyne's gRPC server in minutes.

---

## What is the RPC Server?

The Mnemosyne RPC server provides **remote access** to the memory system via gRPC. This enables:

- External applications to store and retrieve memories
- Cross-language integration (Python, Rust, Go, JavaScript, etc.)
- Distributed deployments with multiple clients
- Programmatic access to semantic search and graph traversal

**Use Cases**:
- Building AI agents that need persistent memory
- Integrating memory into existing applications
- Multi-user memory systems
- Cloud deployments with remote access

---

## Prerequisites

- **Rust toolchain** (1.75+): Install from [rustup.rs](https://rustup.rs)
- **Protocol Buffers compiler**: Required for building

```bash
# macOS
brew install protobuf

# Ubuntu/Debian
sudo apt-get install protobuf-compiler

# Arch Linux
sudo pacman -S protobuf

# Verify installation
protoc --version  # Should show libprotoc 3.0+
```

---

## Installation

### Building from Source

```bash
# Clone the repository
git clone https://github.com/user/mnemosyne.git
cd mnemosyne

# Build the RPC server
cargo build --release --features rpc --bin mnemosyne-rpc

# Binary location
./target/release/mnemosyne-rpc
```

### Running the Server

```bash
# Start on default port (50051)
./target/release/mnemosyne-rpc

# Custom port
./target/release/mnemosyne-rpc --port 8080

# Listen on all interfaces (for remote access)
./target/release/mnemosyne-rpc --host 0.0.0.0 --port 50051

# With LLM enrichment (requires ANTHROPIC_API_KEY)
export ANTHROPIC_API_KEY="your-api-key-here"
./target/release/mnemosyne-rpc --enable-llm

# Custom database location
./target/release/mnemosyne-rpc --db-path /path/to/mnemosyne.db
```

**Server Output**:
```
[INFO] Mnemosyne RPC Server v1.0.0
[INFO] Database: /Users/you/.local/share/mnemosyne/mnemosyne.db
[INFO] Listening on: http://127.0.0.1:50051
[INFO] Services: HealthService, MemoryService
[INFO] LLM enrichment: disabled
```

---

## Your First Client

### Python Client

**Install dependencies**:
```bash
pip install grpcio grpcio-tools
```

**Generate client code**:
```bash
# Download protobuf schemas from the repository
mkdir -p proto/mnemosyne/v1
# Copy *.proto files from repo's proto/mnemosyne/v1/ directory

# Generate Python code
python -m grpc_tools.protoc \
  -I./proto \
  --python_out=. \
  --grpc_python_out=. \
  proto/mnemosyne/v1/*.proto
```

**Store a memory**:
```python
import grpc
from mnemosyne.v1 import memory_pb2, memory_pb2_grpc

# Connect to server
channel = grpc.insecure_channel('localhost:50051')
stub = memory_pb2_grpc.MemoryServiceStub(channel)

# Create a memory
request = memory_pb2.StoreMemoryRequest(
    content="Python is great for rapid prototyping",
    namespace=memory_pb2.Namespace(
        project=memory_pb2.ProjectNamespace(name="my-project")
    ),
    importance=7,
    tags=["python", "programming"],
    skip_llm_enrichment=True  # Skip LLM processing for speed
)

# Store it
response = stub.StoreMemory(request)
print(f"✓ Stored memory: {response.memory_id}")
```

**Retrieve a memory**:
```python
# Get by ID
get_request = memory_pb2.GetMemoryRequest(
    memory_id=response.memory_id
)

get_response = stub.GetMemory(get_request)
memory = get_response.memory

print(f"Content: {memory.content}")
print(f"Importance: {memory.importance}")
print(f"Tags: {', '.join(memory.tags)}")
```

**Search memories**:
```python
# Hybrid search (FTS + semantic + graph)
recall_request = memory_pb2.RecallRequest(
    query="programming languages",
    max_results=5,
    namespace=memory_pb2.Namespace(
        project=memory_pb2.ProjectNamespace(name="my-project")
    )
)

recall_response = stub.Recall(recall_request)

for result in recall_response.results:
    print(f"Score: {result.score:.3f}")
    print(f"  {result.memory.content}")
    print()
```

### Rust Client

**Add dependencies** to `Cargo.toml`:
```toml
[dependencies]
tonic = "0.12"
prost = "0.13"
tokio = { version = "1", features = ["full"] }

[build-dependencies]
tonic-build = "0.12"
```

**Create `build.rs`**:
```rust
fn main() {
    tonic_build::configure()
        .compile(
            &["proto/mnemosyne/v1/memory.proto"],
            &["proto"],
        )
        .expect("Failed to compile protos");
}
```

**Client code**:
```rust
use mnemosyne::v1::{
    memory_service_client::MemoryServiceClient,
    StoreMemoryRequest, Namespace, ProjectNamespace,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect
    let mut client = MemoryServiceClient::connect("http://localhost:50051").await?;

    // Store memory
    let request = tonic::Request::new(StoreMemoryRequest {
        content: "Rust provides memory safety without garbage collection".to_string(),
        namespace: Some(Namespace {
            namespace: Some(mnemosyne::v1::namespace::Namespace::Project(
                ProjectNamespace {
                    name: "my-project".to_string(),
                }
            )),
        }),
        importance: Some(8),
        tags: vec!["rust".to_string(), "systems".to_string()],
        skip_llm_enrichment: true,
        ..Default::default()
    });

    let response = client.store_memory(request).await?;
    println!("✓ Stored memory: {}", response.into_inner().memory_id);

    Ok(())
}
```

---

## Common Operations

### Health Check

Verify the server is running:

**Python**:
```python
from mnemosyne.v1 import health_pb2, health_pb2_grpc

channel = grpc.insecure_channel('localhost:50051')
stub = health_pb2_grpc.HealthServiceStub(channel)

response = stub.HealthCheck(health_pb2.HealthCheckRequest())
print(f"Status: {response.status}")  # SERVING
```

**cURL** (using grpcurl):
```bash
# Install grpcurl
brew install grpcurl  # macOS
go install github.com/fullstorydev/grpcurl/cmd/grpcurl@latest  # Others

# Health check
grpcurl -plaintext localhost:50051 mnemosyne.v1.HealthService/HealthCheck

# List available services
grpcurl -plaintext localhost:50051 list
```

### List Memories

**Python**:
```python
list_request = memory_pb2.ListMemoriesRequest(
    namespace=memory_pb2.Namespace(
        project=memory_pb2.ProjectNamespace(name="my-project")
    ),
    limit=10,
    sort_by="importance",
    sort_desc=True
)

list_response = stub.ListMemories(list_request)

print(f"Found {list_response.total_count} memories:")
for memory in list_response.memories:
    print(f"  [{memory.importance}] {memory.content[:50]}...")
```

### Update Memory

**Python**:
```python
update_request = memory_pb2.UpdateMemoryRequest(
    memory_id="mem_abc123",
    importance=9,  # Increase importance
    add_tags=["critical"],  # Add tag
)

update_response = stub.UpdateMemory(update_request)
print(f"✓ Updated memory: {update_response.memory.importance}")
```

### Delete (Archive) Memory

**Python**:
```python
delete_request = memory_pb2.DeleteMemoryRequest(
    memory_id="mem_abc123"
)

delete_response = stub.DeleteMemory(delete_request)
if delete_response.success:
    print("✓ Memory archived")
```

---

## Streaming APIs

For large result sets or slow operations, use streaming variants.

### Stream Search Results

**Python**:
```python
# Stream recall results progressively
recall_request = memory_pb2.RecallRequest(
    query="machine learning",
    max_results=100  # Large result set
)

# Returns iterator of SearchResult messages
for result in stub.RecallStream(recall_request):
    print(f"Score: {result.score:.3f} - {result.memory.content[:50]}...")
    # Process results as they arrive (no need to wait for all 100)
```

### Stream Memory List

**Python**:
```python
list_request = memory_pb2.ListMemoriesRequest(
    limit=1000,  # Large list
    sort_by="created_at",
    sort_desc=True
)

# Returns iterator of MemoryNote messages
for memory in stub.ListMemoriesStream(list_request):
    print(f"  {memory.created_at}: {memory.content[:30]}...")
```

### Store with Progress

**Python**:
```python
store_request = memory_pb2.StoreMemoryRequest(
    content="Complex memory requiring LLM enrichment",
    importance=9,
    skip_llm_enrichment=False  # Enable LLM (slow operation)
)

# Returns iterator of StoreMemoryProgress messages
for progress in stub.StoreMemoryStream(store_request):
    print(f"{progress.stage}: {progress.percent}%")

    if progress.stage == "complete":
        print(f"✓ Stored: {progress.memory_id}")
```

**Output**:
```
preparing: 10%
enriching: 30%
embedding: 60%
indexing: 90%
complete: 100%
✓ Stored: mem_xyz789
```

---

## Configuration Options

### Command-Line Arguments

| Option | Description | Default |
|--------|-------------|---------|
| `--host <IP>` | Bind address | 127.0.0.1 |
| `--port <PORT>` | Listen port | 50051 |
| `--db-path <PATH>` | Database file | `~/.local/share/mnemosyne/mnemosyne.db` |
| `--enable-llm` | Enable LLM enrichment | false |
| `--anthropic-api-key <KEY>` | Anthropic API key | `$ANTHROPIC_API_KEY` |
| `--log-level <LEVEL>` | Log verbosity | info |

### Environment Variables

- `ANTHROPIC_API_KEY`: Default API key for LLM enrichment
- `RUST_LOG`: Log level (trace, debug, info, warn, error)

**Example**:
```bash
export RUST_LOG=debug
export ANTHROPIC_API_KEY="sk-ant-..."
./target/release/mnemosyne-rpc --enable-llm --host 0.0.0.0
```

---

## Troubleshooting

### Server won't start

**Error**: `Address already in use`
- **Cause**: Port 50051 already in use
- **Fix**: Use a different port: `--port 8080`

**Error**: `Failed to open database`
- **Cause**: Database path not writable or doesn't exist
- **Fix**: Create parent directory or use `--db-path` with valid location

### Client connection fails

**Error**: `failed to connect to all addresses`
- **Cause**: Server not running or wrong address
- **Fix**:
  1. Verify server is running: `ps aux | grep mnemosyne-rpc`
  2. Check server output for actual listen address
  3. Ensure client connects to same host:port

**Error**: `deadline exceeded`
- **Cause**: Network timeout
- **Fix**: Increase client timeout:
```python
# Python
channel = grpc.insecure_channel('localhost:50051', options=[
    ('grpc.max_receive_message_length', -1),
    ('grpc.max_send_message_length', -1),
])
```

### Empty search results

**Cause**: No memories stored yet or namespace mismatch
- **Fix**: Verify namespace matches between store and search:
```python
# Use same namespace for both
ns = memory_pb2.Namespace(
    project=memory_pb2.ProjectNamespace(name="my-project")
)

# Store with namespace
stub.StoreMemory(memory_pb2.StoreMemoryRequest(
    content="...",
    namespace=ns
))

# Search with same namespace
stub.Recall(memory_pb2.RecallRequest(
    query="...",
    namespace=ns
))
```

---

## Next Steps

### Learn More

- **[Full RPC Documentation](../features/RPC.md)** - Complete API reference, deployment options, performance tuning
- **[Module README](../../src/rpc/README.md)** - Technical implementation details
- **[Architecture Guide](../../ARCHITECTURE.md)** - System design and components

### Production Deployment

For production use:
1. **Enable TLS** - Use reverse proxy (nginx, Envoy) with TLS certificates
2. **Add authentication** - API keys, JWT tokens (future feature)
3. **Monitor performance** - Use HealthService metrics
4. **Set up backups** - Regular database backups
5. **Resource limits** - Memory limits, rate limiting (future feature)

See [RPC.md Deployment Section](../features/RPC.md#deployment) for Docker, systemd, and Kubernetes examples.

### Advanced Features

- **Semantic search** with custom embeddings
- **Graph traversal** for related memories
- **Batch operations** for efficiency
- **Custom namespaces** for multi-tenancy

See [Full RPC Documentation](../features/RPC.md) for details.

---

## Quick Reference

### Python Examples

```python
# Connect
channel = grpc.insecure_channel('localhost:50051')
stub = memory_pb2_grpc.MemoryServiceStub(channel)

# Store
stub.StoreMemory(memory_pb2.StoreMemoryRequest(
    content="...", importance=7, tags=["tag1"]
))

# Get
stub.GetMemory(memory_pb2.GetMemoryRequest(memory_id="..."))

# Search
stub.Recall(memory_pb2.RecallRequest(query="...", max_results=10))

# List
stub.ListMemories(memory_pb2.ListMemoriesRequest(limit=20))

# Update
stub.UpdateMemory(memory_pb2.UpdateMemoryRequest(
    memory_id="...", importance=9
))

# Delete
stub.DeleteMemory(memory_pb2.DeleteMemoryRequest(memory_id="..."))
```

### Common Namespaces

```python
# Project namespace
memory_pb2.Namespace(
    project=memory_pb2.ProjectNamespace(name="my-project")
)

# Global namespace
memory_pb2.Namespace(
    global_=memory_pb2.GlobalNamespace()
)

# User namespace
memory_pb2.Namespace(
    user=memory_pb2.UserNamespace(user_id="alice")
)

# Session namespace
memory_pb2.Namespace(
    session=memory_pb2.SessionNamespace(session_id="sess_123")
)
```

---

## Support

**Questions or issues?**
- Check [RPC Documentation](../features/RPC.md#troubleshooting)
- Review [Architecture Guide](../../ARCHITECTURE.md)
- File an issue on GitHub

**Contributing?**
- See [CONTRIBUTING.md](../../CONTRIBUTING.md)
- Read [Development Guide](../features/RPC.md#development)
