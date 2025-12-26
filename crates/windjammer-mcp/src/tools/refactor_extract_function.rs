//! Extract function refactoring tool
//!
//! Transforms selected code into a new reusable function.

use crate::error::{McpError, McpResult};
use crate::protocol::{Range, ToolCallResult};
use crate::tools::text_response;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::Mutex;
use windjammer::lexer::Lexer;
use windjammer::parser::{Expression, Parser, Statement, Type};
use windjammer_lsp::database::WindjammerDatabase;

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractFunctionRequest {
    /// Source code to refactor
    pub code: String,

    /// Selection range to extract
    pub range: Range,

    /// Name for the new function
    pub function_name: String,

    /// Optional: Make function public
    #[serde(default)]
    pub make_public: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ExtractFunctionResponse {
    pub success: bool,

    /// Refactored code with new function
    pub refactored_code: Option<String>,

    /// New function signature
    pub function_signature: Option<String>,

    /// Variables captured from outer scope
    pub captured_variables: Option<Vec<String>>,

    pub error: Option<String>,
}

/// Extract function refactoring tool
pub async fn handle(
    _db: Arc<Mutex<WindjammerDatabase>>,
    arguments: Value,
) -> McpResult<ToolCallResult> {
    let request: ExtractFunctionRequest =
        serde_json::from_value(arguments).map_err(|e| McpError::ValidationError {
            field: "arguments".to_string(),
            message: e.to_string(),
        })?;

    // Parse the source code
    let mut lexer = Lexer::new(&request.code);
    let tokens = lexer.tokenize_with_locations();
    let mut parser = Parser::new(tokens);
    let parse_result = parser.parse();

    let program = match parse_result {
        Ok(prog) => prog,
        Err(e) => {
            let response = ExtractFunctionResponse {
                success: false,
                refactored_code: None,
                function_signature: None,
                captured_variables: None,
                error: Some(format!("Parse error: {}", e)),
            };
            return Ok(text_response(&serde_json::to_string(&response)?));
        }
    };

    // Extract statements in the selected range
    let extracted = extract_statements_in_range(&program, &request.range);

    if extracted.is_empty() {
        let response = ExtractFunctionResponse {
            success: false,
            refactored_code: None,
            function_signature: None,
            captured_variables: None,
            error: Some("No statements found in selection".to_string()),
        };
        return Ok(text_response(&serde_json::to_string(&response)?));
    }

    // Analyze variable usage to determine parameters and return value
    let analysis = analyze_variable_usage(&extracted);

    // Generate new function
    let new_function = generate_function(
        &request.function_name,
        &extracted,
        &analysis,
        request.make_public,
    );

    // Generate function call to replace extracted code
    let function_call = generate_function_call(&request.function_name, &analysis);

    // Build refactored code (simplified - would need proper AST manipulation)
    let refactored_code = format!(
        "// TODO: Implement full AST manipulation\n// New function:\n{}\n\n// Replace selection with:\n{}",
        new_function, function_call
    );

    let response = ExtractFunctionResponse {
        success: true,
        refactored_code: Some(refactored_code),
        function_signature: Some(format!(
            "fn {}({}) -> {}",
            request.function_name,
            analysis
                .parameters
                .iter()
                .map(|(name, ty)| format!("{}: {:?}", name, ty))
                .collect::<Vec<_>>()
                .join(", "),
            if let Some(ret) = &analysis.return_type {
                format!("{:?}", ret)
            } else {
                "()".to_string()
            }
        )),
        captured_variables: Some(analysis.parameters.iter().map(|(n, _)| n.clone()).collect()),
        error: None,
    };

    Ok(text_response(&serde_json::to_string(&response)?))
}

/// Extract statements within a given range
fn extract_statements_in_range(
    _program: &windjammer::parser::Program,
    _range: &Range,
) -> Vec<Statement> {
    // TODO: Implement actual statement extraction based on line/column range
    // For now, return empty vec
    vec![]
}

/// Analysis result for variable usage
#[derive(Debug)]
struct VariableAnalysis {
    parameters: Vec<(String, Type)>,
    return_type: Option<Type>,
    #[allow(dead_code)]
    used_variables: HashSet<String>,
    #[allow(dead_code)]
    defined_variables: HashSet<String>,
}

/// Analyze variable usage in extracted statements
fn analyze_variable_usage(statements: &[Statement]) -> VariableAnalysis {
    let mut used = HashSet::new();
    let mut defined = HashSet::new();

    for stmt in statements {
        collect_variable_usage(stmt, &mut used, &mut defined);
    }

    // Parameters are variables used but not defined in selection
    let parameters: Vec<(String, Type)> = used
        .difference(&defined)
        .map(|name| (name.clone(), Type::String)) // Placeholder type
        .collect();

    // Return type is inferred from return statements or last expression
    let return_type = infer_return_type(statements);

    VariableAnalysis {
        parameters,
        return_type,
        used_variables: used,
        defined_variables: defined,
    }
}

/// Collect variable usage from a statement
fn collect_variable_usage(
    stmt: &Statement,
    used: &mut HashSet<String>,
    defined: &mut HashSet<String>,
) {
    match stmt {
        Statement::Let { pattern, value, .. } => {
            collect_expr_variables(value, used);
            // Only handle simple identifier patterns
            if let windjammer::parser::Pattern::Identifier(name) = pattern {
                defined.insert(name.clone());
            }
        }
        Statement::Assignment {
            target,
            value,
            compound_op: _,
            location: _,
        } => {
            collect_expr_variables(target, used);
            collect_expr_variables(value, used);
        }
        Statement::Return {
            value: Some(expr),
            location: _,
        } => {
            collect_expr_variables(expr, used);
        }
        Statement::Expression { expr, location: _ } => {
            collect_expr_variables(expr, used);
        }
        Statement::If {
            condition,
            then_block,
            else_block,
            location: _,
        } => {
            collect_expr_variables(condition, used);
            for s in then_block {
                collect_variable_usage(s, used, defined);
            }
            if let Some(else_stmts) = else_block {
                for s in else_stmts {
                    collect_variable_usage(s, used, defined);
                }
            }
        }
        Statement::For {
            pattern,
            iterable,
            body,
            location: _,
        } => {
            // Extract identifier from pattern
            if let windjammer::parser::Pattern::Identifier(var) = pattern {
                defined.insert(var.clone());
            }
            collect_expr_variables(iterable, used);
            for s in body {
                collect_variable_usage(s, used, defined);
            }
        }
        Statement::While {
            condition,
            body,
            location: _,
        } => {
            collect_expr_variables(condition, used);
            for s in body {
                collect_variable_usage(s, used, defined);
            }
        }
        Statement::Loop { body, location: _ } => {
            for s in body {
                collect_variable_usage(s, used, defined);
            }
        }
        _ => {}
    }
}

/// Collect variables from an expression
fn collect_expr_variables(expr: &Expression, used: &mut HashSet<String>) {
    match expr {
        Expression::Identifier { name, location: _ } => {
            used.insert(name.clone());
        }
        Expression::Binary { left, right, .. } => {
            collect_expr_variables(left, used);
            collect_expr_variables(right, used);
        }
        Expression::Unary { operand, .. } => {
            collect_expr_variables(operand, used);
        }
        Expression::Call {
            function,
            arguments,
            location: _,
        } => {
            collect_expr_variables(function, used);
            for (_, arg) in arguments {
                collect_expr_variables(arg, used);
            }
        }
        Expression::MethodCall {
            object, arguments, ..
        } => {
            collect_expr_variables(object, used);
            for (_, arg) in arguments {
                collect_expr_variables(arg, used);
            }
        }
        Expression::FieldAccess { object, .. } => {
            collect_expr_variables(object, used);
        }
        Expression::Index {
            object,
            index,
            location: _,
        } => {
            collect_expr_variables(object, used);
            collect_expr_variables(index, used);
        }
        Expression::Block {
            statements: stmts,
            location: _,
        } => {
            let mut defined = HashSet::new();
            for stmt in stmts {
                collect_variable_usage(stmt, used, &mut defined);
            }
        }
        _ => {}
    }
}

/// Infer return type from statements
fn infer_return_type(statements: &[Statement]) -> Option<Type> {
    // Look for explicit return statements
    for stmt in statements {
        if let Statement::Return {
            value: Some(_expr),
            location: _,
        } = stmt
        {
            // TODO: Infer type from expression
            return Some(Type::String); // Placeholder type
        }
    }

    // Check if last statement is an expression (implicit return)
    if let Some(Statement::Expression {
        expr: _expr,
        location: _,
    }) = statements.last()
    {
        // TODO: Infer type from expression
        return Some(Type::String); // Placeholder type
    }

    None
}

/// Generate new function declaration
fn generate_function(
    name: &str,
    _statements: &[Statement],
    analysis: &VariableAnalysis,
    _make_public: bool,
) -> String {
    let params = analysis
        .parameters
        .iter()
        .map(|(name, _ty)| format!("{}: Type", name))
        .collect::<Vec<_>>()
        .join(", ");

    let return_type = if analysis.return_type.is_some() {
        " -> Type"
    } else {
        ""
    };

    format!(
        "fn {}({}){}  {{\n    // Extracted code here\n}}",
        name, params, return_type
    )
}

/// Generate function call to replace extracted code
fn generate_function_call(name: &str, analysis: &VariableAnalysis) -> String {
    let args = analysis
        .parameters
        .iter()
        .map(|(name, _)| name.clone())
        .collect::<Vec<_>>()
        .join(", ");

    format!("{}({})", name, args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_extract_function_basic() {
        let db = Arc::new(Mutex::new(WindjammerDatabase::new()));
        let args = serde_json::json!({
            "code": "fn main() {\n    let x = 1;\n    let y = 2;\n    println!(\"{}\", x + y);\n}",
            "range": {
                "start": { "line": 1, "column": 4 },
                "end": { "line": 2, "column": 17 }
            },
            "function_name": "calculate_sum"
        });

        let result = handle(db, args).await;
        assert!(result.is_ok());
    }
}
