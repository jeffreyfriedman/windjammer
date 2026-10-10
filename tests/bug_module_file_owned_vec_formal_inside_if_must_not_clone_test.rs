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
//!   `UnifiedRenderer::materials_to_palette(materials.clone())`
//! WJ is `UnifiedRenderer::materials_to_palette(materials)` — one use, then done.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Mat {
    pub name: string,
}

pub struct Pal {
    pub n: i32,
}

pub fn materials_to_palette(materials: Vec<Mat>) -> Pal {
    Pal { n: materials.len() as i32 }
}

pub fn upload_materials(test_mode: bool, materials: Vec<Mat>) -> i32 {
    if !test_mode {
        let palette = materials_to_palette(materials)
        palette.n
    } else {
        0
    }
}
"#;

#[test]
fn module_file_owned_vec_formal_inside_if_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let cloned = rs.contains("materials.clone()");
    assert!(
        !cloned,
        "RED: owned Vec formal cloned inside if:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

