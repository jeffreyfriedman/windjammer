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

//! FAILING REPRO — idiomatic flat `src/lib.wj` package must work with `wj test`.
//!
//! Ecosystem seed `wj-dotenv` needs:
//! - `pub fn` exports at lib root importable as `use crate::parse` from `tests/*_test.wj`
//! - `HashMap::get` / `std::strings::join(Vec, …)` without forced module reshuffles
//!
//! Do **not** work around by requiring `src/mod.wj` + nested modules for a one-file library.

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[path = "common/test_utils.rs"]
mod test_utils;

#[test]
#[cfg_attr(tarpaulin, ignore)]
fn test_flat_lib_wj_exports_are_visible_to_package_tests() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    let src = root.join("src");
    let tests = root.join("tests");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&tests).unwrap();

    fs::write(
        root.join("wj.toml"),
        r#"[package]
name = "flat-lib-seed"
version = "0.1.0"
edition = "2025"

[lib]
"#,
    )
    .unwrap();

    fs::write(
        src.join("lib.wj"),
        r#"use std::collections::HashMap
use std::strings

pub fn parse(content: string) -> HashMap<string, string> {
    let mut map = HashMap::new()
    let parts = strings.split(content, "=")
    if parts.len() >= 2 {
        map.insert(strings.trim(parts[0]), strings.trim(parts[1]))
    }
    map
}

pub fn join_tail(parts: Vec<string>) -> string {
    strings.join(parts, "=")
}
"#,
    )
    .unwrap();

    fs::write(
        tests.join("parse_test.wj"),
        r#"use crate::parse
use crate::join_tail

fn test_parse_exports_from_lib_root() {
    let map = parse("HOST=localhost")
    assert(map.get("HOST").is_some(), "HOST must be present")
    assert_eq(map.len(), 1)
}

fn test_strings_join_accepts_owned_vec() {
    let mut parts = Vec::new()
    parts.push("a")
    parts.push("b")
    let joined = join_tail(parts)
    assert_eq(joined, "a=b")
}
"#,
    )
    .unwrap();

    let wj = test_utils::wj_binary();
    let output = Command::new(&wj)
        .arg("test")
        .current_dir(root)
        .output()
        .expect("run wj test");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}\n{stderr}");

    assert!(
        output.status.success(),
        "idiomatic flat src/lib.wj + tests/use crate::fn must pass `wj test`.\n\
         Do not work around by forcing mod.wj nesting.\n{combined}"
    );
    assert!(
        combined.contains("passed") || combined.contains("All tests passed"),
        "expected passing tests in output:\n{combined}"
    );
}
