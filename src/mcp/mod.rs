//! Model Context Protocol (MCP) server implementation
//!
//! Provides a JSON-RPC 2.0 server over stdio for Claude Code integration.
//! Implements 8 core memory tools organized around the OODA loop.

pub mod protocol;
pub mod server;
pub mod tools;

pub use protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
pub use server::McpServer;
pub use tools::ToolHandler;
