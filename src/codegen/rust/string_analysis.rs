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
pub fn collect_concat_parts<'ast>(expr: &Expression<'ast>) -> Vec<Expression<'ast>> {
    let mut parts = Vec::new();
    collect_concat_parts_recursive(expr, &mut parts);
    parts
}

/// Recursively collect string concatenation parts
fn collect_concat_parts_recursive<'ast>(
    expr: &Expression<'ast>,
    parts: &mut Vec<Expression<'ast>>,
) {
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
pub fn collect_concat_parts_static<'ast>(
    expr: &Expression<'ast>,
    parts: &mut Vec<Expression<'ast>>,
) {
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

/// Checks if an expression produces a `String` (owned text).
///
/// Language-level: `format!` / `concat!` macros and WJ `.string()` sugar.
/// Method/associated call sites consult the stdlib signature registry for
/// owned-`String` returns — not a hardcoded `to_string|to_owned` ownership oracle.
pub fn expression_produces_string(expr: &Expression) -> bool {
    use crate::parser::Statement;
    match expr {
        Expression::MacroInvocation { name, .. } => {
            matches!(name.as_str(), "format" | "concat" | "format_args" | "write")
        }
        Expression::Call { function, .. } => match &**function {
            Expression::Identifier { name, .. } => {
                name == "format" || stdlib_method_returns_owned_string(name, None)
            }
            Expression::FieldAccess { object, field, .. } => {
                let receiver = match &**object {
                    Expression::Identifier { name, .. } => Some(name.as_str()),
                    _ => None,
                };
                stdlib_method_returns_owned_string(field, receiver)
            }
            _ => false,
        },
        Expression::MethodCall { method, object, .. } => {
            if method == "string" {
                return true;
            }
            let receiver = match &**object {
                Expression::Identifier { name, .. }
                    if name.chars().next().is_some_and(|c| c.is_uppercase()) =>
                {
                    Some(name.as_str())
                }
                _ => None,
            };
            stdlib_method_returns_owned_string(method, receiver)
                || stdlib_method_returns_owned_string(method, Some("String"))
                || stdlib_method_returns_owned_string(method, Some("str"))
        }
        Expression::Block { statements, .. } => {
            if let Some(last) = statements.last() {
                match last {
                    Statement::Expression { expr, .. } => expression_produces_string(expr),
                    Statement::If {
                        then_block,
                        else_block,
                        ..
                    } => {
                        let then_produces_string = then_block.last().is_some_and(|s| {
                            if let Statement::Expression { expr, .. } = s {
                                expression_produces_string(expr)
                            } else {
                                false
                            }
                        });
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
        Expression::Binary {
            op: crate::parser::BinaryOp::Add,
            left,
            right,
            ..
        } => expression_produces_string(left) || expression_produces_string(right),
        _ => false,
    }
}

/// Stdlib / qualified lookup: does `Receiver::method` return owned text?
fn stdlib_method_returns_owned_string(method: &str, receiver: Option<&str>) -> bool {
    use crate::analyzer::{FunctionSignature, OwnershipMode, SignatureRegistry};
    use crate::parser::Type;

    let is_owned_text = |ty: &Type| -> bool {
        matches!(ty, Type::String)
            || matches!(ty, Type::Custom(n) if n == "String" || n == "string")
            || (crate::codegen::rust::types::is_windjammer_text_type(ty)
                && !matches!(ty, Type::Reference(_) | Type::MutableReference(_)))
    };
    let check = |sig: &FunctionSignature| -> bool {
        matches!(sig.return_ownership, OwnershipMode::Owned)
            && sig.return_type.as_ref().is_some_and(is_owned_text)
    };
    let reg = SignatureRegistry::stdlib();
    if let Some(recv) = receiver {
        let key = format!("{recv}::{method}");
        if let Some(sig) = reg.get_signature(&key) {
            return check(sig);
        }
    }
    reg.get_signature(method).is_some_and(check)
}

pub fn expression_has_as_str(expr: &Expression) -> bool {
    match expr {
        Expression::MethodCall { method, object, .. } => {
            super::rust_stdlib_annotations::is_strip_redundant(method)
                || expression_has_as_str(object)
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
pub fn block_has_as_str<'ast>(stmts: &[&'ast crate::parser::Statement<'ast>]) -> bool {
    stmts.iter().any(|s| statement_has_as_str(s))
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
pub fn block_has_explicit_ref<'ast>(stmts: &[&'ast crate::parser::Statement<'ast>]) -> bool {
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
    use crate::test_utils::test_alloc_expr;
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

        let a_ref = test_alloc_expr(a);
        let b_ref = test_alloc_expr(b);
        let c_ref = test_alloc_expr(c);
        let d_ref = test_alloc_expr(d);

        let ab = test_alloc_expr(Expression::Binary {
            left: a_ref,
            op: BinaryOp::Add,
            right: b_ref,
            location: Some(test_loc()),
        });
        let cd = test_alloc_expr(Expression::Binary {
            left: c_ref,
            op: BinaryOp::Add,
            right: d_ref,
            location: Some(test_loc()),
        });
        let expr = Expression::Binary {
            left: ab,
            op: BinaryOp::Add,
            right: cd,
            location: Some(test_loc()),
        };

        let parts = collect_concat_parts(&expr);
        assert_eq!(parts.len(), 4); // Should flatten to ["a", "b", "c", "d"]
    }

    #[test]
    fn test_contains_string_in_nested_expression() {
        // ((a + b) * c) + "hello"
        let a = test_alloc_expr(Expression::Identifier {
            name: "a".to_string(),
            location: Some(test_loc()),
        });
        let b = test_alloc_expr(Expression::Identifier {
            name: "b".to_string(),
            location: Some(test_loc()),
        });
        let c = test_alloc_expr(Expression::Identifier {
            name: "c".to_string(),
            location: Some(test_loc()),
        });
        let hello = test_alloc_expr(Expression::Literal {
            value: Literal::String("hello".to_string()),
            location: Some(test_loc()),
        });

        let ab = test_alloc_expr(Expression::Binary {
            left: a,
            op: BinaryOp::Add,
            right: b,
            location: Some(test_loc()),
        });
        let ab_mul_c = test_alloc_expr(Expression::Binary {
            left: ab,
            op: BinaryOp::Mul,
            right: c,
            location: Some(test_loc()),
        });
        let expr = Expression::Binary {
            left: ab_mul_c,
            op: BinaryOp::Add,
            right: hello,
            location: Some(test_loc()),
        };

        assert!(contains_string_literal(&expr));
    }

    #[test]
    fn test_no_string_in_complex_expression() {
        // (a + b) * (c - d)
        let a = test_alloc_expr(Expression::Identifier {
            name: "a".to_string(),
            location: Some(test_loc()),
        });
        let b = test_alloc_expr(Expression::Identifier {
            name: "b".to_string(),
            location: Some(test_loc()),
        });
        let c = test_alloc_expr(Expression::Identifier {
            name: "c".to_string(),
            location: Some(test_loc()),
        });
        let d = test_alloc_expr(Expression::Identifier {
            name: "d".to_string(),
            location: Some(test_loc()),
        });

        let ab = test_alloc_expr(Expression::Binary {
            left: a,
            op: BinaryOp::Add,
            right: b,
            location: Some(test_loc()),
        });
        let cd = test_alloc_expr(Expression::Binary {
            left: c,
            op: BinaryOp::Sub,
            right: d,
            location: Some(test_loc()),
        });
        let expr = Expression::Binary {
            left: ab,
            op: BinaryOp::Mul,
            right: cd,
            location: Some(test_loc()),
        };

        assert!(!contains_string_literal(&expr));
    }
}
