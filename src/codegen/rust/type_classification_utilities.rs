//! Type Classification Utilities
//!
//! Helper functions for classifying and analyzing Windjammer types.
//! These are pure functions with no state dependencies.

use crate::parser::{Expression, Type};

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

/// Peel off the outermost reference layer from a type.
/// Returns the inner type if wrapped in Type::Reference, otherwise returns the type unchanged.
pub fn peel_reference_layer(t: &Type) -> &Type {
    match t {
        Type::Reference(inner) => inner.as_ref(),
        _ => t,
    }
}
