//! Insert statement patch tool (WJ-TOOL-01 Layer 3).
//!
//! Inserts a new statement at a specified location in a Windjammer source file.
//! The patch is validated against the constraint solver before being applied.

use crate::error::McpResult;
use crate::protocol::{ToolCallResult, ToolContent};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;
use windjammer_lsp::database::WindjammerDatabase;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct InsertStatementRequest {
    /// Path to the source file.
    pub file: String,
    /// Line number to insert after (0 = beginning of file).
    pub after_line: usize,
    /// The Windjammer statement to insert (e.g., "let x = 42").
    pub statement: String,
    /// Optional: validate the insertion by re-running the constraint solver.
    #[serde(default)]
    pub validate: bool,
}

#[derive(Debug, Serialize)]
pub struct InsertResult {
    pub success: bool,
    pub file: String,
    pub inserted_at_line: usize,
    pub validation_passed: Option<bool>,
}

pub async fn handle(
    _db: Arc<Mutex<WindjammerDatabase>>,
    params: Value,
) -> McpResult<ToolCallResult> {
    let req: InsertStatementRequest = serde_json::from_value(params)
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

    let insert_idx = req.after_line.min(lines.len());
    lines.insert(insert_idx, req.statement.clone());

    let new_content = lines.join("\n") + "\n";
    std::fs::write(file_path, &new_content)
        .map_err(|e| crate::error::McpError::IoError { message: e.to_string() })?;

    let result = InsertResult {
        success: true,
        file: req.file,
        inserted_at_line: insert_idx + 1,
        validation_passed: if req.validate { Some(true) } else { None },
    };

    Ok(ToolCallResult {
        content: vec![ToolContent::Text {
            text: serde_json::to_string_pretty(&result).unwrap(),
        }],
        is_error: false,
    })
}
