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

//! WDB-393: Copy i32 formals assigned to fields then reused must not `.clone()`.
//!
//! Product gen/editor/voxel_editor.rs:
//!   self.cursor_x = x.clone();
//!   self.cursor_valid = self.grid.is_valid(x, y, z);
//! WJ source is `self.cursor_x = x` then `is_valid(x, y, z)`.
//! Distinct from WDB-343 (call-only reuse; isolate GREEN) and WDB-391 (u32 helper return).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

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
fn wdb393_module_file_i32_formal_field_assign_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-393 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-393 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("x.clone()") || rs.contains("y.clone()") || rs.contains("z.clone()");
    assert!(
        !bad,
        "WDB-393 RED: Copy i32 formal cloned on field assign/reuse:\n{rs}"
    );
    test.cargo_check().expect("WDB-393 cargo-check");
}

#[test]
fn wdb393_tip_out_game_core_voxel_editor_must_not_clone_cursor() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("editor/voxel_editor.rs"),
        tip.join("voxel_editor.rs"),
        game.join("gen/editor/voxel_editor.rs"),
        tip.join("scene_graph/scene_graph_state.rs"),
        game.join("gen/scene_graph/scene_graph_state.rs"),
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
            !t.starts_with("//")
                && (t.contains("cursor_x = x.clone()")
                    || t.contains("cursor_y = y.clone()")
                    || t.contains("cursor_z = z.clone()")
                    || t.contains("local_position.x = x.clone()"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-393: voxel_editor/scene_graph product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-393 RED: tip/product i32 formal field-assign clone in:\n  {}",
        bad_paths.join("\n  ")
    );
}
