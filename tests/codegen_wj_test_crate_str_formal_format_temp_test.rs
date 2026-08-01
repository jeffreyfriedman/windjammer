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

//! REGRESSION (`wj test` crate / E3.9.3): library `&str` formals vs format temps + HashMap.
//!
//! `wj test` compiles `@test` files into a separate `windjammer-tests` crate
//! against the library. Library `string` formals often demote to `&str`, while
//! test call sites emit `let _tempN = format!("{}{}", …, ""); fn(_temp0, …)`
//! without `&` → E0308 (`expected &str, found String`).
//!
//! Same harness also keeps `HashMap<String, String>` by value while tests pass
//! `&query` (request-context style `as_of_from_query(&query)`).
//!
//! In-library multipass string assertions can be green while `wj test` still fails —
//! this cargo-backed harness is the contract that unblocks downstream `make test`.

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn wj_test_crate_format_temps_must_match_demoted_str_formals() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();

    fs::write(
        root.join("Cargo.toml"),
        r#"[package]
name = "probe"
version = "0.1.0"
edition = "2021"

[lib]
name = "probe"
path = "src/lib.rs"

[workspace]
"#,
    )
    .unwrap();

    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();
    // Demotion trigger: formals only observed via .len() / sink → `&str` in lib.
    // Call sites in tests still use `"x" + ""` owned format temps.
    fs::write(
        src.join("domain.wj"),
        r#"
use std::collections::HashMap

fn sink(s: string) -> int {
    s.len() as int
}

pub fn make_queue_item(
    account_code: string,
    account_name: string,
    balanced: bool,
    can_finish: bool,
    discrepancy: int,
    uncleared: int,
    as_of: string,
) -> string {
    let _ = sink(account_code)
    let _ = sink(account_name)
    let _ = sink(as_of)
    let _ = balanced
    let _ = can_finish
    let _ = discrepancy
    let _ = uncleared
    "item"
}

pub fn apply_signoff(as_of: string, signed_by: string) -> string {
    let _ = sink(as_of)
    let _ = sink(signed_by)
    "signed"
}

pub fn make_uncleared_line(id: string, date: string, desc: string, amount: int) -> string {
    let _ = sink(id)
    let _ = sink(date)
    let _ = sink(desc)
    let _ = amount
    "line"
}

/// Owned HashMap formal — call sites that pass `&query` must rustc (demote or clone).
pub fn as_of_from_query(query: HashMap<string, string>) -> int {
    let _ = query
    1
}

pub fn query_with(key: string, value: string) -> HashMap<string, string> {
    let mut m = HashMap::new()
    m.insert(key + "", value + "")
    m
}
"#,
    )
    .unwrap();
    fs::write(src.join("mod.wj"), "pub mod domain\n").unwrap();

    let tests = root.join("tests");
    fs::create_dir_all(&tests).unwrap();
    fs::write(
        tests.join("domain_test.wj"),
        r#"
use std::test
use crate::domain::{
    apply_signoff, as_of_from_query, make_queue_item, make_uncleared_line, query_with,
}

@test
fn queue_item_from_owned_concat() {
    let item = make_queue_item(
        "1000" + "",
        "Checking" + "",
        true,
        true,
        0,
        0,
        "2026-07-26" + "",
    )
    assert_eq(item, "item")
}

@test
fn signoff_from_owned_concat() {
    let s = apply_signoff("2026-07-26" + "", "dev-user" + "")
    assert_eq(s, "signed")
}

@test
fn uncleared_from_owned_concat() {
    let line = make_uncleared_line(
        "L1" + "",
        "2026-07-01" + "",
        "Outstanding check" + "",
        -4500,
    )
    assert_eq(line, "line")
}

@test
fn as_of_borrows_query_map() {
    let query = query_with("as_of" + "", "2026-12-31" + "")
    let n = as_of_from_query(&query)
    assert_eq(n, 1)
}
"#,
    )
    .unwrap();

    let wj = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = Command::new(&wj)
        .current_dir(root)
        .args(["test", "tests", "--nocapture"])
        .output()
        .expect("run wj test");

    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        output.status.success(),
        "wj test crate must rustc against demoted &str formals + HashMap borrow. out:\n{combined}"
    );
}

/// Isolated HashMap formal vs `&query` call site (request_context_test).
#[test]
fn wj_test_crate_hashmap_formal_must_accept_borrowed_query() {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();

    fs::write(
        root.join("Cargo.toml"),
        r#"[package]
name = "query-probe"
version = "0.1.0"
edition = "2021"

[lib]
name = "query_probe"
path = "src/lib.rs"

[workspace]
"#,
    )
    .unwrap();

    let src = root.join("src");
    fs::create_dir_all(&src).unwrap();
    fs::write(
        src.join("domain.wj"),
        r#"
use std::collections::HashMap

pub fn as_of_from_query(query: HashMap<string, string>) -> Option<string> {
    match query.get("as_of") {
        Some(v) => Some(v + ""),
        None => None,
    }
}

pub fn query_with(key: string, value: string) -> HashMap<string, string> {
    let mut m = HashMap::new()
    m.insert(key + "", value + "")
    m
}
"#,
    )
    .unwrap();
    fs::write(src.join("mod.wj"), "pub mod domain\n").unwrap();

    let tests = root.join("tests");
    fs::create_dir_all(&tests).unwrap();
    fs::write(
        tests.join("query_test.wj"),
        r#"
use std::test
use crate::domain::{as_of_from_query, query_with}

@test
fn as_of_borrows_query_map() {
    let query = query_with("as_of" + "", "2026-12-31" + "")
    match as_of_from_query(&query) {
        Some(value) => assert_eq(value, "2026-12-31"),
        None => assert(false),
    }
}
"#,
    )
    .unwrap();

    let wj = PathBuf::from(env!("CARGO_BIN_EXE_wj"));
    let output = Command::new(&wj)
        .current_dir(root)
        .args(["test", "tests", "--nocapture"])
        .output()
        .expect("run wj test");

    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        output.status.success(),
        "owned HashMap formal + &query call site must rustc (demote or clone). out:\n{combined}"
    );
}
