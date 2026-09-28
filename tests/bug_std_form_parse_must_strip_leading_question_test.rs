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

//! P3.535: `std::encoding.form_parse` must strip an optional leading `?`
//! (`STDLIB_FORM_HANDOFF.md` / `wj-querystring` `parse("?a=1")`).
//!
//! Runtime today calls `url::form_urlencoded::parse` on the raw bytes, so
//! `"?a=1"` becomes key `"?a"` (Ecosystem thin-wrap then fails
//! `test_parse_strips_leading_question`). Do not keep a package-local
//! `strip_question` — that hits P3.526 (`starts_with` moves `t`).
//! P3.534 is WDB-423 (other agent).

use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn form_parse_must_strip_leading_question() {
    let tmp = TempDir::new().expect("tempdir");
    let src = tmp.path().join("main.wj");
    fs::write(
        &src,
        r#"
use std::encoding
use std::process

fn main() {
    let pairs = encoding.form_parse("?a=1")
    if pairs.len() != 1 {
        eprintln("expected 1 pair, got ${pairs.len()}")
        process.exit(1)
    }
    if pairs[0].0 != "a" {
        eprintln("expected key a, got ${pairs[0].0}")
        process.exit(1)
    }
    if pairs[0].1 != "1" {
        eprintln("expected value 1, got ${pairs[0].1}")
        process.exit(1)
    }
}
"#,
    )
    .unwrap();

    let out = tmp.path().join("build");
    let build = Command::new(env!("CARGO_BIN_EXE_wj"))
        .args(["build", src.to_str().unwrap(), "--output", out.to_str().unwrap()])
        .output()
        .expect("wj build");
    assert!(
        build.status.success(),
        "P3.535 transpile failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );

    let run = Command::new("cargo")
        .current_dir(&out)
        .args(["run", "--quiet"])
        .output()
        .expect("cargo run");
    assert!(
        run.status.success(),
        "P3.535 RED: form_parse(\"?a=1\") must yield key \"a\", not \"?a\".\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
}
