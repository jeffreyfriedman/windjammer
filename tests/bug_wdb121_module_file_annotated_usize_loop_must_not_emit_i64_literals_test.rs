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

//! WDB-121: annotated `let mut i: usize = 0` under multipass soa+query still emits `0_i64`.
//!
//! Tip GREEN'd WDB-117 `&mut` demotion. Same soa+query fixture still cargo-checks RED:
//!   `let mut i: usize = 0` → `let mut i: usize = 0_i64`
//!   `while i < 1` / `i = i + 1` → `1_i64`
//!
//! Single-module annotated loops may already unify (false-GREEN). Gate needs soa + query sibling
//! (same multipass shape as WDB-117). Distinct from WDB-119 (unannotated `i = 0` + `Vec::push`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod soa
pub mod query
"#;

const SOA: &str = r#"
pub struct Column {
    pub component_id: u32
    pub values: Vec<i64>
}

pub struct Archetype {
    pub columns: Vec<Column>
}

pub fn get_component(arch: Archetype, component_id: u32, entity_index: usize) -> i64 {
    for col in arch.columns {
        if col.component_id == component_id {
            if entity_index < col.values.len() {
                return col.values[entity_index]
            }
            return 0
        }
    }
    0
}

pub fn cap_arch() -> Archetype {
    let mut cols: Vec<Column> = Vec::new()
    let mut vals: Vec<i64> = Vec::new()
    vals.push(10)
    cols.push(Column { component_id: 1, values: vals })
    Archetype { columns: cols }
}
"#;

const QUERY: &str = r#"
use crate::soa::Archetype
use crate::soa::get_component
use crate::soa::cap_arch

pub fn query_above(min_value: i64) -> i64 {
    let arch = cap_arch()
    let mut component_id: u32 = 1
    let mut i: usize = 0
    let mut total: i64 = 0
    while i < 1 {
        let v = get_component(arch, component_id, i)
        if v > min_value {
            total = total + v
        }
        i = i + 1
    }
    total
}
"#;

fn wdb121_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("soa.wj", SOA);
    test.add_file("query.wj", QUERY);
    test
}

#[test]
fn wdb121_module_file_annotated_usize_loop_must_not_emit_i64_literals() {
    let mut test = wdb121_fixture();
    let map = test
        .compile()
        .expect("WDB-121 multipass compile should succeed (codegen may still be wrong)");
    let query_rs = map.get("query.rs").expect("query.rs must be generated");

    let bad_i64_lit = query_rs.contains("0_i64") || query_rs.contains("1_i64");
    if bad_i64_lit {
        eprintln!("WDB-121 RED emit query.rs:\n{query_rs}");
    }

    test.cargo_check().expect(
        "WDB-121 RED: multipass annotated `let mut i: usize = 0` must emit usize literals (not 0_i64). Surfaced after tip fixed WDB-117 &mut demotion.",
    );
}
