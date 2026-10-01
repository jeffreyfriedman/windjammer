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

//! P3.524: with `use std::strings`, inline `strings.substring(text, …) == needle`
//! must still demote `text` to `&str` (not `&String`).
//!
//! Product `wj-notes-api` adapter `find_char` stays `text: &String` while
//! `split_header_line(line: &str)` passes `line` → E0308. Domain bind-then-
//! compare greened. Do not reshape notes-api.

use std::process::Command;
use tempfile::TempDir;

#[test]
fn adapter_inline_substring_eq_find_char_must_be_str() {
    let dir = TempDir::new().expect("temp");
    let src = dir.path().join("src");
    std::fs::create_dir_all(src.join("adapters")).unwrap();

    std::fs::write(src.join("mod.wj"), "pub mod adapters\n").unwrap();
    std::fs::write(src.join("adapters").join("mod.wj"), "pub mod http_server\n").unwrap();
    std::fs::write(
        src.join("adapters").join("http_server.wj"),
        r#"use std::strings

fn apply_extra_headers(raw: string) -> int {
    let lines = strings.split(raw, "\n")
    let mut i = 0
    let mut n = 0
    while i < lines.len() {
        let line = strings.trim(lines[i])
        if strings_len(line) == 0 {
            i = i + 1
            continue
        }
        match split_header_line(line) {
            Some(_) => {
                n = n + 1
            },
            None => {},
        }
        i = i + 1
    }
    n
}

fn split_header_line(line: string) -> Option<(string, string)> {
    let colon = find_char(line, ":")
    if colon < 0 {
        return None
    }
    let name = strings.trim(strings.substring(line, 0, colon))
    let value = strings.trim(strings.substring(line, colon + 1, strings.len(line)))
    Some((name, value))
}

fn find_char(text: string, needle: string) -> int {
    let mut i = 0
    let n = strings.len(text)
    while i < n {
        if strings.substring(text, i, i + 1) == needle {
            return i
        }
        i = i + 1
    }
    -1
}

fn strings_len(text: string) -> int {
    strings.len(text)
}

pub fn serve(raw: string) -> int {
    apply_extra_headers(raw)
}
"#,
    )
    .unwrap();

    let out = dir.path().join("out");
    let wj = env!("CARGO_BIN_EXE_wj");
    let build = Command::new(wj)
        .args([
            "build",
            src.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--library",
            "--module-file",
            "--no-cargo",
        ])
        .output()
        .expect("wj build");
    assert!(
        build.status.success(),
        "P3.524 isolate transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let http = std::fs::read_to_string(out.join("adapters").join("http_server.rs"))
        .expect("http_server.rs");
    let find_sig = http
        .lines()
        .find(|l| l.contains("fn find_char("))
        .unwrap_or("")
        .to_string();
    eprintln!("find_char: {find_sig}");

    assert!(
        !find_sig.contains("text: &String"),
        "P3.524 RED: inline substring== must demote text to &str (not &String):\n{find_sig}\n{http}"
    );
    assert!(
        find_sig.contains("text: &str"),
        "P3.524: expected find_char(text: &str, …):\n{find_sig}"
    );
}
