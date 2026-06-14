//! Inline variable refactoring tool — delegates to `windjammer_lsp::refactoring`.

use crate::error::{McpError, McpResult};
use crate::protocol::{Position, ToolCallResult};
use crate::tools::text_response;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_lsp::lsp_types::Url;
use windjammer_lsp::database::WindjammerDatabase;
use windjammer_lsp::refactoring::apply_workspace_edit;
use windjammer_lsp::refactoring::inline::InlineRefactoring;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct InlineVariableRequest {
    pub code: String,
    pub position: Position,
    pub variable_name: Option<String>,
}

#[derive(Debug, Serialize)]
struct InlineVariableResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    refactored_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    occurrences_replaced: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    variable_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub async fn handle(
    db: Arc<Mutex<WindjammerDatabase>>,
    arguments: Value,
) -> McpResult<ToolCallResult> {
    let request: InlineVariableRequest =
        serde_json::from_value(arguments).map_err(|e| McpError::ValidationError {
            field: "arguments".to_string(),
            message: e.to_string(),
        })?;

    let db_guard = db.lock().await;
    let uri = synthetic_uri();
    let lsp_position = tower_lsp::lsp_types::Position {
        line: request.position.line as u32,
        character: request.position.column as u32,
    };
    let refactoring = InlineRefactoring::new(&db_guard, uri, lsp_position);

    match refactoring.execute_with_metadata(&request.code) {
        Ok(result) => {
            if let Some(expected_name) = &request.variable_name {
                if expected_name != &result.variable_name {
                    return Ok(text_response(error_response(format!(
                        "Variable at position is '{}', not '{}'",
                        result.variable_name, expected_name
                    ))));
                }
            }
            match apply_workspace_edit(&request.code, &result.edit) {
                Ok(refactored_code) => {
                    let response = InlineVariableResponse {
                        success: true,
                        refactored_code: Some(refactored_code),
                        occurrences_replaced: Some(result.occurrences_replaced),
                        variable_name: Some(result.variable_name),
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
            }
        }
        Err(e) => Ok(text_response(error_response(e))),
    }
}

fn synthetic_uri() -> Url {
    Url::parse("file:///mcp_input.wj").expect("valid synthetic MCP uri")
}

fn error_response(message: String) -> String {
    serde_json::to_string_pretty(&InlineVariableResponse {
        success: false,
        refactored_code: None,
        occurrences_replaced: None,
        variable_name: None,
        error: Some(message),
    })
    .unwrap_or_else(|_| "{\"success\":false}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inline_request_deserializes() {
        let json = serde_json::json!({
            "code": "let x = 1\nx + 1",
            "position": { "line": 0, "column": 4 }
        });
        let req: InlineVariableRequest = serde_json::from_value(json).unwrap();
        assert_eq!(req.position.line, 0);
    }
}
