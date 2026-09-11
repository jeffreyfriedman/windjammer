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
))]

//! WDB-138: method `put(vertex: i64, …)` receives `&(ids[i])` → E0308
//! (`expected i64, found &i64`).
//!
//! WindjammerDB CQ-C5 (`graph_dense_csr.rs` / `graph_pagerank_engine.rs` /
//! `graph_sssp_engine.rs`):
//!   `map.put(&(csr.vertex_ids[i as usize]), …)` while `put` takes owned `i64`
//!
//! Tip must pass Copy index values by value (no `&` around the index expr).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod vmap
pub mod fill
"#;

const VMAP: &str = r#"
pub struct VertexI64Map {
    pub keys: Vec<i64>,
    pub vals: Vec<i64>,
}

impl VertexI64Map {
    pub fn new() -> VertexI64Map {
        VertexI64Map {
            keys: Vec::new(),
            vals: Vec::new(),
        }
    }

    pub fn put(self, vertex: i64, value: i64) {
        self.keys.push(vertex)
        self.vals.push(value)
    }
}
"#;

const FILL: &str = r#"
use crate::vmap::VertexI64Map

pub fn fill_from_ids(ids: Vec<i64>, scores: Vec<i64>) -> VertexI64Map {
    let mut map = VertexI64Map::new()
    let mut i: i64 = 0
    while i < (ids.len() as i64) {
        map.put(ids[i], scores[i])
        i = i + 1
    }
    map
}

pub fn cap() -> i64 {
    let m = fill_from_ids(vec![10, 20], vec![1, 2])
    m.keys.len() as i64
}
"#;

fn wdb138_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("vmap.wj", VMAP);
    test.add_file("fill.wj", FILL);
    test
}

#[test]
fn wdb138_module_file_map_put_i64_key_must_not_borrow_index_expr() {
    let test = wdb138_fixture();
    let map = test
        .compile()
        .expect("WDB-138 multipass compile should succeed (codegen may still be wrong)");
    let fill_rs = map.get("fill.rs").expect("fill.rs");
    let vmap_rs = map.get("vmap.rs").expect("vmap.rs");

    let formal_owned = vmap_rs.contains("vertex: i64") && !vmap_rs.contains("vertex: &i64");
    let bad = fill_rs.contains("put(&(ids[")
        || fill_rs.contains("put(&(scores[")
        || fill_rs.contains(".put(&(ids");
    let good = fill_rs.contains("put(ids[") || fill_rs.contains(".put(ids[");

    eprintln!("WDB-138 vmap.rs:\n{vmap_rs}\nfill.rs:\n{fill_rs}");
    eprintln!("formal_owned={formal_owned} bad={bad} good={good}");

    if formal_owned && bad {
        panic!(
            "WDB-138 RED: map.put owned i64 key must not wrap index expr in &. \
             Product: graph_dense_csr / pagerank / sssp map.put(&(csr.vertex_ids[i]))."
        );
    }

    test.cargo_check().expect(
        "WDB-138: Copy i64 index into owned put key must cargo-check.",
    );
}
