//! Expression Utilities
//!
//! Miscellaneous helper functions for expression code generation.
//! These are pure functions with no state dependencies.

use crate::parser::{Expression, UnaryOp};

/// Strip a leading `&ident` for collection key methods so `should_add_ref` can re-add `&` only when needed.
/// Parser emits `obj.method(args)` as `Call { function: FieldAccess(obj, method), args }`.
pub fn strip_unary_ref_for_collection_key_arg<'a>(
    method: &str,
    param_idx: usize,
    arg: &'a Expression<'a>,
) -> &'a Expression<'a> {
    let is_key_method =
        super::stdlib_method_traits::is_map_key_method(method) && param_idx == 0;
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
