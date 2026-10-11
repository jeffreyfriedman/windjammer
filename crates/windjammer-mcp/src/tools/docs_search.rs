//! `docs_search`: offline search over stdlib signatures, built-in methods, idioms, verified examples and
//! error codes (the same index as `wj docs <query>`).

use crate::error::McpResult;
use crate::protocol::ToolCallResult;
use crate::tools::agent_loop::{json_response, parse_args};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;
use windjammer::agent_index::docs_search;
use windjammer_lsp::database::WindjammerDatabase;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DocsSearchRequest {
    /// Search terms; all must match (e.g. "hashmap entry", "sort_by_key", "pipe").
    pub query: String,
    /// Maximum results (default 8, max 50).
    #[serde(default)]
    pub limit: Option<usize>,
}

pub async fn handle(
    _db: Arc<Mutex<WindjammerDatabase>>,
    arguments: Value,
) -> McpResult<ToolCallResult> {
    let req: DocsSearchRequest = parse_args(arguments)?;
    let entries = docs_search::embedded_index();
    let hits = docs_search::search(&entries, &req.query, req.limit.unwrap_or(8).clamp(1, 50));
    Ok(json_response(&json!({ "query": req.query, "count": hits.len(), "results": hits })))
}
