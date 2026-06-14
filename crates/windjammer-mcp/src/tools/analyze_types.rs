//! Perform type inference and analysis on Windjammer code

use crate::error::{McpError, McpResult};
use crate::protocol::{Position, ToolCallResult};
use crate::tools::text_response;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use windjammer::ide_analysis::{analyze_source, analyze_source_at_point, IdeAnalysisOptions};
use windjammer_lsp::database::WindjammerDatabase;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AnalyzeTypesRequest {
    pub code: String,
    pub cursor_position: Option<Position>,
}

#[derive(Debug, Serialize)]
struct AnalyzeTypesResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    inferred_types: Option<std::collections::HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    type_at_cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diagnostics: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub async fn handle(
    _db: Arc<Mutex<WindjammerDatabase>>,
    arguments: Value,
) -> McpResult<ToolCallResult> {
    let request: AnalyzeTypesRequest =
        serde_json::from_value(arguments).map_err(|e| McpError::ValidationError {
            field: "arguments".to_string(),
            message: e.to_string(),
        })?;

    let options = IdeAnalysisOptions {
        enable_lint: true,
        file_path: PathBuf::from("mcp_input.wj"),
    };

    let result = if let Some(pos) = request.cursor_position {
        analyze_source_at_point(&request.code, options, pos.line as u32, pos.column as u32)
    } else {
        analyze_source(&request.code, options)
    };

    let diagnostics: Vec<String> = result
        .diagnostics
        .iter()
        .map(|d| d.message.clone())
        .collect();

    let response = AnalyzeTypesResponse {
        success: result.success,
        inferred_types: Some(result.inferred_types),
        type_at_cursor: result.type_at_point,
        diagnostics: if diagnostics.is_empty() {
            None
        } else {
            Some(diagnostics)
        },
        error: if result.success {
            None
        } else {
            Some("Analysis reported errors".to_string())
        },
    };

    let response_json =
        serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
            message: format!("Failed to serialize response: {}", e),
        })?;

    Ok(text_response(response_json))
}
