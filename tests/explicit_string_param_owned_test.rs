#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "analyzer_tests",
))]

//! Pit-of-success: Windjammer `string` in a function signature is owned `String` in Rust.
//! Read-only domain validators and HTTP helpers must not silently become `&str` — that
//! breaks call sites passing owned locals without `+ ""` or `&` workarounds.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn explicit_string_formal_generates_owned_rust_param() {
    let source = r#"
pub fn account_type_valid(account_type: string) -> bool {
    account_type == "Asset"
}

pub fn error_json(message: string) -> string {
    "{\"error\":\"" + message + "\"}"
}

pub fn check(msg: string) -> bool {
    account_type_valid(msg)
}
"#;
    let rust = test_utils::compile_single(source);
    assert!(
        rust.contains("fn account_type_valid(account_type: String)"),
        "explicit string param must codegen as owned String. Got:\n{rust}"
    );
    assert!(
        rust.contains("fn error_json(message: String)"),
        "explicit string param must codegen as owned String. Got:\n{rust}"
    );
    assert!(
        !rust.contains("fn account_type_valid(account_type: &str)"),
        "must not lower read-only string formals to &str. Got:\n{rust}"
    );
}

#[test]
fn string_literal_coerces_for_owned_string_formal() {
    let source = r#"
pub fn account_type_valid(account_type: string) -> bool {
    account_type == "Asset"
}

fn main() {
    account_type_valid("Asset")
}
"#;
    let rust = test_utils::compile_single(source);
    assert!(
        rust.contains(r#"account_type_valid("Asset".to_string())"#)
            || rust.contains(r#"account_type_valid(String::from("Asset"))"#),
        "string literal must coerce to owned String for owned formal. Got:\n{rust}"
    );
}
