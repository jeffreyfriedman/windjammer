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

//! WDB-309: owned binding passed to demoted `&mut` formal must be `mut`.
//!
//! Product tip-out/gen BFS parallel Beamer:
//!   `graph_bfs_run_dense_beamer_parallel(csr: DenseCsr, …)` then
//!   `graph_dense_csr_take_out_edges(&mut csr)` while `csr` is not `mut` → E0596.
//! WJ passes owned `csr` into owned take/restore; tip demotes callees to `&mut`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

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
fn wdb309_module_file_owned_into_mut_ref_must_declare_mut() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-309 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-309 MultiFile lib.rs:\n{rs}");
    // If tip demotes take/restore to &mut, csr binding/param must be mut
    let demoted = rs.contains("fn take_out(csr: &mut Csr")
        || rs.contains("fn take_out(csr:&mut Csr");
    if demoted {
        let bad = rs.contains("&mut csr")
            && !rs.contains("mut csr:")
            && !rs.contains("let mut csr")
            && !rs.contains("fn run_parallel(mut csr:");
        assert!(
            !bad,
            "WDB-309 RED: demoted &mut formal received non-mut csr binding:\n{rs}"
        );
    }
    test.cargo_check().expect("WDB-309 cargo-check");
}

#[test]
fn wdb309_tip_out_bfs_must_declare_mut_csr_for_mut_ref_take() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("graph_bfs_engine.rs"),
        tip.join("graph/graph_bfs_engine.rs"),
        gen.join("graph/graph_bfs_engine.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("bfs");
        // Product: parallel beamer takes &mut csr without mut binding
        let has_mut_ref_call = text.contains("graph_dense_csr_take_out_edges(&mut csr)");
        let fn_owned_param = text.contains("fn graph_bfs_run_dense_beamer_parallel(csr: DenseCsr");
        let mut_param = text.contains("fn graph_bfs_run_dense_beamer_parallel(mut csr: DenseCsr");
        if has_mut_ref_call && fn_owned_param && !mut_param {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-309: tip-out/gen BFS engine missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-309 RED: tip-out/product passes &mut csr without mut param in:\n  {}",
        bad_paths.join("\n  ")
    );
}
