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

//! WDB-117: tip `--module-file` demotes Copy `u32`/`usize` formals to `&mut` when a
//! sibling module passes loop counters into a read-only getter (ecs_soa / ecs_query class).
//!
//! Product: cold tip `wj build src --module-file` on `wdb-layers` emits
//!   `ecs_soa_get_component(..., component_id: &mut u32, entity_index: &mut usize)`
//! while Aug 27 `~/.cargo/bin/wj` emits owned `u32`/`usize`. Tip then fails with
//! thousands of E0277/E0308 (`==` / index on `&mut`).
//!
//! Minimal single-file getter stays owned (false-GREEN). Gate needs soa + query sibling.

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

fn wdb117_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("soa.wj", SOA);
    test.add_file("query.wj", QUERY);
    test
}

#[test]
fn wdb117_module_file_copy_int_formals_must_stay_owned_not_mut_ref() {
    let mut test = wdb117_fixture();
    let map = test
        .compile()
        .expect("WDB-117 multipass compile should succeed (codegen may still be wrong)");
    let soa_rs = map.get("soa.rs").expect("soa.rs must be generated");
    let query_rs = map.get("query.rs").expect("query.rs must be generated");

    let mut_ref_params = soa_rs.contains("component_id: &mut u32")
        || soa_rs.contains("entity_index: &mut usize")
        || soa_rs.contains("component_id:&mut u32")
        || soa_rs.contains("entity_index:&mut usize");

    if mut_ref_params {
        eprintln!("WDB-117 RED emit soa.rs:\n{soa_rs}");
        eprintln!("WDB-117 RED emit query.rs:\n{query_rs}");
    }

    assert!(
        !mut_ref_params,
        "WDB-117 RED: Copy u32/usize formals used in == / index must stay owned when siblings pass loop counters. Product: tip ecs_soa_get_component emits &mut; cargo wj emits owned."
    );

    // Tip (2026-09-04): &mut demotion is fixed for this fixture. Remaining cargo-check
    // failure is annotated-usize + i64 literals → WDB-121 (do not conflate with &mut).
    let query_ok_literals = !query_rs.contains("0_i64") && !query_rs.contains("1_i64");
    if !query_ok_literals {
        eprintln!(
            "WDB-117 note: Copy formals owned, but query.rs still has i64 literals (tracked as WDB-121):\n{query_rs}"
        );
    }
}
