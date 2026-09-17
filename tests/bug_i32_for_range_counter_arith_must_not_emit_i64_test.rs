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

//! P3.334: `for i in 0..seg` with `seg: i32` must bind `i` as i32 so `(i + 1) % seg` stays i32
//! (mesh_primitives.wj cap ring indices).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod mesh
"#;

const MESH: &str = r#"
pub fn cap_ring_index(seg: i32) -> u32 {
    let mut last = 0u32
    for i in 0..seg {
        last = ((i + 1) % seg) as u32
    }
    last
}
"#;

#[test]
fn i32_for_range_counter_arith_must_not_emit_i64() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("mesh.wj", MESH);
    let map = test.compile().expect("P3.334 compile");
    let rs = map.get("mesh.rs").expect("mesh.rs");
    if rs.contains("+ 1_i64") {
        eprintln!("P3.334 RED:\n{rs}");
    }
    assert!(
        !rs.contains("+ 1_i64"),
        "P3.334: i32 range loop counter arith must not emit _i64 peers:\n{rs}"
    );
    test.cargo_check().expect("P3.334 cargo-check");
}
