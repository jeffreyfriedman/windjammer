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

//! Copy `i32` loop var in abs else-branch must not `.clone()`.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn abs_sum(n: i32) -> i32 {
    let mut total = 0
    for dx in -n..(n + 1) {
        let adx = if dx < 0 { -dx } else { dx }
        total = total + adx
    }
    total
}

/// negative literal range + multi-use
/// after abs if/else — must not clone the loop counter in the else arm or later arith.
pub fn rib(cx: i32, cz: i32) -> i32 {
    let mut hits = 0
    for dx in -2..3 {
        for dz in -2..3 {
            let adx = if dx < 0 { -dx } else { dx }
            let adz = if dz < 0 { -dz } else { dz }
            if adx + adz <= 3 && adx + adz > 0 {
                hits = hits + cx + dx + cz + dz
            }
        }
    }
    hits
}
"#;

#[test]
fn module_file_copy_i32_abs_else_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let cloned = rs.lines().any(|line| {
        let t = line.trim_start();
        !t.starts_with("//") && !t.starts_with("///") && t.contains(".clone()")
    });
    assert!(
        !cloned,
        "RED Copy i32 abs else-branch cloned:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
