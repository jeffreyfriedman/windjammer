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

//! WDB-316: owned `String` formals must not receive `&out` (wave1 scale status).
//!
//! Product tip-out/gen:
//!   `out = wave1_scale_status_append_rows(&out, …)` with `out: String` → E0308.
//! Twin of WDB-314 (hardware report `append_bool(&out)`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn append_rows(out: string, ready: bool) -> string {
    if ready {
        return out + "|ready"
    }
    out + "|not"
}

pub fn format_status(ready: bool) -> string {
    let mut out = "hdr"
    out = append_rows(out, ready)
    out
}
"#;

#[test]
fn module_file_owned_string_must_not_receive_ref_out() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let owned = rs.contains("fn append_rows(out: String")
        || rs.contains("fn append_rows(mut out: String");
    assert!(
        owned,
        "expected owned String formal on append_rows:\n{rs}"
    );
    let bad = rs.contains("append_rows(&out") || rs.contains("append_rows(&mut out");
    assert!(
        !bad,
        "owned append_rows received &out:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
