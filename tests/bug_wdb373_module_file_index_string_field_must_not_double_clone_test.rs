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

//! WDB-373: indexed String field must not emit `].clone().path.clone()` (double clone).
//!
//! Product tip game-core `scripting/live_reload.rs`:
//!   `get_affected_files(self.watches[i].clone().path.clone())`
//! Prefer `self.watches[i].path.clone()` once (or borrow if formal is `&str`).
//! Twin of WDB-370 for non-Copy String field path.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Watch {
    pub path: string,
}

pub struct DepGraph {
    pub n: i32,
}

pub fn get_affected_files(dep: DepGraph, path: string) -> Vec<string> {
    let mut out = Vec::new()
    out.push(path)
    out
}

pub struct Loader {
    pub watches: Vec<Watch>,
}

impl Loader {
    pub fn affected(self, dep: DepGraph, i: usize) -> Vec<string> {
        get_affected_files(dep, self.watches[i].path)
    }
}
"#;

#[test]
fn wdb373_module_file_index_string_field_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-373 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-373 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone().path.clone()") || rs.contains("].clone().path");
    assert!(
        !bad,
        "WDB-373 RED: indexed String path double-cloned:\n{rs}"
    );
    test.cargo_check().expect("WDB-373 cargo-check");
}

#[test]
fn wdb373_tip_out_game_core_live_reload_must_not_double_clone_path() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("scripting/live_reload.rs"),
        tip.join("live_reload.rs"),
        tip.join("event/dispatcher.rs"),
        tip.join("editor/asset_browser.rs"),
        tip.join("rendering/pbr_pipeline.rs"),
        game.join("gen/scripting/live_reload.rs"),
        game.join("gen/event/dispatcher.rs"),
        game.join("gen/editor/asset_browser.rs"),
        game.join("gen/rendering/pbr_pipeline.rs"),
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
                && line.contains("].clone().")
                && (line.contains(".path.clone()")
                    || line.contains(".event_description.clone()")
                    || line.contains(".asset_type.clone()")
                    || line.contains(".mesh_id.clone()"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-373: product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-373 RED: tip/product indexed String field double-clone in:\n  {}",
        bad_paths.join("\n  ")
    );
}
