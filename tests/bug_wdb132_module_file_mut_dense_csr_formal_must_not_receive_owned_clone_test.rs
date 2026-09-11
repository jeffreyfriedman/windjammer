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

//! WDB-132: multipass demotes `DenseCsr` formal to `&mut DenseCsr` but call sites still
//! pass `csr.clone()` (owned) → E0308 (`expected &mut DenseCsr, found DenseCsr`).
//!
//! WindjammerDB CQ-C5:
//!   `graph_dense_csr_take_in_edges(csr.clone())` while formal is `&mut DenseCsr`
//!   `graph_pagerank_run_dense(csr.clone(), …)` while formal is `&mut DenseCsr`
//!
//! Distinct from WDB-125 (`&T` + clone → `&clone`). Expected: `&mut csr` (not owned clone).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod csr
pub mod run
"#;

const CSR: &str = r#"
pub struct DenseCsr {
    pub n: u64,
}

pub fn take_in_edges(csr: DenseCsr) -> u64 {
    csr.n
}
"#;

const RUN: &str = r#"
use crate::csr::DenseCsr
use crate::csr::take_in_edges

/// Mutating reuse across calls — tip may demote `take_in_edges` to `&mut DenseCsr`.
pub fn drain(csr: DenseCsr) -> u64 {
    let a = take_in_edges(csr)
    let b = take_in_edges(csr)
    a + b
}

pub fn cap() -> u64 {
    drain(DenseCsr { n: 3 })
}
"#;

fn wdb132_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("csr.wj", CSR);
    test.add_file("run.wj", RUN);
    test
}

#[test]
fn wdb132_module_file_mut_dense_csr_formal_must_not_receive_owned_clone() {
    let test = wdb132_fixture();
    let map = test
        .compile()
        .expect("WDB-132 multipass compile should succeed (codegen may still be wrong)");
    let csr_rs = map.get("csr.rs").expect("csr.rs");
    let run_rs = map.get("run.rs").expect("run.rs");

    let demoted_mut = csr_rs.contains("csr: &mut DenseCsr")
        || csr_rs.contains("csr: & mut DenseCsr");
    let bad_owned_clone = run_rs.contains("take_in_edges(csr.clone())")
        || run_rs.contains("take_in_edges(csr.clone ()");
    let mut_borrow = run_rs.contains("take_in_edges(&mut csr)")
        || run_rs.contains("take_in_edges(& mut csr)");

    eprintln!("WDB-132 csr.rs:\n{csr_rs}\nrun.rs:\n{run_rs}");
    eprintln!("demoted_mut={demoted_mut} bad_owned_clone={bad_owned_clone} mut_borrow={mut_borrow}");

    if demoted_mut && bad_owned_clone && !mut_borrow {
        panic!(
            "WDB-132 RED: &mut DenseCsr formal must not receive csr.clone(). \
             Product: graph_dense_csr_take_in_edges / graph_pagerank_run_dense."
        );
    }

    test.cargo_check().expect(
        "WDB-132: &mut DenseCsr call sites must cargo-check (use &mut csr, not owned clone).",
    );
}
