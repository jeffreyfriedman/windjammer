//! String Utilities
//!
//! Helper functions for string type analysis and codegen decisions.
//! These are pure functions with no state dependencies.

use crate::parser::{Expression, Literal, Statement, Type};

/// Untyped `let`/`let mut` with string literal or string-producing `match` RHS needs `: String`
/// so `"x".into()` resolves (Rust cannot infer from `&str`-accepting call sites alone).
pub fn untyped_let_rhs_needs_string_ascription(value: &Expression) -> bool {
    match value {
        Expression::Literal {
            value: Literal::String(s),
            ..
        } => !s.is_empty(),
        Expression::Block { statements, .. } => statements.iter().any(|stmt| match stmt {
            Statement::Match { arms, .. } => {
                arms.iter().any(|arm| match_arm_needs_string_ascription(&arm.body))
            }
            Statement::If {
                then_block,
                else_block,
                ..
            } => {
                block_tail_is_string_producing(then_block)
                    && else_block
                        .as_ref()
                        .is_some_and(|b| block_tail_is_string_producing(b))
            }
            Statement::Expression { expr, .. } => match_arm_needs_string_ascription(expr),
            _ => false,
        }),
        _ => false,
    }
}

fn block_ends_with_string_literal(stmts: &[&Statement]) -> bool {
    stmts.last().is_some_and(|s| {
        matches!(
            s,
            Statement::Expression {
                expr: Expression::Literal {
                    value: Literal::String(_),
                    ..
                },
                ..
            }
        )
    })
}

/// If/else-if chains used as `let` RHS (including nested `else { if ... }`).
fn block_tail_is_string_producing(stmts: &[&Statement]) -> bool {
    stmts.last().is_some_and(|s| statement_tail_is_string_producing(s))
}

fn statement_tail_is_string_producing(stmt: &Statement) -> bool {
    match stmt {
        Statement::Expression { expr, .. } => match_arm_needs_string_ascription(expr),
        Statement::If {
            then_block,
            else_block,
            ..
        } => {
            block_tail_is_string_producing(then_block)
                && else_block
                    .as_ref()
                    .is_some_and(|b| block_tail_is_string_producing(b))
        }
        _ => false,
    }
}

pub fn match_arm_needs_string_ascription(body: &Expression) -> bool {
    matches!(
        body,
        Expression::Literal {
            value: Literal::String(s),
            ..
        } if !s.is_empty()
    ) || crate::codegen::rust::string_analysis::expression_produces_string(body)
        || crate::codegen::rust::arm_string_analysis::arm_returns_converted_string(body)
}

/// Check if return type expects owned String in Rust.
/// Enclosing function/slot expects owned `String` in Rust (`string` / `String` in Windjammer).
pub fn return_type_expects_owned_string(ret: &Option<Type>) -> bool {
    match ret {
        Some(Type::String) => true,
        Some(Type::Custom(n)) if n == "String" || n == "string" => true,
        _ => false,
    }
}

/// Generated Rust already produces an owned `String` (no second conversion pass).
pub fn already_owned_string_expr(expr_str: &str) -> bool {
    expr_str.ends_with(".to_string()")
        || expr_str.ends_with(".into()")
        || expr_str.ends_with(".clone()")
        || expr_str.starts_with("String::from(")
        || expr_str == "String::new()"
}

/// Idempotent: coerce a generated expression to owned `String` without `.to_string()` leakage.
pub fn coerce_expr_to_owned_string(expr_str: &str) -> String {
    if already_owned_string_expr(expr_str) {
        return expr_str.to_string();
    }
    if expr_str.starts_with('"') {
        return crate::codegen::rust::literals::string_literal_to_owned_rust(expr_str);
    }
    format!("{}.to_string()", expr_str)
}

// =============================================================================
// Shared call-site string coercion predicates
//
// These are used by the three argument-lowering pipelines:
//   regular_call_arguments.rs, method_call_expression_generation/arguments.rs,
//   field_access_method_args.rs
// =============================================================================

/// Parameter type is explicitly `&str` (not `&String`).
/// This indicates the callee wants a string slice — string literals can be passed directly.
pub fn param_is_rust_str_ref(param_type: &Type) -> bool {
    matches!(
        param_type,
        Type::Reference(inner) if matches!(**inner, Type::Custom(ref n) if n == "str")
    )
}

/// Parameter type is an owned Windjammer `string` / Rust `String`.
pub fn param_is_owned_string_type(param_type: &Type) -> bool {
    matches!(param_type, Type::String)
        || matches!(param_type, Type::Custom(n) if n == "string" || n == "String")
}

/// Identifier is a string constant (`SCOPE_*` or a variable bound to a string literal).
pub fn is_string_const_identifier(
    name: &str,
    auto_clone: Option<&crate::auto_clone::AutoCloneAnalysis>,
) -> bool {
    name.starts_with("SCOPE_")
        || auto_clone.is_some_and(|a| a.string_literal_vars.contains(name))
}

/// Callee borrows a string parameter: Rust will receive `&str` or `&String`.
/// True when the signature explicitly marks the param as `Borrowed`, or when
/// no ownership metadata exists but the param type is a Windjammer text type
/// (default borrow for `string` params in non-extern functions).
pub fn callee_borrows_string_param(
    sig: &crate::analyzer::FunctionSignature,
    sig_param_idx: usize,
) -> bool {
    if sig.is_extern {
        return false;
    }
    let is_text = sig
        .param_types
        .get(sig_param_idx)
        .is_some_and(crate::codegen::rust::types::is_windjammer_text_type);

    sig.param_ownership
        .get(sig_param_idx)
        .is_some_and(|&o| matches!(o, crate::analyzer::OwnershipMode::Borrowed))
        || (sig.param_ownership.is_empty() && is_text)
}

/// When `expr_str` ends with `.clone()` and the cloned identifier is a borrowed
/// string parameter, rewrite `.clone()` to `.to_string()`. Cloning a `&str`
/// produces another `&str`; `.to_string()` produces an owned `String`.
///
/// Returns `true` if a rewrite happened.
pub fn rewrite_borrowed_str_clone_to_to_string<'ast>(
    expr_str: &mut String,
    expr: &Expression<'ast>,
    borrowed_params: &std::collections::HashSet<String>,
    function_params: &[crate::parser::Parameter<'ast>],
) -> bool {
    if !expr_str.ends_with(".clone()") {
        return false;
    }
    let ident_name: Option<&str> = match expr {
        Expression::MethodCall { method, object, .. } if method == "clone" => match &**object {
            Expression::Identifier { name, .. } => Some(name.as_str()),
            _ => None,
        },
        _ => None,
    };
    if let Some(name) = ident_name {
        let is_string_type = function_params.iter().any(|p| {
            p.name == name
                && (matches!(p.type_, Type::String)
                    || matches!(p.type_, Type::Custom(ref n) if n == "string"))
        });
        let is_borrowed = borrowed_params.contains(name);
        if is_borrowed && is_string_type {
            *expr_str = expr_str.replace(".clone()", ".to_string()");
            return true;
        }
    }
    false
}

/// Append `.as_str()` to a match scrutinee when the match contains string literal
/// patterns. Skips if the expression is already `&str` (a borrowed param or a
/// param typed as `string`/`str`/`&str`).
pub fn maybe_append_as_str_for_match(
    value_str: &str,
    borrowed_params: &std::collections::HashSet<String>,
    function_params: &[crate::parser::Parameter],
) -> String {
    if value_str.ends_with(".as_str()") {
        return value_str.to_string();
    }
    let is_already_str_ref = borrowed_params.contains(value_str)
        || function_params.iter().any(|p| {
            p.name == value_str
                && (matches!(p.type_, Type::String)
                    || matches!(p.type_, Type::Custom(ref n) if n == "str" || n == "string" || n == "&str"))
        });
    if is_already_str_ref {
        value_str.to_string()
    } else {
        format!("{}.as_str()", value_str)
    }
}
