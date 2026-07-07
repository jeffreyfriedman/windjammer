#![cfg(not(any(
    feature = "parser_tests",
    feature = "analyzer_tests",
    feature = "codegen_tests",
    feature = "interpreter_tests",
    feature = "conformance_tests",
    feature = "integration_tests",
)))]

// Bug: Lexer panics on r#type (raw identifier syntax)
//
// In Windjammer, `type` is a keyword. When a struct field needs to be named
// "type" (e.g. for JSON serialization), the r#type syntax escapes the keyword.
// The lexer panicked with "Unexpected character: #" because it didn't handle
// the r# prefix for raw identifiers.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn raw_identifier_field_in_struct() {
    let source = r#"
struct AccountJson {
    code: string,
    name: string,
    r#type: string,
    balance_cents: int,
}
"#;
    let (generated, success) = test_utils::compile_single_check(source);
    assert!(success, "compilation should succeed for r#type field");
    assert!(
        generated.contains("r#type") || generated.contains("type"),
        "generated code should contain the 'type' field.\nGenerated:\n{}",
        generated
    );
}

#[test]
fn raw_identifier_field_access() {
    let source = r#"
struct Item {
    r#type: string,
}

pub fn get_type(item: Item) -> string {
    item.r#type
}
"#;
    let (_generated, success) = test_utils::compile_single_check(source);
    assert!(success, "compilation should succeed for r#type field access");
}

#[test]
fn raw_identifier_field_assignment() {
    let source = r#"
struct AccountJson {
    code: string,
    r#type: string,
}

pub fn make_account() -> AccountJson {
    AccountJson {
        code: "100",
        r#type: "asset",
    }
}
"#;
    let (generated, success) = test_utils::compile_single_check(source);
    assert!(success, "compilation should succeed for r#type in struct literal");
}

#[test]
fn raw_identifier_compiles_with_rustc() {
    let source = r#"
struct Item {
    r#type: string,
    name: string,
}

pub fn create_item() -> Item {
    Item {
        r#type: "weapon",
        name: "sword",
    }
}
"#;
    let generated = test_utils::compile_single(source);
    test_utils::verify_rust_compiles(&generated).expect("Generated Rust with r#type should compile");
}
