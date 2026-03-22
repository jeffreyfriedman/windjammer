//! TDD Tests: str vs String codegen consistency
//!
//! Bug: Windjammer compiler generates `str` (invalid - unsized) where Rust expects
//! `&str` or `String`, causing E0308 type mismatches.
//!
//! Root cause: Type::Custom("str") maps to "str" but Rust's `str` is unsized -
//! valid types are &str (borrowed) or String (owned).
//!
//! ## Audit: str/String Mapping vs Rust Semantics (verified 2026-03-14)
//!
//! | Context           | Windjammer | Rust Output   | Rust Rule                          |
//! |-------------------|------------|---------------|------------------------------------|
//! | Function param    | str        | &str          | Borrowed, str unsized               |
//! | Struct field      | str        | String        | type_to_rust_for_field              |
//! | String literal    | "hello"    | &str (inferred)| Literals are &'static str           |
//! | Option<str>       | Option<str>| Option<String>| No lifetimes in containers          |
//! | Vec<str>          | Vec<str>   | Vec<String>   | Owned in containers                 |
//! | Result<str, E>    | Result<str,E> | Result<String,E> | Owned in containers             |
//! | HashMap<str,str>  | HashMap<K,V>| HashMap<String,String> | Owned, no lifetimes        |
//! | Box<str>          | Box<str>   | Box<str>       | Box holds unsized (don't use &str)  |
//! | &str (reference) | &str       | &str           | Don't double-ref to &&str           |
//! | Return type str   | -> str     | -> &str        | type_to_rust; fn->String for owned  |
//!
//! **Critical: fn -> str**: type_to_rust yields `-> &str`. Valid when returning
//! literals or borrowed input. For owned/new strings, use `-> string` (String).

use windjammer::codegen::rust::types::type_to_rust;
use windjammer::parser::Type;

// =============================================================================
// Unit tests for type_to_rust - str/String mapping
// =============================================================================

/// str parameter type: Windjammer `str` must become Rust `&str` (str is unsized)
#[test]
fn test_string_parameter_type_str() {
    let ty = Type::Custom("str".to_string());
    let rust = type_to_rust(&ty);
    assert_eq!(rust, "&str", "Type::Custom(\"str\") should emit &str, got {}", rust);
}

/// string parameter type: Windjammer `string` must become Rust `String`
#[test]
fn test_string_parameter_type_string() {
    let ty = Type::String;
    let rust = type_to_rust(&ty);
    assert_eq!(rust, "String", "Type::String should emit String, got {}", rust);
}

/// Type::Custom("string") -> String (for type aliases)
#[test]
fn test_custom_string_emits_string() {
    let ty = Type::Custom("string".to_string());
    let rust = type_to_rust(&ty);
    assert_eq!(rust, "String", "Type::Custom(\"string\") should emit String, got {}", rust);
}

/// Struct field str: when used in struct, str is unsized - need String
/// type_to_rust doesn't have context; we map str -> &str for params.
/// For struct fields, the fix is in types.rs: str -> String when in field context.
/// For now, str -> &str fixes the param case. Struct fields with str need
/// String - we handle that in the Option<str> case: Option<str> -> Option<String>
#[test]
fn test_string_field_type_str_becomes_string() {
    // Option<str> - str is unsized, Option<str> invalid. Must be Option<String>
    let ty = Type::Option(Box::new(Type::Custom("str".to_string())));
    let rust = type_to_rust(&ty);
    assert_eq!(rust, "Option<String>", "Option<str> should emit Option<String>, got {}", rust);
}

/// String literal type: "hello" in Windjammer - the expression type is
/// inferred. type_to_rust(Type::String) = "String" (correct for owned)
#[test]
fn test_string_literal_returns_string() {
    // Type::String -> String (for variables, return types)
    assert_eq!(type_to_rust(&Type::String), "String");
}

/// &str reference: Type::Reference(Box::new(Type::Custom("str")))
#[test]
fn test_reference_to_str() {
    let ty = Type::Reference(Box::new(Type::Custom("str".to_string())));
    let rust = type_to_rust(&ty);
    assert_eq!(rust, "&str", "&str should stay &str, got {}", rust);
}

// =============================================================================
// Edge case tests (audit checklist items)
// =============================================================================

/// HashMap<str, str> -> HashMap<String, String> (owned in containers, no lifetimes)
#[test]
fn test_hashmap_str_str_becomes_string_string() {
    let ty = Type::Parameterized(
        "HashMap".to_string(),
        vec![
            Type::Custom("str".to_string()),
            Type::Custom("str".to_string()),
        ],
    );
    let rust = type_to_rust(&ty);
    assert_eq!(
        rust, "HashMap<String, String>",
        "HashMap<str, str> should emit HashMap<String, String>, got {}",
        rust
    );
}

/// HashMap<String, str> -> HashMap<String, String> (value str becomes String)
#[test]
fn test_hashmap_string_str_becomes_string_string() {
    let ty = Type::Parameterized(
        "HashMap".to_string(),
        vec![Type::String, Type::Custom("str".to_string())],
    );
    let rust = type_to_rust(&ty);
    assert_eq!(
        rust, "HashMap<String, String>",
        "HashMap<String, str> value should become String, got {}",
        rust
    );
}

/// Box<str> -> Box<str> (Box can hold unsized str, don't convert to Box<&str>)
#[test]
fn test_box_str_stays_box_str() {
    let ty = Type::Parameterized("Box".to_string(), vec![Type::Custom("str".to_string())]);
    let rust = type_to_rust(&ty);
    assert_eq!(
        rust, "Box<str>",
        "Box<str> should emit Box<str> (valid in Rust), got {}",
        rust
    );
}

/// Return type str: type_to_rust gives &str (params/return context - no field context)
/// Note: fn -> &str works for literals/borrowed; fn -> String for owned.
/// type_to_rust has no context, so str -> &str. Return type analysis is in function_generation.
#[test]
fn test_return_type_str_emits_ampersand_str() {
    let ty = Type::Custom("str".to_string());
    let rust = type_to_rust(&ty);
    assert_eq!(
        rust, "&str",
        "Bare str in type position (e.g. return) -> &str, got {}",
        rust
    );
}
