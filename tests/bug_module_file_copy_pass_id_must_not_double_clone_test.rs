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
//!   `sorted[i].clone().pass_id.clone()` / `sorted[j].clone().shader_file.clone()`

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub enum PassId {
    A,
    B,
}

pub struct Pass {
    pub pass_id: PassId,
}

pub fn collect_ids(passes: Vec<Pass>) -> Vec<PassId> {
    let mut out = Vec::new()
    let mut i = 0
    while i < passes.len() {
        out.push(passes[i].pass_id)
        i = i + 1
    }
    out
}
"#;

#[test]
fn module_file_copy_pass_id_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone().pass_id") || rs.contains(".pass_id.clone()");
    assert!(
        !bad,
        "RED: indexed Copy PassId double-cloned:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

