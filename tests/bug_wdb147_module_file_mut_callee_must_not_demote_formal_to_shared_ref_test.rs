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

//! WDB-147: if a formal is only read via shared borrows, tip may demote to `&T`,
//! but if the body also calls an `&mut T` API on the same binding, the formal must
//! stay `&mut T` (or the body must clone-to-mut).
//!
//! WindjammerDB CQ-C5 tip sync of PageRank emitted:
//!   `graph_pagerank_run_dense_pull_fused_seeded(csr: &DenseCsr, …)`
//! while the body calls `graph_dense_csr_take_in_edges(csr)` (`csr: &mut DenseCsr`)
//! → E0308 mutability mismatch / E0596 after dogfood.
//!
//! Expected: formal `&mut DenseCsr` when any callee requires `&mut`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod csr
pub mod algo
"#;

const CSR: &str = r#"
pub struct DenseCsr {
    pub n: u32,
}

pub fn take_scratch(csr: DenseCsr) -> u32 {
    csr.n
}

pub fn restore_scratch(csr: DenseCsr, n: u32) -> DenseCsr {
    DenseCsr { n: n }
}
"#;

const ALGO: &str = r#"
use crate::csr::DenseCsr
use crate::csr::take_scratch
use crate::csr::restore_scratch

/// Body mutates via take/restore — formal must allow &mut (or owned mut).
pub fn run_fused(csr: DenseCsr) -> u32 {
    let n = take_scratch(csr)
    let csr2 = restore_scratch(csr, n)
    csr2.n
}

pub fn cap() -> u32 {
    run_fused(DenseCsr { n: 3 })
}
"#;

fn wdb147_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("csr.wj", CSR);
    test.add_file("algo.wj", ALGO);
    test
}

#[test]
fn wdb147_module_file_mut_callee_must_not_demote_formal_to_shared_ref() {
    let test = wdb147_fixture();
    let map = test
        .compile()
        .expect("WDB-147 multipass compile should succeed (codegen may still be wrong)");
    let algo_rs = map.get("algo.rs").expect("algo.rs");
    let csr_rs = map.get("csr.rs").expect("csr.rs");

    // If tip demotes take_scratch to &mut DenseCsr but run_fused formal to &DenseCsr → RED
    let take_mut = csr_rs.contains("take_scratch(csr: &mut DenseCsr)")
        || csr_rs.contains("fn take_scratch(csr: &mut DenseCsr)");
    let fused_shared = {
        let i = algo_rs.find("fn run_fused").unwrap_or(0);
        let sl = &algo_rs[i..algo_rs.len().min(i + 80)];
        sl.contains("csr: &DenseCsr") && !sl.contains("csr: &mut DenseCsr")
    };

    eprintln!("WDB-147 csr.rs:\n{csr_rs}\nalgo.rs:\n{algo_rs}");
    eprintln!("take_mut={take_mut} fused_shared={fused_shared}");

    if take_mut && fused_shared {
        panic!(
            "WDB-147 RED: formal demoted to &T while callee needs &mut T. \
             Product: graph_pagerank_run_dense_pull_fused_seeded + take_in_edges."
        );
    }

    test.cargo_check().expect(
        "WDB-147: mut-callee / formal demotion must cargo-check.",
    );
}
