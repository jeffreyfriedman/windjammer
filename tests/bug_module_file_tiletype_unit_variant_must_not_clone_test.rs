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
//! WJ source is `tile_type: TileType::Empty`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

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
fn module_file_tiletype_unit_variant_must_not_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("TileType::Empty.clone()") || rs.contains("TileType::Solid.clone()");
    assert!(
        !bad,
        "RED: Copy TileType unit variant cloned:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}

