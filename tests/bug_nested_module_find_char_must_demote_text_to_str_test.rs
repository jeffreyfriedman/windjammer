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

//! P3.524 isolate: module-file nested `find_char` must demote to `text: &str`,
//! not `&String`, when the body only uses `strings::len` / `substring` + `==`.
//! Flat private formals false-GREEN `&str`; nested adapters emit `&String`
//! and break `split_header_line(line: &str)` → `find_char(line, …)` (E0308).

use std::process::Command;
use tempfile::TempDir;

#[test]
fn nested_module_find_char_must_demote_text_to_str_not_string_ref() {
    let dir = TempDir::new().expect("temp");
    let src = dir.path().join("src");
    std::fs::create_dir_all(src.join("adapters")).unwrap();
    std::fs::write(src.join("mod.wj"), "pub mod adapters\n").unwrap();
    std::fs::write(src.join("adapters").join("mod.wj"), "pub mod http_server\n").unwrap();
    std::fs::write(
        src.join("adapters").join("http_server.wj"),
        r#"use std::strings

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

pub fn probe(line: string) -> Option<(string, string)> {
    split_header_line(line)
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
        "transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let http = std::fs::read_to_string(out.join("adapters").join("http_server.rs"))
        .expect("http_server.rs");
    let sig = http
        .lines()
        .find(|l| l.contains("fn find_char("))
        .unwrap_or("")
        .to_string();
    eprintln!("nested find_char: {sig}");
    assert!(
        !sig.contains("text: &String"),
        "P3.524 RED: nested find_char must be text: &str, not &String:\n{sig}\n{http}"
    );
    assert!(
        sig.contains("text: &str") || sig.contains("text:&str"),
        "P3.524 RED: expected text: &str formal:\n{sig}"
    );
}
