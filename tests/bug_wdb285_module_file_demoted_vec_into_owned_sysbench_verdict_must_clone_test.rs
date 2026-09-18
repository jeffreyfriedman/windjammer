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

//! WDB-285: demoted `&samples` into owned `sysbench_opt_workload_verdict` must clone.
//!
//! Twin of WDB-241; inverse of WDB-185 (gen demoted samples). Tip-out still emits:
//!   `sysbench_opt_workload_verdict(…, &samples)` while formal is `samples: Vec<u64>` → E0308.

use std::path::PathBuf;

#[test]
fn wdb285_tip_out_sysbench_must_clone_ref_vec_into_owned_workload_verdict() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("sysbench_opt_port.rs"),
        gen.join("graph/sysbench_opt_port.rs"),
        gen.join("relational/sysbench_opt_port.rs"),
    ];
    let mut owned = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("sysbench");
        if text.contains("fn sysbench_opt_workload_verdict(workload_id: u32, samples: Vec<u64>") {
            owned = true;
            break;
        }
    }
    assert!(owned, "WDB-285: owned Vec sysbench_opt_workload_verdict formal missing");

    let engine_paths = if tip.join("sysbench_opt_port.rs").exists() {
        vec![tip.join("sysbench_opt_port.rs")]
    } else {
        vec![gen.join("graph/sysbench_opt_port.rs")]
    };
    let mut saw = false;
    for path in &engine_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("sysbench");
        let bad = text.contains("sysbench_opt_workload_verdict(") && text.contains(", &samples)");
        eprintln!("WDB-285 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-285 RED: tip-out/product passes &Vec into owned sysbench_opt_workload_verdict. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-285: sysbench_opt_port missing");
}
