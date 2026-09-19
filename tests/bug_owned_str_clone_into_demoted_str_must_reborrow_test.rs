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
))]

//! P3.383 / WDB-270 class: `&owned.clone()` into demoted `&str` must reborrow.
//!
//! Product tip-out wave1 emits `helper(&dated_label.clone())` while formal is
//! `label: &str`. Signature-driven: pass `&owned` (no clone into `&str`).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const MOD: &str = r#"
pub mod helpers
pub mod publish
"#;

const HELPERS: &str = r#"
/// Readonly probe — demotes to `&str`.
pub fn dated_label_is_set(label: string) -> bool {
    label.len() > 0
}

pub fn clock_sql(sql: string) -> int {
    sql.len() as int
}
"#;

const PUBLISH: &str = r#"
use crate::helpers::{dated_label_is_set, clock_sql}

pub fn publish_once(dated_label: string, sql: string) -> int {
    let ok = dated_label_is_set(dated_label.clone())
    let n = clock_sql(sql.clone())
    let _ = dated_label.len()
    let _ = sql.len()
    if ok {
        n
    } else {
        0
    }
}
"#;

fn bad_clone_into_str(rs: &str) -> bool {
    rs.contains("dated_label_is_set(&dated_label.clone()")
        || rs.contains("clock_sql(&sql.clone()")
        || rs.contains("dated_label_is_set(dated_label.clone()")
        || rs.contains("clock_sql(sql.clone()")
}

#[test]
fn owned_str_clone_into_demoted_str_must_reborrow() {
    let mut test = MultiFileTest::new();
    test.add_file("mod.wj", MOD);
    test.add_file("helpers.wj", HELPERS);
    test.add_file("publish.wj", PUBLISH);
    let map = test.compile().expect("P3.383 compile");
    let helpers = map.get("helpers.rs").expect("helpers.rs");
    let publish = map.get("publish.rs").expect("publish.rs");
    eprintln!("P3.383 helpers.rs:\n{helpers}\npublish.rs:\n{publish}");
    let demoted = helpers.contains("label: &str") || helpers.contains("sql: &str");
    assert!(
        demoted,
        "P3.383: expected demoted &str formals:\n{helpers}"
    );
    if bad_clone_into_str(publish) {
        eprintln!("P3.383 RED: &owned.clone() into &str");
    }
    assert!(
        !bad_clone_into_str(publish),
        "P3.383: owned.clone() into demoted &str must reborrow (&owned):\n{publish}"
    );
    assert!(
        publish.contains("dated_label_is_set(&dated_label)")
            || publish.contains("dated_label_is_set(dated_label.as_str()"),
        "P3.383: expected &dated_label into demoted &str:\n{publish}"
    );
    test.cargo_check().expect("P3.383 cargo-check");
}
