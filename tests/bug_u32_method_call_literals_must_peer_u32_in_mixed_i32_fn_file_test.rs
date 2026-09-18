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

//! P3.366: In multipass/module files with i32 coord fns, `u32::wrapping_*` method args must
//! not pick up file-wide i32 literal suffixes (`1103515245_i32` → `_u32` or unsuffixed).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod camera
"#;

const CAMERA: &str = r#"
pub fn collides_stub(scale: i32) -> bool {
    let x = scale + 1
    x == 0
}

fn pseudo_random(seed: u32) -> f32 {
    let x = seed.wrapping_mul(1103515245).wrapping_add(12345)
    x as f32
}
"#;

fn bad_u32_wrapping_i32_suffixes(rs: &str) -> bool {
    rs.contains("wrapping_mul(1103515245_i32")
        || rs.contains("wrapping_add(12345_i32")
}

#[test]
fn u32_method_call_literals_must_peer_u32_in_mixed_i32_fn_file() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("camera.wj", CAMERA);
    let map = test.compile().expect("P3.366 compile");
    let rs = map.get("camera.rs").expect("camera.rs");
    if bad_u32_wrapping_i32_suffixes(rs) {
        eprintln!("P3.366 RED:\n{rs}");
    }
    assert!(
        !bad_u32_wrapping_i32_suffixes(rs),
        "P3.366: u32 wrapping_* args must not use _i32 literal peers:\n{rs}"
    );
    test.cargo_check().expect("P3.366 cargo-check");
}
