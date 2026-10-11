//! `check_code`: parse + lint + transpile Windjammer source (what `wj check --json` does).

use crate::error::McpResult;
use crate::protocol::ToolCallResult;
use crate::tools::agent_loop::{json_response, parse_args, Scratch};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use windjammer_lsp::database::WindjammerDatabase;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CheckCodeRequest {
    /// Windjammer source text of a single file.
    pub code: String,
}

pub async fn handle(
    _db: Arc<Mutex<WindjammerDatabase>>,
    arguments: Value,
) -> McpResult<ToolCallResult> {
    let req: CheckCodeRequest = parse_args(arguments)?;
    let scratch = Scratch::new()?;
    let file = scratch.write("main.wj", &req.code)?;
    let report = windjammer::cli::check::check_path(&file);
    let mut v = serde_json::to_value(&report).unwrap_or_default();
    // Spans refer to the scratch file; present them as the logical `main.wj`.
    let scratch_name = file.to_string_lossy().to_string();
    let text = v.to_string().replace(&scratch_name.replace('\\', "\\\\"), "main.wj");
    v = serde_json::from_str(&text).unwrap_or(v);
    let mut res = json_response(&v);
    res.is_error = !report.success;
    Ok(res)
}
