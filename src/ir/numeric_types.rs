//! Numeric type unification for the constraint solver.
//!
//! Replaces the sequential float → integer inference passes with a single
//! unified numeric type system. Both `IntType` and `FloatType` map into
//! `NumericType`, which the solver resolves globally.

use crate::type_inference::float_inference::FloatType;
use crate::type_inference::int_inference::IntType;

/// Unified numeric type covering both integer and float widths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NumericType {
    // Integer types
    I8,
    I16,
    I32,
    I64,
    I128,
    U8,
    U16,
    U32,
    U64,
    U128,
    Isize,
    Usize,
    // Float types
    F32,
    F64,
    // Unknown — not yet resolved
    Unknown,
}

impl NumericType {
    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            NumericType::I8
                | NumericType::I16
                | NumericType::I32
                | NumericType::I64
                | NumericType::I128
                | NumericType::U8
                | NumericType::U16
                | NumericType::U32
                | NumericType::U64
                | NumericType::U128
                | NumericType::Isize
                | NumericType::Usize
        )
    }

    pub fn is_float(&self) -> bool {
        matches!(self, NumericType::F32 | NumericType::F64)
    }

    pub fn is_unknown(&self) -> bool {
        matches!(self, NumericType::Unknown)
    }

    /// Default type for unconstrained integer literals.
    pub fn default_integer() -> Self {
        NumericType::I32
    }

    /// Default type for unconstrained float literals.
    pub fn default_float() -> Self {
        NumericType::F64
    }

    /// Check if an implicit cast from `self` to `target` is safe.
    /// Mirrors the existing `is_safe_implicit_cast` in `int_implicit_casts.rs`.
    pub fn is_safe_implicit_cast_to(&self, target: &NumericType) -> bool {
        use NumericType::*;
        if self == target {
            return true;
        }
        if self.is_unknown() || target.is_unknown() {
            return true;
        }
        // Float types are strict — no implicit f32/f64 mixing
        if self.is_float() || target.is_float() {
            return false;
        }
        // Integer widening casts (safe)
        matches!(
            (self, target),
            // Signed widening
            (I8, I16) | (I8, I32) | (I8, I64) | (I8, I128) |
            (I16, I32) | (I16, I64) | (I16, I128) |
            (I32, I64) | (I32, I128) |
            (I64, I128) |
            // Unsigned widening
            (U8, U16) | (U8, U32) | (U8, U64) | (U8, U128) |
            (U16, U32) | (U16, U64) | (U16, U128) |
            (U32, U64) | (U32, U128) |
            (U64, U128) |
            // Common index patterns (i32 <-> usize, u32 <-> usize)
            (I32, Usize) | (Usize, I32) |
            (U32, Usize) | (Usize, U32) |
            (I32, Isize) | (Isize, I32)
        )
    }

    /// When two types conflict, determine the preferred type.
    /// Returns None if the conflict cannot be resolved.
    pub fn resolve_conflict(&self, other: &NumericType) -> Option<NumericType> {
        if self == other {
            return Some(*self);
        }
        if self.is_unknown() {
            return Some(*other);
        }
        if other.is_unknown() {
            return Some(*self);
        }
        // Prefer specific over I32 default
        if *self == NumericType::I32 && other.is_integer() {
            return Some(*other);
        }
        if *other == NumericType::I32 && self.is_integer() {
            return Some(*self);
        }
        // Check safe implicit casts
        if self.is_safe_implicit_cast_to(other) {
            return Some(*other);
        }
        if other.is_safe_implicit_cast_to(self) {
            return Some(*self);
        }
        None
    }
}

impl From<IntType> for NumericType {
    fn from(it: IntType) -> Self {
        match it {
            IntType::I8 => NumericType::I8,
            IntType::I16 => NumericType::I16,
            IntType::I32 => NumericType::I32,
            IntType::I64 => NumericType::I64,
            IntType::U8 => NumericType::U8,
            IntType::U16 => NumericType::U16,
            IntType::U32 => NumericType::U32,
            IntType::U64 => NumericType::U64,
            IntType::Usize => NumericType::Usize,
            IntType::Isize => NumericType::Isize,
            IntType::Unknown => NumericType::Unknown,
        }
    }
}

impl From<FloatType> for NumericType {
    fn from(ft: FloatType) -> Self {
        match ft {
            FloatType::F32 => NumericType::F32,
            FloatType::F64 => NumericType::F64,
            FloatType::Unknown => NumericType::Unknown,
        }
    }
}

impl NumericType {
    pub fn to_int_type(&self) -> Option<IntType> {
        Some(match self {
            NumericType::I8 => IntType::I8,
            NumericType::I16 => IntType::I16,
            NumericType::I32 => IntType::I32,
            NumericType::I64 => IntType::I64,
            NumericType::U8 => IntType::U8,
            NumericType::U16 => IntType::U16,
            NumericType::U32 => IntType::U32,
            NumericType::U64 => IntType::U64,
            NumericType::Usize => IntType::Usize,
            NumericType::Isize => IntType::Isize,
            NumericType::Unknown => IntType::Unknown,
            _ => return None,
        })
    }

    pub fn to_float_type(&self) -> Option<FloatType> {
        Some(match self {
            NumericType::F32 => FloatType::F32,
            NumericType::F64 => FloatType::F64,
            NumericType::Unknown => FloatType::Unknown,
            _ => return None,
        })
    }
}

/// A numeric constraint in the unified system.
#[derive(Debug, Clone)]
pub enum NumericConstraint {
    /// An expression must be a specific numeric type.
    MustBe {
        expr_id: UnifiedExprId,
        numeric_type: NumericType,
        reason: String,
    },
    /// Two expressions must have the same numeric type.
    MustMatch {
        expr_a: UnifiedExprId,
        expr_b: UnifiedExprId,
        reason: String,
    },
}

/// Unified expression identifier (wraps the existing ExprId concept).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnifiedExprId {
    pub seq_id: usize,
    pub file_id: usize,
    pub line: usize,
    pub col: usize,
}

impl UnifiedExprId {
    pub fn new(seq_id: usize, file_id: usize, line: usize, col: usize) -> Self {
        Self {
            seq_id,
            file_id,
            line,
            col,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_widening_casts() {
        assert!(NumericType::I8.is_safe_implicit_cast_to(&NumericType::I32));
        assert!(NumericType::I32.is_safe_implicit_cast_to(&NumericType::I64));
        assert!(NumericType::U8.is_safe_implicit_cast_to(&NumericType::U32));
        assert!(!NumericType::I64.is_safe_implicit_cast_to(&NumericType::I32));
    }

    #[test]
    fn test_float_strict() {
        assert!(!NumericType::F32.is_safe_implicit_cast_to(&NumericType::F64));
        assert!(!NumericType::F64.is_safe_implicit_cast_to(&NumericType::F32));
    }

    #[test]
    fn test_index_pattern_casts() {
        assert!(NumericType::I32.is_safe_implicit_cast_to(&NumericType::Usize));
        assert!(NumericType::Usize.is_safe_implicit_cast_to(&NumericType::I32));
        assert!(NumericType::U32.is_safe_implicit_cast_to(&NumericType::Usize));
    }

    #[test]
    fn test_resolve_conflict() {
        assert_eq!(
            NumericType::I32.resolve_conflict(&NumericType::U32),
            Some(NumericType::U32)
        );
        assert_eq!(
            NumericType::Unknown.resolve_conflict(&NumericType::F32),
            Some(NumericType::F32)
        );
        assert_eq!(NumericType::F32.resolve_conflict(&NumericType::F64), None);
    }

    #[test]
    fn test_from_int_type() {
        assert_eq!(NumericType::from(IntType::I32), NumericType::I32);
        assert_eq!(NumericType::from(IntType::Usize), NumericType::Usize);
        assert_eq!(NumericType::from(IntType::Unknown), NumericType::Unknown);
    }

    #[test]
    fn test_from_float_type() {
        assert_eq!(NumericType::from(FloatType::F32), NumericType::F32);
        assert_eq!(NumericType::from(FloatType::F64), NumericType::F64);
    }

    #[test]
    fn test_round_trip_int() {
        let orig = IntType::U64;
        let unified = NumericType::from(orig);
        assert_eq!(unified.to_int_type(), Some(IntType::U64));
    }

    #[test]
    fn test_round_trip_float() {
        let orig = FloatType::F32;
        let unified = NumericType::from(orig);
        assert_eq!(unified.to_float_type(), Some(FloatType::F32));
    }
}
