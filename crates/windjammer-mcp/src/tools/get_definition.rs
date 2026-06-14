//! Find the definition of a symbol

use crate::error::{McpError, McpResult};
use crate::protocol::{Position, Range, ToolCallResult};
use crate::tools::text_response;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_lsp::lsp_types::Url;
use windjammer_lsp::database::WindjammerDatabase;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetDefinitionRequest {
    file: Option<String>,
    symbol: String,
    #[allow(dead_code)] // Reserved for LSP position-based lookup
    position: Option<Position>,
}

#[derive(Debug, Serialize)]
struct GetDefinitionResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    definition: Option<Definition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct Definition {
    file: String,
    range: Range,
    signature: String,
}

pub async fn handle(
    db: Arc<Mutex<WindjammerDatabase>>,
    arguments: Value,
) -> McpResult<ToolCallResult> {
    let request: GetDefinitionRequest =
        serde_json::from_value(arguments).map_err(|e| McpError::ValidationError {
            field: "arguments".to_string(),
            message: e.to_string(),
        })?;

    let mut db_guard = db.lock().await;

    let mut files = Vec::new();
    if let Some(ref file_path) = request.file {
        let path = PathBuf::from(file_path);
        let content = fs::read_to_string(&path).map_err(|e| McpError::ValidationError {
            field: "file".to_string(),
            message: format!("Cannot read {}: {}", file_path, e),
        })?;
        let uri = Url::from_file_path(&path).map_err(|_| McpError::ValidationError {
            field: "file".to_string(),
            message: format!("Invalid file path: {}", file_path),
        })?;
        files.push(db_guard.set_source_text(uri, content));
    }

    if files.is_empty() {
        return Ok(text_response(
            serde_json::to_string_pretty(&GetDefinitionResponse {
                success: false,
                definition: None,
                error: Some("file path required for symbol lookup".to_string()),
            })
            .unwrap(),
        ));
    }

    let location = db_guard.find_definition(&request.symbol, &files);

    let response = if let Some(loc) = location {
        let file = loc
            .uri
            .to_file_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| loc.uri.to_string());
        let symbols = db_guard.get_symbols(files[0]);
        let signature = symbols
            .iter()
            .find(|s| s.name == request.symbol)
            .and_then(|s| s.type_info.clone())
            .unwrap_or_else(|| request.symbol.clone());

        GetDefinitionResponse {
            success: true,
            definition: Some(Definition {
                file,
                range: Range {
                    start: Position {
                        line: loc.range.start.line as usize,
                        column: loc.range.start.character as usize,
                    },
                    end: Position {
                        line: loc.range.end.line as usize,
                        column: loc.range.end.character as usize,
                    },
                },
                signature,
            }),
            error: None,
        }
    } else {
        GetDefinitionResponse {
            success: false,
            definition: None,
            error: Some(format!("Symbol '{}' not found", request.symbol)),
        }
    };

    let response_json =
        serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
            message: format!("Failed to serialize response: {}", e),
        })?;

    Ok(text_response(response_json))
}
