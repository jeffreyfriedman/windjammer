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

//! WDB-142: dual bind of a reused `string` must not move twice without clone/to_string.
//!
//! WindjammerDB CQ-C5 stale gen emitted:
//!   `let mut li_path = lineitem_path; let mut collect_path = lineitem_path;` → E0382
//! while source wrote `.clone()`.
//!
//! Tip may demote to `&str` + `.to_string()` (OK) or keep owned + `.clone()` (OK).
//! Forbidden: two bare moves of the same owned binding.

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

fn wdb142_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("paths.wj", PATHS);
    test.add_file("cap.wj", CAP);
    test
}

#[test]
fn wdb142_module_file_explicit_string_clone_must_not_be_stripped() {
    let test = wdb142_fixture();
    let map = test
        .compile()
        .expect("WDB-142 multipass compile should succeed (codegen may still be wrong)");
    let paths_rs = map.get("paths.rs").expect("paths.rs");

    // Stale product shape: owned formal + two bare moves (no clone/to_string).
    let dual_bare_move = paths_rs.contains("let mut li_path = lineitem_path;")
        && paths_rs.contains("let mut collect_path = lineitem_path;");
    let ok_rebind = paths_rs.contains("lineitem_path.clone()")
        || paths_rs.contains("lineitem_path.to_string()");

    eprintln!("WDB-142 paths.rs:\n{paths_rs}");
    eprintln!("dual_bare_move={dual_bare_move} ok_rebind={ok_rebind}");

    if dual_bare_move || !ok_rebind {
        panic!(
            "WDB-142 RED: reused string must clone or to_string on dual bind \
             (not two bare moves). Product: wave1_sf1_cli / binder stale gen."
        );
    }

    test.cargo_check().expect(
        "WDB-142: dual bind of reused string must cargo-check.",
    );
}
