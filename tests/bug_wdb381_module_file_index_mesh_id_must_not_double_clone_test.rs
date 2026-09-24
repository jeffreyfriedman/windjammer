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

//! WDB-381: indexed String `mesh_id` / `event_description` must not double-clone.
//!
//! Product tip game-core:
//!   `rendering/pbr_pipeline.rs`: `self.levels[pi].clone().mesh_id.clone()`
//!   `event/dispatcher.rs`: `self.event_log[i].clone().event_description.clone()`
//! Prefer `self.levels[pi].mesh_id.clone()` once (or borrow). Twin of WDB-373.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct LodLevel {
    pub mesh_id: string,
}

pub struct LodChain {
    pub levels: Vec<LodLevel>,
}

impl LodChain {
    pub fn primary(self, i: usize) -> string {
        self.levels[i].mesh_id
    }
}
"#;

#[test]
fn wdb381_module_file_index_mesh_id_must_not_double_clone() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-381 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-381 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone().mesh_id");
    assert!(
        !bad,
        "WDB-381 RED: indexed mesh_id double-cloned:\n{rs}"
    );
    test.cargo_check().expect("WDB-381 cargo-check");
}

#[test]
fn wdb381_tip_out_game_core_must_not_double_clone_mesh_id() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("rendering/pbr_pipeline.rs"),
        tip.join("pbr_pipeline.rs"),
        tip.join("event/dispatcher.rs"),
        tip.join("dispatcher.rs"),
        game.join("gen/rendering/pbr_pipeline.rs"),
        game.join("gen/event/dispatcher.rs"),
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
                && (line.contains("].clone().mesh_id.clone()")
                    || line.contains("].clone().event_description.clone()"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-381: pbr/dispatcher product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-381 RED: tip/product indexed string double-clone in:\n  {}",
        bad_paths.join("\n  ")
    );
}
