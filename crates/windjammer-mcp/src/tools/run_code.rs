//! `run_code`: run Windjammer source in the interpreter (`wj run --interpret`).

use crate::error::McpResult;
use crate::protocol::ToolCallResult;
use crate::tools::agent_loop::{json_response, parse_args, run_wj, Scratch};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use windjammer_lsp::database::WindjammerDatabase;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct RunCodeRequest {
    /// Windjammer source text with a `fn main()`.
    pub code: String,
    /// Wall-clock limit in seconds (default 10, max 60).
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

pub async fn handle(
    _db: Arc<Mutex<WindjammerDatabase>>,
    arguments: Value,
) -> McpResult<ToolCallResult> {
    let req: RunCodeRequest = parse_args(arguments)?;
    let secs = req.timeout_secs.unwrap_or(10).clamp(1, 60);
    let result = tokio::task::spawn_blocking(move || -> McpResult<Value> {
        let scratch = Scratch::new()?;
        let file = scratch.write("main.wj", &req.code)?;
        let o = run_wj(&["run", "--interpret"], &file, &scratch.0, Duration::from_secs(secs))?;
        Ok(json!({
            "exit_code": o.exit_code,
            "stdout": o.stdout,
            "stderr": o.stderr.replace(&file.to_string_lossy().to_string(), "main.wj"),
            "timed_out": o.timed_out,
            "note": "exit 101 = panic, 2 = unsupported in the interpreter",
        }))
    })
    .await
    .map_err(|e| crate::error::McpError::InternalError { message: e.to_string() })??;
    let failed = result["exit_code"] != 0;
    let mut res = json_response(&result);
    res.is_error = failed;
    Ok(res)
}
