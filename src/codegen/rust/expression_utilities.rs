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

/// Check whether an identifier is already a `&mut` reference, either through
/// explicit declaration (`param: &mut T`) or through ownership inference.
pub fn is_identifier_already_mut_ref(
    arg: &Expression,
    current_function_params: &[Parameter],
    inferred_mut_borrowed_params: &HashSet<String>,
) -> bool {
    if let Expression::Identifier { name, .. } = arg {
        let explicit_mut_ref = current_function_params.iter().any(|param| {
            param.name == *name && matches!(&param.type_, Type::MutableReference(_))
        });
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
        return false;
    }
    strip_trailing_clone(arg_str);
    if arg_str.starts_with('&') && !arg_str.starts_with("&mut ") {
        *arg_str = arg_str[1..].to_string();
    }
    super::rust_coercion_rules::Coercion::BorrowMut.apply(arg_str);
    true
}
