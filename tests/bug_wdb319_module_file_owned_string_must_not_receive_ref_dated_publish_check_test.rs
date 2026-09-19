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

//! WDB-319: owned `String` formals must not receive `&dated` (wave1 publish_check CLI).
//!
//! Product tip-out/gen:
//!   `wave1_publish_check_cli_run_resolved(…, &dated)` with `dated_label: String` → E0308.
//! Twin of WDB-312 (publish_cli uses `&dated.clone()`); this site passes bare `&dated`.

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;
use std::path::PathBuf;

const SRC: &str = r#"
pub fn run_resolved(live: bool, dated_label: string) -> string {
    if live {
        return dated_label + "|live"
    }
    dated_label + "|cap"
}

pub fn main_check(dated: string) -> string {
    run_resolved(true, dated)
}
"#;

#[test]
fn wdb319_module_file_owned_string_must_not_receive_ref_dated() {
    let mut test = MultiFileTest::new();
    test.add_file("lib.wj", SRC);
    let map = test.compile().expect("WDB-319 compile");
    let rs = map.get("lib.rs").expect("lib.rs");
    eprintln!("WDB-319 MultiFile lib.rs:\n{rs}");
    let owned = rs.contains("fn run_resolved(live: bool, dated_label: String")
        || rs.contains("fn run_resolved(live: bool, mut dated_label: String");
    assert!(
        owned,
        "WDB-319: expected owned String formal on run_resolved:\n{rs}"
    );
    let bad = rs.contains("run_resolved(true, &dated")
        || rs.contains("run_resolved(true, &dated.clone()");
    assert!(
        !bad,
        "WDB-319 RED: owned run_resolved received &dated:\n{rs}"
    );
    test.cargo_check().expect("WDB-319 cargo-check");
}

#[test]
fn wdb319_tip_out_publish_check_cli_must_not_pass_ref_dated_into_owned_string() {
    let tip = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".agent-wip/rel_tip_out");
    let gen = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("windjammerdb/crates/wdb-layers/gen");
    let paths = [
        tip.join("wave1_publish_check_cli.rs"),
        tip.join("relational/wave1_publish_check_cli.rs"),
        gen.join("relational/wave1_publish_check_cli.rs"),
    ];
    let mut saw = false;
    let mut bad_paths = Vec::new();
    for path in &paths {
        if !path.exists() {
            continue;
        }
        saw = true;
        let text = std::fs::read_to_string(path).expect("publish_check");
        if text.contains("wave1_publish_check_cli_run_resolved(")
            && (text.contains("&dated)") || text.contains("&dated.clone()") || text.contains(", &dated"))
        {
            bad_paths.push(path.display().to_string());
        }
    }
    assert!(saw, "WDB-319: tip-out/gen publish_check_cli missing");
    assert!(
        bad_paths.is_empty(),
        "WDB-319 RED: tip-out/product passes &dated into owned String in:\n  {}",
        bad_paths.join("\n  ")
    );
}
