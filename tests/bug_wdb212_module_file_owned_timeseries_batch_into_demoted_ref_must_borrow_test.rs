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

//! WDB-212: owned `TimeseriesIngestBatch` into demoted `&TimeseriesIngestBatch` must borrow.
//!
//! Product residual, tip-out/gen ops_full_host:
//!   `timeseries_ingest_batch_point_count(batch: &TimeseriesIngestBatch)`
//!   call `point_count(batch)` with owned batch → E0308.
//! Twin of WDB-181/203/208.

use std::path::PathBuf;

#[test]
fn wdb212_tip_out_ops_full_must_borrow_batch_into_demoted_point_count() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/obs_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("observability_ops_full_host_port.rs"),
        gen.join("observability/observability_ops_full_host_port.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("ops_full");
        let bad = text.contains("timeseries_ingest_batch_point_count(batch)")
            && !text.contains("timeseries_ingest_batch_point_count(&batch)");
        eprintln!("WDB-212 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-212 RED: tip-out/product passes owned batch into demoted &TimeseriesIngestBatch. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-212: ops_full_host missing");
}
