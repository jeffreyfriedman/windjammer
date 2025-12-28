//! AST Utility Functions
//!
//! Pure utility functions for analyzing and extracting information from AST nodes.
//! These functions have no state dependencies and can be used independently.

use crate::parser::{Expression, Statement};

/// Count statements in a function body with weighted complexity
///
/// Used for inline heuristics - simple statements count as 1,
/// control flow statements are weighted more heavily to reflect complexity.
///
/// Weights:
/// - Simple statements (let, const, return, assignment, expression): 1
/// - Control flow (if, while, loop, for): 3
/// - Match statements: 5
/// - Thread/async spawns: 2
pub fn count_statements(body: &[Statement]) -> usize {
    let mut count = 0;
    for stmt in body {
        count += match stmt {
            Statement::Let { .. } => 1,
            Statement::Const { .. } => 1,
            Statement::Static { .. } => 1,
            Statement::Return { .. } => 1,
            Statement::Expression { .. } => 1,
            Statement::If { .. } => 3, // Weighted more heavily
            Statement::While { .. } => 3,
            Statement::Loop { .. } => 3,
            Statement::For { .. } => 3,
            Statement::Match { .. } => 5, // Match statements are complex
            Statement::Assignment { .. } => 1,
            Statement::Thread { .. } => 2, // Thread spawn
            Statement::Async { .. } => 2,  // Async spawn
            Statement::Defer { .. } => 1,
            Statement::Break { .. } => 1,
            Statement::Continue { .. } => 1,
            Statement::Use { .. } => 0, // Use statements don't count toward complexity
        };
    }
    count
}

/// Extract function name from a call expression
///
/// Returns the function name from an Identifier or FieldAccess expression.
/// Returns empty string if the expression doesn't represent a callable.
///
/// Examples:
/// - `foo()` → "foo"
/// - `module.function()` → "function"
/// - `42` → ""
pub fn extract_function_name(expr: &Expression) -> String {
    match expr {
        Expression::Identifier { name, .. } => name.clone(),
        Expression::FieldAccess { field, .. } => field.clone(),
        _ => String::new(), // Can't determine function name
    }
}

/// Extract a field access, method call, or index expression path from an expression
///
/// Recursively builds a path string like "config.paths", "source.get_items()", "items[0]".
/// Returns None if the expression doesn't represent an accessor.
///
/// This function matches the logic in auto_clone.rs for identifying cloneable paths.
///
/// Examples:
/// - `variable` → Some("variable")
/// - `config.name` → Some("config.name")
/// - `app.config.paths` → Some("app.config.paths")
/// - `source.get_items()` → Some("source.get_items()")
/// - `items[0]` → Some("items[0]")
/// - `42` → None
#[allow(clippy::only_used_in_recursion)]
pub fn extract_field_access_path(expr: &Expression) -> Option<String> {
    match expr {
        Expression::Identifier { name, .. } => Some(name.clone()),
        Expression::FieldAccess { object, field, .. } => {
            // Recursively build the path: object.field
            extract_field_access_path(object).map(|base_path| format!("{}.{}", base_path, field))
        }
        Expression::MethodCall { object, method, .. } => {
            // Build path: object.method()
            extract_field_access_path(object).map(|base_path| format!("{}.{}()", base_path, method))
        }
        Expression::Index { object, index, .. } => {
            // Build path: object[index]
            // For display purposes, we try to show the index value if it's a literal
            let index_str = match index {
                Expression::Literal { value, .. } => format!("{:?}", value),
                _ => "_".to_string(), // Non-literal index
            };
            extract_field_access_path(object)
                .map(|base_path| format!("{}[{}]", base_path, index_str))
        }
        _ => None,
    }
}
