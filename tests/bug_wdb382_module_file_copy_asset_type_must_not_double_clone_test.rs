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

//! WDB-382: indexed Copy AssetType must not emit `].clone().asset_type.clone()`.
//!
//! Product tip game-core `editor/asset_browser.rs`:
//!   `asset_type_name(self.assets[i].clone().asset_type.clone())`
//! Prefer `self.assets[i].asset_type` (Copy enum). Twin of WDB-374/377.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub enum AssetType {
    Mesh,
    Texture,
}

pub fn type_name(t: AssetType) -> string {
    match t {
        AssetType::Mesh => "mesh",
        AssetType::Texture => "texture",
    }
}

pub struct AssetEntry {
    pub asset_type: AssetType,
}

pub struct Browser {
    pub assets: Vec<AssetEntry>,
}

impl Browser {
    pub fn name_at(self, i: usize) -> string {
        type_name(self.assets[i].asset_type)
    }
}
"#;

#[test]
fn wdb382_module_file_copy_asset_type_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-382 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-382 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone().asset_type") || rs.contains(".asset_type.clone()");
    assert!(
        !bad,
        "WDB-382 RED: indexed Copy AssetType double-cloned:\n{rs}"
    );
    test.cargo_check().expect("WDB-382 cargo-check");
}

#[test]
fn wdb382_tip_out_game_core_browser_must_not_double_clone_asset_type() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("editor/asset_browser.rs"),
        tip.join("asset_browser.rs"),
        game.join("gen/editor/asset_browser.rs"),
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
                && (line.contains("].clone().asset_type.clone()")
                    || line.contains("].clone().asset_type"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-382: asset_browser product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-382 RED: tip/product indexed asset_type double-clone in:\n  {}",
        bad_paths.join("\n  ")
    );
}
