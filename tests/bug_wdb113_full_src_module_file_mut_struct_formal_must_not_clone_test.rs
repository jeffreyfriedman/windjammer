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
//! Per-file isolate + dogfood kept owned `DenseCsr` formals; full multipass mis-emits ~193 graph E0308.
//!
//! Gate: multipass with `&mut` batch param + owned-struct callee must cargo-check (borrow or move, not clone).

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

const BATCH_CALLER: &str = r#"
use crate::csr_types::DenseCsr
use crate::take_out::take_out_edges

pub fn run_batch(csr: DenseCsr) -> i64 {
    let out = take_out_edges(csr)
    out.offsets.len() as i64
}
"#;

const BATCH_MUT_CALLER: &str = r#"
use crate::csr_types::DenseCsr
use crate::take_out::take_out_edges

pub fn run_batch_mut(csr: DenseCsr) -> i64 {
    let mut csr = csr
    let out = take_out_edges(csr)
    csr.vertex_ids.len() as i64 + out.offsets.len() as i64
}
"#;

#[test]
fn wdb113_full_library_multipass_mut_struct_formal_must_not_clone_owned_at_call_site() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod csr_types
pub mod take_out
pub mod batch
pub mod batch_mut
"#,
    );
    test.add_file("csr_types.wj", CSR_TYPE);
    test.add_file("take_out.wj", TAKE_OUT);
    test.add_file("batch.wj", BATCH_CALLER);
    test.add_file("batch_mut.wj", BATCH_MUT_CALLER);

    let map = test
        .compile()
        .expect("WDB-113 multipass compile should succeed");
    let take_out = map.get("take_out.rs").expect("take_out.rs");
    let batch_mut = map.get("batch_mut.rs").expect("batch_mut.rs");

    if take_out.contains("csr: &mut DenseCsr") {
        assert!(
            !batch_mut.contains("take_out_edges(csr.clone())"),
            "WDB-113: &mut formal must not receive owned clone at call site. Got:\n{batch_mut}"
        );
        assert!(
            batch_mut.contains("take_out_edges(&mut csr)")
                || batch_mut.contains("take_out_edges(csr)")
                || batch_mut.contains("take_out_edges(&csr)"),
            "WDB-113: call site must borrow or move into &mut formal. Got:\n{batch_mut}"
        );
    }
}
