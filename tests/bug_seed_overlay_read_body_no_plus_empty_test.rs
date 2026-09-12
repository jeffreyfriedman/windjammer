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

//! GREEN regression — seed overlay `Ok(body) => body` / `Err(_) => ""` without
//! `+ ""` (LedgerKit `seed_bank_*_overlay.wj`). Distinct from owned-match→formal
//! RED (`bug_owned_match_binding_*`).

#[path = "common/test_utils.rs"]
mod test_utils;

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SOURCE: &str =
    include_str!("fixtures/library_multipass/seed_overlay_read_body_no_plus_empty.wj");

fn assert_seed_overlay_emit(rs: &str) {
    assert!(
        rs.contains("Ok(body) => body") || rs.contains("Ok(body)=> body"),
        "must move owned match binding on Ok arm; got:\n{rs}"
    );
    assert!(
        !rs.contains(r#"format!("{}{}", body, "")"#)
            && !rs.contains(r#"format!("{}{}", "", "")"#),
        "must not emit empty-concat temps; got:\n{rs}"
    );
}

#[test]
fn seed_overlay_read_body_must_cargo_check_without_plus_empty() {
    let (rs, ok) = test_utils::compile_single_check(SOURCE);
    assert_seed_overlay_emit(&rs);
    assert!(
        ok,
        "RED: seed overlay read_overlay_body without + \"\" must cargo-check. Generated:\n{rs}"
    );
}

#[test]
fn hexagonal_seed_overlay_read_body_must_cargo_check_without_plus_empty() {
    let mut project = MultiFileTest::new();
    project.add_file(
        "mod.wj",
        r#"
pub mod domain
pub mod adapters
"#,
    );
    project.add_file("domain/mod.wj", "pub mod overlay_io\n");
    project.add_file("domain/overlay_io.wj", SOURCE);
    project.add_file("adapters/mod.wj", "pub mod seed\n");
    project.add_file(
        "adapters/seed.wj",
        r#"
use super::super::domain::overlay_io::read_overlay_body

pub fn body_or_empty() -> string {
    read_overlay_body()
}
"#,
    );

    let map = project
        .compile()
        .expect("hexagonal seed overlay compile should succeed");
    let key = map
        .keys()
        .find(|k| k.ends_with("overlay_io.rs"))
        .cloned()
        .unwrap_or_else(|| {
            panic!(
                "missing overlay_io.rs; keys: {:?}",
                map.keys().collect::<Vec<_>>()
            )
        });
    assert_seed_overlay_emit(map.get(&key).expect("overlay_io.rs"));
}
