//! Search for code patterns across the workspace

use crate::error::{McpError, McpResult};
use crate::protocol::ToolCallResult;
use crate::tools::text_response;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_lsp::lsp_types::Url;
use windjammer_lsp::database::WindjammerDatabase;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchWorkspaceRequest {
    query: String,
    #[serde(default = "default_file_pattern")]
    file_pattern: String,
    #[serde(default)]
    workspace_root: Option<String>,
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
    db: Arc<Mutex<WindjammerDatabase>>,
    arguments: Value,
) -> McpResult<ToolCallResult> {
    let request: SearchWorkspaceRequest =
        serde_json::from_value(arguments).map_err(|e| McpError::ValidationError {
            field: "arguments".to_string(),
            message: e.to_string(),
        })?;

    let root = request
        .workspace_root
        .map(PathBuf::from)
        .or_else(|| std::env::var("WJ_WORKSPACE_ROOT").ok().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."));

    let mut results = Vec::new();
    let mut db_guard = db.lock().await;

    let mut files = Vec::new();
    collect_wj_files(&root, &mut files)?;

    for path in files.into_iter().filter(|p| matches_pattern(p, &request.file_pattern)).take(50) {
            let content = fs::read_to_string(&path).map_err(|e| McpError::InternalError {
                message: e.to_string(),
            })?;
            if !content.contains(&request.query) {
                continue;
            }

            let uri = Url::from_file_path(&path).map_err(|_| McpError::InternalError {
                message: format!("Invalid path {}", path.display()),
            })?;
            let file = db_guard.set_source_text(uri, content.clone());

            let symbol_hits: Vec<SearchMatch> = db_guard
                .get_symbols(file)
                .iter()
                .filter(|s| s.name.contains(&request.query))
                .map(|s| SearchMatch {
                    line: s.line as usize + 1,
                    signature: s
                        .type_info
                        .clone()
                        .unwrap_or_else(|| format!("{:?}", s.kind)),
                    context: s.name.clone(),
                })
                .collect();

            let text_hits: Vec<SearchMatch> = if symbol_hits.is_empty() {
                content
                    .lines()
                    .enumerate()
                    .filter(|(_, line)| line.contains(&request.query))
                    .map(|(idx, line)| SearchMatch {
                        line: idx + 1,
                        signature: request.query.clone(),
                        context: line.trim().to_string(),
                    })
                    .take(20)
                    .collect()
            } else {
                symbol_hits
            };

            if !text_hits.is_empty() {
                results.push(SearchResult {
                    file: path.display().to_string(),
                    matches: text_hits,
                });
            }
    }

    let response = SearchWorkspaceResponse {
        success: true,
        results: Some(results),
        error: None,
    };

    let response_json =
        serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
            message: format!("Failed to serialize response: {}", e),
        })?;

    Ok(text_response(response_json))
}

fn collect_wj_files(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), McpError> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir).map_err(|e| McpError::InternalError {
        message: e.to_string(),
    })? {
        let entry = entry.map_err(|e| McpError::InternalError {
            message: e.to_string(),
        })?;
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name == "target" || name == "node_modules" || name.starts_with('.') {
                continue;
            }
            collect_wj_files(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("wj") {
            out.push(path);
        }
    }
    Ok(())
}

fn matches_pattern(path: &Path, pattern: &str) -> bool {
    if pattern == "**/*.wj" || pattern == "*.wj" {
        return true;
    }
    path.to_string_lossy().contains(pattern.trim_start_matches("**/"))
}
