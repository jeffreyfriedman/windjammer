//! Expression Utilities
//!
//! Shared helper functions for expression and argument code generation.
//! These are pure functions with no state dependencies on CodeGenerator.

use crate::parser::{Expression, Parameter, Type, UnaryOp};
use std::collections::HashSet;

/// Strip a leading `&ident` for collection key lookups so `should_add_ref` can re-add `&` only when needed.
/// Parser emits `obj.method(args)` as `Call { function: FieldAccess(obj, method), args }`.
///
/// Ownership/borrow decisions require `sig` + `receiver_type`; when `sig` is absent, no strip occurs.
pub fn strip_unary_ref_for_collection_key_arg<'a>(
    param_idx: usize,
    arg: &'a Expression<'a>,
    sig: Option<&crate::analyzer::FunctionSignature>,
    receiver_type: Option<&str>,
) -> &'a Expression<'a> {
    let should_strip = sig.is_some_and(|s| {
        super::stdlib_method_traits::is_collection_key_lookup(s, param_idx, receiver_type)
    });
    if !should_strip {
        return arg;
    }
    if let Expression::Unary {
        op: UnaryOp::Ref,
        operand,
        ..
    } = arg
    {
        if matches!(&**operand, Expression::Identifier { .. }) {
            return operand;
        }
    }
    arg
}

/// Add `*` dereference prefix for comparison operands when needed.
/// Wraps binary expressions in parentheses before dereferencing.
pub fn star_for_deref_compare(expr: &Expression, s: &str) -> String {
    if s.starts_with('*') {
        return s.to_string();
    }
    let inner = if matches!(expr, Expression::Binary { .. }) {
        format!("({})", s)
    } else {
        s.to_string()
    };
    format!("*{}", inner)
}

/// Strip trailing `.clone()` from a generated argument string.
/// Used when the destination parameter is borrowed and doesn't need an owned copy.
pub fn strip_trailing_clone(arg_str: &mut String) {
    if arg_str.ends_with(".clone()") {
        arg_str.truncate(arg_str.len() - 8);
    }
}

/// Collapse `expr.clone().clone()` (and longer chains) to a single `.clone()`.
/// Call-site ownership + auto-clone can each append `.clone()` once.
pub fn collapse_redundant_clones(arg_str: &mut String) {
    while arg_str.ends_with(".clone().clone()") {
        arg_str.truncate(arg_str.len() - 8);
    }
}

/// True when prefix `&` would bind to the first sub-expression only (e.g. `&a + b`).
fn expr_needs_borrow_parentheses(expr_str: &str) -> bool {
    if expr_str.starts_with('(') {
        return false;
    }
    [
        " + ", " - ", " * ", " / ", " % ", " == ", " != ", " < ", " > ", " <= ", " >= ", " && ",
        " || ", " as ",
    ]
    .iter()
    .any(|op| expr_str.contains(op))
}

/// True when `expr_str` is a Rust string literal (`"…"`, `r"…"`, `r#"…"#`).
/// Literals are already `&str`; prefixing `&` yields `&&str`.
pub fn is_rust_string_literal_text(expr_str: &str) -> bool {
    expr_str.starts_with('"') || expr_str.starts_with("r\"") || expr_str.starts_with("r#\"")
}

/// Prefix shared borrow on generated Rust, parenthesizing compound expressions.
pub fn apply_shared_borrow_prefix(expr_str: &mut String) {
    if expr_str.starts_with('&') && !expr_str.starts_with("&&") {
        return;
    }
    // String literals are already `&str` — never emit `&"…"`.
    if is_rust_string_literal_text(expr_str) {
        return;
    }
    if expr_needs_borrow_parentheses(expr_str) {
        *expr_str = format!("&({expr_str})");
    } else {
        *expr_str = format!("&{expr_str}");
    }
}

/// Strip leading Rust borrow prefixes without turning `&mut x` into the invalid `mut x`.
pub fn borrow_base_expr(expr_str: &str) -> &str {
    if let Some(rest) = expr_str.strip_prefix("&mut ") {
        return rest;
    }
    if let Some(rest) = expr_str.strip_prefix('&') {
        return rest;
    }
    if let Some(rest) = expr_str.strip_prefix("mut ") {
        return rest;
    }
    expr_str
}

/// Convert a borrowed/mut-borrowed call arg into an owned value for an owned formal.
/// Avoids `key.clone().clone()` when the arg is already cloned.
pub fn coerce_borrowed_arg_to_owned(expr_str: &str) -> String {
    let base = borrow_base_expr(expr_str);
    if base.ends_with(".clone()") {
        base.to_string()
    } else {
        format!("{base}.clone()")
    }
}

/// Strip a single shared borrow (`&T`), never mutilating `&mut T` into `mut T`.
pub fn strip_shared_borrow_prefix(expr_str: &str) -> String {
    if let Some(rest) = expr_str.strip_prefix("&mut ") {
        return rest.to_string();
    }
    if let Some(rest) = expr_str.strip_prefix('&') {
        return rest.to_string();
    }
    expr_str.to_string()
}

/// Check whether an identifier is already a `&mut` reference, either through
/// explicit declaration (`param: &mut T`) or through ownership inference.
pub fn is_identifier_already_mut_ref(
    arg: &Expression,
    current_function_params: &[Parameter],
    inferred_mut_borrowed_params: &HashSet<String>,
) -> bool {
    if let Expression::Identifier { name, .. } = arg {
        let explicit_mut_ref = current_function_params
            .iter()
            .any(|param| param.name == *name && matches!(&param.type_, Type::MutableReference(_)));
        let inferred_mut_ref = inferred_mut_borrowed_params.contains(name.as_str());
        explicit_mut_ref || inferred_mut_ref
    } else {
        false
    }
}

/// Apply `&mut` coercion to an argument string when the callee expects MutBorrowed.
/// Strips trailing `.clone()` and lone `&` before applying `&mut`.
/// Returns `true` if coercion was applied.
pub fn apply_mut_borrow_coercion(
    arg: &Expression,
    arg_str: &mut String,
    current_function_params: &[Parameter],
    inferred_mut_borrowed_params: &HashSet<String>,
) -> bool {
    if super::expression_helpers::is_reference_expression(arg) {
        return false;
    }
    if is_identifier_already_mut_ref(arg, current_function_params, inferred_mut_borrowed_params) {
        // Reborrow of an existing `&mut` binding — strip spurious `.clone()` from
        // auto-clone / owned-context lowering inside loop bodies.
        strip_trailing_clone(arg_str);
        return false;
    }
    // Owned non-mut parameters cannot be `&mut` coerced (E0596). Downgrade to shared borrow
    // when the callee signature was over-inferred as MutBorrowed (read-only field/index chains).
    if let Expression::Identifier { name, .. } = arg {
        let is_owned_non_mut_param = current_function_params.iter().any(|p| {
            p.name == *name
                && !matches!(&p.type_, Type::Reference(_) | Type::MutableReference(_))
                && !inferred_mut_borrowed_params.contains(name)
        });
        if is_owned_non_mut_param {
            if !arg_str.starts_with('&') {
                super::rust_coercion_rules::Coercion::Borrow.apply(arg_str);
            }
            return true;
        }
    }
    strip_trailing_clone(arg_str);
    if arg_str.starts_with('&') && !arg_str.starts_with("&mut ") {
        *arg_str = arg_str[1..].to_string();
    }
    super::rust_coercion_rules::Coercion::BorrowMut.apply(arg_str);
    true
}

/// Whether `arg` can receive `&mut` at a call site (lvalues: identifiers, fields, indices).
pub fn arg_supports_mut_borrow_coercion(arg: &Expression) -> bool {
    matches!(
        arg,
        Expression::Identifier { .. }
            | Expression::FieldAccess { .. }
            | Expression::Index { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_borrow_parenthesizes_string_concat() {
        let mut s = "prev_hash_hex + &canonical_payload".to_string();
        apply_shared_borrow_prefix(&mut s);
        assert_eq!(s, "&(prev_hash_hex + &canonical_payload)");
    }

    #[test]
    fn shared_borrow_leaves_simple_identifiers_unwrapped() {
        let mut s = "tenant_slug".to_string();
        apply_shared_borrow_prefix(&mut s);
        assert_eq!(s, "&tenant_slug");
    }

    #[test]
    fn shared_borrow_skips_string_literals() {
        let mut s = r#""</div>""#.to_string();
        apply_shared_borrow_prefix(&mut s);
        assert_eq!(s, r#""</div>""#);
        assert!(is_rust_string_literal_text(r#""hi""#));
        assert!(is_rust_string_literal_text(r#"r"raw""#));
    }
}
