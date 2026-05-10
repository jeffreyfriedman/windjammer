//! Float Type Utilities
//!
//! Helper functions for extracting and inferring float types from Windjammer Type AST nodes.
//! These are pure functions with no state dependencies.

use crate::parser::Type;
use crate::type_inference::FloatType;

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

/// Convert Windjammer Type to FloatType enum for type inference.
/// Returns None for Type::Float (generic float, not proof of f64).
pub fn float_type_from_wj_ty(ty: &Type) -> Option<FloatType> {
    match ty {
        Type::Custom(n) if n == "f32" => Some(FloatType::F32),
        Type::Custom(n) if n == "f64" => Some(FloatType::F64),
        // `Type::Float` is the analyzer's generic "float" — it is not proof the value is f64.
        // Treating it as F64 made `(f32_expr, subexpr)` look like (F32, F64) and inserted
        // `f32_side as f64` while the other operand was still emitted as f32 → E0308.
        Type::Float => None,
        Type::Reference(inner) | Type::MutableReference(inner) => {
            float_type_from_wj_ty(inner)
        }
        _ => None,
    }
}

/// Determine cast target for float array element type mismatches.
/// `let x = 1.0` may codegen as `f64` while a struct field is `[f32; N]` — insert `as f32`.
pub fn float_array_elem_cast_target(expected_elem: &Type, actual: &Type) -> Option<&'static str> {
    fn peel(ty: &Type) -> &Type {
        match ty {
            Type::Reference(inner) | Type::MutableReference(inner) => peel(inner),
            t => t,
        }
    }
    let exp = peel(expected_elem);
    let got = peel(actual);
    let want_f32 = matches!(exp, Type::Custom(n) if n == "f32");
    let want_f64 = matches!(exp, Type::Custom(n) if n == "f64");
    let got_f32 = matches!(got, Type::Custom(n) if n == "f32");
    let got_f64 = matches!(got, Type::Custom(n) if n == "f64");
    let got_float = matches!(got, Type::Float);
    if want_f32 && (got_f64 || (got_float && !got_f32)) {
        return Some("f32");
    }
    if want_f64 && got_f32 {
        return Some("f64");
    }
    None
}
