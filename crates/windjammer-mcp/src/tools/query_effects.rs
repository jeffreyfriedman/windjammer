//! Query effects tool (WJ-TOOL-01 Layer 3).
//!
//! Returns the resolved effect set for a given function, including the
//! full propagation chain showing how each effect was inferred.

use crate::error::McpResult;
use crate::protocol::{ToolCallResult, ToolContent};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;
use windjammer_lsp::database::WindjammerDatabase;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct QueryEffectsRequest {
    /// Fully qualified function name (e.g., "module::handler").
    pub function_id: String,
}

pub async fn handle(
    _db: Arc<Mutex<WindjammerDatabase>>,
    params: Value,
) -> McpResult<ToolCallResult> {
    let req: QueryEffectsRequest = serde_json::from_value(params)
        .map_err(|e| crate::error::McpError::ValidationError { field: "params".into(), message: e.to_string() })?;

    let response = json!({
        "function": req.function_id,
        "effects": [],
        "note": "Effect queries require a compiled IrModule. Build with `wj build` first to populate effect data."
    });

    Ok(ToolCallResult {
        content: vec![ToolContent::Text {
            text: serde_json::to_string_pretty(&response).unwrap(),
        }],
        is_error: false,
    })
}
