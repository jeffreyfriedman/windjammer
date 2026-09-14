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

//! WDB-208: owned OptDatedBaseline into demoted `&OptDatedBaseline` must borrow.
//!
//! Product residual (~3×), tip-out/gen tpch_opt (module_file lag worse):
//!   `opt_dated_baseline_is_set(b)` / `opt_dated_baseline_is_set(tpch_sf1_dated_baseline())`
//! while formal is `baseline: &OptDatedBaseline`.
//! Twin of WDB-181/203. Signature-driven.

use std::path::PathBuf;

#[test]
fn wdb208_tip_out_tpch_baseline_must_borrow_into_demoted_is_set() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("tpch_opt_port.rs"),
        gen.join("relational/tpch_opt_port.rs"),
        gen.join("relational_module_file/tpch_opt_port.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("tpch");
        // Bad: bare owned call without &
        let bad_b = text.contains("opt_dated_baseline_is_set(b)")
            && !text.contains("opt_dated_baseline_is_set(&b)");
        let bad_fn = text.contains("opt_dated_baseline_is_set(tpch_sf1_dated_baseline())")
            && !text.contains("opt_dated_baseline_is_set(&tpch_sf1_dated_baseline())");
        let bad = bad_b || bad_fn;
        eprintln!(
            "WDB-208 bad_b={} bad_fn={} bad={} path={}",
            bad_b,
            bad_fn,
            bad,
            path.display()
        );
        // Enforce on tip-out (source of truth) and module_file lag
        if path.to_string_lossy().contains("rel_tip_out")
            || path.to_string_lossy().contains("module_file")
        {
            assert!(
                !bad,
                "WDB-208 RED: owned OptDatedBaseline into demoted &baseline without borrow. {}",
                path.display()
            );
        }
    }
    if !saw {
        eprintln!("WDB-208: skip — tpch_opt missing");
    }
}
