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

//! FAILING REPRO — multipass app: `own()` forwarder into cross-module owned `String` formal
//! emits `&local` (E0308) under `--module-file`.
//!
//! Isolate gate `bug_app_cross_crate_owned_forwarder_emits_borrow_test` is tip GREEN.
//! Ecosystem apps (`wj-todo-cli` / `wj-path`, `wj-sitegen` / `wj-template`) still fail until
//! multipass call sites move owned locals into cross-module callees.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const PATH_PKG: &str = r#"
pub fn join_path(left: string, right: string) -> string {
    left
}
"#;

const APP: &str = r#"
use crate::path_pkg::join_path

pub fn resolve(left: string, right: string) -> string {
    let l = own(left)
    let r = own(right)
    join_path(l, r)
}

fn own(value: string) -> string {
    value
}
"#;

fn app_forwarder_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod path_pkg
pub mod app
"#,
    );
    test.add_file("path_pkg.wj", PATH_PKG);
    test.add_file("app.wj", APP);
    test
}

#[test]
fn cross_module_owned_forwarder_must_move_not_borrow() {
    let mut test = app_forwarder_fixture();
    let map = test
        .compile()
        .expect("app multipass forwarder compile should succeed");
    let app_rs = map.get("app.rs").expect("app.rs must be generated");

    let bad_emit = app_rs.contains("join_path(&l")
        || app_rs.contains("join_path(&left")
        || app_rs.contains("join_path( &l");
    assert!(
        !bad_emit,
        "RED: owned cross-module formal must receive moved String, not borrow. Got:\n{app_rs}"
    );
    assert!(
        app_rs.contains("join_path(l") || app_rs.contains("join_path(l,"),
        "expected bare move into owned String formal:\n{app_rs}"
    );
    test.cargo_check()
        .expect("moved String forwarder must cargo-check");
}
