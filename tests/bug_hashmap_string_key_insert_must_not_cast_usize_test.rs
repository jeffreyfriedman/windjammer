#![cfg(any(
    not(any(
        feature = "parser_tests",
        feature = "analyzer_tests",
        feature = "codegen_tests",
        feature = "interpreter_tests",
        feature = "conformance_tests",
        feature = "integration_tests",
    )),
    feature = "codegen_tests",
    feature = "integration_tests",
))]

//! FAILING REPRO — HashMap<string,string>::insert must not cast key `as usize`.
#[path = "common/test_utils.rs"]
mod test_utils;

const SOURCE: &str =
    include_str!("fixtures/library_multipass/hashmap_string_key_insert_must_not_cast_usize.wj");

#[test]
fn hashmap_string_key_insert_must_not_cast_usize() {
    let (rs, ok) = test_utils::compile_single_check(SOURCE);
    let bad = rs.contains("as usize") && rs.contains("insert");
    if bad || !ok {
        eprintln!("RED HashMap<string,string> insert key as usize:\n{rs}");
    }
    assert!(
        !bad,
        "RED HashMap<string,string>::insert must not cast key as usize. Generated:\n{rs}"
    );
    assert!(
        ok,
        "RED HashMap<string,string>::insert must cargo-check. Generated:\n{rs}"
    );
}
