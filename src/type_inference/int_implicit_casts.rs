/// Safe implicit cast rules for integer types
///
/// Windjammer is more ergonomic than Rust - it allows safe implicit casts
/// where the conversion is lossless or contextually obvious.
///
/// Design principle: "Compiler does the hard work, not the user"
use super::int_inference::IntType;

/// Check if we can safely cast from_ty to to_ty without explicit cast
pub fn is_safe_implicit_cast(from_ty: IntType, to_ty: IntType) -> bool {
    use IntType::*;

    if from_ty == to_ty {
        return true;
    }

    match (from_ty, to_ty) {
        // ALWAYS SAFE: Widening conversions (no data loss)
        (I8, I16 | I32 | I64 | Isize) => true,
        (I16, I32 | I64 | Isize) => true,
        (I32, I64 | U64) => true, // I32 → I64/U64 (positive values fit)

        (U8, U16 | U32 | U64 | Usize | I16 | I32 | I64) => true, // U8 fits in signed types >= I16
        (U16, U32 | U64 | Usize | I32 | I64) => true,            // U16 fits in signed types >= I32
        (U32, U64 | I64) => true,                                // U32 fits in I64
        // usize → i64: same ergonomics as `return items.len()` for `-> int` (Rust: `as i64`)
        (Usize, I64) => true,

        // Small signed → small unsigned (when contextually safe)
        (I32, U8) => true, // Common: literal 0-255 range in practice
        (I16, U8) => true,

        // CONTEXTUAL: Common Rust patterns that Windjammer should handle ergonomically

        // Array indexing: i32 ↔ usize (common source of type errors)
        // Rationale: Array indices are typically small positive numbers
        (I32, Usize) | (Usize, I32) => true,

        // u32 ↔ usize (common for buffer indices, counts, etc.)
        // Rationale: Both represent non-negative counts/indices
        (U32, Usize) | (Usize, U32) => true,

        // i32 ↔ u32 (common for coordinates, counts)
        // Rationale: Signed/unsigned mixing in arithmetic contexts
        (I32, U32) | (U32, I32) => true,

        // For loop ranges: i64 -> i32
        (I64, I32) => true,

        // i64 ↔ u64 (common for IDs, handles, large counts)
        // Rationale: Large integer types mixing signed/unsigned
        (I64, U64) | (U64, I64) => true,

        // i64 (Windjammer `int`) → usize: `Vec::with_capacity(x)` uses `x as usize` in codegen
        (I64, Usize) => true,

        // Unknown defaults to i32
        (Unknown, _) => true,
        (_, Unknown) => true,

        _ => false,
    }
}

/// Get the Rust cast suffix for a safe implicit cast
pub fn get_cast_suffix(to_ty: IntType) -> &'static str {
    use IntType::*;

    match to_ty {
        I8 => "i8",
        I16 => "i16",
        I32 => "i32",
        I64 => "i64",
        U8 => "u8",
        U16 => "u16",
        U32 => "u32",
        U64 => "u64",
        Usize => "usize",
        Isize => "isize",
        Unknown => "i32", // default to i32
    }
}

/// Determine which type to promote to when two types conflict
/// Prefer: wider > narrower, unsigned when mixing signed/unsigned of same width
pub fn promote_types(ty1: IntType, ty2: IntType) -> IntType {
    use IntType::*;

    if ty1 == ty2 {
        return ty1;
    }

    // Unknown defaults to other type or i32
    match (ty1, ty2) {
        (Unknown, Unknown) => return Unknown, // Both unknown stays unknown
        (Unknown, t) | (t, Unknown) => return t, // One unknown uses the other
        _ => {}
    }

    // Widening conversions
    match (ty1, ty2) {
        // Prefer wider integer
        (I8, I16 | I32 | I64 | Isize) | (I16 | I32 | I64 | Isize, I8) => I64,
        (I16, I32 | I64 | Isize) | (I32 | I64 | Isize, I16) => I64,
        (I32, I64 | Isize) | (I64 | Isize, I32) => I64,

        (U8, U16 | U32 | U64 | Usize) | (U16 | U32 | U64 | Usize, U8) => U64,
        (U16, U32 | U64 | Usize) | (U32 | U64 | Usize, U16) => U64,
        (U32, U64 | Usize) | (U64 | Usize, U32) => U64,

        // Mixed signed/unsigned of same width → prefer unsigned
        (I32, U32) | (U32, I32) => U32,
        (I64, U64) | (U64, I64) => U64,
        (Isize, Usize) | (Usize, Isize) => Usize,

        // Special case: usize is common in Rust, prefer it over specific widths
        (Usize, I32 | I64) | (I32 | I64, Usize) => Usize,

        _ => ty1, // fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use IntType::*;

    #[test]
    fn test_widening_always_safe() {
        assert!(is_safe_implicit_cast(I8, I16));
        assert!(is_safe_implicit_cast(I8, I32));
        assert!(is_safe_implicit_cast(I16, I32));
        assert!(is_safe_implicit_cast(I32, I64));

        assert!(is_safe_implicit_cast(U8, U16));
        assert!(is_safe_implicit_cast(U8, U32));
        assert!(is_safe_implicit_cast(U16, U32));
        assert!(is_safe_implicit_cast(U32, U64));
    }

    #[test]
    fn test_i32_usize_bidirectional() {
        // Common pattern: array indexing with i32
        assert!(is_safe_implicit_cast(I32, Usize));
        assert!(is_safe_implicit_cast(Usize, I32));
    }

    #[test]
    fn test_usize_to_i64_for_int_return() {
        assert!(is_safe_implicit_cast(Usize, I64));
    }

    #[test]
    fn test_u32_usize_bidirectional() {
        // Common pattern: counts and indices
        assert!(is_safe_implicit_cast(U32, Usize));
        assert!(is_safe_implicit_cast(Usize, U32));
    }

    #[test]
    fn test_i32_u32_bidirectional() {
        // Common pattern: signed/unsigned mixing
        assert!(is_safe_implicit_cast(I32, U32));
        assert!(is_safe_implicit_cast(U32, I32));
    }

    #[test]
    fn test_type_promotion() {
        // Promote to wider type
        assert_eq!(promote_types(I32, I64), I64);
        assert_eq!(promote_types(I64, I32), I64);

        // Promote to unsigned when mixed
        assert_eq!(promote_types(I32, U32), U32);
        assert_eq!(promote_types(U32, I32), U32);

        // Prefer usize for Rust compatibility
        assert_eq!(promote_types(Usize, I32), Usize);
        assert_eq!(promote_types(I32, Usize), Usize);
    }

    #[test]
    fn test_unknown_defaults() {
        assert!(is_safe_implicit_cast(Unknown, I32));
        assert!(is_safe_implicit_cast(I32, Unknown));

        assert_eq!(promote_types(Unknown, I32), I32);
        assert_eq!(promote_types(I32, Unknown), I32);
        assert_eq!(promote_types(Unknown, Unknown), Unknown); // Both unknown stays unknown
    }
}
