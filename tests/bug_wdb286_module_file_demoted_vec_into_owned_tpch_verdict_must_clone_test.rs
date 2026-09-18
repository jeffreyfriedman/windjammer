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

//! WDB-286: demoted `&samples` into owned `tpch_opt_query_verdict` must clone.
//!
//! Twin of WDB-285/241; inverse of WDB-185. Tip-out still emits:
//!   `tpch_opt_query_verdict(…, &samples)` while formal is `samples: Vec<u64>` → E0308.

use std::path::PathBuf;

#[test]
fn wdb286_tip_out_tpch_must_clone_ref_vec_into_owned_query_verdict() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("tpch_opt_port.rs"),
        gen.join("graph/tpch_opt_port.rs"),
    ];
    let mut owned = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("tpch");
        if text.contains("fn tpch_opt_query_verdict(query_id: u32, samples: Vec<u64>") {
            owned = true;
            break;
        }
    }
    assert!(owned, "WDB-286: owned Vec tpch_opt_query_verdict formal missing");

    let engine_paths = if tip.join("tpch_opt_port.rs").exists() {
        vec![tip.join("tpch_opt_port.rs")]
    } else {
        vec![gen.join("graph/tpch_opt_port.rs")]
    };
    let mut saw = false;
    for path in &engine_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("tpch");
        let bad = text.contains("tpch_opt_query_verdict(") && text.contains(", &samples)");
        eprintln!("WDB-286 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-286 RED: tip-out/product passes &Vec into owned tpch_opt_query_verdict. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-286: tpch_opt_port missing");
}
