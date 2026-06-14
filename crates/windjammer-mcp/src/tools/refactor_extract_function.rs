//! Extract function refactoring tool — delegates to `windjammer_lsp::refactoring`.

use crate::error::{McpError, McpResult};
use crate::protocol::{Range, ToolCallResult};
use crate::tools::text_response;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_lsp::lsp_types::{Position, Url};
use windjammer_lsp::database::WindjammerDatabase;
use windjammer_lsp::refactoring::apply_workspace_edit;
use windjammer_lsp::refactoring::extract_function::ExtractFunction;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ExtractFunctionRequest {
    pub code: String,
    pub range: Range,
    pub function_name: String,
    #[serde(default)]
    pub make_public: bool,
}

#[derive(Debug, Serialize)]
struct ExtractFunctionResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    refactored_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    function_signature: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    captured_variables: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub async fn handle(
    db: Arc<Mutex<WindjammerDatabase>>,
    arguments: Value,
) -> McpResult<ToolCallResult> {
    let request: ExtractFunctionRequest =
        serde_json::from_value(arguments).map_err(|e| McpError::ValidationError {
            field: "arguments".to_string(),
            message: e.to_string(),
        })?;

    let db_guard = db.lock().await;
    let uri = synthetic_uri();
    let lsp_range = mcp_range_to_lsp(&request.range);
    let refactoring = ExtractFunction::new(&db_guard, uri, lsp_range);

    let mut function_name = request.function_name.clone();
    if request.make_public && !function_name.starts_with("pub_") {
        function_name = format!("pub_{}", function_name);
    }

    match refactoring.execute_with_metadata(&function_name, &request.code) {
        Ok(result) => match apply_workspace_edit(&request.code, &result.edit) {
            Ok(refactored_code) => {
                let response = ExtractFunctionResponse {
                    success: true,
                    refactored_code: Some(refactored_code),
                    function_signature: Some(result.function_signature),
                    captured_variables: Some(result.captured_variables),
                    error: None,
                };
                Ok(text_response(
                    serde_json::to_string_pretty(&response).map_err(|e| {
                        McpError::InternalError {
                            message: e.to_string(),
                        }
                    })?,
                ))
            }
            Err(e) => Ok(text_response(error_response(e))),
        },
        Err(e) => Ok(text_response(error_response(e))),
    }
}

fn synthetic_uri() -> Url {
    Url::parse("file:///mcp_input.wj").expect("valid synthetic MCP uri")
}

fn mcp_range_to_lsp(range: &Range) -> tower_lsp::lsp_types::Range {
    tower_lsp::lsp_types::Range {
        start: Position {
            line: range.start.line as u32,
            character: range.start.column as u32,
        },
        end: Position {
            line: range.end.line as u32,
            character: range.end.column as u32,
        },
    }
}

fn error_response(message: String) -> String {
    serde_json::to_string_pretty(&ExtractFunctionResponse {
        success: false,
        refactored_code: None,
        function_signature: None,
        captured_variables: None,
        error: Some(message),
    })
    .unwrap_or_else(|_| "{\"success\":false}".to_string())
}
