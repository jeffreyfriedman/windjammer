//! Literal Expression Generation
//!
//! This module handles conversion of Windjammer literals to Rust literals.
//! Pure functions with no state dependencies.

use crate::parser::Literal;

/// Convert a Windjammer literal to Rust source code
pub fn generate_literal(lit: &Literal) -> String {
    match lit {
        Literal::Int(i) => i.to_string(),
        Literal::IntSuffixed(i, suffix) => format!("{}_{}", i, suffix),
        Literal::Float(f) => {
            let s = f.to_string();
            // Windjammer convention: unconstrained float literals default to f32
            // (game/graphics standard — most APIs use f32).
            // Context-sensitive inference in expression_generation.rs may override this.
            if !s.contains('.') && !s.contains('e') && !s.contains('E') {
                format!("{}.0_f32", s)
            } else {
                format!("{}_f32", s)
            }
        }
        Literal::String(s) => format!("\"{}\"", escape_string(s)),
        Literal::Char(c) => format!("'{}'", escape_char(*c)),
        Literal::Bool(b) => b.to_string(),
    }
}

/// Coerce a Rust string literal expression (`"foo"`) to owned `String` without `.to_string()`.
/// Windjammer user code never writes `.to_string()`; generated Rust uses `.into()` instead.
pub fn string_literal_to_owned_rust(literal_expr: &str) -> String {
    if literal_expr == "\"\"" {
        "String::new()".to_string()
    } else {
        format!("{}.into()", literal_expr)
    }
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
        assert_eq!(generate_literal(&Literal::Float(2.5)), "2.5_f32");
        assert_eq!(generate_literal(&Literal::Float(42.0)), "42.0_f32");
        let result = generate_literal(&Literal::Float(10.0));
        assert!(
            result.contains('.') && result.contains("f32"),
            "Float literal should have decimal point and f32 suffix: {}",
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
