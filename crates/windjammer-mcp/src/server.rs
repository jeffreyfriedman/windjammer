//! MCP server implementation

use crate::error::{McpError, McpResult};
use crate::protocol::*;
use crate::tools::ToolRegistry;
use anyhow::Context;
use serde_json::Value;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};
use windjammer_lsp::database::WindjammerDatabase;

/// MCP server
pub struct McpServer {
    /// Shared database with LSP (Salsa-powered incremental computation)
    #[allow(dead_code)]
    db: Arc<Mutex<WindjammerDatabase>>,

    /// Tool registry
    tools: ToolRegistry,

    /// Server initialized flag
    initialized: Arc<Mutex<bool>>,
}

impl McpServer {
    /// Create a new MCP server
    pub async fn new() -> McpResult<Self> {
        info!("Initializing Windjammer MCP server");

        let db = Arc::new(Mutex::new(WindjammerDatabase::new()));
        let tools = ToolRegistry::new(db.clone());

        Ok(Self {
            db,
            tools,
            initialized: Arc::new(Mutex::new(false)),
        })
    }

    /// Run the server with stdio transport
    pub async fn run_stdio(&self) -> McpResult<()> {
        info!("Starting MCP server with stdio transport");

        let stdin = tokio::io::stdin();
        let mut stdout = tokio::io::stdout();
        let mut reader = BufReader::new(stdin);
        let mut line = String::new();

        loop {
            line.clear();

            match reader.read_line(&mut line).await {
                Ok(0) => {
                    info!("Client disconnected (EOF)");
                    break;
                }
                Ok(_) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    debug!("Received request: {}", trimmed);

                    match self.handle_request(trimmed).await {
                        Ok(response) => {
                            let response_json = serde_json::to_string(&response)?;
                            debug!("Sending response: {}", response_json);

                            stdout.write_all(response_json.as_bytes()).await?;
                            stdout.write_all(b"\n").await?;
                            stdout.flush().await?;
                        }
                        Err(e) => {
                            error!("Error handling request: {}", e);

                            // Try to extract request ID if possible
                            let error_response =
                                if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(trimmed) {
                                    self.create_error_response(req.id, e)
                                } else {
                                    self.create_error_response(Value::Null, e)
                                };

                            let error_json = serde_json::to_string(&error_response)?;
                            stdout.write_all(error_json.as_bytes()).await?;
                            stdout.write_all(b"\n").await?;
                            stdout.flush().await?;
                        }
                    }
                }
                Err(e) => {
                    error!("Error reading from stdin: {}", e);
                    break;
                }
            }
        }

        info!("MCP server stopped");
        Ok(())
    }

    /// Handle a JSON-RPC request
    async fn handle_request(&self, request_str: &str) -> McpResult<JsonRpcResponse> {
        let request: JsonRpcRequest =
            serde_json::from_str(request_str).context("Failed to parse JSON-RPC request")?;

        debug!("Handling method: {}", request.method);

        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize(request.params).await?,
            "initialized" => {
                // Notification - no response needed
                return Ok(JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: request.id,
                    result: Some(Value::Null),
                    error: None,
                });
            }
            "shutdown" => {
                info!("Received shutdown request");
                Value::Null
            }
            "tools/list" => self.handle_list_tools().await?,
            "tools/call" => self.handle_tool_call(request.params).await?,
            _ => {
                return Err(McpError::JsonRpcError {
                    message: format!("Unknown method: {}", request.method),
                });
            }
        };

        Ok(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.id,
            result: Some(result),
            error: None,
        })
    }

    /// Handle initialize request
    async fn handle_initialize(&self, params: Value) -> McpResult<Value> {
        let init_request: InitializeRequest =
            serde_json::from_value(params).context("Failed to parse initialize request")?;

        info!(
            "Client: {} v{}",
            init_request.client_info.name, init_request.client_info.version
        );

        // Check protocol version compatibility
        if init_request.protocol_version != crate::MCP_VERSION {
            warn!(
                "Protocol version mismatch: client={}, server={}",
                init_request.protocol_version,
                crate::MCP_VERSION
            );
        }

        // Mark as initialized
        *self.initialized.lock().await = true;

        let result = InitializeResult {
            protocol_version: crate::MCP_VERSION.to_string(),
            capabilities: ServerCapabilities {
                tools: self.tools.list_tools(),
                experimental: Value::Null,
            },
            server_info: ServerInfo {
                name: "windjammer-mcp".to_string(),
                version: crate::SERVER_VERSION.to_string(),
            },
        };

        Ok(serde_json::to_value(result)?)
    }

    /// Handle tools/list request
    async fn handle_list_tools(&self) -> McpResult<Value> {
        let tools = self.tools.list_tools();
        Ok(serde_json::to_value(tools)?)
    }

    /// Handle tools/call request
    async fn handle_tool_call(&self, params: Value) -> McpResult<Value> {
        // Ensure server is initialized
        if !*self.initialized.lock().await {
            return Err(McpError::JsonRpcError {
                message: "Server not initialized".to_string(),
            });
        }

        let call_request: ToolCallRequest =
            serde_json::from_value(params).context("Failed to parse tool call request")?;

        debug!("Calling tool: {}", call_request.name);

        let result = self
            .tools
            .call_tool(&call_request.name, call_request.arguments)
            .await?;

        Ok(serde_json::to_value(result)?)
    }

    /// Create an error response
    fn create_error_response(&self, id: Value, error: McpError) -> JsonRpcResponse {
        let (code, message) = match &error {
            McpError::ParseError { message, .. } => (-32700, message.clone()),
            McpError::ValidationError { message, .. } => (-32602, message.clone()),
            McpError::ToolNotFound { tool_name } => {
                (-32601, format!("Tool not found: {}", tool_name))
            }
            McpError::Timeout { duration_ms } => {
                (-32000, format!("Timeout after {}ms", duration_ms))
            }
            McpError::InternalError { message } => (-32603, message.clone()),
            _ => (-32603, error.to_string()),
        };

        JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message,
                data: Some(serde_json::to_value(&error).unwrap_or(Value::Null)),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_creation() {
        let server = McpServer::new().await.unwrap();
        assert!(!*server.initialized.lock().await);
    }

    #[tokio::test]
    async fn test_handle_initialize() {
        let server = McpServer::new().await.unwrap();

        let params = serde_json::json!({
            "protocol_version": crate::MCP_VERSION,
            "capabilities": {
                "experimental": null
            },
            "client_info": {
                "name": "test-client",
                "version": "1.0.0"
            }
        });

        let result = server.handle_initialize(params).await.unwrap();
        assert!(result.is_object());
        assert!(*server.initialized.lock().await);
    }

    #[tokio::test]
    async fn test_handle_list_tools() {
        let server = McpServer::new().await.unwrap();
        let result = server.handle_list_tools().await.unwrap();
        assert!(result.is_array());
    }
}
