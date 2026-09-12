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

//! GREEN regression — seed overlay `remember_*` loop/split/append without `+ ""`
//! (LedgerKit `seed_bank_*_overlay.wj`). Tip cargo-checks after OMB ownership fix.

#[path = "common/test_utils.rs"]
mod test_utils;

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SOURCE: &str =
    include_str!("fixtures/library_multipass/seed_overlay_remember_no_plus_empty.wj");

fn assert_no_empty_concat_temps(rs: &str) {
    assert!(
        !rs.contains(r#"format!("{}{}", body, "")"#)
            && !rs.contains(r#"format!("{}{}", row, "")"#)
            && !rs.contains(r#"format!("{}{}", "", "")"#),
        "must not emit empty-concat temps; got:\n{rs}"
    );
}

#[test]
fn seed_overlay_remember_must_cargo_check_without_plus_empty() {
    let (rs, ok) = test_utils::compile_single_check(SOURCE);
    assert_no_empty_concat_temps(&rs);
    assert!(
        ok,
        "RED: seed overlay remember without + \"\" must cargo-check. Generated:\n{rs}"
    );
}

#[test]
fn hexagonal_seed_overlay_remember_must_cargo_check_without_plus_empty() {
    let mut project = MultiFileTest::new();
    project.add_file(
        "mod.wj",
        r#"
pub mod domain
pub mod adapters
"#,
    );
    project.add_file("domain/mod.wj", "pub mod overlay\n");
    project.add_file("domain/overlay.wj", SOURCE);
    project.add_file("adapters/mod.wj", "pub mod seed\n");
    project.add_file(
        "adapters/seed.wj",
        r#"
use super::super::domain::overlay::remember_seed_bank_match

pub fn remember(line_id: string, journal_entry_id: string) -> Result<(), string> {
    remember_seed_bank_match(line_id, journal_entry_id)
}
"#,
    );

    let map = project
        .compile()
        .expect("hexagonal seed remember compile should succeed");
    let key = map
        .keys()
        .find(|k| k.ends_with("overlay.rs"))
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "missing overlay.rs; keys: {:?}",
                map.keys().collect::<Vec<_>>()
            )
        });
    assert_no_empty_concat_temps(map.get(&key).expect("overlay.rs"));
}
