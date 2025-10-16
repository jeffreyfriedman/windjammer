//! Parse Windjammer code and return AST structure

use crate::error::{McpError, McpResult};
use crate::protocol::ToolCallResult;
use crate::tools::{error_response, text_response};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use windjammer::lexer::Lexer;
use windjammer::parser::Parser;
use windjammer_lsp::database::WindjammerDatabase;

#[derive(Debug, Deserialize)]
struct ParseCodeRequest {
    code: String,
    #[serde(default = "default_true")]
    include_diagnostics: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Serialize)]
struct ParseCodeResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    ast: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    diagnostics: Option<Vec<Diagnostic>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct Diagnostic {
    message: String,
    severity: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<DiagnosticLocation>,
}

#[derive(Debug, Serialize)]
struct DiagnosticLocation {
    line: usize,
    column: usize,
}

pub async fn handle(
    _db: Arc<Mutex<WindjammerDatabase>>,
    arguments: Value,
) -> McpResult<ToolCallResult> {
    let request: ParseCodeRequest =
        serde_json::from_value(arguments).map_err(|e| McpError::ValidationError {
            field: "arguments".to_string(),
            message: e.to_string(),
        })?;

    // Validate code length (prevent DoS)
    if request.code.len() > 1_000_000 {
        return Ok(error_response("Code too large (max 1MB)"));
    }

    // Tokenize
    let mut lexer = Lexer::new(&request.code);
    let tokens = lexer.tokenize();

    // Parse
    let mut parser = Parser::new(tokens);
    let parse_result = parser.parse();

    let response = match parse_result {
        Ok(program) => {
            // Serialize AST to JSON (using Debug formatting as fallback)
            // TODO: Add Serialize derive to parser::Program in main crate
            let ast_json = serde_json::json!({
                "type": "Program",
                "items_count": program.items.len(),
                "summary": format!("{} items", program.items.len()),
                "debug": format!("{:#?}", program)
            });

            ParseCodeResponse {
                success: true,
                ast: Some(ast_json),
                diagnostics: if request.include_diagnostics {
                    Some(vec![]) // No errors
                } else {
                    None
                },
                error: None,
            }
        }
        Err(e) => {
            let diagnostics = if request.include_diagnostics {
                Some(vec![Diagnostic {
                    message: e.to_string(),
                    severity: "error".to_string(),
                    location: None, // TODO: Extract location from error
                }])
            } else {
                None
            };

            ParseCodeResponse {
                success: false,
                ast: None,
                diagnostics,
                error: Some(e.to_string()),
            }
        }
    };

    let response_json =
        serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
            message: format!("Failed to serialize response: {}", e),
        })?;

    Ok(text_response(response_json))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_parse_valid_code() {
        let db = Arc::new(Mutex::new(WindjammerDatabase::new()));
        let args = serde_json::json!({
            "code": "fn main() { println!(\"Hello\") }",
            "include_diagnostics": true
        });

        let result = handle(db, args).await.unwrap();
        assert!(!result.is_error);

        let text = match &result.content[0] {
            crate::protocol::ToolContent::Text { text } => text,
            _ => panic!("Expected text content"),
        };

        assert!(text.contains("success") && text.contains("true"));
        assert!(text.contains("items_count"));
    }

    #[tokio::test]
    async fn test_parse_invalid_code() {
        let db = Arc::new(Mutex::new(WindjammerDatabase::new()));
        let args = serde_json::json!({
            "code": "fn main() {{{",  // Invalid syntax
            "include_diagnostics": true
        });

        let result = handle(db, args).await.unwrap();
        assert!(!result.is_error); // Tool succeeded, but parse failed

        let text = match &result.content[0] {
            crate::protocol::ToolContent::Text { text } => text,
            _ => panic!("Expected text content"),
        };

        assert!(text.contains("success") && text.contains("false"));
        assert!(text.contains("error") || text.contains("diagnostics"));
    }

    #[tokio::test]
    async fn test_parse_code_too_large() {
        let db = Arc::new(Mutex::new(WindjammerDatabase::new()));
        let large_code = "x".repeat(2_000_000); // 2MB
        let args = serde_json::json!({
            "code": large_code,
            "include_diagnostics": false
        });

        let result = handle(db, args).await.unwrap();
        assert!(result.is_error);
    }
}
