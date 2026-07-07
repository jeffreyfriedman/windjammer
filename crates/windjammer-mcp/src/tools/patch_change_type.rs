//! Change type annotation patch tool (WJ-TOOL-01 Layer 3).
//!
//! Changes the type annotation of a variable or parameter at a given location.

use crate::error::McpResult;
use crate::protocol::{ToolCallResult, ToolContent};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;
use windjammer_lsp::database::WindjammerDatabase;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ChangeTypeRequest {
    /// Path to the source file.
    pub file: String,
    /// Line number containing the type annotation.
    pub line: usize,
    /// The old type to replace.
    pub old_type: String,
    /// The new type to use.
    pub new_type: String,
}

#[derive(Debug, Serialize)]
pub struct ChangeTypeResult {
    pub success: bool,
    pub file: String,
    pub line: usize,
    pub old_type: String,
    pub new_type: String,
}

pub async fn handle(
    _db: Arc<Mutex<WindjammerDatabase>>,
    params: Value,
) -> McpResult<ToolCallResult> {
    let req: ChangeTypeRequest = serde_json::from_value(params)
        .map_err(|e| crate::error::McpError::ValidationError { field: "params".into(), message: e.to_string() })?;

    let file_path = std::path::Path::new(&req.file);
    if !file_path.exists() {
        return Ok(ToolCallResult {
            content: vec![ToolContent::Text {
                text: json!({"error": format!("File not found: {}", req.file)}).to_string(),
            }],
            is_error: true,
        });
    }

    let content = std::fs::read_to_string(file_path)
        .map_err(|e| crate::error::McpError::IoError { message: e.to_string() })?;
    let mut lines: Vec<String> = content.lines().map(String::from).collect();

    if req.line == 0 || req.line > lines.len() {
        return Ok(ToolCallResult {
            content: vec![ToolContent::Text {
                text: json!({"error": format!("Line {} out of range (file has {} lines)", req.line, lines.len())}).to_string(),
            }],
            is_error: true,
        });
    }

    let line_idx = req.line - 1;
    let original = &lines[line_idx];
    if !original.contains(&req.old_type) {
        return Ok(ToolCallResult {
            content: vec![ToolContent::Text {
                text: json!({"error": format!("Type '{}' not found on line {}", req.old_type, req.line)}).to_string(),
            }],
            is_error: true,
        });
    }

    lines[line_idx] = original.replacen(&req.old_type, &req.new_type, 1);

    let new_content = lines.join("\n") + "\n";
    std::fs::write(file_path, &new_content)
        .map_err(|e| crate::error::McpError::IoError { message: e.to_string() })?;

    let result = ChangeTypeResult {
        success: true,
        file: req.file,
        line: req.line,
        old_type: req.old_type,
        new_type: req.new_type,
    };

    Ok(ToolCallResult {
        content: vec![ToolContent::Text {
            text: serde_json::to_string_pretty(&result).unwrap(),
        }],
        is_error: false,
    })
}
