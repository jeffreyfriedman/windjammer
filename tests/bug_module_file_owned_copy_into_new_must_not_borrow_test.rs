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

//! Regression: module file owned copy into new must not borrow.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Palette {
    pub n: i32,
}

impl Palette {
    pub fn copy(self) -> Palette {
        Palette { n: self.n }
    }
}

pub struct Editor {
    pub palette: Palette,
}

impl Editor {
    pub fn new(palette: Palette) -> Editor {
        Editor { palette: palette }
    }
}

pub fn make() -> Editor {
    let palette = Palette { n: 1 }
    Editor::new(palette.copy())
}
"#;

/// Shape: cross-module associated `VoxelMaterialEditor::new(palette: MaterialPalette)`
/// was demoted to `&MaterialPalette`, forcing `&palette.copy()` at the call site.
const PALETTE_SRC: &str = r#"
pub struct MaterialPalette {
    pub n: i32,
    pub tags: Vec<string>,
}

impl MaterialPalette {
    pub fn new() -> MaterialPalette {
        MaterialPalette { n: 0, tags: Vec::new() }
    }
    pub fn copy(self) -> MaterialPalette {
        MaterialPalette { n: self.n, tags: self.tags }
    }
}

pub struct VoxelMaterialEditor {
    pub palette: MaterialPalette,
}

impl VoxelMaterialEditor {
    pub fn new(palette: MaterialPalette) -> VoxelMaterialEditor {
        let current = palette.copy()
        VoxelMaterialEditor { palette: current }
    }
}
"#;

const EDITOR_SRC: &str = r#"
use crate::material::MaterialPalette
use crate::material::VoxelMaterialEditor

pub struct VoxelEditor {
    material_editor: VoxelMaterialEditor,
}

impl VoxelEditor {
    pub fn new(grid_size: i32) -> VoxelEditor {
        let palette = MaterialPalette::new()
        let material_editor = VoxelMaterialEditor::new(palette.copy())
        VoxelEditor { material_editor: material_editor }
    }
}
"#;

#[test]
fn module_file_owned_copy_into_new_must_not_borrow() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("&palette.copy()") || rs.contains("& palette.copy()");
    assert!(
        !bad,
        "RED: owned copy() into new() was borrowed:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

#[test]
fn module_file_associated_new_owned_palette_must_not_demote() {
    let mut test = MultiFileTest::new();
    test.add_file("material.wj", PALETTE_SRC);
    test.add_file("editor.wj", EDITOR_SRC);
    let map = test.compile().expect("compile");
    let material = map.get("material.rs").expect("material.rs");
    let editor = map.get("editor.rs").expect("editor.rs");
    eprintln!("material.rs:\n{material}\n--- editor.rs:\n{editor}");
    assert!(
        material.contains("new(palette: MaterialPalette)")
            || material.contains("new(palette: crate::material::MaterialPalette)"),
        "RED: associated new demoted owned MaterialPalette:\n{material}"
    );
    assert!(
        !material.contains("new(palette: &MaterialPalette)")
            && !material.contains("new(palette: &crate::material::MaterialPalette)"),
        "RED: associated new formal is shared-ref:\n{material}"
    );
    assert!(
        !editor.contains("&palette.copy()"),
        "RED: call site borrowed palette.copy():\n{editor}"
    );
    test.cargo_check().expect("cargo-check");
}
