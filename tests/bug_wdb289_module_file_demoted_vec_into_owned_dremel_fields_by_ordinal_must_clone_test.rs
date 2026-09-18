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

//! WDB-289: demoted `&fields` into owned `document_dremel_query_fields_by_ordinal` must clone.
//!
//! Twin of WDB-241/285; product tip-out coverage for WDB-136 class. Tip-out still emits:
//!   `document_dremel_query_fields_by_ordinal(&fields, 1_u32)`
//! while formal is `fields: Vec<DremelEncodedField>` → E0308.

use std::path::PathBuf;

#[test]
fn wdb289_tip_out_dremel_must_clone_ref_vec_into_owned_fields_by_ordinal() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("document_dremel_query_port.rs"),
        tip.join("document/document_dremel_query_port.rs"),
        gen.join("document/document_dremel_query_port.rs"),
    ];
    let mut owned = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        let text = std::fs::read_to_string(path).expect("dremel");
        if text.contains(
            "fn document_dremel_query_fields_by_ordinal(fields: Vec<DremelEncodedField>"
        ) {
            owned = true;
            break;
        }
    }
    assert!(
        owned,
        "WDB-289: owned Vec document_dremel_query_fields_by_ordinal formal missing"
    );

    let engine_paths = if tip.join("document_dremel_query_port.rs").exists() {
        vec![tip.join("document_dremel_query_port.rs")]
    } else {
        vec![gen.join("document/document_dremel_query_port.rs")]
    };
    let mut saw = false;
    for path in &engine_paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("dremel");
        let bad = text.contains("document_dremel_query_fields_by_ordinal(&fields");
        eprintln!("WDB-289 bad={} path={}", bad, path.display());
        assert!(
            !bad,
            "WDB-289 RED: tip-out/product passes &Vec into owned document_dremel_query_fields_by_ordinal. {}",
            path.display()
        );
    }
    assert!(saw, "WDB-289: document_dremel_query_port missing");
}
