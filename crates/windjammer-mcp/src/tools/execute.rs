//! `execute`: code mode (ADR-0021 section 12). Run a Windjammer program that calls MCP tools through typed bindings, inside the
//! capability gate and resource limits of this server's policy. The tool's arguments are written by the model, so authority comes
//! from the operator (`WJ_MCP_EXECUTE_ALLOW`, `WJ_MCP_EXECUTE_SERVERS`, `WJ_MCP_EXECUTE_LIMITS`), never from the request: a request
//! can only pick from the operator's grants and servers and lower the limits.

use crate::error::{McpError, McpResult};
use crate::protocol::ToolCallResult;
use crate::tools::agent_loop::{json_response, parse_args};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use windjammer::cli::code_mode::{execute, Policy};
use windjammer_lsp::database::WindjammerDatabase;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExecuteRequest {
    /// Windjammer source with a `fn main()`. Paste the generated bindings of the servers you use above it.
    pub code: String,
    /// Grants the program needs, each exactly as the operator listed it (for example `effects=mcp.github.list_issues`). Anything the
    /// operator did not list is refused.
    #[serde(default)]
    pub allow: Vec<String>,
    /// Names of configured MCP servers to bind.
    #[serde(default)]
    pub servers: Vec<String>,
    /// Lower resource limits, `fuel=N,wall=SECS,output=BYTES,calls=N,mem=MB`; they cannot exceed the operator's.
    #[serde(default)]
    pub limits: Option<String>,
}

pub async fn handle(_db: Arc<Mutex<WindjammerDatabase>>, arguments: Value) -> McpResult<ToolCallResult> {
    let req: ExecuteRequest = parse_args(arguments)?;
    let plan = Policy::from_env().plan(&req.code, &req.allow, &req.servers, req.limits.as_deref()).map_err(|message| McpError::AuthorizationError { message })?;
    let resp = tokio::task::spawn_blocking(move || execute(&plan)).await.map_err(|e| McpError::InternalError { message: e.to_string() })?;
    let failed = resp.exit != 0;
    let mut res = json_response(&resp.to_json());
    res.is_error = failed;
    Ok(res)
}
