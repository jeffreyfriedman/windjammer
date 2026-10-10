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
//!   self.cursor_x = x.clone();
//!   self.cursor_valid = self.grid.is_valid(x, y, z);
//! WJ source is `self.cursor_x = x` then `is_valid(x, y, z)`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Grid {
    pub w: i32,
}

impl Grid {
    pub fn is_valid(self, x: i32, y: i32, z: i32) -> bool {
        x >= 0 && y >= 0 && z >= 0 && x < self.w
    }
}

pub struct Editor {
    pub cursor_x: i32,
    pub cursor_y: i32,
    pub cursor_z: i32,
    pub cursor_valid: bool,
    pub grid: Grid,
}

impl Editor {
    pub fn set_cursor(self, x: i32, y: i32, z: i32) {
        self.cursor_x = x
        self.cursor_y = y
        self.cursor_z = z
        self.cursor_valid = self.grid.is_valid(x, y, z)
    }
}
"#;

#[test]
fn module_file_i32_formal_field_assign_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("x.clone()") || rs.contains("y.clone()") || rs.contains("z.clone()");
    assert!(
        !bad,
        "RED: Copy i32 formal cloned on field assign/reuse:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

