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

//! WDB-387: indexed `&self` `get_id()` must not emit `].clone().get_id()`.
//!
//! Product tip game-core `world_partition/streaming.rs`:
//!   `self.cells[i].clone().get_id()`
//! WJ source is `self.cells[i].get_id()`. Twin of WDB-362 (`].clone().mesh_id()`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Cell {
    pub id: u32,
}

impl Cell {
    pub fn get_id(self) -> u32 {
        self.id
    }
}

pub struct Partition {
    pub cells: Vec<Cell>,
}

impl Partition {
    pub fn first_id(self, i: usize) -> u32 {
        self.cells[i].get_id()
    }
}
"#;

#[test]
fn wdb387_module_file_index_get_id_must_not_clone_element() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-387 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-387 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("].clone().get_id()");
    assert!(
        !bad,
        "WDB-387 RED: indexed get_id cloned the element:\n{rs}"
    );
    test.cargo_check().expect("WDB-387 cargo-check");
}

#[test]
fn wdb387_tip_out_game_core_partition_must_not_clone_before_get_id() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("world_partition/streaming.rs"),
        tip.join("streaming.rs"),
        game.join("gen/world_partition/streaming.rs"),
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
            !line.trim_start().starts_with("//") && line.contains("].clone().get_id()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-387: world_partition streaming product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-387 RED: tip/product ].clone().get_id() in:\n  {}",
        bad_paths.join("\n  ")
    );
}
