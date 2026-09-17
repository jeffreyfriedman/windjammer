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

//! WDB-236: `HashMap<String, _>::contains_key` / `get` require `&Q` but tip-out
//! emits owned `String` key → E0308 (`expected &_`, found `String`).
//!
//! Twin of WDB-131 (i64 keys). Product residual (~5× &_←String) tip-out/gen
//! lsqb_typed_graph:
//!   `next.contains_key(key)` / `next.get(key)` / `graph.out_adj.get(key)`
//! Expected: `contains_key(&key)` / `get(&key)`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod adj
pub mod use_adj
"#;

const ADJ: &str = r#"
use std::collections::HashMap

pub fn push_adj(map: HashMap<string, Vec<int>>, key: string, neighbor: int) -> HashMap<string, Vec<int>> {
    let mut next = map
    if next.contains_key(key) {
        if let Some(neighbors) = next.get(key) {
            let mut updated = neighbors
            updated.push(neighbor)
            next.insert(key, updated)
            return next
        }
    }
    let mut neighbors = Vec::new()
    neighbors.push(neighbor)
    next.insert(key, neighbors)
    next
}
"#;

const USE_ADJ: &str = r#"
use crate::adj::push_adj

pub fn seed() -> int {
    let map: HashMap<string, Vec<int>> = HashMap::new()
    let next = push_adj(map, "out:Person", 7)
    next.len() as int
}
"#;

fn wdb236_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("adj.wj", ADJ);
    test.add_file("use_adj.wj", USE_ADJ);
    test
}

#[test]
fn wdb236_codegen_hashmap_string_contains_key_get_must_borrow_key() {
    let test = wdb236_fixture();
    let map = test
        .compile()
        .expect("WDB-236 multipass compile should succeed (codegen may still be wrong)");
    let adj_rs = map.get("adj.rs").expect("adj.rs");

    let borrows = adj_rs.contains("contains_key(&key)")
        || adj_rs.contains("contains_key(& key)")
        || adj_rs.contains("get(&key)")
        || adj_rs.contains("get(& key)");
    let bad_contains = adj_rs.contains("contains_key(key)") && !adj_rs.contains("contains_key(&key)");
    let bad_get = adj_rs.contains(".get(key)") && !adj_rs.contains(".get(&key)");
    let bad = bad_contains || bad_get;

    eprintln!("WDB-236 adj.rs:\n{adj_rs}");
    eprintln!("borrows={borrows} bad_contains={bad_contains} bad_get={bad_get}");

    if bad {
        panic!(
            "WDB-236 RED: HashMap<String,_>::contains_key/get must borrow key. \
             Product: lsqb_typed_graph tip-out (~5× &_←String)."
        );
    }

    test.cargo_check().expect(
        "WDB-236 RED: contains_key(&key)/get(&key) must cargo-check. Product: lsqb_typed_graph.",
    );
}

#[test]
fn wdb236_tip_out_lsqb_must_borrow_string_keys_into_hashmap_get() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    // Prefer tip-out when present — gen lag must not poison tip truth.
    let tip_path = tip.join("lsqb_typed_graph.rs");
    let paths = if tip_path.exists() {
        vec![tip_path]
    } else {
        vec![gen.join("graph/lsqb_typed_graph.rs")]
    };
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("typed");
        let bad_contains = text.contains("contains_key(key)")
            && !text.contains("contains_key(&key)")
            && !text.contains("contains_key(& key)");
        let bad_get = (text.contains(".get(key)") || text.contains("get(key)"))
            && !text.contains(".get(&key)")
            && !text.contains("get(&key)");
        let bad = bad_contains || bad_get;
        eprintln!(
            "WDB-236 tip bad_contains={} bad_get={} bad={} path={}",
            bad_contains,
            bad_get,
            bad,
            path.display()
        );
        assert!(
            !bad,
            "WDB-236 RED: tip-out/product passes owned String into HashMap get/contains_key. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-236: lsqb_typed_graph missing");
}
