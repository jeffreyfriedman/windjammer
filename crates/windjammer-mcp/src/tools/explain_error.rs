//! Explain Windjammer compiler errors in plain English

use crate::error::{McpError, McpResult};
use crate::protocol::ToolCallResult;
use crate::tools::text_response;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use windjammer_lsp::database::WindjammerDatabase;

#[derive(Debug, Deserialize)]
struct ExplainErrorRequest {
    error: String,
    code_context: Option<String>,
}

#[derive(Debug, Serialize)]
struct ExplainErrorResponse {
    success: bool,
    explanation: String,
    suggestion: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    corrected_code: Option<String>,
}

pub async fn handle(
    _db: Arc<Mutex<WindjammerDatabase>>,
    arguments: Value,
) -> McpResult<ToolCallResult> {
    let request: ExplainErrorRequest =
        serde_json::from_value(arguments).map_err(|e| McpError::ValidationError {
            field: "arguments".to_string(),
            message: e.to_string(),
        })?;

    let (explanation, suggestion, corrected_code) =
        explain_error(&request.error, request.code_context.as_deref());

    let response = ExplainErrorResponse {
        success: true,
        explanation,
        suggestion,
        corrected_code,
    };

    let response_json =
        serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
            message: format!("Failed to serialize response: {}", e),
        })?;

    Ok(text_response(response_json))
}

/// Explain common Windjammer errors
fn explain_error(error: &str, code_context: Option<&str>) -> (String, String, Option<String>) {
    let error_lower = error.to_lowercase();

    // Type mismatch errors
    if error_lower.contains("mismatched types")
        || error_lower.contains("expected") && error_lower.contains("found")
    {
        let explanation =
            "You're trying to use a value of one type where a different type is expected. \
                          Windjammer requires types to match exactly."
                .to_string();
        let suggestion = "Check that your variable types match what the function or operation expects. \
                         You may need to convert between types or change your variable declaration.".to_string();

        // Try to provide corrected code if context is available
        let corrected = code_context.map(|ctx| {
            if ctx.contains("let x: int = \"hello\"") {
                "let x: string = \"hello\"  // or: let x: int = 42".to_string()
            } else {
                "// Check your type annotations and conversions".to_string()
            }
        });

        return (explanation, suggestion, corrected);
    }

    // Undefined variable/function
    if error_lower.contains("cannot find") || error_lower.contains("undefined") {
        let explanation =
            "You're trying to use a variable or function that hasn't been defined yet, \
                          or it's not in scope."
                .to_string();
        let suggestion =
            "Make sure you've declared the variable with 'let' or defined the function \
                         with 'fn' before using it. Also check that it's spelled correctly."
                .to_string();
        return (explanation, suggestion, None);
    }

    // Borrow checker errors
    if error_lower.contains("borrow") || error_lower.contains("moved") {
        let explanation = "There's an ownership or borrowing issue. Windjammer (like Rust) tracks \
                          who owns each value and who can borrow it."
            .to_string();
        let suggestion = "You might be trying to use a value after it's been moved, or borrow it \
                         in conflicting ways. Consider cloning the value or restructuring your code.".to_string();
        return (explanation, suggestion, None);
    }

    // Parse errors
    if error_lower.contains("unexpected token") || error_lower.contains("expected") {
        let explanation = "There's a syntax error in your code. The parser encountered something \
                          it didn't expect."
            .to_string();
        let suggestion = "Check for missing semicolons, unmatched braces, or incorrect syntax. \
                         Make sure your code follows Windjammer's syntax rules."
            .to_string();
        return (explanation, suggestion, None);
    }

    // Default explanation
    let explanation = format!("Error: {}", error);
    let suggestion = "Review the error message carefully and check the Windjammer documentation \
                     for guidance on this issue."
        .to_string();
    (explanation, suggestion, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_explain_type_mismatch() {
        let db = Arc::new(Mutex::new(WindjammerDatabase::new()));
        let args = serde_json::json!({
            "error": "error: mismatched types\n  expected `i64`, found `&str`",
            "code_context": "let x: int = \"hello\""
        });

        let result = handle(db, args).await.unwrap();
        assert!(!result.is_error);

        let text = match &result.content[0] {
            crate::protocol::ToolContent::Text { text } => text,
            _ => panic!("Expected text content"),
        };

        assert!(text.contains("types to match"));
        assert!(text.contains("suggestion"));
    }

    #[tokio::test]
    async fn test_explain_undefined_variable() {
        let db = Arc::new(Mutex::new(WindjammerDatabase::new()));
        let args = serde_json::json!({
            "error": "error: cannot find value `foo` in this scope"
        });

        let result = handle(db, args).await.unwrap();
        let text = match &result.content[0] {
            crate::protocol::ToolContent::Text { text } => text,
            _ => panic!("Expected text content"),
        };

        assert!(text.contains("defined"));
        assert!(text.contains("scope"));
    }
}
