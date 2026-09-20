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

//! WDB-329: `Vec::remove(idx as usize)` must not emit `&idx as usize`.
//!
//! Product tip game-core `behavior_tree/blackboard.rs`:
//!   `self.entries.remove(&idx as usize)` → E0606 (can't cast `&i32` as usize).
//! Source WJ: `self.entries.remove(idx as usize)`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Entry {
    pub key: string,
}

pub struct Board {
    pub entries: Vec<Entry>,
}

pub fn remove_at(board: Board, idx: i32) -> Board {
    let mut b = board
    if idx >= 0 {
        b.entries.remove(idx as usize)
    }
    b
}
"#;

#[test]
fn wdb329_module_file_vec_remove_cast_must_not_borrow_idx() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-329 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-329 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("&idx as usize") || rs.contains("remove(&idx");
    assert!(
        !bad,
        "WDB-329 RED: Vec::remove emitted &idx as usize:\n{rs}"
    );
    test.cargo_check().expect("WDB-329 cargo-check");
}

#[test]
fn wdb329_tip_out_game_core_blackboard_must_not_cast_ref_idx_as_usize() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("blackboard.rs"),
        tip.join("behavior_tree/blackboard.rs"),
        game.join("gen/behavior_tree/blackboard.rs"),
        game.join("behavior_tree/blackboard.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("blackboard");
        let bad = text.contains("&idx as usize") || text.contains("remove(&idx as");
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-329: game-core/tip blackboard missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-329 RED: tip/product casts &idx as usize in:\n  {}",
        bad_paths.join("\n  ")
    );
}
