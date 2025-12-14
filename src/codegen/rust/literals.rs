//! Literal Expression Generation
//!
//! This module handles conversion of Windjammer literals to Rust literals.
//! Pure functions with no state dependencies.

use crate::parser::Literal;

/// Convert a Windjammer literal to Rust source code
pub fn generate_literal(lit: &Literal) -> String {
    match lit {
        Literal::Int(i) => i.to_string(),
        Literal::Float(f) => {
            let s = f.to_string();
            // Ensure float literals have a decimal point
            if !s.contains('.') && !s.contains('e') && !s.contains('E') {
                format!("{}.0", s)
            } else {
                s
            }
        }
        Literal::String(s) => format!("\"{}\"", escape_string(s)),
        Literal::Char(c) => format!("'{}'", escape_char(*c)),
        Literal::Bool(b) => b.to_string(),
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
        assert_eq!(generate_literal(&Literal::Float(3.14)), "3.14");
        assert_eq!(generate_literal(&Literal::Float(42.0)), "42.0");
        // Integer-like floats should have .0
        let result = generate_literal(&Literal::Float(10.0));
        assert!(result.contains('.'), "Float literal should have decimal point: {}", result);
    }

    #[test]
    fn test_string_literal() {
        assert_eq!(generate_literal(&Literal::String("hello".to_string())), "\"hello\"");
        assert_eq!(generate_literal(&Literal::String("".to_string())), "\"\"");
    }

    #[test]
    fn test_string_escaping() {
        assert_eq!(generate_literal(&Literal::String("hello\nworld".to_string())), "\"hello\\nworld\"");
        assert_eq!(generate_literal(&Literal::String("say \"hi\"".to_string())), "\"say \\\"hi\\\"\"");
        assert_eq!(generate_literal(&Literal::String("path\\to\\file".to_string())), "\"path\\\\to\\\\file\"");
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

