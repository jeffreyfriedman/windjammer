//! `std::uuid.v4()` — new std module wired to runtime (or composition API).

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

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn std_uuid_v4_codegen_resolves() {
    let source = r#"
use std::uuid

pub fn new_id() -> string {
    uuid.v4()
}
"#;
    test_utils::assert_stdlib_runtime_links_any(source, &["uuid::v4", "uuid::new_v4"]);
}
