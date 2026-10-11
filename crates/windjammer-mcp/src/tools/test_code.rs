//! `test_code`: run `@test` / `test_*` functions of a source file in the interpreter
//! (`wj test --interpret`).

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
pub struct TestCodeRequest {
    /// Windjammer source with `@test` functions (and the code under test).
    pub code: String,
    /// Run only tests whose name contains this text.
    #[serde(default)]
    pub filter: Option<String>,
}

pub async fn handle(
    _db: Arc<Mutex<WindjammerDatabase>>,
    arguments: Value,
) -> McpResult<ToolCallResult> {
    let req: TestCodeRequest = parse_args(arguments)?;
    let result = tokio::task::spawn_blocking(move || -> McpResult<Value> {
        let scratch = Scratch::new()?;
        let file = scratch.write("suite_test.wj", &req.code)?;
        let mut args = vec!["test", "--interpret"];
        if let Some(f) = req.filter.as_deref() {
            args.push("--filter");
            args.push(f);
        }
        let o = run_wj(&args, &file, &scratch.0, Duration::from_secs(60))?;
        let summary = o
            .stdout
            .lines()
            .find(|l| l.starts_with("test result:"))
            .unwrap_or("")
            .to_string();
        Ok(json!({
            "passed": o.exit_code == 0 && !o.timed_out,
            "exit_code": o.exit_code,
            "summary": summary,
            "output": o.stdout,
            "stderr": o.stderr,
            "timed_out": o.timed_out,
        }))
    })
    .await
    .map_err(|e| crate::error::McpError::InternalError { message: e.to_string() })??;
    let failed = result["passed"] != true;
    let mut res = json_response(&result);
    res.is_error = failed;
    Ok(res)
}
