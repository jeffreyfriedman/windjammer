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

//! FAILING REPRO / GREEN gate — seed overlay `int_to_string` / `parse_int_string`
//! without `+ ""` (LedgerKit `seed_bank_recon*_overlay.wj`).
//!
//! Product still uses `"0" + ""` / `raw + ""` / `text + ""` workarounds in recon
//! overlays; `request_context` already dogfoods the clean shape.

#[path = "common/test_utils.rs"]
mod test_utils;

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SOURCE: &str =
    include_str!("fixtures/library_multipass/seed_overlay_int_parse_format_no_plus_empty.wj");

fn assert_no_empty_concat_temps(rs: &str) {
    assert!(
        !rs.contains(r#""0".to_string() + &"".to_string()"#)
            && !rs.contains(r#"format!("{}{}", "0", "")"#)
            && !rs.contains(r#"format!("{}{}", raw, "")"#)
            && !rs.contains(r#"format!("{}{}", text, "")"#)
            && !rs.contains(r#"format!("{}{}", ch, "")"#)
            && !rs.contains(r#"format!("{}{}", "", "")"#),
        "must not emit empty-concat temps; got:\n{rs}"
    );
}

#[test]
fn seed_overlay_int_parse_format_must_cargo_check_without_plus_empty() {
    let (rs, ok) = test_utils::compile_single_check(SOURCE);
    assert_no_empty_concat_temps(&rs);
    assert!(
        ok,
        "RED P3.258: seed overlay int parse/format without + \"\" must cargo-check. Generated:\n{rs}"
    );
}

#[test]
fn hexagonal_seed_overlay_int_parse_format_must_cargo_check_without_plus_empty() {
    let mut project = MultiFileTest::new();
    project.add_file(
        "mod.wj",
        r#"
pub mod domain
pub mod adapters
"#,
    );
    project.add_file("domain/mod.wj", "pub mod int_string\n");
    project.add_file("domain/int_string.wj", SOURCE);
    project.add_file("adapters/mod.wj", "pub mod overlay\n");
    project.add_file(
        "adapters/overlay.wj",
        r#"
use super::super::domain::int_string::{int_to_string, parse_int_string, round_trip}

pub fn encode(cents: int) -> string {
    int_to_string(cents)
}

pub fn decode(raw: string) -> int {
    parse_int_string(raw)
}

pub fn check(value: int) -> int {
    round_trip(value)
}
"#,
    );

    let map = project
        .compile()
        .expect("hexagonal int_string overlay compile should succeed");
    let key = map
        .keys()
        .find(|k| k.ends_with("int_string.rs"))
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "missing int_string.rs; keys: {:?}",
                map.keys().collect::<Vec<_>>()
            )
        });
    let rs = map.get(&key).expect("int_string.rs");
    assert_no_empty_concat_temps(rs);
}
