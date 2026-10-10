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

//! read-only `string_len(line)` after `&line` must not emit `line.clone()`.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
fn slen(s: string) -> u32 {
    if s == "" {
        0
    } else {
        1
    }
}

fn start(line: string, key: string) -> u32 {
    slen(key)
}

pub fn present(line: string, key: string) -> bool {
    start(line, key) < slen(line)
}
"#;

#[test]
fn module_file_readonly_string_len_must_not_clone_line() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let cloned = rs.contains("line.clone()") || rs.contains("slen(line.clone()");
    assert!(
        !cloned,
        "RED read-only string_len reused line via clone:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
