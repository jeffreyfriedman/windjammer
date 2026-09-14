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

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const MOD: &str = r#"
pub mod ingest
pub mod host
"#;

const INGEST: &str = r#"
pub struct TimeseriesIngestBatch {
    pub timestamps: Vec<i64>,
}

/// Read-only probe — tip demotes to `&TimeseriesIngestBatch` (product point_count).
pub fn timeseries_ingest_batch_point_count(batch: TimeseriesIngestBatch) -> usize {
    batch.timestamps.len()
}
"#;

const HOST: &str = r#"
use crate::ingest::TimeseriesIngestBatch
use crate::ingest::timeseries_ingest_batch_point_count

pub fn host_tick() -> usize {
    let batch = TimeseriesIngestBatch { timestamps: Vec::new() }
    timeseries_ingest_batch_point_count(batch)
}
"#;

fn wdb212_fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("ingest.wj", INGEST);
    test.add_file("host.wj", HOST);
    test
}

#[test]
fn wdb212_multipass_owned_batch_into_demoted_point_count_must_borrow() {
    let test = wdb212_fixture();
    let map = test
        .compile()
        .expect("WDB-212 multipass compile should succeed");
    let ingest_rs = map.get("ingest.rs").expect("ingest.rs");
    let host_rs = map.get("host.rs").expect("host.rs");
    eprintln!("WDB-212 ingest.rs:\n{ingest_rs}\nhost.rs:\n{host_rs}");
    let demoted = ingest_rs.contains("batch: &TimeseriesIngestBatch")
        || ingest_rs.contains("batch:&TimeseriesIngestBatch");
    if demoted {
        let bad = host_rs.contains("timeseries_ingest_batch_point_count(batch)")
            && !host_rs.contains("timeseries_ingest_batch_point_count(&batch");
        assert!(
            !bad,
            "WDB-212 RED: owned batch into demoted & formal must borrow. Got:\n{host_rs}"
        );
    }
    test.cargo_check()
        .expect("WDB-212: batch call site must cargo-check");
}

#[test]
fn wdb212_tip_out_ops_full_must_borrow_batch_into_demoted_point_count() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/obs_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let ingest_paths = [
        tip.join("timeseries_ingest_port.rs"),
        gen.join("timeseries/timeseries_ingest_port.rs"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("windjammerdb/crates/wdb-layers/build/timeseries/timeseries_ingest_port.rs"),
    ];
    let batch_demoted = ingest_paths.iter().any(|path| {
        path.exists()
            && std::fs::read_to_string(path)
                .map(|t| {
                    t.contains("fn timeseries_ingest_batch_point_count(batch: &TimeseriesIngestBatch")
                        || t.contains("batch: &TimeseriesIngestBatch) -> usize")
                })
                .unwrap_or(false)
    });
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
        let bad = batch_demoted
            && text.contains("timeseries_ingest_batch_point_count(batch)")
            && !text.contains("timeseries_ingest_batch_point_count(&batch");
        eprintln!(
            "WDB-212 demoted={} bad={} path={}",
            batch_demoted,
            bad,
            path.display()
        );
        assert!(
            !bad,
            "WDB-212 RED: tip-out/product passes owned batch into demoted &TimeseriesIngestBatch. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-212: ops_full_host missing");
}
