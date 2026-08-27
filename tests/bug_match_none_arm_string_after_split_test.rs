//! Match `None` arm / early-return reusing a `string` param after `split_once` /
//! `strings.contains` must own `String` — not demoted `.clone()` as `&str`.
//!
//! Ecosystem `wj-url` (`split_host_port`, `split_path_query_fragment`):
//! - `match split_once(text, sep) { Some(p) => p, None => (text, "") }`
//! - `if !strings.contains(text, ":") { return Ok((text, "")) }`
//! both fail `cargo check` with E0308 under `wj test`.

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

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[path = "common/test_utils.rs"]
mod test_utils;

fn write_seed(name: &str, lib: &str) -> TempDir {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    fs::create_dir_all(root.join("src")).unwrap();
    fs::write(
        root.join("wj.toml"),
        format!(
            r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2025"

[lib]
"#
        ),
    )
    .unwrap();
    fs::write(root.join("src/lib.wj"), lib).unwrap();
    temp
}

fn assert_wj_build_ok(root: &std::path::Path, label: &str) {
    let status = Command::new(test_utils::wj_binary())
        .args(["build", "src"])
        .current_dir(root)
        .status()
        .expect("run wj build");
    let generated = fs::read_to_string(root.join("build/lib.rs")).unwrap_or_default();
    assert!(
        status.success(),
        "{label} must cargo-check:\n{generated}"
    );
}

#[test]
fn match_none_arm_reuses_string_param_after_split_must_cargo_check() {
    let temp = write_seed(
        "match-none-string-seed",
        // r## so embedded `"#"` (hash separator) does not terminate the raw string.
        r##"
use std::strings

fn split_once(text: string, sep: string) -> Option<(string, string)> {
    let mut i = 0
    while i < strings.len(text) {
        let rest = strings.substring(text, i, strings.len(text))
        if strings.starts_with(rest, sep) {
            return Some((
                strings.substring(text, 0, i),
                strings.substring(text, i + strings.len(sep), strings.len(text)),
            ))
        }
        i = i + 1
    }
    None
}

pub fn host_or_pair(authority: string) -> Result<(string, string), string> {
    match split_once(authority, ":") {
        Some(parts) => Ok((parts.0, parts.1)),
        None => Ok((authority, "")),
    }
}

pub fn path_and_frag(text: string) -> (string, string) {
    match split_once(text, "#") {
        Some(pf) => pf,
        None => (text, ""),
    }
}
"##,
    );
    assert_wj_build_ok(temp.path(), "match None arm reusing string after split_once");
}

#[test]
fn early_return_after_contains_reuses_string_must_cargo_check() {
    let temp = write_seed(
        "contains-early-return-seed",
        r#"
use std::strings

pub fn host_or_plain(authority: string) -> Result<(string, string), string> {
    if !strings.contains(authority, ":") {
        return Ok((authority, ""))
    }
    Ok((authority, "has-colon"))
}
"#,
    );
    assert_wj_build_ok(
        temp.path(),
        "early return Ok((text, \"\")) after strings.contains",
    );
}

#[test]
fn if_else_int_index_vs_strings_len_must_unify() {
    let temp = write_seed(
        "int-usize-if-else-seed",
        r#"
use std::strings

pub fn path_end(text: string, at: int, use_at: bool) -> string {
    let n = strings.len(text)
    let end = if use_at {
        at
    } else {
        n
    }
    strings.substring(text, 0, end)
}
"#,
    );
    assert_wj_build_ok(
        temp.path(),
        "if/else unifying int index with strings.len (usize)",
    );
}

#[test]
fn owned_string_formals_must_not_receive_borrow_at_call_site() {
    let temp = write_seed(
        "join-path-owned-formals-seed",
        r#"
use std::strings

fn join_path(base_path: string, relative: string) -> string {
    if strings.starts_with(relative, "/") {
        return relative
    }
    "${base_path}/${relative}"
}

pub struct Parts {
    pub path: string,
    pub leaf: string,
}

pub fn combine(p: Parts) -> string {
    join_path(p.path, p.leaf)
}
"#,
    );
    assert_wj_build_ok(
        temp.path(),
        "owned string formals must receive moved fields, not &field",
    );
}
