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

//! an owned struct formal → E0308 (`expected DenseCsr, found usize`).
//!
//!   `drain((DenseCsr { n: 3_u64 }) as usize)`
//!
//! Expected: `drain(DenseCsr { n: 3_u64 })` with no cast.

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

pub fn drain(csr: DenseCsr) -> u64 {
    take_in_edges(csr)
}

pub fn cap() -> u64 {
    drain(DenseCsr { n: 3 })
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("csr.wj", CSR);
    test.add_file("run.wj", RUN);
    test
}

#[test]
fn module_file_struct_literal_arg_must_not_cast_as_usize() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed (codegen may still be wrong)");
    let run_rs = map.get("run.rs").expect("run.rs");

    let bad_cast = run_rs.contains("as usize")
        && (run_rs.contains("DenseCsr {") || run_rs.contains("DenseCsr{"));
    let good = run_rs.contains("drain(DenseCsr {")
        || run_rs.contains("drain(DenseCsr{")
        || (run_rs.contains("cap()") && run_rs.contains("DenseCsr") && !run_rs.contains("as usize"));

    eprintln!("run.rs:\n{run_rs}");
    eprintln!("bad_cast={bad_cast} good={good}");

    if bad_cast {
        panic!(
            "RED: struct literal into owned DenseCsr formal must not emit `as usize`. \
             "
        );
    }

    test.cargo_check().expect(
        "RED: DenseCsr struct literal arg must cargo-check without `as usize` cast.",
    );

    assert!(good || !bad_cast, "expected clean DenseCsr literal call");
}
