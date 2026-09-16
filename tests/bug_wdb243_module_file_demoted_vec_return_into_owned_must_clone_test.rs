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

//! WDB-243: demoted `&Vec` return into owned `Vec` must clone (federation scan).
//!
//! Product residual tip-out/gen federation_layer:
//!   `federated_scan_rows(&self, local_rows: &Vec<LogicalRow>) -> Vec<LogicalRow> { local_rows }`
//!   → E0308 expected `Vec<_>`, found `&Vec<_>`.
//! Signature-driven: `local_rows.clone()` / `local_rows.to_vec()`.

use std::path::PathBuf;

#[test]
fn wdb243_tip_out_federation_must_clone_demoted_rows_into_owned_return() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("federation_layer.rs"),
        gen.join("federation/federation_layer.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("fed");
        let demoted_ret = text.contains("local_rows: &Vec<LogicalRow>) -> Vec<LogicalRow>")
            || (text.contains("local_rows: &Vec") && text.contains("-> Vec<LogicalRow>"));
        // Bare `local_rows` as sole return body (not clone/to_vec)
        let bad = demoted_ret
            && (text.contains("-> Vec<LogicalRow> {\n        local_rows\n}")
                || text.contains("-> Vec<LogicalRow> {\n    local_rows\n}")
                || text.contains(") -> Vec<LogicalRow> {\n        local_rows\n}")
                || (text.contains("federated_scan_rows")
                    && text.contains("local_rows: &Vec")
                    && !text.contains("local_rows.clone()")
                    && !text.contains("local_rows.to_vec()")
                    && text.lines().any(|l| l.trim() == "local_rows")));
        eprintln!(
            "WDB-243 demoted_ret={} bad={} path={}",
            demoted_ret,
            bad,
            path.display()
        );
        assert!(
            !bad,
            "WDB-243 RED: tip-out/product returns &Vec local_rows as owned Vec without clone. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-243: federation_layer missing");
}
