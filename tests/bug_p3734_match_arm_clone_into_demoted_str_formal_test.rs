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

//! P3.734 isolate: one owned `json: string` match feeds both owned parsers and
//! a parser the compiler demotes to `&str`. The demoted call must borrow.
//! Product `render_read_model` still emits `json.clone()` or a bare `String`
//! into `fn parse_*(json: &str)`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SRC: &str = r#"
pub fn json_i64_field(json: string, key: string) -> int {
    match json.find(key) {
        Some(start) => start,
        None => 0,
    }
}

pub fn keep_owned(json: string, key: string) -> string {
    match json.find(key) {
        Some(idx) => json.substring(idx, json.len()),
        None => json,
    }
}

pub fn parse_sheet(json: string) -> int {
    let n = json_i64_field(json, "assets_cents")
    let _raw = keep_owned(json, "lines")
    n
}

pub fn render(kind: string, json: string) -> int {
    match kind {
        "a" => keep_owned(json, "a").len(),
        "b" => keep_owned(json, "b").len(),
        "c" => keep_owned(json, "c").len(),
        "d" => keep_owned(json, "d").len(),
        "sheet" => parse_sheet(json),
        _ => 0,
    }
}
"#;

fn feeds_owned_string(rs: &str) -> bool {
    rs.contains("parse_sheet(json.clone())")
        || (rs.contains("parse_sheet(json)") && !rs.contains("parse_sheet(&json)"))
}

#[test]
fn match_arm_owned_json_into_demoted_str_parser_must_borrow() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("P3.734 isolate compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("P3.734 isolate lib.rs:\n{rs}");
    if !rs.contains("fn parse_sheet(json: &str)") {
        eprintln!("P3.734 isolate: parse_sheet stayed owned");
        return;
    }
    assert!(
        !feeds_owned_string(rs),
        "P3.734 RED: demoted &str parser fed owned String:\n{rs}"
    );
}
