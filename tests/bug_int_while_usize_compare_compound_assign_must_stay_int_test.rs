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

//! P3.304: `while (i as usize) < vec.len()` must not make `i += 1` emit `1 as usize`
//! when `i` is `i32`/`int` (product: astar_grid.rs).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

#[test]
fn int_while_usize_compare_compound_assign_must_stay_int() {
    let mut t = MultiFileTest::new();
    t.add_file("mod.wj", "pub mod astar\n");
    t.add_file(
        "astar.wj",
        r#"
pub struct Node {
    f_score: f32,
}

pub fn pick_best(nodes: Vec<Node>) -> i32 {
    let mut best_idx: i32 = 0
    let mut best_f = nodes[0].f_score
    let mut i: i32 = 1
    while (i as usize) < nodes.len() {
        if nodes[i as usize].f_score < best_f {
            best_f = nodes[i as usize].f_score
            best_idx = i
        }
        i += 1
    }
    best_idx
}
"#,
    );
    let out = t.compile().expect("compile");
    let rs = out.get("astar.rs").expect("astar.rs");
    assert!(
        !rs.contains("i += 1 as usize") && !rs.contains("i += 1_usize"),
        "i32 counter must increment with plain 1, not usize literal\\n{rs}"
    );
    assert!(
        rs.contains("i += 1") || rs.contains("i = i + 1"),
        "expected int increment\\n{rs}"
    );
    t.cargo_check().expect("cargo check");
}
