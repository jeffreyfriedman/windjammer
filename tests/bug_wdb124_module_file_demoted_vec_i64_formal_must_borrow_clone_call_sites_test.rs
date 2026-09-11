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

//! WDB-124: multipass demotes read-only `Vec<i64>` formal to `&Vec<i64>`; call sites must borrow.
//!
//! WindjammerDB CQ-C5 product class (~22× when gen stale):
//!   `ldbc_validation_vertex_in_list(csr_vertices.clone(), …)` without `&`.
//! Fresh cargo-wj 0.50.0 multipass already emits `vertex_in_list(&csr.clone(), …)`.
//!
//! Gate: when formal is `&Vec<i64>`, call sites must borrow (regression / product sync check).
//! Related: `bug_cross_crate_vec_helper_must_auto_borrow_test` (string Vec).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod list
pub mod check
"#;

const LIST: &str = r#"
pub fn vertex_in_list(list: Vec<i64>, vertex: i64) -> bool {
    let mut i = 0
    while i < list.len() {
        if list[i] == vertex {
            return true
        }
        i = i + 1
    }
    false
}
"#;

const CHECK: &str = r#"
use crate::list::vertex_in_list

/// Reuse `csr` then pass `.clone()` — demotes formal to `&Vec` in multipass (product LDBC).
pub fn both_in_csr(csr: Vec<i64>, a: i64, b: i64) -> bool {
    let ok_a = vertex_in_list(csr.clone(), a)
    let ok_b = vertex_in_list(csr.clone(), b)
    ok_a && ok_b
}

pub fn cap() -> bool {
    both_in_csr(vec![1, 2, 3], 2, 3)
}
"#;

fn wdb124_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("list.wj", LIST);
    test.add_file("check.wj", CHECK);
    test
}

#[test]
fn wdb124_module_file_demoted_vec_i64_formal_must_borrow_clone_call_sites() {
    let mut test = wdb124_fixture();
    let map = test
        .compile()
        .expect("WDB-124 multipass compile should succeed (codegen may still be wrong)");
    let list_rs = map.get("list.rs").expect("list.rs");
    let check_rs = map.get("check.rs").expect("check.rs");

    let demoted = list_rs.contains("list: &Vec<i64>") || list_rs.contains("list: &Vec <i64>");
    let borrows = check_rs.contains("vertex_in_list(&csr")
        || check_rs.contains("vertex_in_list(&csr.clone()");
    let bad_owned_clone = check_rs.contains("vertex_in_list(csr.clone()") && !borrows;

    if demoted {
        assert!(
            borrows && !bad_owned_clone,
            "WDB-124: demoted &Vec<i64> must borrow at clone call sites. list:\n{list_rs}\ncheck:\n{check_rs}"
        );
    }

    test.cargo_check().expect(
        "WDB-124: demoted &Vec<i64> + clone call sites must cargo-check (borrow). Product: graph_ldbc_validation_engine.",
    );
}
