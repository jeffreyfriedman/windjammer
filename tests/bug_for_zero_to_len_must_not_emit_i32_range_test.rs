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

//! P3.359: `for i in 0..vec.len()` must stay usize range, not `0_i32..len()`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod csg
"#;

const CSG: &str = r#"
pub struct Node {
    pub params: Vec<f32>,
}

pub fn count_slots(node: Node) -> int {
    let mut n = 0
    for i in 0..node.params.len() {
        if node.params[i as usize] > 0.0 {
            n = n + 1
        }
    }
    n
}
"#;

fn bad_i32_len_range(rs: &str) -> bool {
    rs.contains("0_i32..") && rs.contains(".len()")
}

#[test]
fn for_zero_to_len_must_not_emit_i32_range() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("csg.wj", CSG);
    let map = test.compile().expect("P3.358 compile");
    let rs = map.get("csg.rs").expect("csg.rs");
    if bad_i32_len_range(rs) {
        eprintln!("P3.358 RED:\n{rs}");
    }
    assert!(
        !bad_i32_len_range(rs),
        "P3.358: 0..len() loops must not use i32 range start:\n{rs}"
    );
    test.cargo_check().expect("P3.358 cargo-check");
}
