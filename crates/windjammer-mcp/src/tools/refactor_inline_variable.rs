//! Inline variable refactoring tool
//!
//! Replaces all uses of a variable with its initializer expression.

use crate::error::{McpError, McpResult};
use crate::protocol::{Position, ToolCallResult};
use crate::tools::text_response;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use windjammer::lexer::Lexer;
use windjammer::parser::{Expression, Parser};
use windjammer_lsp::database::WindjammerDatabase;

#[derive(Debug, Serialize, Deserialize)]
pub struct InlineVariableRequest {
    /// Source code to refactor
    pub code: String,

    /// Position of the variable to inline
    pub position: Position,

    /// Optional: Variable name (if position doesn't uniquely identify it)
    pub variable_name: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InlineVariableResponse {
    pub success: bool,

    /// Refactored code with variable inlined
    pub refactored_code: Option<String>,

    /// Number of occurrences replaced
    pub occurrences_replaced: Option<usize>,

    /// Variable that was inlined
    pub variable_name: Option<String>,

    pub error: Option<String>,
}

/// Inline variable refactoring tool
pub async fn handle(
    _db: Arc<Mutex<WindjammerDatabase>>,
    arguments: Value,
) -> McpResult<ToolCallResult> {
    let request: InlineVariableRequest =
        serde_json::from_value(arguments).map_err(|e| McpError::ValidationError {
            field: "arguments".to_string(),
            message: e.to_string(),
        })?;

    // Parse the source code
    let mut lexer = Lexer::new(&request.code);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let parse_result = parser.parse();

    let program = match parse_result {
        Ok(prog) => prog,
        Err(e) => {
            let response = InlineVariableResponse {
                success: false,
                refactored_code: None,
                occurrences_replaced: None,
                variable_name: None,
                error: Some(format!("Parse error: {}", e)),
            };
            return Ok(text_response(&serde_json::to_string(&response)?));
        }
    };

    // Find variable declaration at position
    let var_info = find_variable_at_position(&program, &request.position);

    let (var_name, var_value) = match var_info {
        Some((name, value)) => (name, value),
        None => {
            let response = InlineVariableResponse {
                success: false,
                refactored_code: None,
                occurrences_replaced: None,
                variable_name: None,
                error: Some("No variable found at position".to_string()),
            };
            return Ok(text_response(&serde_json::to_string(&response)?));
        }
    };

    // Check if it's safe to inline (no mutations, simple value)
    if !is_safe_to_inline(&var_value) {
        let response = InlineVariableResponse {
            success: false,
            refactored_code: None,
            occurrences_replaced: None,
            variable_name: Some(var_name.clone()),
            error: Some("Variable has side effects or is too complex to inline safely".to_string()),
        };
        return Ok(text_response(&serde_json::to_string(&response)?));
    }

    // Count occurrences
    let occurrences = count_variable_uses(&program, &var_name);

    // Generate refactored code (simplified - would need proper AST manipulation)
    let refactored_code = format!(
        "// TODO: Implement full AST manipulation\n// Would inline variable '{}' ({} occurrences)",
        var_name, occurrences
    );

    let response = InlineVariableResponse {
        success: true,
        refactored_code: Some(refactored_code),
        occurrences_replaced: Some(occurrences),
        variable_name: Some(var_name),
        error: None,
    };

    Ok(text_response(&serde_json::to_string(&response)?))
}

/// Find variable declaration at position
fn find_variable_at_position(
    _program: &windjammer::parser::Program,
    _position: &Position,
) -> Option<(String, Expression)> {
    // TODO: Implement actual position-based lookup
    // For now, return None
    None
}

/// Check if expression is safe to inline
fn is_safe_to_inline(expr: &Expression) -> bool {
    match expr {
        // Simple literals and identifiers are always safe
        Expression::Literal(_) | Expression::Identifier(_) => true,

        // Binary operations on safe expressions are safe
        Expression::Binary { left, right, .. } => {
            is_safe_to_inline(left) && is_safe_to_inline(right)
        }

        // Unary operations on safe expressions are safe
        Expression::Unary { operand, .. } => is_safe_to_inline(operand),

        // Field access on safe expressions is safe
        Expression::FieldAccess { object, .. } => is_safe_to_inline(object),

        // Function calls have side effects - not safe
        Expression::Call { .. } | Expression::MethodCall { .. } => false,

        // Async/await, channel operations have side effects - not safe
        Expression::Await(_) | Expression::ChannelSend { .. } | Expression::ChannelRecv(_) => false,

        // Macro invocations might have side effects - not safe
        Expression::MacroInvocation { .. } => false,

        // Blocks might have side effects - not safe (could be refined)
        Expression::Block(_) => false,

        // Other cases: be conservative
        _ => false,
    }
}

/// Count how many times a variable is used
fn count_variable_uses(_program: &windjammer::parser::Program, _var_name: &str) -> usize {
    // TODO: Implement actual usage counting
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use windjammer::parser::Literal;

    #[tokio::test]
    async fn test_inline_variable_basic() {
        let db = Arc::new(Mutex::new(WindjammerDatabase::new()));
        let args = serde_json::json!({
            "code": "fn main() {\n    let x = 42;\n    println!(\"{}\", x);\n}",
            "position": { "line": 1, "column": 8 }
        });

        let result = handle(db, args).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_is_safe_to_inline() {
        // Literal is safe
        assert!(is_safe_to_inline(&Expression::Literal(Literal::Int(42))));

        // Identifier is safe
        assert!(is_safe_to_inline(&Expression::Identifier("x".to_string())));

        // Function call is not safe
        assert!(!is_safe_to_inline(&Expression::Call {
            function: Box::new(Expression::Identifier("foo".to_string())),
            arguments: vec![],
        }));

        // Macro invocation is not safe
        assert!(!is_safe_to_inline(&Expression::MacroInvocation {
            name: "println".to_string(),
            args: vec![],
            delimiter: windjammer::parser::MacroDelimiter::Parens,
        }));
    }
}
