//! Error types for the Windjammer MCP server

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type McpResult<T> = Result<T, McpError>;

/// MCP server errors
#[derive(Error, Debug, Serialize, Deserialize)]
#[serde(tag = "error_type")]
pub enum McpError {
    /// Parse error in Windjammer code
    #[error("Parse error: {message}")]
    ParseError {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        location: Option<Location>,
    },

    /// Type inference or analysis error
    #[error("Analysis error: {message}")]
    AnalysisError {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        location: Option<Location>,
    },

    /// Input validation error
    #[error("Validation error in field '{field}': {message}")]
    ValidationError { field: String, message: String },

    /// Tool not found
    #[error("Tool not found: {tool_name}")]
    ToolNotFound { tool_name: String },

    /// File not found
    #[error("File not found: {path}")]
    FileNotFound { path: String },

    /// Symbol not found
    #[error("Symbol not found: {symbol}")]
    SymbolNotFound { symbol: String },

    /// Operation timed out
    #[error("Operation timed out after {duration_ms}ms")]
    Timeout { duration_ms: u64 },

    /// Internal server error
    #[error("Internal error: {message}")]
    InternalError { message: String },

    /// JSON-RPC error
    #[error("JSON-RPC error: {message}")]
    JsonRpcError { message: String },

    /// IO error
    #[error("IO error: {message}")]
    IoError { message: String },

    /// Authentication error
    #[error("Authentication error: {message}")]
    AuthenticationError { message: String },

    /// Authorization error
    #[error("Authorization error: {message}")]
    AuthorizationError { message: String },
}

/// Source code location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub file: Option<String>,
    pub line: usize,
    pub column: usize,
}

impl Location {
    pub fn new(file: Option<String>, line: usize, column: usize) -> Self {
        Self { file, line, column }
    }
}

/// Convert from anyhow::Error
impl From<anyhow::Error> for McpError {
    fn from(err: anyhow::Error) -> Self {
        McpError::InternalError {
            message: err.to_string(),
        }
    }
}

/// Convert from std::io::Error
impl From<std::io::Error> for McpError {
    fn from(err: std::io::Error) -> Self {
        McpError::IoError {
            message: err.to_string(),
        }
    }
}

/// Convert from serde_json::Error
impl From<serde_json::Error> for McpError {
    fn from(err: serde_json::Error) -> Self {
        McpError::JsonRpcError {
            message: err.to_string(),
        }
    }
}
