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

//! WDB-216: demoted `&Vec<u32>` into owned FFI `Vec<u32>` must clone.
//!
//! Product residual (~32× Vec<u32>←&Vec), tip-out/gen graph_simd:
//!   wrapper `a: &Vec<u32>` forwards bare `a` into `*_ffi(a: Vec<u32>, …)`.
//! Twin of WDB-195. Signature-driven.

use std::path::PathBuf;

#[test]
fn wdb216_tip_out_simd_must_clone_demoted_vec_into_owned_ffi() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("graph_simd_port.rs"),
        gen.join("graph/graph_simd_port.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("simd");
        let demoted = text.contains("a: &Vec<u32>")
            || text.contains("neighbors: &Vec<u32>")
            || text.contains("inv_deg: &Vec<f64>");
        let ffi_owned = text.contains("graph_simd_u32_sorted_intersection_count_ffi(a: Vec<u32>")
            || text.contains("_ffi(a: Vec<u32>");
        let bare = text.contains("graph_simd_u32_sorted_intersection_count_ffi(a,")
            && !text.contains("graph_simd_u32_sorted_intersection_count_ffi(a.clone(),");
        let bad = demoted && ffi_owned && bare;
        eprintln!(
            "WDB-216 demoted={} ffi_owned={} bare={} bad={} path={}",
            demoted,
            ffi_owned,
            bare,
            bad,
            path.display()
        );
        assert!(
            !bad,
            "WDB-216 RED: tip-out/product forwards &Vec into owned SIMD FFI without clone. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-216: graph_simd missing");
}
