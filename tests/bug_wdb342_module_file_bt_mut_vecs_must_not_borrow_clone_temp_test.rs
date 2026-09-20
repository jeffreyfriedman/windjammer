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

//! WDB-342: `&mut Vec` formals must not receive `&mut active.clone()` (BT tick).
//!
//! Product tip game-core `behavior_tree/executor.rs`:
//!   `tick_node(tree, root, &mut active.clone(), &mut running.clone(), …)`
//!   with `active: &mut Vec<i32>` → E0716 / discarded mutation.
//! Twin of WDB-336/337; distinct product surface (BT executor + mesh_generator/
//! csg/coverage still open beyond WDB-337 tip paths).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn tick_node(active: Vec<i32>, running: Vec<i32>) -> i32 {
    active.push(1)
    running.push(2)
    active.len() as i32
}

pub fn tick() -> i32 {
    let mut active = Vec::new()
    let mut running = Vec::new()
    tick_node(active, running)
}
"#;

#[test]
fn wdb342_module_file_bt_mut_vecs_must_not_borrow_clone_temp() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-342 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-342 MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("&mut active.clone()")
        || rs.contains("&mut running.clone()")
        || (rs.contains("&mut ") && rs.contains(".clone()"));
    assert!(
        !bad,
        "WDB-342 RED: mut Vec received &mut <temp>.clone():\n{rs}"
    );
    test.cargo_check().expect("WDB-342 cargo-check");
}

#[test]
fn wdb342_tip_out_game_core_bt_executor_csg_coverage_must_not_mut_borrow_clone_temp() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("executor.rs"),
        tip.join("behavior_tree/executor.rs"),
        tip.join("csg.rs"),
        tip.join("editor/csg.rs"),
        tip.join("coverage.rs"),
        tip.join("testing/coverage.rs"),
        tip.join("mesh_generator.rs"),
        tip.join("rendering/mesh_generator.rs"),
        game.join("gen/behavior_tree/executor.rs"),
        game.join("gen/editor/csg.rs"),
        game.join("gen/testing/coverage.rs"),
        game.join("gen/rendering/mesh_generator.rs"),
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
            line.contains("&mut ") && line.contains(".clone()")
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-342: game-core/tip executor/csg/coverage/mesh_generator missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-342 RED: tip/product uses &mut <temp>.clone() in:\n  {}",
        bad_paths.join("\n  ")
    );
}
