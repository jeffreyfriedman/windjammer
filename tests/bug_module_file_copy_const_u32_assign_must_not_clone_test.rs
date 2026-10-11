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

//! Copy module `const` `u32` must not `.clone()` on assign.
//!
//! Product `voxel/svo64_convert.rs`:
//!   `nodes[node_index] = LEAF_FLAG.clone()`
//! WJ is `nodes[node_index] = LEAF_FLAG`.
//! Distinct from (`f32::MAX.clone()`), (local i32 assign),
//! and (Copy f32 index arith).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
const LEAF_FLAG: u32 = 0x100

pub fn store_leaf(nodes: Vec<u32>, idx: int) {
    nodes[idx] = LEAF_FLAG
    nodes[idx + 1] = LEAF_FLAG
}
"#;

#[test]
fn module_file_copy_const_u32_assign_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let cloned = rs.contains("LEAF_FLAG.clone()");
    assert!(
        !cloned,
        "Copy const u32 assign cloned:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
