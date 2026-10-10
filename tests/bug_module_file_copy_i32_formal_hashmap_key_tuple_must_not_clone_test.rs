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
//!   `g_score.insert((start_x.clone(), start_y.clone()), 0.0_f32)`
//! WJ is `g_score.insert((start_x, start_y), 0.0)`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
use std::collections::HashMap

pub fn seed_g_score(start_x: i32, start_y: i32) -> HashMap<(i32, i32), f32> {
    let mut g_score: HashMap<(i32, i32), f32> = HashMap::new()
    g_score.insert((start_x, start_y), 0.0)
    g_score
}

pub fn update_g_score(g_score: HashMap<(i32, i32), f32>, nx: i32, ny: i32, tentative_g: f32) {
    g_score.insert((nx, ny), tentative_g)
}
"#;

#[test]
fn module_file_copy_i32_formal_hashmap_key_tuple_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("start_x.clone()")
        || rs.contains("start_y.clone()")
        || rs.contains("nx.clone()")
        || rs.contains("ny.clone()");
    assert!(
        !bad,
        "RED: Copy i32 formal cloned into HashMap key tuple:\n{rs}"
    );
    let _ = test.cargo_check();
}

