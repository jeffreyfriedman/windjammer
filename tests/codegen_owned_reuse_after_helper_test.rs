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

//! Gate (StatusChip pattern): owned string used after pass-by-value helper.
//!
//! Source:
//!   let status = self.status
//!   let v = variant_for(status)   // consumes String
//!   Badge::new(status)            // needs status again
//!
//! Desired: auto-clone before the consuming call, or accept &str in helper,
//! so generated Rust compiles without manual .clone().

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn owned_string_reuse_after_by_value_helper_should_clone() {
    let source = r#"
pub fn variant_for(status: string) -> int {
    if status.to_lowercase() == "paid" { 1 } else { 0 }
}

pub fn chip(status: string) -> string {
    let label = status
    let v = variant_for(label)
    format!("{}:{}", v, label)
}

fn main() {
    let _ = chip("paid".to_string())
}
"#;

    let result = test_utils::compile_single(source);

    let has_clone = result.contains("label.clone()")
        || result.contains("status.clone()")
        || result.contains(".clone()");

    // Must compile as valid Rust — either clone or helper takes &str.
    assert!(
        has_clone || result.contains("variant_for(&label)") || result.contains("variant_for(label.as_str())"),
        "owned string reused after by-value helper should auto-clone or borrow. Got:\n{}",
        result
    );
}

/// WDB-100 / Phase 128: two-arg owned helper then substring of the same String.
/// IR call-site coercion must clone or borrow before reuse after by-value helper.
#[test]
fn owned_string_reuse_after_two_arg_by_value_helper_should_clone() {
    let source = r#"
use std::strings

pub fn find_word(hay: string, needle: string) -> int {
    if hay.len() == 0 { return -1 }
    0
}

pub fn after_sum(sql: string) -> string {
    let i = find_word(sql, "sum(")
    strings::substring(sql, i as usize, sql.len())
}

fn main() {
    let _ = after_sum("SELECT SUM(qty)")
}
"#;

    let result = test_utils::compile_single(source);

    let has_clone = result.contains("sql.clone()") || result.contains(".clone()");
    let borrowed_formals = result.contains("sql: &str") && result.contains("find_word(sql,");
    assert!(
        has_clone
            || borrowed_formals
            || result.contains("find_word(&sql")
            || result.contains("find_word(sql.as_str()"),
        "owned string reused after two-arg by-value helper should auto-clone or borrow. Got:\n{}",
        result
    );
}
