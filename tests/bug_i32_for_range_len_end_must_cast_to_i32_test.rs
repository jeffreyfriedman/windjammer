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

//! P3.337: `for i in 0..vec.len()` with i32 loop counter must cast len end to i32.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod scene
"#;

const SCENE: &str = r#"
pub struct Params {
    pub values: Vec<f32>,
}

pub struct Node {
    pub params: Params,
}

pub fn count_params(node: Node) -> i32 {
    let mut n = 0
    for i in 0..node.params.values.len() {
        if node.params.values[i as usize] > 0.0 {
            n = n + 1
        }
    }
    n
}
"#;

#[test]
fn i32_for_range_len_end_must_cast_to_i32() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("scene.wj", SCENE);
    let map = test.compile().expect("P3.336 compile");
    let rs = map.get("scene.rs").expect("scene.rs");
    assert!(!rs.contains("0_i32..node.params.values.len()"), "P3.337:\n{rs}");
    assert!(rs.contains("as i32"), "P3.336 cast:\n{rs}");
    test.cargo_check().expect("P3.336 cargo-check");
}
