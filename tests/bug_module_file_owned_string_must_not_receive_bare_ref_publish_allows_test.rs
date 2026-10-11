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

//! owned `String` formals must not receive bare `&dated_label` (wave1_publish_allows).
//!
//! Product tip-out/gen:
//!   `wave1_publish_allows(…, &dated_label, …)` with `dated_label: String` → E0308.
//! Distinct from (`&dated_label.clone()` into append_gate_rows).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

// Readonly `len`/`is_empty` demotes to `&str` (). Force owned String formals
// via struct store — mirrors product `wave1_publish_allows(..., dated_label: String, ...)`.
const SRC: &str = r#"
pub struct Gate {
    pub dated_label: string,
    pub live: bool,
}

pub fn publish_allows(live: bool, dated_label: string) -> Gate {
    Gate {
        dated_label: dated_label,
        live: live,
    }
}

pub fn write_report(dated_label: string, live: bool) -> Gate {
    let g = publish_allows(live, dated_label)
    let _g2 = publish_allows(live, dated_label)
    g
}
"#;

#[test]
fn module_file_owned_string_must_not_receive_bare_ref() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let owned = rs.contains("fn publish_allows(live: bool, dated_label: String")
        || rs.contains("fn publish_allows(live: bool, mut dated_label: String");
    assert!(
        owned,
        "expected owned String formal on publish_allows:\n{rs}"
    );
    let bad = rs.contains("publish_allows(live, &dated_label")
        || rs.contains("publish_allows(live, &dated_label.clone()");
    assert!(
        !bad,
        "owned publish_allows received &dated_label:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
