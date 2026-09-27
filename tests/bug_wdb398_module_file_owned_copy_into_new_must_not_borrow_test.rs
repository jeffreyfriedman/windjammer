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

//! DB-398: owned `.copy()` into `new(owned)` must not emit `&palette.copy()`.
//!
//! Product gen/editor/voxel_editor.rs:
//!   VoxelMaterialEditor::new(&palette.copy())
//! WJ source is `VoxelMaterialEditor::new(palette.copy())` and
//! `new` takes owned `MaterialPalette`. Distinct from DB-388 (`Vec3::new(&a)`).
//! Numbered 398 because compiler agent already used DB-395/396.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

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

#[test]
fn wdb398_module_file_owned_copy_into_new_must_not_borrow() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("DB-398 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("DB-398 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("&palette.copy()") || rs.contains("& palette.copy()");
    assert!(
        !bad,
        "DB-398 RED: owned copy() into new() was borrowed:\n{rs}"
    );
    test.cargo_check().expect("DB-398 cargo-check");
}

#[test]
fn wdb398_tip_out_game_core_voxel_editor_must_not_borrow_palette_copy() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("game-host/game-host-core");
    let paths = [
        tip.join("editor/voxel_editor.rs"),
        tip.join("voxel_editor.rs"),
        game.join("gen/editor/voxel_editor.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("product");
        let bad = text.lines().any(|line| {
            let t = line.trim_start();
            !t.starts_with("//") && t.contains("&palette.copy()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "DB-398: voxel_editor product files missing");
    assert!(
        bad_paths.is_empty(),
        "DB-398 RED: tip/product &palette.copy() into owned new() in:\n  {}",
        bad_paths.join("\n  ")
    );
}
