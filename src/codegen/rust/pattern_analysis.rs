// Pattern analysis utilities
//
// This module provides functions for analyzing patterns in match expressions:
// - Detecting string literals in patterns
// - Checking if patterns extract values (causing moves)
// - Extracting identifiers from patterns

use crate::parser::{EnumPatternBinding, Literal, Pattern};

/// Checks if a pattern contains a string literal (recursively)
///
/// This is useful for determining if string conversion logic is needed
/// in match expressions.
///
/// # Examples
/// ```
/// // "hello" → true
/// // 42 → false
/// // ("hello", x) → true
/// // Some(_) → false
/// ```
pub fn pattern_has_string_literal(pattern: &Pattern) -> bool {
    match pattern {
        Pattern::Literal(Literal::String(_)) => true,
        Pattern::Tuple(patterns) => patterns.iter().any(pattern_has_string_literal),
        Pattern::Or(patterns) => patterns.iter().any(pattern_has_string_literal),
        _ => false,
    }
}

/// Checks if a pattern extracts a value (causing a move)
///
/// Returns `true` if the pattern binds variables or extracts data
/// from enum variants, which would cause the matched value to be moved.
///
/// # Examples
/// ```
/// // _ → false (no binding)
/// // 42 → false (literal match, no binding)
/// // x → true (binds x, moves value)
/// // Some(x) → true (extracts x, moves value)
/// // Some(_) → false (wildcard, no move)
/// // (x, y) → true (binds x and y)
/// ```
pub fn pattern_extracts_value(pattern: &Pattern) -> bool {
    match pattern {
        Pattern::Wildcard | Pattern::Literal(_) => false,
        Pattern::Identifier(_) => true, // Binding moves the value
        Pattern::Reference(inner) => pattern_extracts_value(inner),
        Pattern::Ref(_) | Pattern::RefMut(_) => false, // ref/ref mut borrow, don't move
        Pattern::Tuple(patterns) => patterns.iter().any(pattern_extracts_value),
        Pattern::EnumVariant(_, binding) => match binding {
            EnumPatternBinding::None | EnumPatternBinding::Wildcard => false,
            EnumPatternBinding::Single(_) => true, // Some(id) extracts id
            EnumPatternBinding::Tuple(patterns) => patterns.iter().any(pattern_extracts_value),
            EnumPatternBinding::Struct(fields, _) => {
                fields.iter().any(|(_, p)| pattern_extracts_value(p))
            }
        },
        Pattern::Or(patterns) => patterns.iter().any(pattern_extracts_value),
    }
}

/// Extracts the identifier name from a simple identifier pattern
///
/// Returns `Some(name)` if the pattern is a simple identifier binding,
/// `None` otherwise.
///
/// # Examples
/// ```
/// // x → Some("x")
/// // my_var → Some("my_var")
/// // _ → None
/// // 42 → None
/// // (x, y) → None (tuple, not simple identifier)
/// ```
pub fn extract_pattern_identifier(pattern: &Pattern) -> Option<String> {
    match pattern {
        Pattern::Identifier(name) => Some(name.clone()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nested_tuple_extraction() {
        // ((x, _), y) → true
        let pattern = Pattern::Tuple(vec![
            Pattern::Tuple(vec![
                Pattern::Identifier("x".to_string()),
                Pattern::Wildcard,
            ]),
            Pattern::Identifier("y".to_string()),
        ]);
        assert!(pattern_extracts_value(&pattern));
    }

    #[test]
    fn test_deeply_nested_or_with_string() {
        // ("a" | "b") | ("c" | "d") → true
        let pattern = Pattern::Or(vec![
            Pattern::Or(vec![
                Pattern::Literal(Literal::String("a".to_string())),
                Pattern::Literal(Literal::String("b".to_string())),
            ]),
            Pattern::Or(vec![
                Pattern::Literal(Literal::String("c".to_string())),
                Pattern::Literal(Literal::String("d".to_string())),
            ]),
        ]);
        assert!(pattern_has_string_literal(&pattern));
    }

    #[test]
    fn test_complex_enum_struct_pattern() {
        // Person { name, age: _ } where name binds
        let pattern = Pattern::EnumVariant(
            "Person".to_string(),
            EnumPatternBinding::Struct(
                vec![
                    ("name".to_string(), Pattern::Identifier("name".to_string())),
                    ("age".to_string(), Pattern::Wildcard),
                ],
                false,
            ),
        );
        assert!(pattern_extracts_value(&pattern));
    }

    #[test]
    fn test_all_wildcards_no_extraction() {
        // (_, (_, _), _) → false
        let pattern = Pattern::Tuple(vec![
            Pattern::Wildcard,
            Pattern::Tuple(vec![Pattern::Wildcard, Pattern::Wildcard]),
            Pattern::Wildcard,
        ]);
        assert!(!pattern_extracts_value(&pattern));
    }
}
