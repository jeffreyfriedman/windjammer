//! Get Windjammer compiler and MCP version information.

use crate::error::McpResult;
use crate::protocol::ToolCallResult;
use crate::tools::text_response;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use windjammer_lsp::database::WindjammerDatabase;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetLanguageInfoRequest {
    #[serde(default)]
    pub include_agent_index: bool,
}

#[derive(Debug, Serialize)]
struct LanguageInfoResponse {
    windjammer_version: String,
    mcp_version: String,
    protocol_version: String,
    agent_index: Option<Value>,
}

pub async fn handle(
    _db: Arc<Mutex<WindjammerDatabase>>,
    arguments: Value,
) -> McpResult<ToolCallResult> {
    let req: GetLanguageInfoRequest =
        serde_json::from_value(arguments).unwrap_or(GetLanguageInfoRequest {
            include_agent_index: false,
        });

    let agent_index = if req.include_agent_index {
        crate::agent_index::load_embedded_index().ok()
    } else {
        None
    };

    let response = LanguageInfoResponse {
        windjammer_version: env!("CARGO_PKG_VERSION").to_string(),
        mcp_version: crate::SERVER_VERSION.to_string(),
        protocol_version: crate::MCP_VERSION.to_string(),
        agent_index,
    };

    Ok(text_response(
        serde_json::to_string_pretty(&response).unwrap_or_default(),
    ))
}
