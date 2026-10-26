//! Integer width model: every integer value carries its Rust width so overflow,
//! wrapping and casts behave like the compiled program (panic on overflow).

use crate::parser::Type;

/// Rust integer widths. `Lit` is an unsuffixed literal whose width is not yet known;
/// it adopts the width of the other operand / the declared type it is bound to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntTy {
    I8,
    I16,
    I32,
    I64,
    I128,
    Isize,
    U8,
    U16,
    U32,
    U64,
    U128,
    Usize,
    Lit,
}

impl IntTy {
    pub fn from_name(name: &str) -> Option<IntTy> {
        Some(match name {
            "i8" => IntTy::I8,
            "i16" => IntTy::I16,
            "i32" => IntTy::I32,
            "i64" | "int" => IntTy::I64,
            "i128" => IntTy::I128,
            "isize" => IntTy::Isize,
            "u8" => IntTy::U8,
            "u16" => IntTy::U16,
            "u32" => IntTy::U32,
            "u64" | "uint" => IntTy::U64,
            "u128" => IntTy::U128,
            "usize" => IntTy::Usize,
            _ => return None,
        })
    }

    pub fn from_type(t: &Type) -> Option<IntTy> {
        match t {
            Type::Int => Some(IntTy::I64),
            Type::Int32 => Some(IntTy::I32),
            Type::Uint => Some(IntTy::U64),
            Type::Custom(n) => IntTy::from_name(n),
            Type::Reference(inner) | Type::MutableReference(inner) => IntTy::from_type(inner),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            IntTy::I8 => "i8",
            IntTy::I16 => "i16",
            IntTy::I32 => "i32",
            IntTy::I64 | IntTy::Lit => "i64",
            IntTy::I128 => "i128",
            IntTy::Isize => "isize",
            IntTy::U8 => "u8",
            IntTy::U16 => "u16",
            IntTy::U32 => "u32",
            IntTy::U64 => "u64",
            IntTy::U128 => "u128",
            IntTy::Usize => "usize",
        }
    }

    pub fn bits(self) -> u32 {
        match self {
            IntTy::I8 | IntTy::U8 => 8,
            IntTy::I16 | IntTy::U16 => 16,
            IntTy::I32 | IntTy::U32 => 32,
            IntTy::I64 | IntTy::U64 | IntTy::Isize | IntTy::Usize | IntTy::Lit => 64,
            IntTy::I128 | IntTy::U128 => 128,
        }
    }

    pub fn signed(self) -> bool {
        matches!(
            self,
            IntTy::I8 | IntTy::I16 | IntTy::I32 | IntTy::I64 | IntTy::I128 | IntTy::Isize | IntTy::Lit
        )
    }

    pub fn min(self) -> i128 {
        if self.signed() {
            if self.bits() == 128 {
                i128::MIN
            } else {
                -(1i128 << (self.bits() - 1))
            }
        } else {
            0
        }
    }

    pub fn max(self) -> i128 {
        if self.signed() {
            if self.bits() == 128 {
                i128::MAX
            } else {
                (1i128 << (self.bits() - 1)) - 1
            }
        } else if self.bits() >= 127 {
            i128::MAX // the largest literal; larger u128 values are bit patterns (`u128ops.rs`)
        } else {
            (1i128 << self.bits()) - 1
        }
    }

    #[inline(always)]
    pub fn fits(self, v: i128) -> bool {
        // (min, max) per variant, in declaration order (hot: every checked operation)
        const RANGE: [(i128, i128); 13] = [
            (i8::MIN as i128, i8::MAX as i128),
            (i16::MIN as i128, i16::MAX as i128),
            (i32::MIN as i128, i32::MAX as i128),
            (i64::MIN as i128, i64::MAX as i128),
            (i128::MIN, i128::MAX),
            (i64::MIN as i128, i64::MAX as i128),
            (0, u8::MAX as i128),
            (0, u16::MAX as i128),
            (0, u32::MAX as i128),
            (0, u64::MAX as i128),
            (0, i128::MAX),
            (0, u64::MAX as i128),
            (i64::MIN as i128, i64::MAX as i128),
        ];
        let (lo, hi) = RANGE[self as usize];
        v >= lo && v <= hi
    }

    /// Wrap `v` into this width (two's complement), as `as` and `wrapping_*` do.
    pub fn wrap(self, v: i128) -> i128 {
        let bits = self.bits();
        if bits >= 128 {
            return v;
        }
        let modulus = 1i128 << bits;
        let mut r = v.rem_euclid(modulus);
        if self.signed() && r >= (modulus >> 1) {
            r -= modulus;
        }
        r
    }

    /// Result width of a binary operation: a literal adopts the other side.
    pub fn unify(a: IntTy, b: IntTy) -> IntTy {
        if a == IntTy::Lit {
            b
        } else {
            a
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_and_wrapping() {
        assert_eq!(IntTy::U8.max(), 255);
        assert_eq!(IntTy::I8.min(), -128);
        assert_eq!(IntTy::I32.wrap(2147483648), -2147483648);
        assert_eq!(IntTy::U8.wrap(-1), 255);
        assert_eq!(IntTy::U64.wrap(-1), 18446744073709551615);
        assert!(!IntTy::I32.fits(2147483648));
    }
}
