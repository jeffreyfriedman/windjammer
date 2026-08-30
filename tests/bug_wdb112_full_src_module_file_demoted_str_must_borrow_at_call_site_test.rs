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

//! WDB-112: full-library `--module-file` demotes owned `string` formals to `&str` but leaves
//! owned `.clone()` call sites (E0308: expected `&str`, found `String`).
//!
//! WindjammerDB Phase 193 observed after `./scripts/build_gen.sh` (full `src` multipass):
//!   `wave1_sf1_cli_run_parquet_load(lineitem_path: &str, …)` in generated `wave1_sf1_cli.rs`
//! while CLI tests still emit:
//!   `run_parquet_load(li_path.clone(), ord_path.clone(), …)` → `String` args.
//!
//! Relational-only module-file (`src/relational`) keeps owned `String` formals (WDB-111 GREEN).
//! Full `src` module-file incorrectly demotes when sibling modules also call the callee with
//! borrowed locals — call sites with explicit `.clone()` must auto-borrow or callee must stay owned.
//!
//! Gate A: multipass with demote + clone callers must cargo-check.
//! Gate B: emitted call site must borrow owned temps when formal is `&str`.

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

const SF1_CLI_DEMOTE_CALLER: &str = r#"
use crate::sf1_cli::run_parquet_load

/// Bare `path` reuse demotes callee formals in full-library multipass (WindjammerDB full `src`).
pub fn load_from_borrowed(path: string) -> u64 {
    run_parquet_load(path, path, 3, 0, false)
}
"#;

const SF1_CLI_CLONE_CALLER: &str = r#"
use crate::sf1_cli::run_parquet_load

pub fn load_from_paths(li_path: string, ord_path: string) -> u64 {
    run_parquet_load(li_path.clone(), ord_path.clone(), 3, 0, false)
}
"#;

#[test]
fn wdb112_full_library_multipass_demoted_str_formal_must_borrow_clone_call_sites() {
    let mut test = MultiFileTest::new();
    test.add_file(
        "mod.wj",
        r#"
pub mod sf1_cli
pub mod sf1_cli_demote
pub mod sf1_cli_clone
"#,
    );
    test.add_file("sf1_cli.wj", SF1_CLI);
    test.add_file("sf1_cli_demote.wj", SF1_CLI_DEMOTE_CALLER);
    test.add_file("sf1_cli_clone.wj", SF1_CLI_CLONE_CALLER);

    let map = test
        .compile()
        .expect("WDB-112 multipass compile should succeed");
    let cli = map.get("sf1_cli.rs").expect("sf1_cli.rs must be generated");
    let clone_caller = map
        .get("sf1_cli_clone.rs")
        .expect("sf1_cli_clone.rs must be generated");

    if cli.contains("lineitem_path: &str") && cli.contains("orders_path: &str") {
        assert!(
            clone_caller.contains("run_parquet_load(&li_path")
                || clone_caller.contains("run_parquet_load(&li_path.clone()")
                || clone_caller.contains("run_parquet_load(li_path.as_"),
            "WDB-112: when multipass demotes to &str, .clone() call sites must borrow. Got:\n{clone_caller}"
        );
    } else {
        assert!(
            cli.contains("lineitem_path: String") && cli.contains("orders_path: String"),
            "WDB-112: owned String formals are also acceptable when demote caller coexists.\n{cli}"
        );
    }
}
