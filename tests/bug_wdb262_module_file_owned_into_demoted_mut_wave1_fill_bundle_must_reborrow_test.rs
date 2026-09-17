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

//! WDB-262: owned / `&mut x.clone()` into demoted `&mut OptOperatorFill` fill_bundle must reborrow.
//!
//! Gen lag vs tip-out wave1_opt_live_port: tip keeps owned formals + owned clones (OK),
//! but gen demotes to `&mut OptOperatorFill` while still emitting:
//!   `fill_bundle_from_six(&mut u.clone(), …)` and/or `fill_bundle_from_six(q1.clone(), …)`
//! → E0308 / temp mut-ref. Twin of WDB-257 (init_scores `&mut vertices.clone()`).
//! Signature-driven: pass `&mut q1` / `&mut u` when formals are `&mut`.

use std::path::PathBuf;

#[test]
fn wdb262_tip_out_wave1_must_reborrow_into_demoted_mut_fill_bundle() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("wave1_opt_live_port.rs"),
        gen.join("relational_module_file/wave1_opt_live_port.rs"),
        gen.join("relational/wave1_opt_live_port.rs"),
    ];
    let mut saw = false;
    let mut any_bad = false;
    let mut bad_path = String::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("wave1");
        let demoted = text.contains("fn fill_bundle_from_six(q1: &mut OptOperatorFill");
        let bad = demoted
            && (text.contains("fill_bundle_from_six(&mut u.clone()")
                || text.contains("fill_bundle_from_six(q1.clone(), q6.clone(), q12.clone(),"));
        eprintln!(
            "WDB-262 demoted={} bad={} path={}",
            demoted,
            bad,
            path.display()
        );
        if bad {
            any_bad = true;
            bad_path = path.display().to_string();
        }
    }
    assert!(saw, "WDB-262: wave1_opt_live_port missing");
    assert!(
        !any_bad,
        "WDB-262 RED: tip-out/product passes owned/&mut.clone into demoted &mut fill_bundle. {}",
        bad_path
    );
}
