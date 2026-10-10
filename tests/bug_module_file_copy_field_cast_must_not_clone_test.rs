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

//! Copy `u64` **field** cloned before cast/arith must not `.clone()`.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Node {
    pub mesh_id: u64,
}

pub fn sum_mesh_ids(nodes: Vec<Node>) -> u64 {
    let mut total: u64 = 0u64
    for node in nodes {
        if node.mesh_id > 0u64 {
            total = total + node.mesh_id
        }
    }
    total
}
"#;

#[test]
fn module_file_copy_field_cast_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("mesh_id.clone()");
    assert!(
        !bad,
        "RED Copy field cloned before arith/cast:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
