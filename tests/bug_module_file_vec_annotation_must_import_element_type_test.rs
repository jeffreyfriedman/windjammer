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

//! Regression: module file vec annotation must import element type.
//!
//! Gate A: multipass module-file test must cargo-check when emit includes `use ...::JobRecord`.
//! Gate B: until fixed, missing import assertion fails (RED).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const JOB_QUEUE: &str = r#"
pub struct JobRecord {
    pub name: string,
    pub status: u32,
}

pub fn queue_cap_demo() -> Vec<JobRecord> {
    let mut jobs: Vec<JobRecord> = Vec::new()
    jobs.push(JobRecord { name: "heartbeat", status: 0 })
    jobs.push(JobRecord { name: "reaper", status: 1 })
    jobs
}
"#;

const JOB_STORE: &str = r#"
use crate::job_queue::JobRecord
use crate::job_queue::queue_cap_demo

pub fn job_store_load() -> (u64, Vec<JobRecord>) {
    (10, queue_cap_demo())
}
"#;

const JOB_STORE_TEST: &str = r#"
use crate::job_store::job_store_load

pub fn test_job_store_load_cap_demo_jobs() {
    let loaded = job_store_load()
    let jobs = loaded.1
    if jobs.len() != 2 {
        panic("Cap demo store must persist two jobs")
    }
}
"#;

fn fixture() -> MultiFileTest {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", "pub mod job_queue\npub mod job_store\npub mod job_store_test");
    test.add_file("job_queue.wj", JOB_QUEUE);
    test.add_file("job_store.wj", JOB_STORE);
    test.add_file("job_store_test.wj", JOB_STORE_TEST);
    test
}

#[test]
fn module_file_vec_type_annotation_must_import_element_type() {
    let test = fixture();
    let map = test
        .compile()
        .expect("multipass compile should succeed");
    let test_rs = map
        .get("job_store_test.rs")
        .expect("job_store_test.rs must be generated");
    assert!(
        test_rs.contains("Vec<JobRecord>"),
        "generated test must annotate Vec<JobRecord> from tuple field access"
    );
    assert!(
        test_rs.contains("use crate::job_queue::JobRecord")
            || test_rs.contains("use super::job_queue::JobRecord"),
        "generated test must import JobRecord when emitting Vec<JobRecord> annotation; got:\n{}",
        test_rs
    );
    test.cargo_check().expect(
        "generated module-file test must cargo-check when JobRecord is imported",
    );
}
