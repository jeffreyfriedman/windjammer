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
//!   `let grid_x: usize = (x / self.scale) as i32 as usize`

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Terrain {
    pub scale: f32,
}

pub fn grid_index(t: Terrain, x: f32) -> usize {
    (x / t.scale) as usize
}
"#;

const SRC_NESTED: &str = r#"
pub struct Terrain {
    pub scale: f32,
}

impl Terrain {
    pub fn get_height(self, x: f32, z: f32) -> usize {
        let grid_x: usize = (((x / self.scale) as i32) as usize)
        let grid_z: usize = (((z / self.scale) as i32) as usize)
        grid_x + grid_z
    }

    pub fn apply_radius(self, x: f32) {
        let grid_x = (x / self.scale) as i32
        let dx = 1
        let px: usize = ((grid_x + dx) as i32) as usize
        let _ = px
    }
}
"#;

#[test]
fn module_file_f32_to_usize_must_not_double_cast_via_i32() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("as i32 as usize");
    assert!(
        !bad,
        "RED: double cast as i32 as usize:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

#[test]
fn module_file_nested_i32_usize_cast_must_fold() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC_NESTED);
    let map = test.compile().expect("nested compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("nested lib.rs:\n{rs}");
    let bad = rs.contains("as i32 as usize");
    assert!(
        !bad,
        "RED: nested (e as i32) as usize not folded:\n{rs}"
    );
    assert!(
        rs.contains("as usize"),
        "expected single as usize. Got:\n{rs}"
    );
    test.cargo_check().expect("nested cargo-check");
}

