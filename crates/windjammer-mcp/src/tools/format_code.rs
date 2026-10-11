//! `format_code`: format Windjammer source (the `wj fmt` formatter, in-process).

use crate::error::McpResult;
use crate::protocol::ToolCallResult;
use crate::tools::agent_loop::{json_response, parse_args};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;
use windjammer_lsp::database::WindjammerDatabase;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct FormatCodeRequest {
    /// Windjammer source text.
    pub code: String,
}

pub async fn handle(
    _db: Arc<Mutex<WindjammerDatabase>>,
    arguments: Value,
) -> McpResult<ToolCallResult> {
    let req: FormatCodeRequest = parse_args(arguments)?;
    match windjammer::formatter::format_source(&req.code, &windjammer::formatter::FormatOptions::default()) {
        Ok(formatted) => Ok(json_response(&json!({
            "formatted": formatted,
            "changed": formatted != req.code,
        }))),
        Err(e) => {
            let mut res = json_response(&json!({ "error": e.to_string() }));
            res.is_error = true;
            Ok(res)
        }
    }
}
