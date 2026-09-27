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

//! WDB-396: mutually recursive BT tick helpers must agree on Vec mutability.
//!
//! Product tip `behavior_tree/executor.rs` (hottest leftover, 24 rustc errors):
//!   `tick_node(..., active: &mut Vec<i32>, …)` then
//!   `tick_composite_sequence(..., active: Vec<i32>, …)` then
//!   `tick_node(..., &mut active, …)` while `active` is a non-mut owned formal
//!   → E0596. Match arms also pass `active.clone()` into the owned sibling,
//!   discarding mutations.
//!
//! WDB-342 isolate is GREEN for a one-way `tick() → tick_node` with `.push`.
//! Product is a match-dispatched SCC: node mutates + forwards, composite
//! forwards back. All Vec formals in the SCC must be `&mut Vec` (or owned
//! `mut`) and call sites must pass the binding, not `&mut` on a non-mut
//! owned formal and not `.clone()` into a mut-borrow slot.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub enum Kind {
    Leaf,
    Seq,
}

pub fn tick_node(id: i32, kind: Kind, active: Vec<i32>, running: Vec<i32>) -> i32 {
    let status = match kind {
        Kind::Leaf => 1,
        Kind::Seq => tick_seq(id, active, running),
    }
    active.push(id)
    running.push(id)
    status
}

pub fn tick_seq(id: i32, active: Vec<i32>, running: Vec<i32>) -> i32 {
    tick_node(id + 1, Kind::Leaf, active, running)
}

pub fn tick() -> i32 {
    let mut active = Vec::new()
    let mut running = Vec::new()
    tick_node(0, Kind::Seq, active, running)
}
"#;

fn owned_vec_formal_not_mut(rs: &str, fn_name: &str) -> bool {
    rs.lines().any(|line| {
        line.contains(&format!("fn {fn_name}("))
            && line.contains("active: Vec<i32>")
            && !line.contains("mut active: Vec<i32>")
            && !line.contains("active: &mut Vec<i32>")
    })
}

#[test]
fn wdb396_module_file_bt_recursive_vec_must_agree_mut() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-396 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-396 MultiFile lib.rs:\n{rs}");
    let seq_owned_not_mut = owned_vec_formal_not_mut(rs, "tick_seq");
    let node_mut_ref = rs.contains("fn tick_node(") && rs.contains("active: &mut Vec<i32>");
    let seq_calls_mut_on_owned = rs.contains("tick_node(")
        && (rs.contains("&mut active") || rs.contains("& mut active"))
        && seq_owned_not_mut;
    let cloned_into_mut = rs.contains("active.clone()") && node_mut_ref;
    assert!(
        !seq_owned_not_mut || !seq_calls_mut_on_owned,
        "WDB-396 RED: tick_seq owned non-mut Vec + &mut pass into tick_node:\n{rs}"
    );
    assert!(
        !cloned_into_mut,
        "WDB-396 RED: clone into mut-borrow BT sibling discards mutations:\n{rs}"
    );
    assert!(
        rs.contains("active: &mut Vec<i32>") || rs.contains("mut active: Vec<i32>"),
        "WDB-396 RED: SCC Vec formals must agree as &mut Vec or mut owned:\n{rs}"
    );
    test.cargo_check().expect("WDB-396 cargo-check");
}

#[test]
fn wdb396_tip_out_game_core_bt_executor_must_agree_mut() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let game = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammer-game/windjammer-game-core");
    let paths = [
        tip.join("behavior_tree/executor.rs"),
        tip.join("executor.rs"),
        game.join("gen/behavior_tree/executor.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("product");
        let seq_owned = text.contains("active: Vec<i32>") && !text.contains("mut active: Vec<i32>");
        let node_mut = text.contains("active: &mut Vec<i32>");
        if seq_owned && node_mut {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-396: BT executor product files missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-396 RED: tip/product BT Vec mut disagreement in:\n  {}",
        bad_paths.join("\n  ")
    );
}
