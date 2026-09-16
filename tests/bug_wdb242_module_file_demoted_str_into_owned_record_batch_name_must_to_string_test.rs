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

//! WDB-242: demoted `&str` `vertex_id_name` into owned `String` record-batch ctor must `.to_string()`.
//!
//! Product residual tip-out/gen graph_sql_arrow_ffi_port (~37× String←&str):
//!   `graph_sql_record_batch_from_ids_labels(vertex_id_name: String, …)`
//!   called with bare `vertex_id_name` while caller formal is `&str` → E0308.
//! Twin of WDB-240 (sql parse_ast). Signature-driven: `.to_string()`.

use std::path::PathBuf;

#[test]
fn wdb242_tip_out_arrow_ffi_must_to_string_demoted_name_into_owned_ids_labels() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let batch_paths = [
        tip.join("graph_sql_record_batch_port.rs"),
        gen.join("graph/graph_sql_record_batch_port.rs"),
    ];
    let mut owned = false;
    for path in &batch_paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("batch");
        if text.contains("fn graph_sql_record_batch_from_ids_labels(vertex_id_name: String") {
            owned = true;
            break;
        }
    }
    assert!(owned, "WDB-242: owned String formal for from_ids_labels missing");

    let paths = [
        tip.join("graph_sql_arrow_ffi_port.rs"),
        gen.join("graph/graph_sql_arrow_ffi_port.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("ffi");
        let has_demoted = text.contains("vertex_id_name: &str");
        let bad = has_demoted
            && text.contains("graph_sql_record_batch_from_ids_labels(vertex_id_name,")
            && !text.contains("graph_sql_record_batch_from_ids_labels(vertex_id_name.to_string(),")
            && !text.contains("graph_sql_record_batch_from_ids_labels(vertex_id_name.to_owned(),");
        eprintln!(
            "WDB-242 has_demoted={} bad={} path={}",
            has_demoted,
            bad,
            path.display()
        );
        assert!(
            !bad,
            "WDB-242 RED: tip-out/product passes &str vertex_id_name into owned from_ids_labels. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-242: graph_sql_arrow_ffi_port missing");
}
