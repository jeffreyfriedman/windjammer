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

//! `format!` must not lower to `write!(&mut __s, …).unwrap()`.
#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn label(name: string, n: i32) -> string {
    let s = format!("{}-{}", name, n)
    s
}

pub fn json_pair(id: string, name: string) -> string {
    format!("{{\"id\":\"{}\",\"name\":\"{}\"", id, name)
}
"#;

#[test]
fn module_file_format_must_not_emit_write_unwrap() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("MultiFile lib.rs:\n{rs}");
    let bad = rs.contains("write!(&mut __s") || rs.contains(".unwrap()");
    assert!(
        !bad,
        "RED format! lowered to write!/unwrap:\n{rs}"
    );
    test.cargo_check().expect("cargo-check");
}
