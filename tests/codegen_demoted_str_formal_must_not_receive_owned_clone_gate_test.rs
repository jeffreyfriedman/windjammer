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
))]

//! Demoted `&str` callee formals must not receive `String.clone()` at call sites.
//!
//! Dogfood (finance-screens): `render_memorized_handoff_from_json(json)` demotes
//! formal to `&str` but call sites must not emit `json.clone()` (E0308 / wasteful).
//! Read-only helper demotion uses comparison/`strings.len` (same as loop-reuse repros).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn demoted_str_formal_must_not_receive_cloned_string() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "handoff.wj",
        r#"
use std::strings

pub fn parse_body(json: string) -> string {
    if strings.len(json) == 0 {
        return ""
    }
    let s = "${json.trim()}"
    s
}

pub fn render_from_json(json: string) -> string {
    parse_body(json)
}
"#,
    );
    test.add_file(
        "read.wj",
        r#"
use crate::handoff::render_from_json

pub fn dispatch(kind: string, json: string) -> string {
    if kind == "handoff" && render_from_json(json) != "" {
        json
    } else {
        ""
    }
}
"#,
    );

    let map = test
        .compile()
        .expect("library multipass compile should succeed");
    let handoff = map
        .get("handoff.rs")
        .expect("handoff.rs output");
    let read = map.get("read.rs").expect("read.rs output");

    assert!(
        handoff.contains("json: &str"),
        "repro must demote read-only string formals to &str. handoff=\n{handoff}"
    );

    assert!(
        !read.contains("render_from_json(json.clone())")
            && !read.contains("render_from_json(json.to_string())"),
        "demoted &str formal must not receive String clone. handoff=\n{handoff}\nread=\n{read}"
    );

    test.cargo_check()
        .expect("demoted str formal call site must cargo check");
}
