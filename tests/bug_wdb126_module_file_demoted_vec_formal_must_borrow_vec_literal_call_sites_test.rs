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

//! WDB-126: multipass demotes `Vec<i64>` formal to `&Vec<i64>` but call sites pass
//! owned `vec![…]` literals → E0308 (`expected &Vec<i64>, found Vec<i64>`).
//!
//! WindjammerDB CQ-C5 (~54× `&Vec`←`Vec`):
//!   `ecs_soa_spawn_entity(arch, vec![0, 10])` while formal is `&Vec<i64>`
//!
//! Expected: `&vec![0, 10]` (or keep owned formal). Related to WDB-124 (clone path);
//! this gate is **literal** `vec![]` into demoted `&Vec`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod soa
pub mod query
"#;

const SOA: &str = r#"
pub struct Archetype {
    pub entity_count: u64,
}

pub fn spawn_entity(arch: Archetype, component_values: Vec<i64>) -> Archetype {
    let mut next = arch
    next.entity_count = next.entity_count + (component_values.len() as u64)
    next
}
"#;

const QUERY: &str = r#"
use crate::soa::Archetype
use crate::soa::spawn_entity

/// Two spawn calls with lit vecs — demotes `component_values` to `&Vec<i64>` in multipass.
pub fn seed(arch: Archetype) -> Archetype {
    let a = spawn_entity(arch, vec![0, 10])
    spawn_entity(a, vec![1, 50])
}

pub fn cap() -> Archetype {
    seed(Archetype { entity_count: 0 })
}
"#;

fn wdb126_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("soa.wj", SOA);
    test.add_file("query.wj", QUERY);
    test
}

#[test]
fn wdb126_module_file_demoted_vec_formal_must_borrow_vec_literal_call_sites() {
    let mut test = wdb126_fixture();
    let map = test
        .compile()
        .expect("WDB-126 multipass compile should succeed (codegen may still be wrong)");
    let soa_rs = map.get("soa.rs").expect("soa.rs");
    let query_rs = map.get("query.rs").expect("query.rs");

    let demoted = soa_rs.contains("component_values: &Vec<i64>")
        || soa_rs.contains("component_values: &Vec <i64>");
    let borrows = query_rs.contains("spawn_entity(arch, &vec![")
        || query_rs.contains("spawn_entity(a, &vec![")
        || query_rs.contains("&vec![0")
        || query_rs.contains("&vec![1");
    let bad_owned_lit = (query_rs.contains("spawn_entity(arch, vec![")
        || query_rs.contains("spawn_entity(a, vec!["))
        && !borrows;

    if demoted && bad_owned_lit {
        eprintln!("WDB-126 RED soa.rs:\n{soa_rs}\nquery.rs:\n{query_rs}");
    }

    test.cargo_check().expect(
        "WDB-126 RED: demoted &Vec<i64> + owned vec![…] call sites must borrow. Product: ecs_query_port (~54 wdb-layers E0308).",
    );
}
