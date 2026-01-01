//! Helper utilities for Rust code generation
//!
//! This module contains utility functions for:
//! - Operator precedence
//! - Operator conversion (Windjammer -> Rust)
//! - Pattern matching helpers
//! - Indentation utilities

use crate::parser::*;

/// Convert a binary operator to its Rust equivalent
pub fn binary_op_to_rust(op: &BinaryOp) -> &str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Mod => "%",
        BinaryOp::Eq => "==",
        BinaryOp::Ne => "!=",
        BinaryOp::Lt => "<",
        BinaryOp::Le => "<=",
        BinaryOp::Gt => ">",
        BinaryOp::Ge => ">=",
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
        BinaryOp::BitAnd => "&",
        BinaryOp::BitOr => "|",
        BinaryOp::BitXor => "^",
        BinaryOp::Shl => "<<",
        BinaryOp::Shr => ">>",
    }
}

/// Get the precedence of a binary operator (higher = tighter binding)
pub fn op_precedence(op: &BinaryOp) -> i32 {
    match op {
        BinaryOp::Or => 1,
        BinaryOp::And => 2,
        BinaryOp::BitOr => 3,
        BinaryOp::BitXor => 4,
        BinaryOp::BitAnd => 5,
        BinaryOp::Eq | BinaryOp::Ne => 6,
        BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => 7,
        BinaryOp::Shl | BinaryOp::Shr => 8,
        BinaryOp::Add | BinaryOp::Sub => 9,
        BinaryOp::Mul | BinaryOp::Div | BinaryOp::Mod => 10,
    }
}

/// Convert a unary operator to its Rust equivalent
pub fn unary_op_to_rust(op: &UnaryOp) -> &str {
    match op {
        UnaryOp::Not => "!",
        UnaryOp::Neg => "-",
        UnaryOp::Ref => "&",
        UnaryOp::MutRef => "&mut ",
        UnaryOp::Deref => "*",
    }
}

/// Convert a literal to Rust code
pub fn literal_to_rust(lit: &Literal) -> String {
    match lit {
        Literal::Int(i) => i.to_string(),
        Literal::Float(f) => {
            let s = f.to_string();
            if s.contains('.') {
                s
            } else {
                format!("{}.0", s)
            }
        }
        Literal::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
        Literal::Char(c) => format!("'{}'", c),
        Literal::Bool(b) => b.to_string(),
    }
}

/// Check if a pattern contains a string literal
pub fn pattern_has_string_literal(pattern: &Pattern) -> bool {
    match pattern {
        Pattern::Literal(Literal::String(_)) => true,
        Pattern::Tuple(patterns) => patterns.iter().any(pattern_has_string_literal),
        Pattern::Or(patterns) => patterns.iter().any(pattern_has_string_literal),
        _ => false,
    }
}

/// Generate indentation string
pub fn indent(level: usize) -> String {
    "    ".repeat(level)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_op_conversion() {
        assert_eq!(binary_op_to_rust(&BinaryOp::Add), "+");
        assert_eq!(binary_op_to_rust(&BinaryOp::Eq), "==");
        assert_eq!(binary_op_to_rust(&BinaryOp::And), "&&");
    }

    #[test]
    fn test_op_precedence() {
        assert!(op_precedence(&BinaryOp::Mul) > op_precedence(&BinaryOp::Add));
        assert!(op_precedence(&BinaryOp::Add) > op_precedence(&BinaryOp::Eq));
        assert!(op_precedence(&BinaryOp::And) > op_precedence(&BinaryOp::Or));
    }

    #[test]
    fn test_unary_op_conversion() {
        assert_eq!(unary_op_to_rust(&UnaryOp::Not), "!");
        assert_eq!(unary_op_to_rust(&UnaryOp::Neg), "-");
        assert_eq!(unary_op_to_rust(&UnaryOp::Ref), "&");
    }

    #[test]
    fn test_literal_conversion() {
        assert_eq!(literal_to_rust(&Literal::Int(42)), "42");
        assert_eq!(literal_to_rust(&Literal::Float(2.5)), "2.5");
        assert_eq!(
            literal_to_rust(&Literal::String("hello".to_string())),
            "\"hello\""
        );
        assert_eq!(literal_to_rust(&Literal::Bool(true)), "true");
    }

    #[test]
    fn test_indent() {
        assert_eq!(indent(0), "");
        assert_eq!(indent(1), "    ");
        assert_eq!(indent(2), "        ");
    }
}
