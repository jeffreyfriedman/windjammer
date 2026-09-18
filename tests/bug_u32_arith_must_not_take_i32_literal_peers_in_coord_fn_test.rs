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

//! P3.360: P3.356 coord i32 literal promotion must not widen u32 mesh/editor arith (`n / 2`, `i0 + 1`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod mesh
pub mod paint
"#;

const MESH: &str = r#"
pub fn next_index(i0: u32) -> u32 {
    i0 + 1
}

pub fn half_count(n: u32) -> u32 {
    n / 2
}
"#;

const PAINT: &str = r#"
pub fn bump_pair(b: u32) -> u32 {
    b + 1
}
"#;

fn bad_u32_i32_literal_peers(rs: &str) -> bool {
    rs.contains("+ 1_i32")
        || rs.contains("/ 2_i32")
        || rs.contains("+ 2_i32")
}

#[test]
fn u32_arith_must_not_take_i32_literal_peers_in_coord_fn() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("mesh.wj", MESH);
    test.add_file("paint.wj", PAINT);
    let map = test.compile().expect("P3.357 compile");
    let combined = format!(
        "{}\n{}",
        map.get("mesh.rs").expect("mesh.rs"),
        map.get("paint.rs").expect("paint.rs")
    );
    if bad_u32_i32_literal_peers(&combined) {
        eprintln!("P3.357 RED:\n{combined}");
    }
    assert!(
        !bad_u32_i32_literal_peers(&combined),
        "P3.357: u32 arith must not use _i32 literal peers in coord-preferring fns:\n{combined}"
    );
    test.cargo_check().expect("P3.357 cargo-check");
}
