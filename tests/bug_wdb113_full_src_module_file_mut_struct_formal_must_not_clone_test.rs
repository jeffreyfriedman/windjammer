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

//! WDB-113: full-library `--module-file` demotes owned struct formal to `&mut T` but call sites
//! emit owned `.clone()` (E0308: expected `&mut DenseCsr`, found `DenseCsr`).
//!
//! WindjammerDB Phase 193 `./scripts/build_gen.sh` (full `src` multipass) observed:
//!   `graph_dense_csr_take_out_edges(csr: &mut DenseCsr)` in generated callee
//! while batch engines emit:
//!   `graph_dense_csr_take_out_edges(csr.clone())` → owned move, not `&mut`.
//!
//! Gate A: multipass with demote + clone callers must cargo-check when emit is correct.
//! Gate B: until fixed, demoted `&mut T` + owned `.clone()` call sites fail the emit assertion (RED).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const CSR_TYPE: &str = r#"
pub struct DenseCsr {
    pub vertex_ids: Vec<i64>,
    pub offsets: Vec<i32>,
    pub neighbors: Vec<i32>,
}

pub struct GraphOutEdgesCopy {
    pub offsets: Vec<i32>,
    pub neighbors: Vec<i32>,
}
"#;

const TAKE_OUT: &str = r#"
use crate::csr_types::{DenseCsr, GraphOutEdgesCopy}

pub fn take_out_edges(csr: DenseCsr) -> GraphOutEdgesCopy {
    GraphOutEdgesCopy {
        offsets: csr.offsets,
        neighbors: csr.neighbors,
    }
}
"#;

const BATCH_DEMOTE_CALLER: &str = r#"
use crate::csr_types::DenseCsr
use crate::take_out::take_out_edges

/// Bare `csr` reuse after `mut` binding demotes callee formal in multipass.
pub fn run_batch_mut(csr: DenseCsr) -> i64 {
    let mut csr = csr
    let out = take_out_edges(csr)
    csr.vertex_ids.len() as i64 + out.offsets.len() as i64
}
"#;

const BATCH_CLONE_CALLER: &str = r#"
use crate::csr_types::DenseCsr
use crate::take_out::take_out_edges

pub fn run_batch_clone(csr: DenseCsr) -> i64 {
    let out = take_out_edges(csr.clone())
    out.offsets.len() as i64
}
"#;

fn wdb113_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod csr_types
pub mod take_out
pub mod batch_mut
pub mod batch_clone
"#,
    );
    test.add_file("csr_types.wj", CSR_TYPE);
    test.add_file("take_out.wj", TAKE_OUT);
    test.add_file("batch_mut.wj", BATCH_DEMOTE_CALLER);
    test.add_file("batch_clone.wj", BATCH_CLONE_CALLER);
    test
}

#[test]
fn wdb113_full_library_multipass_mut_struct_formal_must_not_clone_owned_at_call_site() {
    let mut test = wdb113_fixture();
    let map = test
        .compile()
        .expect("WDB-113 multipass compile should succeed");
    let take_out = map.get("take_out.rs").expect("take_out.rs");
    let clone_caller = map.get("batch_clone.rs").expect("batch_clone.rs");

    let demoted = take_out.contains("csr: &mut DenseCsr");
    let bad_clone_emit = clone_caller.contains("take_out_edges(csr.clone())")
        || clone_caller.contains("take_out_edges( csr.clone()");

    if demoted {
        assert!(
            !bad_clone_emit,
            "WDB-113 RED: demoted &mut struct + owned .clone() call sites must borrow. Got:\n{clone_caller}"
        );
        let borrowed = clone_caller.contains("take_out_edges(&mut csr")
            || clone_caller.contains("take_out_edges(& csr")
            || (clone_caller.contains("take_out_edges(csr)")
                && !clone_caller.contains(".clone()"));
        assert!(
            borrowed,
            "WDB-113: when multipass demotes to &mut, .clone() call sites must borrow. Got:\n{clone_caller}"
        );
        test.cargo_check()
            .expect("WDB-113: borrowed call sites must cargo-check");
    } else if bad_clone_emit {
        panic!(
            "WDB-113 RED: explicit .clone() call sites must not mismatch callee formals. callee:\n{take_out}\ncaller:\n{clone_caller}"
        );
    } else {
        assert!(
            take_out.contains("csr: DenseCsr"),
            "WDB-113: owned DenseCsr formal is acceptable when multipass does not demote.\n{take_out}"
        );
        test.cargo_check()
            .expect("WDB-113: owned struct formals must cargo-check");
    }
}
