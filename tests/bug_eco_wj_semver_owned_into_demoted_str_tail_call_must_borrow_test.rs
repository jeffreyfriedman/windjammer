//! WDB-214 class / `wj-semver`: owned `string` locals into demoted `&str` sibling
//! callees must borrow at the call site even when the caller keeps owned formals.

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

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
fn eco_wj_semver_owned_param_into_demoted_str_tail_call_must_borrow() {
    let source = r#"
use std::strings

fn to_int(text: string) -> int {
    strings.len(text)
}

fn cmp_string(a: string, b: string) -> int {
    let mut i = 0
    while i < strings.len(a) && i < strings.len(b) {
        let la = strings.byte_at(a, i)
        let lb = strings.byte_at(b, i)
        if la != lb { return 1 }
        i = i + 1
    }
    0
}

fn is_all_digits(text: string) -> bool {
    strings.len(text) > 0
}

fn compare_identifiers(left: string, right: string) -> int {
    if is_all_digits(left) && is_all_digits(right) {
        return to_int(left) - to_int(right)
    }
    cmp_string(left, right)
}

pub fn probe(a: string, b: string) -> int {
    compare_identifiers(a, b)
}
"#;
    let (generated, ok) = test_utils::compile_single_check(source);
    assert!(
        ok,
        "wj-semver compare_identifiers shape must compile, got:\n{generated}"
    );
    if generated.contains("fn cmp_string(a: &str") || generated.contains("fn cmp_string(a:&str") {
        assert!(
            generated.contains("cmp_string(&left, &right)")
                || generated.contains("cmp_string(&left,&right)"),
            "owned params into demoted &str cmp_string must borrow at call site:\n{generated}"
        );
    }
}
