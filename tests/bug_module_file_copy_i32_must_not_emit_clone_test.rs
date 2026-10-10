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

//! Copy `i32` must not emit `.clone()` on casts/bindings.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub const MAT_TRIM: u32 = 7
pub const VIEWER_GRID: u32 = 64

pub fn set_cell(x: i32, mat: i32) -> i32 {
    x + mat
}

pub fn place(x: i32) -> i32 {
    let y = x
    let mut iy = y
    while iy < x + 3 {
        iy = iy + 1
    }
    set_cell(x, 7)
    set_cell(x, MAT_TRIM)
    set_cell(x, MAT_TRIM)
    set_cell(x, VIEWER_GRID)
    set_cell(x, VIEWER_GRID)
}

pub fn place_array(cx: i32, cz: i32) {
    for x in [cx - 6, cx + 5] {
        for z in [cz - 4, cz + 4] {
            set_cell(x, MAT_TRIM)
            set_cell(z, VIEWER_GRID)
        }
    }
}

pub fn place_max(w: i32, h: i32, d: i32) -> i32 {
    let max_size = w.max(h).max(d)
    set_cell(max_size, MAT_TRIM)
    set_cell(max_size, VIEWER_GRID)
}
"#;

#[test]
fn module_file_copy_i32_must_not_emit_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".clone()");
    assert!(
        !bad,
        "RED Copy i32 emitted .clone():\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
