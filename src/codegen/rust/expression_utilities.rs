//! Expression Utilities
//!
//! Shared helper functions for expression and argument code generation.
//! These are pure functions with no state dependencies on CodeGenerator.

use crate::parser::{Expression, Parameter, Type, UnaryOp};
use std::collections::HashSet;

/// Strip a leading `&ident` for collection key methods so `should_add_ref` can re-add `&` only when needed.
/// Parser emits `obj.method(args)` as `Call { function: FieldAccess(obj, method), args }`.
pub fn strip_unary_ref_for_collection_key_arg<'a>(
    method: &str,
    param_idx: usize,
    arg: &'a Expression<'a>,
) -> &'a Expression<'a> {
    let is_key_method = super::stdlib_method_traits::is_map_key_method(method) && param_idx == 0;
    if !is_key_method {
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

/// True when prefix `&` would bind to the first sub-expression only (e.g. `&a + b`).
fn expr_needs_borrow_parentheses(expr_str: &str) -> bool {
    if expr_str.starts_with('(') {
        return false;
    }
    [
        " + ", " - ", " * ", " / ", " % ", " == ", " != ", " < ", " > ", " <= ", " >= ", " && ",
        " || ",
    ]
    .iter()
    .any(|op| expr_str.contains(op))
}

/// Prefix shared borrow on generated Rust, parenthesizing compound expressions.
pub fn apply_shared_borrow_prefix(expr_str: &mut String) {
    if expr_str.starts_with('&') && !expr_str.starts_with("&&") {
        return;
    }
    if expr_needs_borrow_parentheses(expr_str) {
        *expr_str = format!("&({expr_str})");
    } else {
        *expr_str = format!("&{expr_str}");
    }
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
}
