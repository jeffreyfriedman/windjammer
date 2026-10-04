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

//! P3.646: `HashMap`/`Map::values()` yields `&usize`. Summing into a `usize`
//! accumulator must `*count`, not `count as usize` (E0606).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
use std::map::Map

pub struct Bus {
    counts: Map<string, usize>,
}

impl Bus {
    pub fn listener_count(self) -> usize {
        let mut total: usize = 0usize
        for count in self.counts.values() {
            total = total + count
        }
        total
    }
}
"#;

#[test]
fn hashmap_values_copy_elem_must_deref_into_usize_add() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs").clone();
    eprintln!("P3.646 emit:\n{rs}");
    assert!(
        !rs.contains("count as usize"),
        "P3.646 RED: must not cast &usize via `as usize`:\n{rs}"
    );
    let ok = rs.contains("*count")
        || rs.lines().any(|l| l.contains("total") && l.contains("*count"));
    assert!(
        ok,
        "P3.646 RED: HashMap values() &usize into usize add must deref:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
