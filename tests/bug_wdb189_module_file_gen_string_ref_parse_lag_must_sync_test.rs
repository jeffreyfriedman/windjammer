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

//! WDB-189: product **gen** `&String` into owned `parse_i64` must clone/move (tip-out lag).
//!
//! WDB-186 tip-out is GREEN, but gen `observability_job_store_port` still calls
//! `relational_sql_parse_i64(&status_text)` → ~4× String←&String. Sync tip-out → gen.

use std::path::PathBuf;

#[test]
fn wdb189_product_gen_job_store_must_not_borrow_string_into_owned_parse() {
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let job = gen.join("observability/observability_job_store_port.rs");
    if !job.exists() {
        eprintln!("WDB-189: skip — gen job_store missing");
        return;
    }
    let job_text = std::fs::read_to_string(&job).expect("job");
    let bad = job_text.contains("relational_sql_parse_i64(&status_text)")
        || job_text.contains("relational_sql_parse_i64(&priority_text)")
        || job_text.contains("relational_sql_parse_i64(&attempt_text)")
        || job_text.contains("relational_sql_parse_i64(&heartbeat_text)");
    eprintln!("WDB-189 product gen bad={} path={}", bad, job.display());
    assert!(
        !bad,
        "WDB-189 RED: product gen still passes &String into owned parse_i64 (tip-out green). {}",
        job.display()
    );
}
