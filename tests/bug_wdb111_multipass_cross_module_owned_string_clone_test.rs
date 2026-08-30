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

//! WDB-111: multipass cross-module owned `string` formals with `.clone()` call sites.
//!
//! WindjammerDB per-file isolate drift (WDB-110 on wj 0.50.0) is avoided when the relational
//! slice is built with `wj build --module-file mod.wj` (shared ownership registry). This gate
//! locks the Wave1 SF1 CLI load shape under multipass:
//!
//!   `run_parquet_load(li_path.clone(), ord_path.clone(), …)` → owned `String` formals, no `&`.
//!
//! Gate A: multipass `MultiFileTest` (correct product path).
//! Gate B: tip isolate callee + caller (WDB-110 sibling — minimal isolate already covered).

#[path = "common/integration_test_helpers.rs"]
mod integration_test_helpers;

use integration_test_helpers::MultiFileTest;

const SF1_CLI: &str = r#"
use windjammer_runtime::strings

pub fn run_parquet_load(
    lineitem_path: string,
    orders_path: string,
    max_rows: u64,
    postgres_line_rows: u64,
    postgres_ready: bool,
) -> u64 {
    let mut rows = max_rows
    if !strings::is_empty(lineitem_path) {
        rows = rows + (lineitem_path.len() as u64)
    }
    if !strings::is_empty(orders_path) {
        rows = rows + 1
    }
    if postgres_ready {
        rows = rows + postgres_line_rows
    }
    rows
}
"#;

const SF1_CLI_TEST: &str = r#"
use crate::sf1_cli::run_parquet_load

pub fn load_from_paths(li_path: string, ord_path: string) -> u64 {
    run_parquet_load(li_path.clone(), ord_path.clone(), 3, 0, false)
}
"#;

fn assert_multipass_no_borrow_on_owned_string_clone(test_rs: &str) {
    assert!(
        !test_rs.contains("run_parquet_load(&li_path.clone()")
            && !test_rs.contains(", &ord_path.clone(),"),
        "WDB-111: multipass must not borrow .clone() into owned String formals. Got:\n{test_rs}"
    );
    assert!(
        test_rs.contains("run_parquet_load(li_path.clone()")
            || test_rs.contains("run_parquet_load(li_path,"),
        "WDB-111: expected move/clone into owned String formal. Got:\n{test_rs}"
    );
}

#[test]
fn wdb111_multipass_cross_module_owned_string_clone_must_not_borrow() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod sf1_cli
pub mod sf1_cli_test
"#,
    );
    test.add_file("sf1_cli.wj", SF1_CLI);
    test.add_file("sf1_cli_test.wj", SF1_CLI_TEST);

    let map = test
        .compile()
        .expect("WDB-111 multipass compile should succeed");
    let cli = map.get("sf1_cli.rs").expect("sf1_cli.rs must be generated");
    let cli_test = map
        .get("sf1_cli_test.rs")
        .expect("sf1_cli_test.rs must be generated");
    assert!(
        cli.contains("lineitem_path: String") && cli.contains("orders_path: String"),
        "WDB-111: callee must keep owned String formals.\n{cli}"
    );
    assert_multipass_no_borrow_on_owned_string_clone(cli_test);
}
