//! MCP tool registry types.

use crate::error::McpResult;
use crate::protocol::{ToolCallResult, ToolContent};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use windjammer_lsp::database::WindjammerDatabase;

pub type ToolHandler = Box<
    dyn Fn(
            Arc<Mutex<WindjammerDatabase>>,
            Value,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = McpResult<ToolCallResult>> + Send>>
        + Send
        + Sync,
>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolStability {
    Stub,
    Beta,
    Stable,
}

impl ToolStability {
    pub fn as_str(self) -> &'static str {
        match self {
            ToolStability::Stub => "stub",
            ToolStability::Beta => "beta",
            ToolStability::Stable => "stable",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ToolMeta {
    pub name: &'static str,
    pub description: &'static str,
    pub category: &'static str,
    pub stability: ToolStability,
}

pub fn text_response(text: impl Into<String>) -> ToolCallResult {
    ToolCallResult {
        content: vec![ToolContent::Text { text: text.into() }],
        is_error: false,
    }
}

pub fn error_response(text: impl Into<String>) -> ToolCallResult {
    ToolCallResult {
        content: vec![ToolContent::Text { text: text.into() }],
        is_error: true,
    }
}
