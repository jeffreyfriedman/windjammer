// String expression analysis utilities
//
// This module provides functions for analyzing string-related expressions:
// - Collecting string concatenation parts
// - Detecting string literals in expressions

use crate::parser::{BinaryOp, Expression, Literal};

/// Collects all parts of a string concatenation chain
///
/// For expressions like `"a" + "b" + "c"`, this returns `["a", "b", "c"]`.
/// For non-concatenation expressions, returns the expression itself as a single element.
///
/// # Examples
/// ```
/// // "hello" + "world" → ["hello", "world"]
/// // "a" + variable → ["a", variable]
/// // a * b → [a * b] (not a concatenation)
/// ```
pub fn collect_concat_parts(expr: &Expression) -> Vec<Expression> {
    let mut parts = Vec::new();
    collect_concat_parts_recursive(expr, &mut parts);
    parts
}

/// Recursively collect string concatenation parts
fn collect_concat_parts_recursive(expr: &Expression, parts: &mut Vec<Expression>) {
    match expr {
        Expression::Binary {
            left,
            op: BinaryOp::Add,
            right,
            ..
        } => {
            // Recursively collect parts from both sides of the + operator
            collect_concat_parts_recursive(left, parts);
            collect_concat_parts_recursive(right, parts);
        }
        _ => {
            // Not an addition, treat as a single part
            parts.push(expr.clone());
        }
    }
}

/// Collects string concatenation parts into a mutable Vec (static version for use without `self`)
///
/// This is the same as `collect_concat_parts` but uses a mutable reference
/// instead of returning a Vec, avoiding unnecessary allocation in some contexts.
pub fn collect_concat_parts_static(expr: &Expression, parts: &mut Vec<Expression>) {
    collect_concat_parts_recursive(expr, parts);
}

/// Checks if an expression contains a string literal (recursively)
///
/// This is useful for detecting string operations that might need special handling.
///
/// # Examples
/// ```
/// // "hello" → true
/// // 42 → false
/// // "hello" + variable → true
/// // variable + "world" → true
/// // a + b → false
/// ```
pub fn contains_string_literal(expr: &Expression) -> bool {
    match expr {
        Expression::Literal {
            value: Literal::String(_),
            ..
        } => true,
        Expression::Binary { left, right, .. } => {
            // Recursively check both sides
            contains_string_literal(left) || contains_string_literal(right)
        }
        _ => false,
    }
}

/// Checks if an expression produces a String (not &str)
///
/// Detects expressions that return owned String values like:
/// - `format!("hello")`  
/// - `obj.to_string()`
/// - `s.to_owned()`
/// - `String::from("text")`
/// - Blocks that end in String-producing expressions
///
/// # Examples
/// ```
/// // format!() → true
/// // .to_string() → true
/// // String::from() → true
/// // .len() → false
/// ```
pub fn expression_produces_string(expr: &Expression) -> bool {
    use crate::parser::Statement;
    match expr {
        // Macro invocations like format!(...) produce String
        Expression::MacroInvocation { name, .. } => {
            // format!, concat!, and write!-like macros produce String
            matches!(name.as_str(), "format" | "concat" | "format_args" | "write")
        }
        // Function calls like String::from, format() without !
        Expression::Call { function, .. } => {
            if let Expression::Identifier { name, .. } = &**function {
                name == "format" || name == "String" || name == "to_string"
            } else if let Expression::FieldAccess { field, .. } = &**function {
                field == "from" || field == "to_string"
            } else {
                false
            }
        }
        // Method calls like .to_string()
        Expression::MethodCall { method, .. } => method == "to_string" || method == "to_owned",
        // Blocks - check last statement for String-producing expression
        Expression::Block { statements, .. } => {
            if let Some(last) = statements.last() {
                match last {
                    Statement::Expression { expr, .. } => expression_produces_string(expr),
                    // If statements - check if branches return String
                    Statement::If {
                        then_block,
                        else_block,
                        ..
                    } => {
                        // Check if then branch produces String
                        let then_produces_string = then_block.last().is_some_and(|s| {
                            if let Statement::Expression { expr, .. } = s {
                                expression_produces_string(expr)
                            } else {
                                false
                            }
                        });
                        // Check else branch if present
                        let else_produces_string = else_block.as_ref().is_some_and(|block| {
                            block.last().is_some_and(|s| {
                                if let Statement::Expression { expr, .. } = s {
                                    expression_produces_string(expr)
                                } else {
                                    false
                                }
                            })
                        });
                        then_produces_string || else_produces_string
                    }
                    _ => false,
                }
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Checks if an expression contains .as_str() call (recursively)
///
/// This is useful for detecting when string conversion should be suppressed
/// because the user explicitly wants a &str.
///
/// # Examples
/// ```
/// // s.as_str() → true
/// // s.trim().as_str() → true (nested)
/// // obj.field.as_str() → true (field access)
/// // s.to_string() → false
/// ```
pub fn expression_has_as_str(expr: &Expression) -> bool {
    match expr {
        Expression::MethodCall { method, object, .. } => {
            method == "as_str" || expression_has_as_str(object)
        }
        Expression::Block { statements, .. } => block_has_as_str(statements),
        Expression::FieldAccess { object, .. } => expression_has_as_str(object),
        _ => false,
    }
}

/// Checks if a statement contains .as_str() call
///
/// Recursively checks the statement and any nested statements (like in if/else).
///
/// # Examples
/// ```
/// // let x = s.as_str(); → true
/// // return s.as_str(); → true
/// // if true { s.as_str() } → true
/// ```
pub fn statement_has_as_str(stmt: &crate::parser::Statement) -> bool {
    use crate::parser::Statement;
    match stmt {
        Statement::Expression { expr, .. } => expression_has_as_str(expr),
        Statement::Return {
            value: Some(expr), ..
        } => expression_has_as_str(expr),
        Statement::If {
            then_block,
            else_block,
            ..
        } => {
            block_has_as_str(then_block) || else_block.as_ref().is_some_and(|b| block_has_as_str(b))
        }
        _ => false,
    }
}

/// Checks if a block of statements contains .as_str() call
///
/// Returns true if any statement in the block contains .as_str().
///
/// # Examples
/// ```
/// // { s.as_str(); } → true
/// // { let x = 1; s.as_str(); } → true
/// // {} → false
/// ```
pub fn block_has_as_str(stmts: &[crate::parser::Statement]) -> bool {
    stmts.iter().any(statement_has_as_str)
}

// =============================================================================
// Explicit Reference Detection (for String Conversion Suppression)
// =============================================================================

/// Check if a block's LAST expression (return value) is an explicit reference
///
/// Used to suppress string literal conversion when one if-else branch returns
/// an explicit ref (&self.field, &var, etc.)
///
/// # Examples
/// ```
/// // { &x } → true
/// // { let y = 1; &x } → true
/// // { x } → false
/// // {} → false
/// ```
pub fn block_has_explicit_ref(stmts: &[crate::parser::Statement]) -> bool {
    use crate::parser::Statement;
    if stmts.is_empty() {
        return false;
    }

    // Only check the LAST statement (the return value of the block)
    if let Some(last_stmt) = stmts.last() {
        match last_stmt {
            Statement::Expression { expr, .. } => expression_is_explicit_ref(expr),
            Statement::Return {
                value: Some(expr), ..
            } => expression_is_explicit_ref(expr),
            _ => false,
        }
    } else {
        false
    }
}

/// Check if an expression is an explicit reference (&expr)
///
/// Returns true for &x, &self.field, etc.
/// Recursively checks blocks.
///
/// # Examples
/// ```
/// // &x → true
/// // &self.field → true
/// // { &x } → true (recursive)
/// // x → false
/// ```
pub fn expression_is_explicit_ref(expr: &Expression) -> bool {
    match expr {
        Expression::Unary {
            op: crate::parser::UnaryOp::Ref,
            ..
        } => true,
        Expression::Block { statements, .. } => block_has_explicit_ref(statements),
        _ => false,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_map::Location;
    use std::path::PathBuf;

    fn test_loc() -> Location {
        Location {
            file: PathBuf::from(""),
            line: 0,
            column: 0,
        }
    }

    #[test]
    fn test_collect_single_expression() {
        let expr = Expression::Identifier {
            name: "x".to_string(),
            location: Some(test_loc()),
        };

        let parts = collect_concat_parts(&expr);
        assert_eq!(parts.len(), 1);
    }

    #[test]
    fn test_collect_nested_concatenation() {
        // ("a" + "b") + ("c" + "d")
        let a = Expression::Literal {
            value: Literal::String("a".to_string()),
            location: Some(test_loc()),
        };
        let b = Expression::Literal {
            value: Literal::String("b".to_string()),
            location: Some(test_loc()),
        };
        let c = Expression::Literal {
            value: Literal::String("c".to_string()),
            location: Some(test_loc()),
        };
        let d = Expression::Literal {
            value: Literal::String("d".to_string()),
            location: Some(test_loc()),
        };

        let ab = Expression::Binary {
            left: Box::new(a),
            op: BinaryOp::Add,
            right: Box::new(b),
            location: Some(test_loc()),
        };
        let cd = Expression::Binary {
            left: Box::new(c),
            op: BinaryOp::Add,
            right: Box::new(d),
            location: Some(test_loc()),
        };
        let expr = Expression::Binary {
            left: Box::new(ab),
            op: BinaryOp::Add,
            right: Box::new(cd),
            location: Some(test_loc()),
        };

        let parts = collect_concat_parts(&expr);
        assert_eq!(parts.len(), 4); // Should flatten to ["a", "b", "c", "d"]
    }

    #[test]
    fn test_contains_string_in_nested_expression() {
        // ((a + b) * c) + "hello"
        let a = Expression::Identifier {
            name: "a".to_string(),
            location: Some(test_loc()),
        };
        let b = Expression::Identifier {
            name: "b".to_string(),
            location: Some(test_loc()),
        };
        let c = Expression::Identifier {
            name: "c".to_string(),
            location: Some(test_loc()),
        };
        let hello = Expression::Literal {
            value: Literal::String("hello".to_string()),
            location: Some(test_loc()),
        };

        let ab = Expression::Binary {
            left: Box::new(a),
            op: BinaryOp::Add,
            right: Box::new(b),
            location: Some(test_loc()),
        };
        let ab_mul_c = Expression::Binary {
            left: Box::new(ab),
            op: BinaryOp::Mul,
            right: Box::new(c),
            location: Some(test_loc()),
        };
        let expr = Expression::Binary {
            left: Box::new(ab_mul_c),
            op: BinaryOp::Add,
            right: Box::new(hello),
            location: Some(test_loc()),
        };

        assert!(contains_string_literal(&expr));
    }

    #[test]
    fn test_no_string_in_complex_expression() {
        // (a + b) * (c - d)
        let a = Expression::Identifier {
            name: "a".to_string(),
            location: Some(test_loc()),
        };
        let b = Expression::Identifier {
            name: "b".to_string(),
            location: Some(test_loc()),
        };
        let c = Expression::Identifier {
            name: "c".to_string(),
            location: Some(test_loc()),
        };
        let d = Expression::Identifier {
            name: "d".to_string(),
            location: Some(test_loc()),
        };

        let ab = Expression::Binary {
            left: Box::new(a),
            op: BinaryOp::Add,
            right: Box::new(b),
            location: Some(test_loc()),
        };
        let cd = Expression::Binary {
            left: Box::new(c),
            op: BinaryOp::Sub,
            right: Box::new(d),
            location: Some(test_loc()),
        };
        let expr = Expression::Binary {
            left: Box::new(ab),
            op: BinaryOp::Mul,
            right: Box::new(cd),
            location: Some(test_loc()),
        };

        assert!(!contains_string_literal(&expr));
    }
}
