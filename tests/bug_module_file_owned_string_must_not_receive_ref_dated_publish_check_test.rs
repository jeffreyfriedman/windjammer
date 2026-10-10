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

//! owned `String` formals must not receive `&dated` (wave1 publish_check CLI).
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn run_resolved(live: bool, dated_label: string) -> string {
    if live {
        return dated_label + "|live"
    }
    dated_label + "|cap"
}

pub fn main_check(dated: string) -> string {
    run_resolved(true, dated)
}
"#;

#[test]
fn module_file_owned_string_must_not_receive_ref_dated() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let owned = rs.contains("fn run_resolved(live: bool, dated_label: String")
        || rs.contains("fn run_resolved(live: bool, mut dated_label: String");
    assert!(
        owned,
        "expected owned String formal on run_resolved:\n{rs}"
    );
    let bad = rs.contains("run_resolved(true, &dated")
        || rs.contains("run_resolved(true, &dated.clone()");
    assert!(
        !bad,
        "RED owned run_resolved received &dated:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
