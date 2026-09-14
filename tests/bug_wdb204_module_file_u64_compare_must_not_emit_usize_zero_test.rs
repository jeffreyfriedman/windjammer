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

//! WDB-204: `u64` compare must not emit `0_usize` literal.
//!
//! Product residual (~3× u64←usize), tip-out/gen sysbench:
//!   `if median == 0_usize` while `median: u64` → E0308 + E0277.
//! Zero literal must match u64 (`0` / `0_u64`).

use std::path::PathBuf;

#[test]
fn wdb204_tip_out_sysbench_must_not_compare_u64_to_usize_zero() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let sysbench = if tip.join("sysbench_opt_port.rs").exists() {
        tip.join("sysbench_opt_port.rs")
    } else {
        gen.join("relational/sysbench_opt_port.rs")
    };
    if !sysbench.exists() {
        eprintln!("WDB-204: skip — sysbench missing");
        return;
    }
    let text = std::fs::read_to_string(&sysbench).expect("sysbench");
    let bad = text.contains("== 0_usize") || text.contains("0_usize");
    eprintln!("WDB-204 tip-out bad={} path={}", bad, sysbench.display());
    assert!(
        !bad,
        "WDB-204 RED: tip-out/product compares u64 median to 0_usize. {}",
        sysbench.display()
    );
}
