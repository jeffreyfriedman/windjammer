//! FAILING REPRO — `std::uuid` (or documented std composition) for v4 IDs.
//!
//! Ecosystem `wj-uuid` is blocked on random/crypto/time. Languages ship UUID in
//! std or a first-party module. Prefer `std::uuid.v4() -> string`.

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
    let (generated, ok) = test_utils::compile_single_check(source);
    assert!(
        ok,
        "std::uuid.v4 must compile (add std/uuid.wj + runtime), got:\n{generated}"
    );
    let wired = generated.contains("uuid::v4")
        || generated.contains("windjammer_runtime::uuid::v4");
    assert!(
        wired,
        "uuid.v4 must map to runtime, got:\n{generated}"
    );
}
