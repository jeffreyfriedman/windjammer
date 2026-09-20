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

//! WDB-353: f32→usize index must not emit double cast `as i32 as usize`.
//!
//! Product tip game-core `terrain/terrain.rs`:
//!   `let grid_x: usize = (x / self.scale) as i32 as usize`
//! Prefer a single cast to usize (or i32 then cast once at use). Distinct from WDB-339.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Terrain {
    pub scale: f32,
}

pub fn grid_index(t: Terrain, x: f32) -> usize {
    (x / t.scale) as usize
}
"#;

#[test]
fn wdb353_module_file_f32_to_usize_must_not_double_cast_via_i32() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-353 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-353 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("as i32 as usize");
    assert!(
        !bad,
        "WDB-353 RED: double cast as i32 as usize:\n{rs}"
    );
    test.cargo_check().expect("WDB-353 cargo-check");
}

#[test]
fn wdb353_tip_out_game_core_terrain_must_not_double_cast() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("terrain.rs"),
        tip.join("terrain/terrain.rs"),
        game.join("gen/terrain/terrain.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("terrain");
        let bad = text.lines().any(|line| {
            line.contains("as i32 as usize") && !line.trim_start().starts_with("//")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-353: game-core/tip terrain missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-353 RED: tip/product as i32 as usize in:\n  {}",
        bad_paths.join("\n  ")
    );
}
