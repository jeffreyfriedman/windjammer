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

//! WDB-134: `HashMap<i64, _>::get` requires `&Q` but multipass/product emit
//! `get(label)` with owned `i64` → E0308 (`expected &i64, found i64`).
//!
//! WindjammerDB CQ-C5 (`gen/graph/graph_vertex_map.rs`):
//!   `self.inner.get(label)` after tip sync still owned in product until dogfood.
//!
//! Sibling of WDB-131 (`contains_key`). Field-get gate exists elsewhere; this is
//! the multipass module-file product class for `VertexMap` helpers.

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

impl VertexMap {
    pub fn get_label(self, label: i64) -> Option<i64> {
        match self.inner.get(label) {
            Some(v) => Some(*v),
            None => None,
        }
    }
}
"#;

const USE_MAP: &str = r#"
use crate::vmap::VertexMap

pub fn peek(map: VertexMap, label: i64) -> i64 {
    match map.get_label(label) {
        Some(v) => v,
        None => 0,
    }
}

pub fn cap() -> i64 {
    peek(VertexMap { inner: HashMap::new() }, 7)
}
"#;

fn wdb134_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("vmap.wj", VMAP);
    test.add_file("use_map.wj", USE_MAP);
    test
}

#[test]
fn wdb134_module_file_hashmap_i64_get_must_borrow_key() {
    let test = wdb134_fixture();
    let map = test
        .compile()
        .expect("WDB-134 multipass compile should succeed (codegen may still be wrong)");
    let vmap_rs = map.get("vmap.rs").expect("vmap.rs");

    let borrows = vmap_rs.contains("get(&label)") || vmap_rs.contains("get(& label)");
    let bad = vmap_rs.contains(".get(label)") && !borrows;

    eprintln!("WDB-134 vmap.rs:\n{vmap_rs}");
    eprintln!("borrows={borrows} bad={bad}");

    if bad {
        panic!(
            "WDB-134 RED: HashMap<i64,_>::get must borrow key. \
             Product: graph_vertex_map.rs self.inner.get(label)."
        );
    }

    test.cargo_check().expect(
        "WDB-134 RED: get(&label) must cargo-check. Product: graph_vertex_map.",
    );
}
