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

//! WDB-224: demoted `&Vec<T>` into owned `Vec<T>` formal must clone (struct element Vec).
//!
//! Product residual (~54× Vec←&Vec), tip-out/gen document_dremel_query_port:
//!   `document_dremel_query_fields_by_ordinal(fields: Vec<DremelEncodedField>, …)`
//!   called with `&fields` → E0308 expected `Vec<_>`, found `&Vec<_>`.
//! Generalizes WDB-175 (`Vec<u8>`) to non-Copy struct element Vec. Signature-driven.

#[path = "common/test_utils.rs"]
mod test_utils;

const FIXTURE: &str = include_str!(
    "fixtures/library_multipass/wdb224_demoted_struct_vec_into_owned_must_clone.wj"
);

#[test]
fn wdb224_codegen_demoted_struct_vec_into_owned_must_clone() {
    let (rs, ok) = test_utils::compile_single_check(FIXTURE);
    let callee_owned = rs.contains("fn by_ordinal(fields: Vec<Field>")
        || rs.contains("by_ordinal(fields: Vec<Field>");
    let caller_has_amp = rs.contains("by_ordinal(&fields");
    let caller_clones = rs.contains("by_ordinal(fields.clone()")
        || rs.contains("by_ordinal((*fields).clone()")
        || rs.contains("by_ordinal(fields.to_vec()");
    if callee_owned && caller_has_amp && !caller_clones {
        panic!(
            "WDB-224 RED: demoted &Vec<Field> passed into owned by_ordinal without clone. Generated:\n{rs}"
        );
    }
    if callee_owned {
        assert!(
            !caller_has_amp || caller_clones,
            "WDB-224: demoted &Vec into owned Vec must clone. Generated:\n{rs}"
        );
    }
    assert!(ok, "WDB-224 fixture must cargo-check. Generated:\n{rs}");
}

use std::path::PathBuf;

#[test]
fn wdb224_tip_out_dremel_must_clone_demoted_fields_into_owned_by_ordinal() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("document_dremel_query_port.rs"),
        gen.join("document/document_dremel_query_port.rs"),
    ];
    let mut saw = false;
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("dremel");
        let owned_formal = text.contains(
            "fn document_dremel_query_fields_by_ordinal(fields: Vec<DremelEncodedField>",
        );
        let bad = text.contains("document_dremel_query_fields_by_ordinal(&fields,")
            && !text.contains("document_dremel_query_fields_by_ordinal(fields.clone(),")
            && !text.contains("document_dremel_query_fields_by_ordinal(fields.to_vec(),");
        eprintln!(
            "WDB-224 owned_formal={} bad={} path={}",
            owned_formal,
            bad,
            path.display()
        );
        if owned_formal {
            assert!(
                !bad,
                "WDB-224 RED: tip-out/product passes &fields into owned Vec formal. {}",
                path.display()
            );
        }
    }
    assert!(saw, "WDB-224: document_dremel_query_port missing");
}
