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
fn module_file_bt_recursive_vec_must_agree_mut() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let seq_owned_not_mut = owned_vec_formal_not_mut(rs, "tick_seq");
    let node_mut_ref = rs.contains("fn tick_node(") && rs.contains("active: &mut Vec<i32>");
    let seq_calls_mut_on_owned = rs.contains("tick_node(")
        && (rs.contains("&mut active") || rs.contains("& mut active"))
        && seq_owned_not_mut;
    let cloned_into_mut = rs.contains("active.clone()") && node_mut_ref;
    assert!(
        !seq_owned_not_mut || !seq_calls_mut_on_owned,
        "tick_seq owned non-mut Vec + &mut pass into tick_node:\n{rs}"
    );
    assert!(
        !cloned_into_mut,
        "clone into mut-borrow BT sibling discards mutations:\n{rs}"
    );
    assert!(
        rs.contains("active: &mut Vec<i32>") || rs.contains("mut active: Vec<i32>"),
        "SCC Vec formals must agree as &mut Vec or mut owned:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
