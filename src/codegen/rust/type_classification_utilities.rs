//! Type Classification Utilities
//!
//! Helper functions for classifying and analyzing Windjammer types.
//! These are pure functions with no state dependencies.

use crate::parser::{Expression, Literal, Type};

/// Check if a type is an integer type (Int, Int32, Uint, or custom integer types).
pub fn is_integer_type(t: &Type) -> bool {
    match t {
        Type::Int | Type::Int32 | Type::Uint => true,
        Type::Custom(n) => crate::type_classification::is_integer_type(n),
        _ => false,
    }
}

/// Check if a type is a float type (Float, f32, or f64).
pub fn is_float_type(t: &Type) -> bool {
    match t {
        Type::Float => true,
        Type::Custom(n) => matches!(n.as_str(), "f32" | "f64"),
        _ => false,
    }
}

/// Determine the target float type for casting (f32 or f64).
/// Defaults to f32 unless explicitly f64.
pub fn float_target(t: &Type) -> &str {
    match t {
        Type::Custom(n) if n == "f64" => "f64",
        _ => "f32",
    }
}

/// Generate cast expression from integer to float.
/// Wraps binary expressions in parentheses before casting.
pub fn cast_int_to_float(s: &str, expr: &Expression, target: &str) -> String {
    if s.contains(" as ") || matches!(expr, Expression::Binary { .. }) {
        format!("({}) as {}", s, target)
    } else {
        format!("{} as {}", s, target)
    }
}

/// Auto-cast an integer call argument to float when the parameter expects f32/f64.
///
/// Returns `true` (and mutates `arg_str`) if a cast was applied.
/// Skips if the arg string already contains ` as f32`/` as f64`.
pub fn maybe_cast_int_arg_to_float(
    arg_str: &mut String,
    arg_expr: &Expression,
    param_type: &Type,
    arg_type: Option<&Type>,
) -> bool {
    let param_is_f32 =
        matches!(param_type, Type::Float) || matches!(param_type, Type::Custom(n) if n == "f32");
    let param_is_f64 = matches!(param_type, Type::Custom(n) if n == "f64");
    if !param_is_f32 && !param_is_f64 {
        return false;
    }

    let arg_is_int = matches!(
        arg_expr,
        Expression::Literal {
            value: Literal::Int(_),
            ..
        }
    ) || arg_type.is_some_and(|t| {
        matches!(t, Type::Int)
            || matches!(t, Type::Custom(n) if crate::type_classification::is_integer_type(n))
    });
    if !arg_is_int {
        return false;
    }
    if arg_str.contains(" as f32") || arg_str.contains(" as f64") {
        return false;
    }

    let target = if param_is_f32 { "f32" } else { "f64" };
    *arg_str = cast_int_to_float(arg_str, arg_expr, target);
    true
}

/// Peel off the outermost reference layer from a type.
/// Returns the inner type if wrapped in Type::Reference, otherwise returns the type unchanged.
pub fn peel_reference_layer(t: &Type) -> &Type {
    match t {
        Type::Reference(inner) => inner.as_ref(),
        _ => t,
    }
}
