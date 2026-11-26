//! Literal Expression Generation
//!
//! This module handles conversion of Windjammer literals to Rust literals.
//! Pure functions with no state dependencies.

use crate::parser::Literal;

/// Rust text of a suffixed integer literal. The AST stores the two's-complement bit pattern, so
/// a `u64`/`usize` above `i64::MAX` arrives as a negative `i64` and must print as unsigned.
pub fn int_suffixed_literal(i: i64, suffix: &str) -> String {
    if i < 0 && matches!(suffix, "u64" | "usize") {
        format!("{}_{}", i as u64, suffix)
    } else {
        format!("{}_{}", i, suffix)
    }
}

/// A literal at a declared 128-bit width (`let h: u128 = 18446744073709551615`): the declared width wins,
/// decided by the one conversion table. `None` when the literal does not convert there (or `to` is
/// narrower than 128 bits, which the width-suffix machinery already handles).
pub fn literal_for_declared_width(
    lit: &Literal,
    to: crate::type_inference::int_conversion_table::IntWidth,
) -> Option<String> {
    use crate::type_inference::int_conversion_table::{conversion, Conversion, IntWidth};
    if to.bits() != 128 {
        return None;
    }
    let name = to.rust_name();
    match lit {
        Literal::Int(i) => (*i >= 0 || to.is_signed()).then(|| format!("{i}_{name}")),
        Literal::IntSuffixed(i, suf) => {
            let from = IntWidth::from_name(suf)?;
            if conversion(from, to) == Conversion::Error {
                return None;
            }
            // `u64`/`usize` payloads above i64::MAX are stored as their bit pattern.
            let v = if from.is_signed() { *i as i128 } else { *i as u64 as i128 };
            (v >= 0 || to.is_signed()).then(|| format!("{v}_{name}"))
        }
        Literal::Wide(v) => match to {
            IntWidth::U128 => Some(format!("{v}_u128")),
            _ if *v <= i128::MAX as u128 => Some(format!("{v}_{name}")),
            _ => None,
        },
        _ => None,
    }
}

/// Convert a Windjammer literal to Rust source code
pub fn generate_literal(lit: &Literal) -> String {
    match lit {
        Literal::Int(i) => i.to_string(),
        Literal::Wide(v) => v.to_string(),
        Literal::IntSuffixed(i, suffix) => int_suffixed_literal(*i, suffix),
        Literal::Float(f) => {
            let s = f.to_string();
            // ADR-0015 #1: unconstrained float literals are `float` = f64 (the interpreter and the
            // portable backends agree); annotations/inference that fix f32 override this downstream.
            if !s.contains('.') && !s.contains('e') && !s.contains('E') {
                format!("{}.0_f64", s)
            } else {
                format!("{}_f64", s)
            }
        }
        Literal::String(s) => format!("\"{}\"", escape_string(s)),
        Literal::Char(c) => format!("'{}'", escape_char(*c)),
        Literal::Bool(b) => b.to_string(),
    }
}

/// Convert a string literal to an owned `String` expression in Rust.
/// Uses `String::from()` which works in all contexts (match arms, tuple
/// destructuring, return position) without leaking `.to_string()`.
pub fn string_literal_to_owned_rust(literal_expr: &str) -> String {
    if literal_expr == "\"\"" {
        "String::new()".to_string()
    } else {
        format!("String::from({})", literal_expr)
    }
}

/// Returns true if the expression is already an owned `String` (no further conversion needed).
///
/// Equivalent to [`crate::codegen::rust::string_utilities::already_owned_string_expr`].
pub fn is_already_owned_string(expr: &str) -> bool {
    crate::codegen::rust::string_utilities::already_owned_string_expr(expr)
}

/// Escape special characters in strings for Rust string literals
fn escape_string(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            '"' => vec!['\\', '"'],
            '\\' => vec!['\\', '\\'],
            '\n' => vec!['\\', 'n'],
            '\r' => vec!['\\', 'r'],
            '\t' => vec!['\\', 't'],
            '\0' => vec!['\\', '0'],
            c => vec![c],
        })
        .collect()
}

/// Escape special characters in chars for Rust char literals
fn escape_char(c: char) -> String {
    match c {
        '\'' => "\\'".to_string(),
        '\\' => "\\\\".to_string(),
        '\n' => "\\n".to_string(),
        '\r' => "\\r".to_string(),
        '\t' => "\\t".to_string(),
        '\0' => "\\0".to_string(),
        c => c.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_literal() {
        assert_eq!(generate_literal(&Literal::Int(42)), "42");
        assert_eq!(generate_literal(&Literal::Int(-1)), "-1");
        assert_eq!(generate_literal(&Literal::Int(0)), "0");
    }

    #[test]
    fn test_float_literal() {
        assert_eq!(generate_literal(&Literal::Float(2.5)), "2.5_f64");
        assert_eq!(generate_literal(&Literal::Float(42.0)), "42.0_f64");
        let result = generate_literal(&Literal::Float(10.0));
        assert!(
            result.contains('.') && result.contains("f64"),
            "Float literal should have decimal point and f64 suffix: {}",
            result
        );
    }

    #[test]
    fn test_string_literal() {
        assert_eq!(
            generate_literal(&Literal::String("hello".to_string())),
            "\"hello\""
        );
        assert_eq!(generate_literal(&Literal::String("".to_string())), "\"\"");
    }

    #[test]
    fn test_string_escaping() {
        assert_eq!(
            generate_literal(&Literal::String("hello\nworld".to_string())),
            "\"hello\\nworld\""
        );
        assert_eq!(
            generate_literal(&Literal::String("say \"hi\"".to_string())),
            "\"say \\\"hi\\\"\""
        );
        assert_eq!(
            generate_literal(&Literal::String("path\\to\\file".to_string())),
            "\"path\\\\to\\\\file\""
        );
    }

    #[test]
    fn test_char_literal() {
        assert_eq!(generate_literal(&Literal::Char('a')), "'a'");
        assert_eq!(generate_literal(&Literal::Char('\'')), "'\\\''");
        assert_eq!(generate_literal(&Literal::Char('\n')), "'\\n'");
    }

    #[test]
    fn test_bool_literal() {
        assert_eq!(generate_literal(&Literal::Bool(true)), "true");
        assert_eq!(generate_literal(&Literal::Bool(false)), "false");
    }
}
