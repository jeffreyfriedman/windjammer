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

//! WDB-107: isolate-transpile without shared callee metadata must not emit owned
//! empty-string literals (`"".to_string()` / `String::new()`) into demoted `&str`
//! formals (E0308).
//!
//! WindjammerDB Phase 158 observed:
//!   `wave1_sf1_cli_run_parquet_load(lineitem_path: &str, …)`
//!   `wave1_sf1_cli_run_parquet_load("".to_string(), "", …)`  // from isolate test
//!
//! Gate A: same-file tip must cargo-check.
//! Gate B: tip isolate of callee + caller (no shared multipass metadata); if
//! callee demotes to `&str`, caller must not own empty literals at that call site.

#[path = "common/test_utils.rs"]
mod test_utils;

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use tempfile::TempDir;
use windjammer::build_project;
use windjammer::CompilationTarget;

/// Read-only string formals demote to `&str` under IR emission contract.
const CALLEE: &str = r#"
use windjammer_runtime::strings

pub fn run_parquet_load(lineitem_path: string, orders_path: string, max_rows: u64) -> u64 {
    if strings::is_empty(lineitem_path) {
        return 0
    }
    if strings::is_empty(orders_path) {
        return 1
    }
    max_rows + (lineitem_path.len() as u64) + (orders_path.len() as u64)
}
"#;

const CALLER: &str = r#"
use crate::callee::run_parquet_load

pub fn call() -> u64 {
    run_parquet_load("", "", 3)
}
"#;

fn assert_caller_compatible_with_callee(callee_rs: &str, caller_rs: &str) {
    let first_str = callee_rs.contains("lineitem_path: &str");
    let second_str = callee_rs.contains("orders_path: &str");
    if first_str {
        assert!(
            !caller_rs.contains("run_parquet_load(\"\".to_string()")
                && !caller_rs.contains("run_parquet_load(String::new()"),
            "WDB-107: first empty literal owned while formal is &str.\n--- callee ---\n{callee_rs}\n--- caller ---\n{caller_rs}"
        );
    }
    if second_str {
        // Second arg owned empty: `..., "".to_string(), ...` or `..., String::new(), ...`
        assert!(
            !caller_rs.contains(", \"\".to_string()")
                && !caller_rs.contains(", String::new()"),
            "WDB-107: second empty literal owned while formal is &str.\n--- callee ---\n{callee_rs}\n--- caller ---\n{caller_rs}"
        );
    }
}

#[test]
fn wdb107_same_file_empty_literal_into_demoted_str_formals_cargo_checks() {
    // Same compilation unit: demotion + call site must agree.
    let source = r#"
use windjammer_runtime::strings

pub fn run_parquet_load(lineitem_path: string, orders_path: string, max_rows: u64) -> u64 {
    if strings::is_empty(lineitem_path) {
        return 0
    }
    if strings::is_empty(orders_path) {
        return 1
    }
    max_rows + (lineitem_path.len() as u64) + (orders_path.len() as u64)
}

pub fn call() -> u64 {
    run_parquet_load("", "", 3)
}
"#;
    let tmp = TempDir::new().expect("tempdir");
    let wj = tmp.path().join("test.wj");
    fs::write(&wj, source).unwrap();
    let out = tmp.path().join("build");
    build_project(&wj, &out, CompilationTarget::Rust, false).expect("transpile");
    test_utils::cargo_check_generated(&out);
}

fn read_first_rs(dir: &std::path::Path) -> String {
    if let Ok(s) = fs::read_to_string(dir.join("callee.rs")) {
        return s;
    }
    if let Ok(s) = fs::read_to_string(dir.join("caller.rs")) {
        return s;
    }
    if let Ok(s) = fs::read_to_string(dir.join("test.rs")) {
        return s;
    }
    fs::read_dir(dir)
        .ok()
        .and_then(|d| {
            d.filter_map(|e| e.ok())
                .find(|e| e.path().extension().is_some_and(|x| x == "rs"))
                .and_then(|e| fs::read_to_string(e.path()).ok())
        })
        .unwrap_or_default()
}

fn isolate_transpile_with(wj_bin: &std::path::Path) -> (String, String) {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(src.join("callee.wj"), CALLEE).unwrap();
    fs::write(src.join("caller.wj"), CALLER).unwrap();

    let callee_out = tmp.path().join("callee_out");
    let caller_out = tmp.path().join("caller_out");

    for (wj, out) in [
        (src.join("callee.wj"), &callee_out),
        (src.join("caller.wj"), &caller_out),
    ] {
        let build = Command::new(wj_bin)
            .args([
                "build",
                wj.to_str().unwrap(),
                "-o",
                out.to_str().unwrap(),
                "--no-cargo",
                "--library",
            ])
            .output()
            .unwrap_or_else(|e| panic!("run {}: {e}", wj_bin.display()));
        assert!(
            build.status.success(),
            "{} build failed for {}:\n{}",
            wj_bin.display(),
            wj.display(),
            String::from_utf8_lossy(&build.stderr)
        );
    }

    let callee_rs = read_first_rs(&callee_out);
    let caller_rs = read_first_rs(&caller_out);
    assert!(
        !callee_rs.is_empty() && !caller_rs.is_empty(),
        "missing isolate .rs"
    );
    (callee_rs, caller_rs)
}

#[test]
fn wdb107_tip_isolate_caller_must_not_own_empty_literal_into_str_formal() {
    let wj = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let (callee_rs, caller_rs) = isolate_transpile_with(&wj);
    assert_caller_compatible_with_callee(&callee_rs, &caller_rs);
}
