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

//! WDB-330: u32 bitwise/shift must not take `_i64` lit peers.
//!
//! Product tip game-core `camera/fps_camera.rs`:
//!   `let bits: u32 = x >> 16_i64 & 32767_i64` → E0308/E0277.
//! Source WJ: `(x >> 16) & 0x7FFF`. Prefer `16_u32` / `32767_u32`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

// Product shape (fps_camera::pseudo_random): wrapping_* intermediate + annotated
// `let bits: u32 = (x >> 16) & 0x7FFF` inside a non-u32-returning fn. The assign
// slot must keep `_u32` peers — not be overwritten by a default `_i64` peer of `x`.
const SRC: &str = r#"
fn pseudo_random(seed: u32) -> f32 {
    let x = seed.wrapping_mul(1103515245).wrapping_add(12345)
    let bits: u32 = (x >> 16) & 0x7FFF
    bits as f32 / 16383.5 - 1.0
}

pub fn hash_bits(x: u32) -> u32 {
    (x >> 16) & 32767
}
"#;

#[test]
fn wdb330_module_file_u32_bitwise_must_not_take_i64_lit_peers() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-330 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-330 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("_i64") || rs.contains("16_i32") || rs.contains("32767_i32");
    assert!(
        !bad,
        "WDB-330 RED: u32 bitwise emitted non-u32 lit peers:\n{rs}"
    );
    test.cargo_check().expect("WDB-330 cargo-check");
}

#[test]
fn wdb330_tip_out_game_core_fps_camera_must_not_mix_u32_i64_bitwise() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("fps_camera.rs"),
        tip.join("camera/fps_camera.rs"),
        game.join("gen/camera/fps_camera.rs"),
        game.join("camera/fps_camera.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("fps_camera");
        let bad = text.lines().any(|line| {
            line.contains("bits: u32")
                && (line.contains("_i64") || line.contains("16_i64") || line.contains("32767_i64"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-330: game-core/tip fps_camera missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-330 RED: tip/product mixes u32 bitwise with i64 lits in:\n  {}",
        bad_paths.join("\n  ")
    );
}
