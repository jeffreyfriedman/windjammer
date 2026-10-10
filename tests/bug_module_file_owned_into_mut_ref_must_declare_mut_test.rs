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

//!
//!   `graph_bfs_run_dense_beamer_parallel(csr: DenseCsr, …)` then
//!   `graph_dense_csr_take_out_edges(&mut csr)` while `csr` is not `mut` → E0596.
//! WJ passes owned `csr` into owned take/restore; the compiler demotes callees to `&mut`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub struct Csr {
    pub n: int,
}

pub fn take_out(csr: Csr) -> int {
    csr.n
}

pub fn restore(csr: Csr, n: int) -> Csr {
    Csr { n: n }
}

pub fn run_parallel(csr: Csr) -> int {
    let out = take_out(csr)
    let _ = restore(csr, out)
    out
}
"#;

#[test]
fn module_file_owned_into_mut_ref_must_declare_mut() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    // If the compiler demotes take/restore to &mut, csr binding/param must be mut
    let demoted = rs.contains("fn take_out(csr: &mut Csr")
        || rs.contains("fn take_out(csr:&mut Csr");
    if demoted {
        let bad = rs.contains("&mut csr")
            && !rs.contains("mut csr:")
            && !rs.contains("let mut csr")
            && !rs.contains("fn run_parallel(mut csr:");
        assert!(
            !bad,
            "RED: demoted &mut formal received non-mut csr binding:\n{rs}"
        );
    }
    test.cargo_check().expect("cargo-check");
}

