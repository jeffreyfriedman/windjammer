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
//!   `materials.insert(node.material_id.clone())`
//! WJ is `materials.insert(node.material_id)`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Node {
    pub material_id: u64,
}

pub fn collect_ids(nodes: Vec<Node>) -> Vec<u64> {
    let mut ids: Vec<u64> = Vec::new()
    for node in nodes {
        if node.material_id > 0u64 {
            ids.push(node.material_id)
        }
    }
    ids
}
"#;

#[test]
fn module_file_copy_u64_field_into_insert_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("material_id.clone()");
    assert!(
        !bad,
        "RED: Copy u64 field cloned into push/insert:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

