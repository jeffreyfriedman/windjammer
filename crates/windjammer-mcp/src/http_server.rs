//! Streamable HTTP transport for MCP (2025-06-18 specification)
//!
//! This module implements the MCP HTTP transport which supports:
//! - Single POST endpoint for request/response
//! - Session management with Mcp-Session-Id header
//! - Streaming responses for long-running operations
//! - OAuth 2.0 authentication
//!
//! Reference: https://modelcontextprotocol.io/specification/2025-06-18/basic/transports

use crate::error::{McpError, McpResult};
use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::server::McpServer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};

/// Session state for a connected client
#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub created_at: std::time::SystemTime,
    pub last_used: std::time::SystemTime,
    pub metadata: HashMap<String, String>,
}

impl Session {
    pub fn new(id: String) -> Self {
        let now = std::time::SystemTime::now();
        Self {
            id,
            created_at: now,
            last_used: now,
            metadata: HashMap::new(),
        }
    }

    pub fn touch(&mut self) {
        self.last_used = std::time::SystemTime::now();
    }
}

/// Session manager for tracking active sessions
pub struct SessionManager {
    sessions: RwLock<HashMap<String, Arc<Mutex<Session>>>>,
    ttl_seconds: u64,
}

impl SessionManager {
    pub fn new(ttl_seconds: u64) -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
            ttl_seconds,
        }
    }

    /// Create a new session or retrieve an existing one
    pub async fn get_or_create(&self, session_id: Option<String>) -> Arc<Mutex<Session>> {
        // Clean up expired sessions first
        self.cleanup_expired().await;

        let session_id = session_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let mut sessions = self.sessions.write().await;

        if let Some(session) = sessions.get(&session_id) {
            let mut sess = session.lock().await;
            sess.touch();
            drop(sess);
            return session.clone();
        }

        let session = Arc::new(Mutex::new(Session::new(session_id.clone())));
        sessions.insert(session_id, session.clone());
        session
    }

    /// Clean up expired sessions
    async fn cleanup_expired(&self) {
        let now = std::time::SystemTime::now();
        let ttl = std::time::Duration::from_secs(self.ttl_seconds);

        let sessions = self.sessions.read().await;
        let mut to_remove = Vec::new();

        for (id, session) in sessions.iter() {
            let session = session.lock().await;
            if let Ok(elapsed) = now.duration_since(session.last_used) {
                if elapsed >= ttl {
                    to_remove.push(id.clone());
                }
            }
        }
        drop(sessions);

        if !to_remove.is_empty() {
            let mut sessions = self.sessions.write().await;
            for id in to_remove {
                sessions.remove(&id);
            }
        }
    }

    /// Get session count
    pub async fn count(&self) -> usize {
        self.sessions.read().await.len()
    }
}

/// MCP HTTP server configuration
#[derive(Debug, Clone)]
pub struct HttpServerConfig {
    pub host: String,
    pub port: u16,
    pub session_ttl_seconds: u64,
    pub enable_oauth: bool,
    pub oauth_client_id: Option<String>,
    pub oauth_client_secret: Option<String>,
}

impl Default for HttpServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            session_ttl_seconds: 3600, // 1 hour
            enable_oauth: false,
            oauth_client_id: None,
            oauth_client_secret: None,
        }
    }
}

/// HTTP request body for MCP
#[derive(Debug, Deserialize)]
pub struct McpHttpRequest {
    #[serde(flatten)]
    pub rpc: JsonRpcRequest,
}

/// HTTP response body for MCP
#[derive(Debug, Serialize)]
pub struct McpHttpResponse {
    #[serde(flatten)]
    pub rpc: JsonRpcResponse,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
}

/// MCP HTTP server
pub struct McpHttpServer {
    config: HttpServerConfig,
    session_manager: Arc<SessionManager>,
    mcp_server: Arc<Mutex<McpServer>>,
}

impl McpHttpServer {
    pub fn new(config: HttpServerConfig, mcp_server: Arc<Mutex<McpServer>>) -> Self {
        let session_manager = Arc::new(SessionManager::new(config.session_ttl_seconds));

        Self {
            config,
            session_manager,
            mcp_server,
        }
    }

    /// Handle an incoming HTTP request
    pub async fn handle_request(
        &self,
        session_id: Option<String>,
        request: McpHttpRequest,
    ) -> McpResult<McpHttpResponse> {
        // Get or create session
        let session = self.session_manager.get_or_create(session_id).await;
        let session_id = {
            let sess = session.lock().await;
            sess.id.clone()
        };

        // TODO: Implement actual RPC handling through the MCP server
        // For now, return a placeholder response
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: request.rpc.id.clone(),
            result: Some(serde_json::json!({
                "message": "Not yet implemented"
            })),
            error: None,
        };

        Ok(McpHttpResponse {
            rpc: response,
            session_id: Some(session_id),
        })
    }

    /// Get server address
    pub fn address(&self) -> String {
        format!("{}:{}", self.config.host, self.config.port)
    }

    /// Get session count
    pub async fn session_count(&self) -> usize {
        self.session_manager.count().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_creation() {
        let manager = SessionManager::new(3600);

        let session1 = manager.get_or_create(None).await;
        let session2 = manager.get_or_create(None).await;

        let id1 = session1.lock().await.id.clone();
        let id2 = session2.lock().await.id.clone();

        assert_ne!(id1, id2);
        assert_eq!(manager.count().await, 2);
    }

    #[tokio::test]
    async fn test_session_reuse() {
        let manager = SessionManager::new(3600);

        let session1 = manager
            .get_or_create(Some("test-session".to_string()))
            .await;
        let session2 = manager
            .get_or_create(Some("test-session".to_string()))
            .await;

        let id1 = session1.lock().await.id.clone();
        let id2 = session2.lock().await.id.clone();

        assert_eq!(id1, id2);
        assert_eq!(manager.count().await, 1);
    }

    #[tokio::test]
    async fn test_session_expiry() {
        let manager = SessionManager::new(1); // 1 second TTL

        let session = manager
            .get_or_create(Some("test-session".to_string()))
            .await;
        assert_eq!(manager.count().await, 1);

        // Wait for expiry
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Trigger cleanup
        manager.cleanup_expired().await;
        assert_eq!(manager.count().await, 0);
    }
}
