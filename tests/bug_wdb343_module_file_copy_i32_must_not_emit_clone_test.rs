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

//! WDB-343: Copy `i32` must not emit `.clone()` on casts/bindings.
//!
//! Product tip game-core:
//!   `scene/component_viewer_controls.rs`: `(MAT_* as i32).clone()`, `(x as i32).clone()`
//!   `camera/fps_camera.rs`: `let mut iy = y_lo.clone()`
//!   `voxel/svo_convert.rs`: `(max_size as i32).clone()`
//! Prefer bare Copy values (no `.clone()`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

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
fn wdb343_module_file_copy_i32_must_not_emit_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-343 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-343 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains(".clone()");
    assert!(
        !bad,
        "WDB-343 RED: Copy i32 emitted .clone():\n{rs}"
    );
    test.cargo_check().expect("WDB-343 cargo-check");
}

#[test]
fn wdb343_tip_out_game_core_copy_i32_must_not_emit_clone() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("component_viewer_controls.rs"),
        tip.join("scene/component_viewer_controls.rs"),
        tip.join("fps_camera.rs"),
        tip.join("camera/fps_camera.rs"),
        tip.join("svo_convert.rs"),
        tip.join("voxel/svo_convert.rs"),
        game.join("gen/scene/component_viewer_controls.rs"),
        game.join("gen/camera/fps_camera.rs"),
        game.join("gen/voxel/svo_convert.rs"),
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
            (line.contains("as i32).clone()")
                || line.contains("_lo.clone()")
                || line.contains("y_lo.clone()")
                || line.contains("(max_size as i32).clone()"))
                && !line.trim_start().starts_with("//")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-343: game-core/tip product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-343 RED: tip/product Copy i32 .clone() in:\n  {}",
        bad_paths.join("\n  ")
    );
}
