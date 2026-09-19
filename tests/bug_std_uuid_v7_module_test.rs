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

//! `std::uuid.v7` / `v7_from_timestamp` — runtime wire for `wj-uuid` graduation.

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn std_uuid_v7_must_wire() {
    let source = r#"
use std::uuid

pub fn gen() -> string {
    uuid.v7()
}
"#;
    test_utils::assert_stdlib_runtime_links(
        source,
        &["windjammer_runtime::uuid", "uuid::v7"],
    );
}

#[test]
fn std_uuid_v7_from_timestamp_must_wire_rfc9562_vector() {
    let source = r#"
use std::uuid

pub fn gen(ms: int, rand_hex: string) -> Result<string, string> {
    uuid.v7_from_timestamp(ms, rand_hex)
}
"#;
    test_utils::assert_stdlib_runtime_links(
        source,
        &["windjammer_runtime::uuid", "v7_from_timestamp"],
    );
}
