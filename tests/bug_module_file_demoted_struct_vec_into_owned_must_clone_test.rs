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

//!
//!   `document_dremel_query_fields_by_ordinal(fields: Vec<DremelEncodedField>, …)`
//!   called with `&fields` → E0308 expected `Vec<_>`, found `&Vec<_>`.

#[path = "common/test_utils.rs"]
mod test_utils;

const FIXTURE: &str = include_str!(
    "fixtures/library_multipass/demoted_struct_vec_into_owned_must_clone.wj"
);

#[test]
fn codegen_demoted_struct_vec_into_owned_must_clone() {
    let (rs, ok) = test_utils::compile_single_check(FIXTURE);
    let callee_owned = rs.contains("fn by_ordinal(fields: Vec<Field>")
        || rs.contains("by_ordinal(fields: Vec<Field>");
    let caller_has_amp = rs.contains("by_ordinal(&fields");
    let caller_clones = rs.contains("by_ordinal(fields.clone()")
        || rs.contains("by_ordinal((*fields).clone()")
        || rs.contains("by_ordinal(fields.to_vec()");
    if callee_owned && caller_has_amp && !caller_clones {
        panic!(
            "RED: demoted &Vec<Field> passed into owned by_ordinal without clone. Generated:\n{rs}"
        );
    }
    if callee_owned {
        assert!(
            !caller_has_amp || caller_clones,
            "demoted &Vec into owned Vec must clone. Generated:\n{rs}"
        );
    }
    assert!(ok, "fixture must cargo-check. Generated:\n{rs}");
}

