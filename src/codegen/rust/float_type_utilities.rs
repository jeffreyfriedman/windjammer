//! Float Type Utilities
//!
//! Helper functions for extracting and inferring float types from Windjammer Type AST nodes.
//! These are pure functions with no state dependencies.

use crate::parser::Type;

/// Extract float suffix for a literal based on assignment LHS type.
/// Returns Some("f32") or Some("f64") if the type contains a float, None otherwise.
pub fn float_literal_suffix_from_assignment_lhs(ty: &Type) -> Option<&'static str> {
    try_extract_float_type(ty)
}

/// Helper: Extract float type from a Type (handles tuples, arrays, Vec, Option, Result, etc.)
/// Searches recursively for float types, prioritizing f32 over f64.
/// Returns None if no float type is found in the type tree.
pub fn try_extract_float_type(ty: &Type) -> Option<&'static str> {
    match ty {
        Type::Custom(name) if name == "f32" => Some("f32"),
        Type::Custom(name) if name == "f64" => Some("f64"),
        Type::Float => Some("f64"),
        Type::Vec(inner) | Type::Array(inner, _) => try_extract_float_type(inner),
        Type::Option(inner) => try_extract_float_type(inner),
        Type::Result(ok_type, _) => try_extract_float_type(ok_type),
        Type::Reference(inner) | Type::MutableReference(inner) => {
            try_extract_float_type(inner)
        }
        Type::Tuple(types) => {
            for t in types {
                if let Some("f32") = try_extract_float_type(t) {
                    return Some("f32");
                }
            }
            for t in types {
                if let Some("f64") = try_extract_float_type(t) {
                    return Some("f64");
                }
            }
            None
        }
        Type::Parameterized(name, args) => {
            if (name == "Option" || name == "Result") && !args.is_empty() {
                try_extract_float_type(&args[0])
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Wrapper that defaults to f32 when no float type is found in context.
/// Windjammer convention: unconstrained float literals default to f32 (game/graphics standard).
pub fn extract_float_type_from_context(ty: &Type) -> &'static str {
    try_extract_float_type(ty).unwrap_or("f32")
}
