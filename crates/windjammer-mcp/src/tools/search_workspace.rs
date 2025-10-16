//! Search for code patterns across the workspace

use crate::error::{McpError, McpResult};
use crate::protocol::ToolCallResult;
use crate::tools::text_response;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use windjammer_lsp::database::WindjammerDatabase;

#[derive(Debug, Deserialize)]
struct SearchWorkspaceRequest {
    query: String,
    #[serde(default = "default_file_pattern")]
    file_pattern: String,
}

fn default_file_pattern() -> String {
    "**/*.wj".to_string()
}

#[derive(Debug, Serialize)]
struct SearchWorkspaceResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    results: Option<Vec<SearchResult>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct SearchResult {
    file: String,
    matches: Vec<SearchMatch>,
}

#[derive(Debug, Serialize)]
struct SearchMatch {
    line: usize,
    signature: String,
    context: String,
}

pub async fn handle(
    _db: Arc<Mutex<WindjammerDatabase>>,
    arguments: Value,
) -> McpResult<ToolCallResult> {
    let _request: SearchWorkspaceRequest =
        serde_json::from_value(arguments).map_err(|e| McpError::ValidationError {
            field: "arguments".to_string(),
            message: e.to_string(),
        })?;

    // TODO: Implement actual workspace search using the request
    // For now, return empty results
    let response = SearchWorkspaceResponse {
        success: true,
        results: Some(vec![]),
        error: None,
    };

    let response_json =
        serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
            message: format!("Failed to serialize response: {}", e),
        })?;

    Ok(text_response(response_json))
}
