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

//! WDB-323: owned `String` formals must not receive `&vname`/`&lname` (Arrow FFI roundtrip).
//!
//! Product tip-out/gen graph_sql_arrow_ffi_port:
//!   `graph_sql_record_batch_from_arrow(handle.clone(), &vname, &lname)`
//!   with `vertex_id_name: String, label_name: String` → E0308.
//! Twin of WDB-306/310 (owned String call args).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub struct Batch {
    pub vertex_id_name: string,
    pub label_name: string,
}

pub fn from_arrow(vertex_id_name: string, label_name: string) -> Batch {
    Batch {
        vertex_id_name: vertex_id_name,
        label_name: label_name,
    }
}

pub fn roundtrip(batch: Batch) -> Batch {
    let vname = batch.vertex_id_name
    let lname = batch.label_name
    from_arrow(vname, lname)
}
"#;

#[test]
fn wdb323_module_file_owned_string_must_not_receive_ref_names() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-323 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-323 MultiFile lib.rs:\n{rs}");
    let owned = rs.contains("fn from_arrow(vertex_id_name: String")
        || rs.contains("fn from_arrow(mut vertex_id_name: String");
    assert!(
        owned,
        "WDB-323: expected owned String formals on from_arrow:\n{rs}"
    );
    let bad = rs.contains("from_arrow(&vname") || rs.contains("from_arrow(&");
    assert!(
        !bad,
        "WDB-323 RED: owned from_arrow received &vname/&lname:\n{rs}"
    );
    test.cargo_check().expect("WDB-323 cargo-check");
}

#[test]
fn wdb323_tip_out_arrow_ffi_must_not_pass_ref_names_into_owned_string() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("graph_sql_arrow_ffi_port.rs"),
        tip.join("graph/graph_sql_arrow_ffi_port.rs"),
        gen.join("graph/graph_sql_arrow_ffi_port.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("arrow_ffi");
        let bad = text.lines().any(|line| {
            line.contains("graph_sql_record_batch_from_arrow(")
                && (line.contains("&vname") || line.contains("&lname"))
        });
        if bad {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-323: tip-out/gen graph_sql_arrow_ffi_port missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-323 RED: tip-out/product passes &vname/&lname into owned String in:\n  {}",
        bad_paths.join("\n  ")
    );
}
