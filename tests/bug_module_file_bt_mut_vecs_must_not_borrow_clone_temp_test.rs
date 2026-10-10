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

//! Regression: module file bt mut vecs must not borrow clone temp.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn tick_node(active: Vec<i32>, running: Vec<i32>) -> i32 {
    active.push(1)
    running.push(2)
    active.len() as i32
}

pub fn tick() -> i32 {
    let mut active = Vec::new()
    let mut running = Vec::new()
    tick_node(active, running)
}
"#;

#[test]
fn module_file_bt_mut_vecs_must_not_borrow_clone_temp() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("&mut active.clone()")
        || rs.contains("&mut running.clone()");
    assert!(
        !bad,
        "RED: mut Vec received &mut <temp>.clone():\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
