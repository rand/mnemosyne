---
name: mnemosyne-mcp-protocol
description: MCP server implementation patterns for Mnemosyne
---

# Mnemosyne MCP Protocol

**Scope**: Model Context Protocol (MCP) server implementation for Mnemosyne
**Lines**: ~350
**Last Updated**: 2025-10-27

## When to Use This Skill

Activate this skill when:
- Implementing new MCP tools for Mnemosyne
- Understanding MCP server architecture
- Working with JSON-RPC 2.0 protocol
- Debugging MCP communication issues
- Adding new memory operations
- Testing MCP tool implementations

## MCP Protocol Overview

**Model Context Protocol (MCP)**:
- JSON-RPC 2.0 over stdin/stdout
- Enables Claude Code to communicate with Mnemosyne
- 8 OODA-aligned tools
- Asynchronous request/response model
- Type-safe schema validation

**Communication Flow**:
```
Claude Code → JSON-RPC Request → stdin → Mnemosyne Server
Mnemosyne Server → JSON-RPC Response → stdout → Claude Code
```

**Logs**: Sent to stderr (not stdout) to avoid protocol corruption

## 8 OODA-Aligned Tools

### OBSERVE Tools

**1. mnemosyne.recall** - Search memories by query
```json
{
  "name": "mnemosyne.recall",
  "arguments": {
    "query": "database decisions",
    "namespace": "project:myapp",
    "max_results": 10,
    "min_importance": 5
  }
}
```

**2. mnemosyne.list** - List recent memories
```json
{
  "name": "mnemosyne.list",
  "arguments": {
    "namespace": "project:myapp",
    "limit": 20,
    "sort_by": "created_at"
  }
}
```

### ORIENT Tools

**3. mnemosyne.graph** - Get memory graph from seed IDs
```json
{
  "name": "mnemosyne.graph",
  "arguments": {
    "seed_ids": ["uuid-1", "uuid-2"],
    "max_hops": 2
  }
}
```

**4. mnemosyne.context** - Get full context for memory IDs
```json
{
  "name": "mnemosyne.context",
  "arguments": {
    "memory_ids": ["uuid-1", "uuid-2"],
    "include_links": true
  }
}
```

### DECIDE Tools

**5. mnemosyne.remember** - Store new memory with LLM enrichment
```json
{
  "name": "mnemosyne.remember",
  "arguments": {
    "content": "Decided to use PostgreSQL...",
    "namespace": "project:myapp",
    "importance": 9,
    "context": "Database selection discussion"
  }
}
```

**6. mnemosyne.consolidate** - Merge/supersede similar memories
```json
{
  "name": "mnemosyne.consolidate",
  "arguments": {
    "memory_ids": ["uuid-1", "uuid-2"],
    "namespace": "project:myapp"
  }
}
```

### ACT Tools

**7. mnemosyne.update** - Update existing memory
```json
{
  "name": "mnemosyne.update",
  "arguments": {
    "memory_id": "uuid",
    "content": "Updated content",
    "importance": 10,
    "add_tags": ["critical", "reviewed"]
  }
}
```

**8. mnemosyne.delete** - Archive (soft delete) memory
```json
{
  "name": "mnemosyne.delete",
  "arguments": {
    "memory_id": "uuid"
  }
}
```

## Implementation Patterns

### Tool Registration

**Register tools in `src/mcp/tools.rs`**:
```rust
pub fn register_tools() -> Vec<Tool> {
    vec![
        Tool {
            name: "mnemosyne.recall".to_string(),
            description: "Search memories by semantic query...".to_string(),
            input_schema: recall_schema(),
        },
        // ... other tools
    ]
}
```

### Tool Handler Pattern

**Implement handler function**:
```rust
pub async fn handle_tool_call(
    name: &str,
    arguments: serde_json::Value,
    storage: &SqliteStorage,
    llm: &LlmService,
) -> Result<ToolResult, ToolError> {
    match name {
        "mnemosyne.recall" => handle_recall(arguments, storage).await,
        "mnemosyne.remember" => handle_remember(arguments, storage, llm).await,
        // ... other tools
        _ => Err(ToolError::UnknownTool(name.to_string())),
    }
}
```

### Request/Response Types

**JSON-RPC 2.0 Request**:
```rust
#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,  // Must be "2.0"
    pub method: String,
    pub params: Option<serde_json::Value>,
    pub id: RequestId,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    Number(u64),
    String(String),
    Null,
}
```

**JSON-RPC 2.0 Response**:
```rust
#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,  // Always "2.0"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: RequestId,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}
```

### Error Codes

**Standard JSON-RPC errors**:
```rust
pub const PARSE_ERROR: i32 = -32700;
pub const INVALID_REQUEST: i32 = -32600;
pub const METHOD_NOT_FOUND: i32 = -32601;
pub const INVALID_PARAMS: i32 = -32602;
pub const INTERNAL_ERROR: i32 = -32603;
pub const APPLICATION_ERROR: i32 = -32000;
```

## Server Implementation

### Main Server Loop

**Async server in `src/mcp/server.rs`**:
```rust
pub async fn serve(
    storage: Arc<SqliteStorage>,
    llm: Arc<LlmService>,
) -> Result<(), ServerError> {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut writer = BufWriter::new(stdout);

    loop {
        let mut line = String::new();
        reader.read_line(&mut line).await?;

        if line.is_empty() {
            break; // EOF
        }

        let response = handle_request(&line, &storage, &llm).await;
        let json = serde_json::to_string(&response)?;
        writer.write_all(json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
    }

    Ok(())
}
```

### Request Handling

**Parse and route request**:
```rust
async fn handle_request(
    line: &str,
    storage: &SqliteStorage,
    llm: &LlmService,
) -> JsonRpcResponse {
    // Parse JSON-RPC request
    let request: JsonRpcRequest = match serde_json::from_str(line) {
        Ok(req) => req,
        Err(e) => {
            return error_response(PARSE_ERROR, e.to_string(), RequestId::Null);
        }
    };

    // Route to method handler
    let result = match request.method.as_str() {
        "initialize" => handle_initialize().await,
        "tools/list" => handle_tools_list().await,
        "tools/call" => handle_tools_call(request.params, storage, llm).await,
        _ => Err(method_not_found(&request.method)),
    };

    // Build response
    match result {
        Ok(value) => success_response(value, request.id),
        Err(e) => error_response(e.code, e.message, request.id),
    }
}
```

### Tool Call Handling

**Extract and validate tool parameters**:
```rust
async fn handle_tools_call(
    params: Option<serde_json::Value>,
    storage: &SqliteStorage,
    llm: &LlmService,
) -> Result<ToolCallResponse, ToolError> {
    // Extract tool call params
    let params: ToolCallParams = serde_json::from_value(params.ok_or(
        ToolError::InvalidParams("params required")
    )?)?;

    // Validate tool name
    let tool_name = &params.name;
    if !is_valid_tool(tool_name) {
        return Err(ToolError::UnknownTool(tool_name.clone()));
    }

    // Execute tool
    let result = handle_tool_call(
        tool_name,
        params.arguments,
        storage,
        llm
    ).await?;

    // Wrap in MCP response format
    Ok(ToolCallResponse {
        content: vec![Content::text(serde_json::to_string(&result)?)]
    })
}
```

## Schema Definitions

### Input Schema (JSON Schema)

**Define tool input schema**:
```rust
fn recall_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "Search query for finding memories"
            },
            "namespace": {
                "type": "string",
                "description": "Memory namespace filter (e.g., 'project:myapp')"
            },
            "max_results": {
                "type": "integer",
                "minimum": 1,
                "maximum": 100,
                "default": 10
            },
            "min_importance": {
                "type": "integer",
                "minimum": 1,
                "maximum": 10,
                "default": 1
            }
        },
        "required": ["query"]
    })
}
```

### Response Format

**Tool result structure**:
```rust
#[derive(Debug, Serialize)]
pub struct ToolResult {
    pub content: Vec<Content>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum Content {
    #[serde(rename = "text")]
    Text { text: String },
}

impl Content {
    pub fn text(s: String) -> Self {
        Content::Text { text: s }
    }
}
```

## Testing MCP Tools

### Unit Tests

**Test tool logic independently**:
```rust
#[tokio::test]
async fn test_recall_tool() {
    let storage = create_test_storage().await;
    let memories = setup_test_memories(&storage).await;

    let args = json!({
        "query": "architecture",
        "namespace": "project:test",
        "max_results": 5
    });

    let result = handle_recall(args, &storage).await.unwrap();
    assert!(result.len() > 0);
}
```

### Integration Tests

**Test via JSON-RPC**:
```rust
#[tokio::test]
async fn test_jsonrpc_recall() {
    let server = start_test_server().await;

    let request = json!({
        "jsonrpc": "2.0",
        "method": "tools/call",
        "params": {
            "name": "mnemosyne.recall",
            "arguments": {
                "query": "test"
            }
        },
        "id": 1
    });

    let response = server.handle_request(request.to_string()).await;
    assert!(response.result.is_some());
    assert_eq!(response.id, RequestId::Number(1));
}
```

### Manual Testing

**Test with echo/curl**:
```bash
# Initialize
echo '{"jsonrpc":"2.0","method":"initialize","id":1}' | cargo run -- serve

# List tools
echo '{"jsonrpc":"2.0","method":"tools/list","id":2}' | cargo run -- serve

# Call tool
echo '{"jsonrpc":"2.0","method":"tools/call","params":{"name":"mnemosyne.recall","arguments":{"query":"test"}},"id":3}' | cargo run -- serve
```

## Best Practices

### DO

- Always validate input parameters
- Return structured JSON responses
- Use appropriate error codes
- Log to stderr, never stdout
- Handle EOF gracefully
- Buffer I/O operations
- Use async/await throughout
- Test with malformed requests

### DON'T

- Write logs to stdout (breaks protocol)
- Return non-JSON on stdout
- Block the async runtime
- Ignore validation errors
- Use custom error codes without reason
- Skip schema definitions
- Forget to flush stdout buffer

## Common Pitfalls

**Stdout corruption**:
- Logs written to stdout break JSON-RPC
- Always use stderr for logs
- Use `eprintln!()` not `println!()`

**Schema mismatches**:
- Input validation must match schema
- Test with invalid inputs
- Provide clear error messages

**Async issues**:
- Don't block the runtime with sync operations
- Use `tokio::spawn` for parallel work
- Handle task cancellation properly

## Further Reading

- `MCP_SERVER.md`: Tool reference and examples
- `mnemosyne-rust-development.md`: Rust patterns
- MCP Specification: https://modelcontextprotocol.io/
- JSON-RPC 2.0: https://www.jsonrpc.org/specification
