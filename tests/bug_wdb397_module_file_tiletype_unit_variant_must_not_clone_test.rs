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

//! WDB-397: Copy `TileType::Empty` must not emit `.clone()` (WDB-384 path miss).
//!
//! WDB-384/392 cover FaceDirection / Direction; product
//! gen/world/tilemap.rs still has `TileType::Empty.clone()`.
//! WJ source is `tile_type: TileType::Empty`.
//! Numbered 397 because compiler agent already used WDB-395/396.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub enum TileType {
    Empty,
    Solid,
}

pub struct Tile {
    pub tile_type: TileType,
    pub sprite_index: i32,
}

pub fn empty_tile() -> Tile {
    Tile {
        tile_type: TileType::Empty,
        sprite_index: 0,
    }
}

pub fn solid_tile() -> Tile {
    Tile {
        tile_type: TileType::Solid,
        sprite_index: 1,
    }
}
"#;

#[test]
fn wdb397_module_file_tiletype_unit_variant_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-397 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-397 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("TileType::Empty.clone()") || rs.contains("TileType::Solid.clone()");
    assert!(
        !bad,
        "WDB-397 RED: Copy TileType unit variant cloned:\n{rs}"
    );
    test.cargo_check().expect("WDB-397 cargo-check");
}

#[test]
fn wdb397_tip_out_game_core_tilemap_must_not_clone_tiletype() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("world/tilemap.rs"),
        tip.join("tilemap.rs"),
        game.join("gen/world/tilemap.rs"),
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
            !line.trim_start().starts_with("//")
                && line.contains("TileType::")
                && line.contains(".clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-397: tilemap product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-397 RED: tip/product TileType::*.clone() in:\n  {}",
        bad_paths.join("\n  ")
    );
}
