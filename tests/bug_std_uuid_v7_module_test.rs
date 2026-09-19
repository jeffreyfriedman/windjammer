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
    feature = "integration_tests",
))]

//! FAILING REPRO — `std::uuid.v7` / `v7_from_timestamp` for `wj-uuid` graduation.
//!
//! Ecosystem already implements RFC 9562 v7 in pure WJ. Std target: runtime
//! `uuid` crate (`v7` feature) so the package can thin-wrap.

#[path = "common/test_utils.rs"]
mod test_utils;

const V7_NOW: &str = r#"
use std::uuid

pub fn gen() -> string {
    uuid.v7()
}
"#;

const V7_FROM_TS: &str = r#"
use std::uuid

pub fn gen(ms: int, rand_hex: string) -> Result<string, string> {
    uuid.v7_from_timestamp(ms, rand_hex)
}
"#;

#[test]
fn std_uuid_v7_must_wire() {
    let generated = test_utils::assert_stdlib_runtime_links(
        V7_NOW,
        &["windjammer_runtime::uuid", "uuid::v7"],
    );
    assert!(
        generated.contains("uuid::v7") || generated.contains("::v7("),
        "std::uuid.v7 must reach runtime:\n{generated}"
    );
}

#[test]
fn std_uuid_v7_from_timestamp_must_wire_rfc9562_vector() {
    let generated = test_utils::assert_stdlib_runtime_links(
        V7_FROM_TS,
        &["windjammer_runtime::uuid", "v7_from_timestamp"],
    );
    assert!(
        generated.contains("v7_from_timestamp"),
        "std::uuid.v7_from_timestamp must reach runtime:\n{generated}"
    );
}
