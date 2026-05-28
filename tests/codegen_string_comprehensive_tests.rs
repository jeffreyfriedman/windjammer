#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
))]

//! Comprehensive Codegen String Handling Tests
//!
//! These tests verify that the Windjammer compiler correctly handles
//! string type conversions, including:
//! - String literals to String (.to_string())
//! - String literals to &str (no conversion)
//! - String concatenation
//! - String method calls
//! - format!() macro generation

#[path = "common/test_utils.rs"]
mod test_utils;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

// ============================================================================
// STRING LITERALS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_literal_assignment() {
    let code = r#"
pub fn greeting() -> string {
    let s = "hello".to_string()
    s
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(
        success,
        "String literal assignment should compile. Error: {}",
        err
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_literal_return() {
    let code = r#"
pub fn hello() -> string {
    "hello"
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);

    // Should convert to String for return
    assert!(success, "String return should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_mutable_string_initialization() {
    let code = r#"
pub fn build_message() -> string {
    let mut s = ""
    s += "Hello"
    s += ", "
    s += "World"
    s
}
"#;
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    // Mutable string should be converted to String, not &str
    assert!(
        success,
        "Mutable string should compile. Generated:\n{}\nError: {}",
        generated, err
    );
}

// ============================================================================
// STRING FUNCTION PARAMETERS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_param_to_function() {
    // Test that string parameters are properly handled
    let code = r#"
pub fn get_length(s: string) -> i32 {
    s.len() as i32
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "String param should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_borrowed_string_param() {
    let code = r#"
pub fn length(s: &string) -> i32 {
    s.len() as i32
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(
        success,
        "Borrowed string param should compile. Error: {}",
        err
    );
}

// ============================================================================
// STRING METHODS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_contains() {
    let code = r#"
pub fn has_hello(s: string) -> bool {
    s.contains("hello")
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);

    // contains takes &str, so literal should NOT have .to_string()
    assert!(success, "String contains should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_replace() {
    let code = r#"
pub fn sanitize(s: string) -> string {
    s.replace("bad", "good")
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);

    // replace takes Pattern which &str implements
    assert!(success, "String replace should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_split() {
    // split() returns an iterator of &str - test basic split functionality
    let code = r#"
pub fn count_words(s: string) -> i32 {
    s.split(" ").count() as i32
}
"#;
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };
    assert!(
        success,
        "String split should compile. Generated:\n{}\nError: {}",
        generated, err
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_trim() {
    // trim() returns &str, so we need explicit .to_string() for now
    let code = r#"
pub fn clean(s: string) -> string {
    s.trim().to_string()
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "String trim should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_to_uppercase() {
    let code = r#"
pub fn shout(s: string) -> string {
    s.to_uppercase()
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(
        success,
        "String to_uppercase should compile. Error: {}",
        err
    );
}

// ============================================================================
// STRING CONCATENATION
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_concat_literals() {
    let code = r#"
pub fn full_name() -> string {
    "John" + " " + "Doe"
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "String concat should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_concat_with_variable() {
    let code = r#"
pub fn greet(name: string) -> string {
    "Hello, " + name + "!"
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(
        success,
        "String concat with variable should compile. Error: {}",
        err
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_concat_compound() {
    let code = r#"
pub fn build_list() -> string {
    let mut result = ""
    result += "item1"
    result += ", "
    result += "item2"
    result
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(
        success,
        "Compound string concat should compile. Error: {}",
        err
    );
}

// ============================================================================
// MATCH WITH STRINGS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_match_return_string() {
    let code = r#"
pub fn describe(n: i32) -> string {
    match n {
        0 => "zero",
        1 => "one",
        _ => "many",
    }
}
"#;
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    // All match arms should be converted to String consistently
    assert!(
        success,
        "Match with string should compile. Generated:\n{}\nError: {}",
        generated, err
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_match_mixed_return() {
    let code = r#"
pub fn get_message(code: i32) -> string {
    match code {
        0 => "OK",
        1 => "ERROR".to_uppercase(),
        _ => "UNKNOWN",
    }
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(
        success,
        "Match with mixed returns should compile. Error: {}",
        err
    );
}

// ============================================================================
// STRUCT FIELDS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_struct_with_string_field() {
    let code = r#"
@derive(Clone, Debug)
pub struct Person {
    name: string,
    age: i32,
}

impl Person {
    pub fn new(name: string, age: i32) -> Person {
        Person { name: name, age: age }
    }
    
    pub fn greet(&self) -> string {
        "Hello, " + self.name.clone()
    }
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(
        success,
        "Struct with string field should compile. Error: {}",
        err
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_struct_string_field_init() {
    let code = r#"
@derive(Clone, Debug)
pub struct Config {
    name: string,
}

pub fn create_config() -> Config {
    Config { name: "default" }
}
"#;
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    // String literal in struct init should be converted
    assert!(
        success,
        "Struct string field init should compile. Generated:\n{}\nError: {}",
        generated, err
    );
}

// ============================================================================
// VEC WITH STRINGS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_vec_push_string() {
    let code = r#"
pub fn create_list() -> Vec<string> {
    let mut list = Vec::new()
    list.push("first")
    list.push("second")
    list
}
"#;
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };

    // String literals pushed to Vec<String> should be converted
    assert!(
        success,
        "Vec push string should compile. Generated:\n{}\nError: {}",
        generated, err
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_vec_string_iteration() {
    let code = r#"
pub fn join_all(items: Vec<string>) -> string {
    let mut result = ""
    for item in items {
        result += item
        result += ", "
    }
    result
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(
        success,
        "Vec string iteration should compile. Error: {}",
        err
    );
}

// ============================================================================
// HASHMAP WITH STRINGS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_hashmap_string_keys() {
    let code = r#"
use std::collections::HashMap

pub fn create_map() -> HashMap<string, i32> {
    let mut map = HashMap::new()
    map.insert("one", 1)
    map.insert("two", 2)
    map
}
"#;
    let (generated, success) = test_utils::compile_single_check(code);
    let err = if !success { &generated } else { "" };
    assert!(
        success,
        "HashMap string keys should compile. Generated:\n{}\nError: {}",
        generated, err
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_hashmap_get_string() {
    // Test hashmap with string keys - basic containment check
    let code = r#"
use std::collections::HashMap

pub fn has_key(map: &HashMap<string, i32>, key: &string) -> bool {
    map.contains_key(key)
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(
        success,
        "HashMap get with string should compile. Error: {}",
        err
    );
}

// ============================================================================
// INTERPOLATED STRINGS
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_interpolation() {
    let code = r#"
pub fn format_greeting(name: string) -> string {
    "Hello, ${name}!"
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);

    // Should generate format!() macro
    assert!(
        success,
        "String interpolation should compile. Error: {}",
        err
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_interpolation_expression() {
    let code = r#"
pub fn format_sum(a: i32, b: i32) -> string {
    "${a} + ${b} = ${a + b}"
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(
        success,
        "String interpolation with expr should compile. Error: {}",
        err
    );
}

// ============================================================================
// EDGE CASES
// ============================================================================

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_empty_string() {
    let code = r#"
pub fn empty() -> string {
    ""
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "Empty string should compile. Error: {}", err);
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_with_escapes() {
    let code = r#"
pub fn with_escapes() -> string {
    "line1\nline2\ttab"
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(
        success,
        "String with escapes should compile. Error: {}",
        err
    );
}

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_string_clone() {
    let code = r#"
pub fn duplicate(s: string) -> string {
    s.clone()
}
"#;
    let (success, _generated, err) = test_utils::compile_via_cli(code);
    assert!(success, "String clone should compile. Error: {}", err);
}
