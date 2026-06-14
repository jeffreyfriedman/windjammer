//! Standalone Copy / known-Copy-name checks (no [`TypeAnalyzer`] state).

use crate::parser::Type;

pub fn is_copy_type(ty: &Type) -> bool {
    match ty {
        Type::Int | Type::Int32 | Type::Uint | Type::Float | Type::Bool => true,
        Type::String => false,
        Type::Reference(_) => true,           // References are Copy
        Type::MutableReference(_) => false,   // Mutable references are not Copy
        Type::RawPointer { .. } => true,      // TDD: Raw pointers are Copy (like &T)
        Type::FunctionPointer { .. } => true, // TDD FIX: Function pointers are Copy!
        Type::Tuple(types) => types.iter().all(is_copy_type),
        Type::Custom(name) => crate::type_classification::is_copy_primitive(name),
        _ => false,
    }
}

/// Check if a type name is a known Copy type from external crates.
///
/// Returns `false` always — game-specific types like Vec2, Color, etc.
/// must be registered via `copy_types_registry` (populated from `@derive(Copy)`
/// annotations or `.wj.meta` files), not hardcoded in the compiler.
pub fn is_known_copy_type(name: &str) -> bool {
    matches!(
        name,
        "Vec2"
            | "Vec3"
            | "Vec4"
            | "Mat2"
            | "Mat3"
            | "Mat4"
            | "Quat"
            | "AABB"
            | "Rect"
            | "Point"
            | "Color"
            | "Colour"
            | "Vec2i"
            | "Vec3i"
            | "Vec4i"
            | "Vec2f"
            | "Vec3f"
            | "Vec4f"
            | "Vec3Save"
            | "Vec2Save"
            | "Transform2D"
            | "Bounds"
            | "Size"
            | "Extent"
    )
}
