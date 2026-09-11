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

//! WDB-131: `HashMap<i64, _>::contains_key` requires `&Q` but multipass emits
//! `contains_key(label)` with owned `i64` → E0308 (`expected &i64, found i64`).
//!
//! WindjammerDB CQ-C5 (`gen/graph/graph_vertex_map.rs` after tip cluster sync):
//!   `if self.inner.contains_key(label) { … }`
//!
//! Distinct from WDB-101 (borrowed getter call site). Expected: `contains_key(&label)`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod vmap
pub mod use_map
"#;

const VMAP: &str = r#"
use std::collections::HashMap

pub struct VertexMap {
    pub inner: HashMap<i64, i64>,
}

pub fn contains_label(map: VertexMap, label: i64) -> bool {
    map.inner.contains_key(label)
}
"#;

const USE_MAP: &str = r#"
use crate::vmap::VertexMap
use crate::vmap::contains_label

pub fn has(map: VertexMap, label: i64) -> bool {
    contains_label(map, label)
}

pub fn cap() -> bool {
    has(VertexMap { inner: HashMap::new() }, 7)
}
"#;

fn wdb131_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("vmap.wj", VMAP);
    test.add_file("use_map.wj", USE_MAP);
    test
}

#[test]
fn wdb131_module_file_hashmap_i64_contains_key_must_borrow_key() {
    let test = wdb131_fixture();
    let map = test
        .compile()
        .expect("WDB-131 multipass compile should succeed (codegen may still be wrong)");
    let vmap_rs = map.get("vmap.rs").expect("vmap.rs");

    let borrows = vmap_rs.contains("contains_key(&label)")
        || vmap_rs.contains("contains_key(& label)");
    let bad = vmap_rs.contains("contains_key(label)") && !borrows;

    eprintln!("WDB-131 vmap.rs:\n{vmap_rs}");
    eprintln!("borrows={borrows} bad={bad}");

    if bad {
        panic!(
            "WDB-131 RED: HashMap<i64,_>::contains_key must borrow key. \
             Product: graph_vertex_map.rs after tip cluster sync (~13 wdb-layers E0308)."
        );
    }

    test.cargo_check().expect(
        "WDB-131 RED: contains_key(&label) must cargo-check. Product: graph_vertex_map.",
    );
}
