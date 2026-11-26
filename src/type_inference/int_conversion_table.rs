//! The ONE table of implicit integer width conversions at a declared-width site
//! (annotated binding, argument to a declared parameter, return, struct field init).
//! Design: `docs/design/int-conversion-table.md`. Every site asks `conversion(from, to)`;
//! `tests/int_conversion_table_rustc_test.rs` checks every pair against rustc.
//!
//! Pointer-sized types are 64-bit (the supported targets), so `usize <-> u64` and `isize <-> i64`
//! are lossless. Integers never convert to floats (that is a diagnostic, not a table row).

/// Every integer width a program can declare.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntWidth {
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
}

pub const ALL_WIDTHS: [IntWidth; 12] = [
    IntWidth::I8,
    IntWidth::I16,
    IntWidth::I32,
    IntWidth::I64,
    IntWidth::I128,
    IntWidth::Isize,
    IntWidth::U8,
    IntWidth::U16,
    IntWidth::U32,
    IntWidth::U64,
    IntWidth::U128,
    IntWidth::Usize,
];

/// What happens when a value of width `from` meets a declared width `to`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Conversion {
    /// Same width: nothing to do.
    Same,
    /// Lossless widening: implicit, emitted as `as`.
    Widen,
    /// The existing lossy ergonomics rule (`len()` into `i32`, `i32` index into `usize`): implicit, emitted as `as`.
    LossyAs,
    /// Not implicit: a type error.
    Error,
}

impl IntWidth {
    pub fn rust_name(self) -> &'static str {
        match self {
            IntWidth::I8 => "i8",
            IntWidth::I16 => "i16",
            IntWidth::I32 => "i32",
            IntWidth::I64 => "i64",
            IntWidth::I128 => "i128",
            IntWidth::Isize => "isize",
            IntWidth::U8 => "u8",
            IntWidth::U16 => "u16",
            IntWidth::U32 => "u32",
            IntWidth::U64 => "u64",
            IntWidth::U128 => "u128",
            IntWidth::Usize => "usize",
        }
    }

    /// A declared Windjammer or Rust integer type name (`int` is `i64`, `uint` is `u32`).
    pub fn from_name(name: &str) -> Option<IntWidth> {
        Some(match name {
            "i8" => IntWidth::I8,
            "i16" => IntWidth::I16,
            "i32" => IntWidth::I32,
            "i64" | "int" => IntWidth::I64,
            "i128" => IntWidth::I128,
            "isize" => IntWidth::Isize,
            "u8" => IntWidth::U8,
            "u16" => IntWidth::U16,
            "u32" | "uint" => IntWidth::U32,
            "u64" => IntWidth::U64,
            "u128" => IntWidth::U128,
            "usize" => IntWidth::Usize,
            _ => return None,
        })
    }

    pub fn is_signed(self) -> bool {
        matches!(
            self,
            IntWidth::I8 | IntWidth::I16 | IntWidth::I32 | IntWidth::I64 | IntWidth::I128 | IntWidth::Isize
        )
    }

    /// Bits on the supported (64-bit) targets.
    pub fn bits(self) -> u32 {
        match self {
            IntWidth::I8 | IntWidth::U8 => 8,
            IntWidth::I16 | IntWidth::U16 => 16,
            IntWidth::I32 | IntWidth::U32 => 32,
            IntWidth::I64 | IntWidth::U64 | IntWidth::Isize | IntWidth::Usize => 64,
            IntWidth::I128 | IntWidth::U128 => 128,
        }
    }
}

/// Every value of `from` fits `to`.
pub fn is_lossless(from: IntWidth, to: IntWidth) -> bool {
    match (from.is_signed(), to.is_signed()) {
        (true, true) | (false, false) => to.bits() >= from.bits(),
        (false, true) => to.bits() > from.bits(),
        (true, false) => false,
    }
}

/// Pairs the language already converts implicitly although they can lose a value. Kept exactly.
const LOSSY_AS_PAIRS: &[(IntWidth, IntWidth)] = &[
    (IntWidth::I32, IntWidth::U64),
    (IntWidth::I32, IntWidth::U8),
    (IntWidth::I16, IntWidth::U8),
    (IntWidth::I32, IntWidth::Usize),
    (IntWidth::Usize, IntWidth::I32),
    (IntWidth::U32, IntWidth::I32),
    (IntWidth::Usize, IntWidth::U32),
    (IntWidth::I32, IntWidth::U32),
    (IntWidth::I64, IntWidth::I32),
    (IntWidth::I64, IntWidth::U64),
    (IntWidth::U64, IntWidth::I64),
    (IntWidth::I64, IntWidth::Usize),
    (IntWidth::Usize, IntWidth::I64),
];

pub fn conversion(from: IntWidth, to: IntWidth) -> Conversion {
    if from == to {
        Conversion::Same
    } else if is_lossless(from, to) {
        Conversion::Widen
    } else if LOSSY_AS_PAIRS.contains(&(from, to)) {
        Conversion::LossyAs
    } else {
        Conversion::Error
    }
}

/// The value converts at a declared-width site without a diagnostic.
pub fn is_implicit(from: IntWidth, to: IntWidth) -> bool {
    conversion(from, to) != Conversion::Error
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usize_widens_to_u64_and_i128() {
        assert_eq!(conversion(IntWidth::Usize, IntWidth::U64), Conversion::Widen);
        assert_eq!(conversion(IntWidth::Usize, IntWidth::U128), Conversion::Widen);
        assert_eq!(conversion(IntWidth::Usize, IntWidth::I32), Conversion::LossyAs);
        assert_eq!(conversion(IntWidth::U64, IntWidth::U32), Conversion::Error);
    }
}
