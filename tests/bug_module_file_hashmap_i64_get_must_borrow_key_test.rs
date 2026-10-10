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

//! Regression: module file hashmap i64 get must borrow key.

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

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("vmap.wj", VMAP);
    test.add_file("use_map.wj", USE_MAP);
    test
}

#[test]
fn module_file_hashmap_i64_get_must_borrow_key() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed (codegen may still be wrong)");
    let vmap_rs = map.get("vmap.rs").expect("vmap.rs");

    let borrows = vmap_rs.contains("get(&label)") || vmap_rs.contains("get(& label)");
    let bad = vmap_rs.contains(".get(label)") && !borrows;

    eprintln!("vmap.rs:\n{vmap_rs}");
    eprintln!("borrows={borrows} bad={bad}");

    if bad {
        panic!(
            "RED: HashMap<i64,_>::get must borrow key. \
            "
        );
    }

    test.cargo_check().expect(
        "RED: get(&label) must cargo-check.",
    );
}
