// TDD Tests for pattern analysis functions
//
// This module contains pure functions for analyzing patterns in match expressions:
// - Detecting string literals in patterns
// - Checking if patterns extract values (causing moves)
// - Extracting identifiers from patterns

use windjammer::codegen::rust::pattern_analysis::{
    extract_pattern_identifier, pattern_extracts_value, pattern_has_string_literal,
};
use windjammer::parser::{EnumPatternBinding, Literal, Pattern};
use windjammer::test_utils::test_alloc_pattern;

#[cfg(test)]
mod pattern_has_string_literal_tests {
    use super::*;

    #[test]
    fn test_string_literal_pattern() {
        // Test: "hello" → true
        let pattern = Pattern::Literal(Literal::String("hello".to_string()));
        assert!(pattern_has_string_literal(&pattern));
    }

    #[test]
    fn test_integer_literal_pattern() {
        // Test: 42 → false
        let pattern = Pattern::Literal(Literal::Int(42));
        assert!(!pattern_has_string_literal(&pattern));
    }

    #[test]
    fn test_wildcard_pattern() {
        // Test: _ → false
        let pattern = Pattern::Wildcard;
        assert!(!pattern_has_string_literal(&pattern));
    }

    #[test]
    fn test_identifier_pattern() {
        // Test: x → false
        let pattern = Pattern::Identifier("x".to_string());
        assert!(!pattern_has_string_literal(&pattern));
    }

    #[test]
    fn test_tuple_with_string() {
        // Test: ("hello", 42) → true
        let pattern = Pattern::Tuple(vec![
            Pattern::Literal(Literal::String("hello".to_string())),
            Pattern::Literal(Literal::Int(42)),
        ]);
        assert!(pattern_has_string_literal(&pattern));
    }

    #[test]
    fn test_tuple_without_string() {
        // Test: (42, true) → false
        let pattern = Pattern::Tuple(vec![
            Pattern::Literal(Literal::Int(42)),
            Pattern::Literal(Literal::Bool(true)),
        ]);
        assert!(!pattern_has_string_literal(&pattern));
    }

    #[test]
    fn test_or_pattern_with_string() {
        // Test: "hello" | "world" → true
        let pattern = Pattern::Or(vec![
            Pattern::Literal(Literal::String("hello".to_string())),
            Pattern::Literal(Literal::String("world".to_string())),
        ]);
        assert!(pattern_has_string_literal(&pattern));
    }

    #[test]
    fn test_or_pattern_without_string() {
        // Test: 1 | 2 | 3 → false
        let pattern = Pattern::Or(vec![
            Pattern::Literal(Literal::Int(1)),
            Pattern::Literal(Literal::Int(2)),
            Pattern::Literal(Literal::Int(3)),
        ]);
        assert!(!pattern_has_string_literal(&pattern));
    }
}

#[cfg(test)]
mod pattern_extracts_value_tests {
    use super::*;

    #[test]
    fn test_wildcard_no_extract() {
        // Test: _ → false (doesn't extract)
        let pattern = Pattern::Wildcard;
        assert!(!pattern_extracts_value(&pattern));
    }

    #[test]
    fn test_literal_no_extract() {
        // Test: 42 → false (doesn't extract)
        let pattern = Pattern::Literal(Literal::Int(42));
        assert!(!pattern_extracts_value(&pattern));
    }

    #[test]
    fn test_identifier_extracts() {
        // Test: x → true (binds and extracts)
        let pattern = Pattern::Identifier("x".to_string());
        assert!(pattern_extracts_value(&pattern));
    }

    #[test]
    fn test_reference_propagates() {
        // Test: &x → true (still extracts through reference)
        let inner = test_alloc_pattern(Pattern::Identifier("x".to_string()));
        let pattern = Pattern::Reference(inner);
        assert!(pattern_extracts_value(&pattern));
    }

    #[test]
    fn test_reference_wildcard() {
        // Test: &_ → false
        let inner = test_alloc_pattern(Pattern::Wildcard);
        let pattern = Pattern::Reference(inner);
        assert!(!pattern_extracts_value(&pattern));
    }

    #[test]
    fn test_tuple_with_extraction() {
        // Test: (x, _) → true
        let pattern = Pattern::Tuple(vec![
            Pattern::Identifier("x".to_string()),
            Pattern::Wildcard,
        ]);
        assert!(pattern_extracts_value(&pattern));
    }

    #[test]
    fn test_tuple_no_extraction() {
        // Test: (_, _) → false
        let pattern = Pattern::Tuple(vec![Pattern::Wildcard, Pattern::Wildcard]);
        assert!(!pattern_extracts_value(&pattern));
    }

    #[test]
    fn test_enum_variant_none() {
        // Test: Some → false (no binding)
        let pattern = Pattern::EnumVariant("Some".to_string(), EnumPatternBinding::None);
        assert!(!pattern_extracts_value(&pattern));
    }

    #[test]
    fn test_enum_variant_wildcard() {
        // Test: Some(_) → false
        let pattern = Pattern::EnumVariant("Some".to_string(), EnumPatternBinding::Wildcard);
        assert!(!pattern_extracts_value(&pattern));
    }

    #[test]
    fn test_enum_variant_single() {
        // Test: Some(x) → true (extracts x)
        let pattern = Pattern::EnumVariant(
            "Some".to_string(),
            EnumPatternBinding::Single("x".to_string()),
        );
        assert!(pattern_extracts_value(&pattern));
    }

    #[test]
    fn test_enum_variant_tuple() {
        // Test: Ok((x, _)) → true
        let pattern = Pattern::EnumVariant(
            "Ok".to_string(),
            EnumPatternBinding::Tuple(vec![
                Pattern::Identifier("x".to_string()),
                Pattern::Wildcard,
            ]),
        );
        assert!(pattern_extracts_value(&pattern));
    }

    #[test]
    fn test_enum_variant_struct() {
        // Test: Point { x, y: _ } → true
        let pattern = Pattern::EnumVariant(
            "Point".to_string(),
            EnumPatternBinding::Struct(
                vec![
                    ("x".to_string(), Pattern::Identifier("x".to_string())),
                    ("y".to_string(), Pattern::Wildcard),
                ],
                false,
            ),
        );
        assert!(pattern_extracts_value(&pattern));
    }

    #[test]
    fn test_or_pattern_any_extraction() {
        // Test: Some(x) | None → true (first arm extracts)
        let pattern = Pattern::Or(vec![
            Pattern::EnumVariant(
                "Some".to_string(),
                EnumPatternBinding::Single("x".to_string()),
            ),
            Pattern::EnumVariant("None".to_string(), EnumPatternBinding::None),
        ]);
        assert!(pattern_extracts_value(&pattern));
    }

    #[test]
    fn test_or_pattern_no_extraction() {
        // Test: None | Some(_) → false
        let pattern = Pattern::Or(vec![
            Pattern::EnumVariant("None".to_string(), EnumPatternBinding::None),
            Pattern::EnumVariant("Some".to_string(), EnumPatternBinding::Wildcard),
        ]);
        assert!(!pattern_extracts_value(&pattern));
    }
}

#[cfg(test)]
mod extract_pattern_identifier_tests {
    use super::*;

    #[test]
    fn test_identifier_pattern() {
        // Test: x → Some("x")
        let pattern = Pattern::Identifier("x".to_string());
        assert_eq!(extract_pattern_identifier(&pattern), Some("x".to_string()));
    }

    #[test]
    fn test_identifier_with_complex_name() {
        // Test: my_variable → Some("my_variable")
        let pattern = Pattern::Identifier("my_variable".to_string());
        assert_eq!(
            extract_pattern_identifier(&pattern),
            Some("my_variable".to_string())
        );
    }

    #[test]
    fn test_wildcard_pattern() {
        // Test: _ → None
        let pattern = Pattern::Wildcard;
        assert_eq!(extract_pattern_identifier(&pattern), None);
    }

    #[test]
    fn test_literal_pattern() {
        // Test: 42 → None
        let pattern = Pattern::Literal(Literal::Int(42));
        assert_eq!(extract_pattern_identifier(&pattern), None);
    }

    #[test]
    fn test_tuple_pattern() {
        // Test: (x, y) → None (tuple, not simple identifier)
        let pattern = Pattern::Tuple(vec![
            Pattern::Identifier("x".to_string()),
            Pattern::Identifier("y".to_string()),
        ]);
        assert_eq!(extract_pattern_identifier(&pattern), None);
    }

    #[test]
    fn test_enum_variant_pattern() {
        // Test: Some(x) → None (enum variant, not simple identifier)
        let pattern = Pattern::EnumVariant(
            "Some".to_string(),
            EnumPatternBinding::Single("x".to_string()),
        );
        assert_eq!(extract_pattern_identifier(&pattern), None);
    }
}
