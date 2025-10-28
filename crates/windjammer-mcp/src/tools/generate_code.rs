//! Generate Windjammer code from natural language description

use crate::error::{McpError, McpResult};
use crate::protocol::ToolCallResult;
use crate::tools::text_response;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use windjammer_lsp::database::WindjammerDatabase;

#[derive(Debug, Deserialize)]
struct GenerateCodeRequest {
    description: String,
    #[allow(dead_code)]
    context: Option<Value>,
}

#[derive(Debug, Serialize)]
struct GenerateCodeResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    explanation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

pub async fn handle(
    _db: Arc<Mutex<WindjammerDatabase>>,
    arguments: Value,
) -> McpResult<ToolCallResult> {
    let request: GenerateCodeRequest =
        serde_json::from_value(arguments).map_err(|e| McpError::ValidationError {
            field: "arguments".to_string(),
            message: e.to_string(),
        })?;

    // TODO: Implement actual code generation using AI/templates
    // For now, provide a template-based response for common patterns
    let code = generate_from_description(&request.description);

    let response = GenerateCodeResponse {
        success: true,
        code: Some(code.clone()),
        explanation: Some(format!(
            "Generated Windjammer code based on description: {}",
            request.description
        )),
        error: None,
    };

    let response_json =
        serde_json::to_string_pretty(&response).map_err(|e| McpError::InternalError {
            message: format!("Failed to serialize response: {}", e),
        })?;

    Ok(text_response(response_json))
}

/// Simple template-based code generation
fn generate_from_description(description: &str) -> String {
    let desc_lower = description.to_lowercase();

    if desc_lower.contains("filter") && desc_lower.contains("even") {
        return r#"fn filter_evens(numbers: Vec<int>) -> Vec<int> {
    numbers.iter().filter(|&n| n % 2 == 0).collect()
}"#
        .to_string();
    }

    if desc_lower.contains("http") && desc_lower.contains("server") {
        return r#"use std.http

@async
fn main() {
    let router = Router.new()
        .get("/", handle_index)
    
    http.serve("0.0.0.0:3000", router).await
}

@async
fn handle_index() -> string {
    "Hello, Windjammer!"
}"#
        .to_string();
    }

    // Default: simple function template
    format!(
        r#"// Generated from: {}
fn example() {{
    // TODO: Implement your logic here
}}"#,
        description
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_filter_code() {
        let db = Arc::new(Mutex::new(WindjammerDatabase::new()));
        let args = serde_json::json!({
            "description": "Create a function that filters even numbers"
        });

        let result = handle(db, args).await.unwrap();
        assert!(!result.is_error);

        let text = match &result.content[0] {
            crate::protocol::ToolContent::Text { text } => text,
            _ => panic!("Expected text content"),
        };

        assert!(text.contains("filter_evens"));
        assert!(text.contains("% 2 == 0"));
    }
}
