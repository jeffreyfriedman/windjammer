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

//! Phase-2: read-only WJ `string` formals (equality / `strings.len`) demote to `&str`.
//! Concat / store / return-consuming formals stay owned `String`. Call sites borrow
//! demoted formals; literals into owned formals still `.to_string()`.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn comparison_only_string_formal_demotes_to_str() {
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
        rust.contains("fn account_type_valid(account_type: &str)"),
        "comparison-only string formal demotes to &str (Phase-2 / loop reuse). Got:\n{rust}"
    );
    assert!(
        rust.contains("fn error_json(message: String)"),
        "concat-consuming string formal stays owned String. Got:\n{rust}"
    );
    assert!(
        rust.contains("account_type_valid(&msg)")
            || rust.contains("account_type_valid(msg.as_str())")
            || rust.contains("account_type_valid(&*msg)")
            || rust.contains("account_type_valid(msg)"),
        "caller must pass into demoted &str formal (borrow owned or forward &str). Got:\n{rust}"
    );
}

#[test]
fn string_literal_coerces_for_owned_string_formal() {
    let source = r#"
pub fn error_json(message: string) -> string {
    "{\"error\":\"" + message + "\"}"
}

fn main() {
    error_json("boom")
}
"#;
    let rust = test_utils::compile_single(source);
    assert!(
        rust.contains(r#"error_json("boom".to_string())"#)
            || rust.contains(r#"error_json(String::from("boom"))"#),
        "string literal must coerce to owned String for owned formal. Got:\n{rust}"
    );
}

#[test]
fn string_literal_stays_bare_for_demoted_str_formal() {
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
        rust.contains(r#"account_type_valid("Asset")"#)
            && !rust.contains(r#"account_type_valid("Asset".to_string())"#),
        "literal into demoted &str formal stays bare. Got:\n{rust}"
    );
}
