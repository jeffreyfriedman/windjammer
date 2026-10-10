#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "integration_tests",
    feature = "codegen_tests",
))]

//!
//! → E0308 expected `&str`, found `&impl Into<String>`.

#[path = "common/test_utils.rs"]
mod test_utils;

const FIXTURE: &str = include_str!(
    "fixtures/library_multipass/string_formal_must_not_emit_impl_into_for_push_str.wj"
);

#[test]
fn codegen_string_formal_must_not_emit_impl_into_for_push_str() {
    let (rs, ok) = test_utils::compile_single_check(FIXTURE);
    let into_formal = rs.contains("impl Into<String>") || rs.contains("impl Into<string>");
    let bad_push = rs.contains("push_str(&edge_kind)") && into_formal;
    if into_formal || bad_push {
        panic!(
            "RED: string formal emitted impl Into / push_str(&Into). Generated:\n{rs}"
        );
    }
    assert!(ok, "fixture must cargo-check. Generated:\n{rs}");
}

