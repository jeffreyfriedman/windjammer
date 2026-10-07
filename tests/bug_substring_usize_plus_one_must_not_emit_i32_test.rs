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

//! P3.705 / LedgerKit `json.wj` `parse_json_string_array`:
//! `section.substring(i, i + 1)` with `i` driven by `.len()` must not emit
//! `i + 1_i32` (E0277 usize + i32). Tip `--module-file` currently peels
//! substring to a slice end of `1_i32`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn parse_json_string_array(section: string) -> Vec<string> {
    let mut out: Vec<string> = Vec::new()
    let mut i = 0
    let len = section.len()
    while i < len {
        let ch = section.substring(i, i + 1)
        if ch == "\"" {
            i = i + 1
        } else {
            i = i + 1
        }
        let _ = ch
    }
    out
}
"#;

#[test]
fn substring_usize_plus_one_must_not_emit_i32() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("P3.705 substring compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("P3.705 substring MultiFile lib.rs:\n{rs}");
    assert!(
        !rs.contains("+ 1_i32") && !rs.contains("+1_i32"),
        "P3.705 RED: substring(i, i + 1) must not emit 1_i32 beside usize i:\n{rs}"
    );
    test.cargo_check().expect("P3.705 substring cargo-check");
}
