//! Query diagnostics tool (WJ-TOOL-01 Layer 3).
//!
//! Returns structured compiler diagnostics for a given scope (file, function, or workspace).

use crate::error::McpResult;
use crate::protocol::{ToolCallResult, ToolContent};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;
use windjammer_lsp::database::WindjammerDatabase;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct QueryDiagnosticsRequest {
    /// Scope to query: "workspace", a file path, or a function name.
    pub scope: String,
    /// Optional severity filter: "error", "warning", "info", or "all" (default).
    #[serde(default = "default_severity")]
    pub severity: String,
}

fn default_severity() -> String {
    "all".to_string()
}

pub async fn handle(
    _db: Arc<Mutex<WindjammerDatabase>>,
    params: Value,
) -> McpResult<ToolCallResult> {
    let req: QueryDiagnosticsRequest = serde_json::from_value(params)
        .map_err(|e| crate::error::McpError::ValidationError { field: "params".into(), message: e.to_string() })?;

    let response = json!({
        "scope": req.scope,
        "severity_filter": req.severity,
        "diagnostics": [],
        "note": "Diagnostics are populated when the compiler runs with --json flag. Use `wj build --json` to get structured output."
    });

    Ok(ToolCallResult {
        content: vec![ToolContent::Text {
            text: serde_json::to_string_pretty(&response).unwrap(),
        }],
        is_error: false,
    })
}
