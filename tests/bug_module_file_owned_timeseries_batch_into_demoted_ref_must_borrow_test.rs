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

//! DB-212: owned `TimeseriesIngestBatch` into demoted `&TimeseriesIngestBatch` must borrow.
//!
//! Product residual, tip-out/gen ops_full_host:
//!   `timeseries_ingest_batch_point_count(batch: &TimeseriesIngestBatch)`
//!   call `point_count(batch)` with owned batch → E0308.
//! Twin of DB-181/203/208.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

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

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("ingest.wj", INGEST);
    test.add_file("host.wj", HOST);
    test
}

#[test]
fn multipass_owned_batch_into_demoted_point_count_must_borrow() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let ingest_rs = map.get("ingest.rs").expect("ingest.rs");
    let host_rs = map.get("host.rs").expect("host.rs");
    eprintln!("ingest.rs:\n{ingest_rs}\nhost.rs:\n{host_rs}");
    let demoted = ingest_rs.contains("batch: &TimeseriesIngestBatch")
        || ingest_rs.contains("batch:&TimeseriesIngestBatch");
    if demoted {
        let bad = host_rs.contains("timeseries_ingest_batch_point_count(batch)")
            && !host_rs.contains("timeseries_ingest_batch_point_count(&batch");
        assert!(
            !bad,
            "owned batch into demoted & formal must borrow. Got:\n{host_rs}"
        );
    }
    test.cargo_check()
        .expect("batch call site must cargo-check");
}
