//! MCP server with stdio transport
//!
//! Implements JSON-RPC 2.0 server that communicates over stdin/stdout.
//! Handles tool discovery and execution.

use super::protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
use super::tools::ToolHandler;
use crate::error::Result;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{debug, error, info};

/// MCP server that handles JSON-RPC requests over stdio
pub struct McpServer {
    tool_handler: ToolHandler,
}

impl McpServer {
    /// Create a new MCP server
    pub fn new(tool_handler: ToolHandler) -> Self {
        Self { tool_handler }
    }

    /// Run the server (blocking, processes stdin/stdout)
    pub async fn run(&self) -> Result<()> {
        info!("MCP server started, listening on stdin...");

        let stdin = tokio::io::stdin();
        let mut reader = BufReader::new(stdin);
        let mut stdout = tokio::io::stdout();

        let mut line = String::new();

        loop {
            line.clear();

            // Read line from stdin
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    // EOF
                    debug!("Received EOF, shutting down");
                    break;
                }
                Ok(_) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    debug!("Received request: {}", line);

                    // Process request
                    let response = self.process_request(line).await;

                    // Write response to stdout
                    let response_json = serde_json::to_string(&response).unwrap_or_else(|e| {
                        error!("Failed to serialize response: {}", e);
                        serde_json::to_string(&JsonRpcResponse::error(
                            None,
                            JsonRpcError::internal_error(format!("Serialization error: {}", e)),
                        ))
                        .unwrap()
                    });

                    debug!("Sending response: {}", response_json);

                    if let Err(e) = stdout.write_all(response_json.as_bytes()).await {
                        error!("Failed to write response: {}", e);
                        break;
                    }

                    if let Err(e) = stdout.write_all(b"\n").await {
                        error!("Failed to write newline: {}", e);
                        break;
                    }

                    if let Err(e) = stdout.flush().await {
                        error!("Failed to flush stdout: {}", e);
                        break;
                    }
                }
                Err(e) => {
                    error!("Failed to read from stdin: {}", e);
                    break;
                }
            }
        }

        info!("MCP server shutting down");
        Ok(())
    }

    /// Process a single JSON-RPC request
    async fn process_request(&self, line: &str) -> JsonRpcResponse {
        // Parse request
        let request: JsonRpcRequest = match serde_json::from_str(line) {
            Ok(req) => req,
            Err(e) => {
                return JsonRpcResponse::error(
                    None,
                    JsonRpcError::parse_error(format!("Invalid JSON: {}", e)),
                );
            }
        };

        // Validate JSON-RPC version
        if request.jsonrpc != "2.0" {
            return JsonRpcResponse::error(
                request.id.clone(),
                JsonRpcError::invalid_request("jsonrpc must be '2.0'"),
            );
        }

        // Route to handler
        match request.method.as_str() {
            // MCP protocol methods
            "initialize" => self.handle_initialize(request),
            "tools/list" => self.handle_tools_list(request),
            "tools/call" => self.handle_tools_call(request).await,

            // Unknown method
            _ => {
                JsonRpcResponse::error(request.id, JsonRpcError::method_not_found(&request.method))
            }
        }
    }

    /// Handle initialize request
    fn handle_initialize(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        debug!("Handling initialize");

        JsonRpcResponse::success(
            request.id,
            serde_json::json!({
                "protocolVersion": "2024-11-05",
                "serverInfo": {
                    "name": "mnemosyne",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "capabilities": {
                    "tools": {}
                }
            }),
        )
    }

    /// Handle tools/list request
    fn handle_tools_list(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        debug!("Handling tools/list");

        let tools = self.tool_handler.list_tools();

        JsonRpcResponse::success(
            request.id,
            serde_json::json!({
                "tools": tools
            }),
        )
    }

    /// Handle tools/call request
    async fn handle_tools_call(&self, request: JsonRpcRequest) -> JsonRpcResponse {
        debug!("Handling tools/call");

        // Extract tool name and arguments from params
        let params = match request.params.as_object() {
            Some(obj) => obj,
            None => {
                return JsonRpcResponse::error(
                    request.id,
                    JsonRpcError::invalid_params("params must be an object"),
                );
            }
        };

        let tool_name = match params.get("name").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => {
                return JsonRpcResponse::error(
                    request.id,
                    JsonRpcError::invalid_params("missing 'name' field"),
                );
            }
        };

        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or(Value::Object(serde_json::Map::new()));

        // Execute tool
        match self.tool_handler.execute(tool_name, arguments).await {
            Ok(result) => JsonRpcResponse::success(
                request.id,
                serde_json::json!({
                    "content": [
                        {
                            "type": "text",
                            "text": serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string())
                        }
                    ]
                }),
            ),
            Err(e) => JsonRpcResponse::error(
                request.id,
                JsonRpcError::application_error(-32000, format!("Tool execution failed: {}", e)),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_routing() {
        // Test that we can parse valid JSON-RPC requests
        let request = r#"{"jsonrpc":"2.0","method":"tools/list","id":1}"#;
        let parsed: JsonRpcRequest = serde_json::from_str(request).unwrap();

        assert_eq!(parsed.jsonrpc, "2.0");
        assert_eq!(parsed.method, "tools/list");
        assert_eq!(parsed.id, Some(serde_json::json!(1)));
    }
}
