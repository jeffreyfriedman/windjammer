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

//! Regression: module file explicit string clone must not be stripped.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod paths
pub mod cap
"#;

const PATHS: &str = r#"
pub fn dual_bind(lineitem_path: string) -> string {
    let mut li_path = lineitem_path.clone()
    let mut collect_path = lineitem_path.clone()
    if li_path == "" {
        li_path = "cap_fixture"
        collect_path = ""
    }
    if collect_path == "" {
        return li_path
    }
    collect_path
}
"#;

const CAP: &str = r#"
use crate::paths::dual_bind

pub fn cap_empty() -> string {
    dual_bind("")
}

pub fn cap_file() -> string {
    dual_bind("/tmp/lineitem.tbl")
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("paths.wj", PATHS);
    test.add_file("cap.wj", CAP);
    test
}

#[test]
fn module_file_explicit_string_clone_must_not_be_stripped() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed (codegen may still be wrong)");
    let paths_rs = map.get("paths.rs").expect("paths.rs");

    // Stale shape: owned formal + two bare moves (no clone/to_string).
    let dual_bare_move = paths_rs.contains("let mut li_path = lineitem_path;")
        && paths_rs.contains("let mut collect_path = lineitem_path;");
    let ok_rebind = paths_rs.contains("lineitem_path.clone()")
        || paths_rs.contains("lineitem_path.to_string()");

    eprintln!("paths.rs:\n{paths_rs}");
    eprintln!("dual_bare_move={dual_bare_move} ok_rebind={ok_rebind}");

    if dual_bare_move || !ok_rebind {
        panic!(
            "RED: reused string must clone or to_string on dual bind \
             (not two bare moves)."
        );
    }

    test.cargo_check().expect(
        "dual bind of reused string must cargo-check.",
    );
}
