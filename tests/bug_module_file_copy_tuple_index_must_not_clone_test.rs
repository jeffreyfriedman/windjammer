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
//!   `let (ox, oz) = offsets[(i as usize)].clone();`
//! WJ is `let (ox, oz) = offsets[i]`.
//! First array element may be `(-1, 0)` — Neg must not break Copy inference.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn pick(i: i32) -> f32 {
    let offsets = [
        (1.0, 0.0, 0.0),
        (-1.0, 0.0, 0.0),
    ]
    let (ox, oy, oz) = offsets[i]
    ox + oy + oz
}

pub fn collides(cx: i32, cz: i32) -> bool {
    let offsets = [(-1, 0), (1, 0), (0, -1), (0, 1)]
    let mut i = 0
    while i < 4 {
        let (ox, oz) = offsets[i]
        let check_x = cx + ox
        let check_z = cz + oz
        if check_x == 0 && check_z == 0 {
            return true
        }
        i = i + 1
    }
    false
}
"#;

#[test]
fn module_file_copy_tuple_index_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let cloned = rs.contains(".clone()");
    assert!(
        !cloned,
        "RED: Copy tuple array index cloned on destructure:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

