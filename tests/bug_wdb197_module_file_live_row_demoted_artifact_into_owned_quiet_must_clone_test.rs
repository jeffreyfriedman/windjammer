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

//! WDB-197: demoted `&OptDatedArtifact` in live_row into owned quiet must clone.
//!
//! Product residual (~7× OptDated←&OptDated), tip-out/gen wave1_opt_live_port:
//!   `wave1_opt_live_row_publishable(…, artifact: &OptDatedArtifact)`
//!   body calls `opt_dated_quiet_run_live_publishable(…, artifact)` owned
//! → E0308. Same class as WDB-178 (sysbench claim) for the live_row forwarder.
//! Signature-driven.

use std::path::PathBuf;

#[test]
fn wdb197_tip_out_live_row_must_clone_demoted_artifact_into_owned_quiet() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let live = if tip.join("wave1_opt_live_port.rs").exists() {
        tip.join("wave1_opt_live_port.rs")
    } else {
        gen.join("relational/wave1_opt_live_port.rs")
    };
    let quiet = gen.join("stats/opt_dated_quiet_run_port.rs");
    if !live.exists() || !quiet.exists() {
        eprintln!("WDB-197: skip — live/quiet missing");
        return;
    }
    let live_text = std::fs::read_to_string(&live).expect("live");
    let quiet_text = std::fs::read_to_string(&quiet).expect("quiet");
    let publish_owned = {
        let i = quiet_text
            .find("fn opt_dated_quiet_run_live_publishable")
            .unwrap_or(0);
        let sl = &quiet_text[i..quiet_text.len().min(i + 220)];
        sl.contains("artifact: OptDatedArtifact") && !sl.contains("artifact: &OptDatedArtifact")
    };
    let caller_demoted = live_text.contains(
        "fn wave1_opt_live_row_publishable(claim_ready: bool, wdb_median_nanos: u64, contended: bool, artifact: &OptDatedArtifact",
    );
    let bare = live_text.contains(
        "opt_dated_quiet_run_live_publishable(claim_ready, wdb_median_nanos, contended, artifact)",
    ) && !live_text.contains("artifact.clone()");
    eprintln!(
        "WDB-197 tip-out publish_owned={} demoted={} bare={} path={}",
        publish_owned,
        caller_demoted,
        bare,
        live.display()
    );
    assert!(
        !(publish_owned && caller_demoted && bare),
        "WDB-197 RED: tip-out live_row passes &OptDatedArtifact into owned quiet. {}",
        live.display()
    );
}
