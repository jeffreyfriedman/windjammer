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

//! Regression: module file hashmap string get must borrow key.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

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

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("adj.wj", ADJ);
    test.add_file("use_adj.wj", USE_ADJ);
    test
}

#[test]
fn codegen_hashmap_string_contains_key_get_must_borrow_key() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed (codegen may still be wrong)");
    let adj_rs = map.get("adj.rs").expect("adj.rs");

    let borrows = adj_rs.contains("contains_key(&key)")
        || adj_rs.contains("contains_key(& key)")
        || adj_rs.contains("get(&key)")
        || adj_rs.contains("get(& key)");
    let bad_contains = adj_rs.contains("contains_key(key)") && !adj_rs.contains("contains_key(&key)");
    let bad_get = adj_rs.contains(".get(key)") && !adj_rs.contains(".get(&key)");
    let bad = bad_contains || bad_get;

    eprintln!("adj.rs:\n{adj_rs}");
    eprintln!("borrows={borrows} bad_contains={bad_contains} bad_get={bad_get}");

    if bad {
        panic!(
            "RED: HashMap<String,_>::contains_key/get must borrow key. \
            "
        );
    }

    test.cargo_check().expect(
        "RED: contains_key(&key)/get(&key) must cargo-check.",
    );
}
